//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! A codec for `NMEA 0183`/`IEC 61162-1` sentences,
//! with support for tag blocks and encapsulated messages such as sentences used for `AIS`.
//!
//! The crate defines native message for well-known formats.
//! Others are mapped to generic types enabling support for unrecognized messages.
//! Messages are modeled in a Rust-native form, using the [`uom`] crate to improve type safety.
//! Conversion to text is performed during encoding and decoding.
//! The implementation also supports `Proprietary` and `Query` sentences.
//!
//! Encodes and decodes NMEA sentences to [`BytesMut`] buffers using the Tokio codec framework.
//! The implementation does not require an async runtime but does require an allocator.
//!
//! The `ais` feature enables binding to type-safe messages; if this feature is not enabled then
//! AIS messages are mapped to generic sentences.
//!
//! # Encoding
//!
//! ## Simple parametric sentence
//! ```rust
//! use bytes::BytesMut;
//! use nmea_codec::types::{Latitude, Longitude, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, Talker};
//! use nmea_codec::{NmeaCodec, Sentence};
//! use tokio_util::codec::Encoder;
//! use uom::si::angle::degree;
//! use uom::si::f64::Angle;
//!
//! # fn main() -> Result<(), std::io::Error> {
//! let mut encoder = NmeaCodec::new();
//! let mut buf = BytesMut::with_capacity(1024);
//! let sentence = Sentence::RMC {
//!     talker: Talker::GNSS,
//!     status: PositioningSystemStatus::Valid,
//!     mode: PositioningSystemMode::Autonomous,
//!     nav_status: NavigationalStatus::NotValid,
//!     time_of_fix: None,
//!     position: Some(Position {
//!         latitude: Latitude(Angle::new::<degree>(47.61)),
//!         longitude: Longitude(Angle::new::<degree>(-122.4)),
//!     }),
//!     sog: None,
//!     cog: None,
//!     variation: None,
//! };
//! encoder.encode(sentence, &mut buf)?;
//! assert_eq!(&buf[..], b"$GNRMC,,A,4736.6000,N,12224.0000,W,,,,,,A,V*01\r\n");
//! # Ok(())
//! # }
//! ```
//! ## Encapsulated sentence
//! With the `ais` feature, an AIS message can be encapsulated in VDM sentences:
//! ```rust
//! # #[cfg(feature = "ais")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ais_message::Message::ScheduledPositionReport;
//! use ais_message::message::PositionReport;
//! use ais_message::types::{CommunicationState, MMSI, NavigationalStatus, RepeatCount};
//! use bytes::BytesMut;
//! use deku::DekuContainerWrite;
//! use nmea_codec::NmeaCodec;
//! use nmea_codec::encapsulation::{Encapsulation, Fields};
//! use nmea_codec::types::{AisChannel, Talker};
//! use tokio_util::codec::Encoder;
//!
//! let mut encoder = NmeaCodec::new();
//! let mut buf = BytesMut::with_capacity(1024);
//! let mut report = PositionReport::new(MMSI::new(127)?, CommunicationState { sync: 0, slot_time_out: 0, sub_message: 0 });
//! report.repeat = RepeatCount::Twice;
//! report.status = NavigationalStatus::UnderWay;
//! let encapsulation = Encapsulation {
//!     talker: Talker::AIS,
//!     sequence: None,
//!     fields: Fields::VDM { channel: Some(AisChannel::A) },
//!     bits: ScheduledPositionReport(report).to_bits()?,
//! };
//! encoder.encode(encapsulation, &mut buf)?;
//! assert!(buf.starts_with(b"!AIVDM,1,1,,A,"));
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "ais"))]
//! # fn main() {}
//! ```
//! If required, the encoder will split the encapsulated message into multiple sentences to fit
//! within the [`MAX_SENTENCE`] limit.

/// A [`Decoder`] and [`Encoder`] for NMEA 0183 messages.
///
/// [`Decoder`]: Decoder
/// [`Encoder`]: Encoder
use show_option::prelude::*;

use bitvec::prelude::{BitVec, Msb0};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::io::{Error, ErrorKind};
use std::str::FromStr;

pub mod encapsulation;
pub mod tag_block;
pub mod types;

#[cfg(feature = "ais")]
pub mod ais;
#[cfg(feature = "ais")]
use crate::encapsulation::into_armored;
#[cfg(feature = "ais")]
use crate::types::{AisChannel, MMSI, TransmitChannel};
#[cfg(feature = "ais")]
use ais::AisMessage;

use crate::encapsulation::{Encapsulation, Sequence};
use bytes::{BufMut, BytesMut};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use tag_block::TagBlock;
use tokio_util::codec::{Decoder, Encoder};
use tracing::{trace, warn};
use types::{
    CourseOverGround, MagneticVariation, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus,
    SpeedOverGround, Talker,
};
use uom::si::velocity::knot;

/// Maximum number of ASCII characters in a sentence, including the initial `$` or `!`
/// and the terminating `<CR><LF>`.
pub const MAX_SENTENCE: usize = 82;

/// Maximum number of ASCII characters in the data fields, excluding the delimiters and checksum.
pub const MAX_DATA: usize = MAX_SENTENCE - 1 - 3 - 2;

/// A [`Decoder`] and [`Encoder`] for NMEA 0183 messages.
///
/// # Decoding
/// Each line is decoded to a message, or to the error that made it invalid. A line with a bad
/// checksum or an unknown format does not end the stream, so a reader can log it and carry on.
///
/// Encapsulated messages that span multiple sentences are reassembled: the decoder buffers each
/// sentence until the last one in the group arrives, then returns a single message containing
/// all the data. Its sequence has a total of 1, and its fields are those of the first sentence.
/// A group with a missing or out-of-order sentence is discarded. A codec created with
/// [`NmeaCodec::without_reassembly`] returns each sentence of a group as it is decoded instead.
///
/// # Encoding
/// ## Encoding a simple parametric sentence.
/// ```rust
/// # use bytes::BytesMut;
/// # use nmea_codec::types::{Latitude, Longitude, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, Talker};
/// # use nmea_codec::{NmeaCodec, Sentence};
/// # use tokio_util::codec::Encoder;
/// # use uom::si::angle::degree;
/// # use uom::si::f64::Angle;
/// # fn main() -> Result<(), std::io::Error> {
/// let mut encoder = NmeaCodec::new();
/// let mut buf = BytesMut::new();
/// let sentence = Sentence::RMC {
///     talker: Talker::GNSS,
///     status: PositioningSystemStatus::Valid,
///     mode: PositioningSystemMode::Autonomous,
///     nav_status: NavigationalStatus::NotValid,
///     time_of_fix: None,
///     position: Some(Position {
///         latitude: Latitude(Angle::new::<degree>(47.61)),
///         longitude: Longitude(Angle::new::<degree>(-122.4)),
///     }),
///     sog: None,
///     cog: None,
///     variation: None,
/// };
/// encoder.encode(sentence, &mut buf)?;
/// assert_eq!(&buf[..], b"$GNRMC,,A,4736.6000,N,12224.0000,W,,,,,,A,V*01\r\n");
/// # Ok(())
/// # }
/// ```
/// ## Encoding a message with a tag block.
/// ```rust
/// # use bytes::BytesMut;
/// # use nmea_codec::types::{NavigationalStatus, PositioningSystemMode, PositioningSystemStatus, Talker};
/// # use nmea_codec::tag_block::{Tag, TagBlock};
/// # use nmea_codec::{Message, NmeaCodec, Sentence};
/// # use tokio_util::codec::Encoder;
/// # fn main() -> Result<(), std::io::Error> {
/// let mut encoder = NmeaCodec::new();
/// let mut buf = BytesMut::new();
/// let tag_blocks = vec![TagBlock { tags: vec![Tag::Source("GP0001".to_string())] }];
/// let sentence = Sentence::RMC {
///     talker: Talker::GNSS,
///     status: PositioningSystemStatus::Warning,
///     mode: PositioningSystemMode::NoFix,
///     nav_status: NavigationalStatus::NotValid,
///     time_of_fix: None,
///     position: None,
///     sog: None,
///     cog: None,
///     variation: None,
/// };
/// encoder.encode(Message { tag_blocks, sentence }, &mut buf)?;
/// assert_eq!(&buf[..], b"\\s:GP0001*5F\\$GNRMC,,V,,,,,,,,,,N,V*37\r\n");
/// # Ok(())
/// # }
/// ```
/// ## Encoding encapsulated data.
/// With the `ais` feature, an AIS message can be encapsulated in VDM sentences:
/// ```rust
/// # #[cfg(feature = "ais")]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use ais_message::Message::ScheduledPositionReport;
/// # use ais_message::message::PositionReport;
/// # use ais_message::types::{CommunicationState, MMSI, NavigationalStatus, RepeatCount};
/// # use bytes::BytesMut;
/// # use deku::DekuContainerWrite;
/// # use nmea_codec::NmeaCodec;
/// # use nmea_codec::encapsulation::{Encapsulation, Fields};
/// # use nmea_codec::types::{AisChannel, Talker};
/// # use tokio_util::codec::Encoder;
/// let mut encoder = NmeaCodec::new();
/// let mut buf = BytesMut::new();
/// let mut report = PositionReport::new(MMSI::new(127)?, CommunicationState { sync: 0, slot_time_out: 0, sub_message: 0 });
/// report.repeat = RepeatCount::Twice;
/// report.status = NavigationalStatus::UnderWay;
/// let encapsulation = Encapsulation {
///     talker: Talker::AIS,
///     sequence: None,
///     fields: Fields::VDM { channel: Some(AisChannel::A) },
///     bits: ScheduledPositionReport(report).to_bits()?,
/// };
/// encoder.encode(encapsulation, &mut buf)?;
/// assert!(buf.starts_with(b"!AIVDM,1,1,,A,"));
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "ais"))]
/// # fn main() {}
/// ```
///
/// [`Decoder`]: Decoder
/// [`Encoder`]: Encoder
pub struct NmeaCodec {
    next: usize,
    /// Whether multi-sentence messages are reassembled when decoding.
    reassembly: bool,
    /// The number of sentences decoded, used to age partially received messages.
    sentences: u64,
    /// Partially received multi-sentence messages, keyed by talker, mnemonic and sequence id, with
    /// the number of the sentence last added to them.
    fragments: HashMap<(Talker, String, Option<u8>), (u64, Message)>,
}

/// The number of sentences within which the next sentence of a multi-sentence message must arrive.
///
/// The sentences of a message are normally sent together, so if the next one is not among the
/// following sentences it has been lost. Discarding the incomplete message limits the memory used
/// and stops it being joined to a later message that reuses its sequence id.
const FRAGMENT_LIFETIME: u64 = 10;

impl Default for NmeaCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl NmeaCodec {
    /// Creates a new codec that reassembles multi-sentence messages when decoding.
    #[must_use]
    pub fn new() -> Self {
        Self { next: 0, reassembly: true, sentences: 0, fragments: HashMap::new() }
    }

    /// Creates a new codec that decodes each sentence of a multi-sentence message separately,
    /// such as to forward the sentences as they were received.
    #[must_use]
    pub fn without_reassembly() -> Self {
        Self { reassembly: false, ..Self::new() }
    }

    /// Adds a decoded message to any multi-sentence group it belongs to.
    ///
    /// Returns the message if it is complete, either because it was a single sentence or it was
    /// the last in a group, or if reassembly is disabled.
    fn reassemble(&mut self, mut message: Message) -> Option<Message> {
        if !self.reassembly {
            return Some(message);
        }
        self.sentences += 1;
        let now = self.sentences;
        self.fragments.retain(|key, (updated, _)| {
            let current = now - *updated <= FRAGMENT_LIFETIME;
            if !current {
                warn!("discarding incomplete {} message, next sentence not received", key.1);
            }
            current
        });

        let Some((talker, mnemonic, sequence, data)) = message.sentence.encapsulation_mut() else {
            return Some(message);
        };
        if sequence.total <= 1 {
            return Some(message);
        }
        let key = (talker, mnemonic.to_string(), sequence.id);
        if sequence.number == 1 {
            if self.fragments.contains_key(&key) {
                warn!("discarding incomplete {} message", key.1);
            }
            self.fragments.insert(key, (now, message));
            return None;
        }

        let Some((_, mut partial)) = self.fragments.remove(&key) else {
            warn!("discarding {} sentence {} of {}, first sentence missing", key.1, sequence.number, sequence.total);
            return None;
        };
        let (_, _, partial_sequence, partial_data) = partial.sentence.encapsulation_mut().unwrap();
        if sequence.total != partial_sequence.total || sequence.number != partial_sequence.number + 1 {
            warn!(
                "discarding incomplete {} message, expected sentence {} of {}",
                key.1,
                partial_sequence.number + 1,
                partial_sequence.total
            );
            return None;
        }
        partial_data.extend_from_bitslice(data);
        partial_sequence.number = sequence.number;
        if sequence.number < sequence.total {
            self.fragments.insert(key, (now, partial));
            return None;
        }
        *partial_sequence = Sequence { total: 1, number: 1, id: sequence.id };
        Some(partial)
    }
}

/// A NMEA 0183 message.
///
/// **Message format:** \[`tag_block` ...\] \<sentence\> \r\n
#[derive(Clone)]
pub struct Message {
    pub tag_blocks: Vec<TagBlock>,
    pub sentence: Sentence,
}

/// Well-known NMEA 0183 sentences.
#[derive(Clone, Debug)]
pub enum Sentence {
    // Well-known parametric sentences.
    /// Recommended minimum specific GNSS data.
    ///
    /// Time, date, position, course and speed provided by a GNSS navigation receiver.
    RMC {
        /// The device sending the message.
        talker: Talker,
        /// Status of the positioning system.
        status: PositioningSystemStatus,
        /// Mode the position system is operating in.
        mode: PositioningSystemMode,
        /// Navigation system status.
        nav_status: NavigationalStatus,
        /// UTC time of position fix.
        time_of_fix: Option<DateTime<Utc>>,
        /// Position fix.
        position: Option<Position>,
        /// Speed over ground.
        sog: Option<SpeedOverGround>,
        /// True course over ground.
        cog: Option<CourseOverGround>,
        /// Magnetic variation at position.
        ///
        /// East is positive and subtracts from True course.
        /// West is negative and adds to True course.
        variation: Option<MagneticVariation>,
    },

    // Encapsulated formats.
    /// AIS Addressed Binary Message
    ///
    /// Typically sent to an AIS unit to initiate transmission of a AIS addressed binary message
    /// (types 6, 12 and 25).
    #[cfg(feature = "ais")]
    ABM {
        /// The device sending the message.
        talker: Talker,
        /// Sequence information.
        sequence: Sequence,
        /// The MMSI of the destination AIS unit.
        destination: Option<MMSI>,
        /// The AIS channel to transmit on: 0 for no preference, 1 for A, 2 for B or 3 for both.
        channel: Option<TransmitChannel>,
        /// The AIS message type (6, 12, or 25).
        message_id: Option<u8>,
        /// The binary message chunk.
        data: BitVec<u8, Msb0>,
    },
    /// AIS Broadcast Binary Message
    ///
    /// Typically sent to an AIS unit to initiate transmission of a AIS broadcast binary message
    /// (types 8, 14 and 25).
    #[cfg(feature = "ais")]
    BBM {
        /// The device sending the message.
        talker: Talker,
        /// Sequence information.
        sequence: Sequence,
        /// The AIS channel to transmit on: 0 for no preference, 1 for A, 2 for B or 3 for both.
        channel: Option<TransmitChannel>,
        /// The AIS message type (8, 14, or 25).
        message_id: Option<u8>,
        /// The binary message chunk.
        data: BitVec<u8, Msb0>,
    },
    /// An AIS message received by this AIS unit.
    #[cfg(feature = "ais")]
    VDM(AisMessage),
    /// An AIS message transmitted by this AIS unit.
    #[cfg(feature = "ais")]
    VDO(AisMessage),

    // Generic, catch-all variants.
    /// A general Parametric Sentence.
    ///
    /// This variant can be used to send arbitrary data. It is produced by the [`Decoder`] when
    /// the formatter mnemonic code is not recognized.
    ///
    /// [`Decoder`]: Decoder
    Parametric {
        /// The device sending the message.
        talker: Talker,
        /// The mnemonic code for the sentence formatter.
        mnemonic: String,
        /// The fields in the sentence.
        ///
        /// Reserved and non-printable characters are sent in their `^hh` hex form, and so may be
        /// used in a field. Any character must be in ISO/IEC 8859-1.
        fields: Vec<String>,
    },
    /// A general purpose Sentence with encapsulated data.
    ///
    /// This variant can be used to send arbitrary binary messages. It is produced by the
    /// [`Decoder`] when the formatter mnemonic code is not recognized.
    ///
    /// [`Decoder`]: Decoder
    Encapsulated {
        /// The device sending the message.
        talker: Talker,
        /// The mnemonic code for the sentence formatter.
        mnemonic: String,
        /// Sequence information.
        sequence: Sequence,
        /// Additional data fields sent with the message, which are escaped like those of
        /// [`Sentence::Parametric`].
        fields: Vec<String>,
        /// The binary data that was encapsulated.
        data: BitVec<u8, Msb0>,
    },
    /// A query sentence requesting transmission of an approved sentence.
    Query {
        /// The device that sent the query.
        talker: Talker,
        /// The device from which data is being requested.
        target: Talker,
        /// The approved sentence formatter being requested.
        mnemonic: String,
    },
    /// A variant for handling undocumented, proprietary sentences.
    Proprietary {
        /// The manufacturer's mnemonic code.
        mnemonic: String,
        /// The manufacturer's data.
        data: String,
    },
}

impl Sentence {
    /// Returns the talker, mnemonic, sequence and data of an encapsulated sentence.
    fn encapsulation_mut(&mut self) -> Option<(Talker, &str, &mut Sequence, &mut BitVec<u8, Msb0>)> {
        match self {
            #[cfg(feature = "ais")]
            Sentence::ABM { talker, sequence, data, .. } => Some((*talker, "ABM", sequence, data)),
            #[cfg(feature = "ais")]
            Sentence::BBM { talker, sequence, data, .. } => Some((*talker, "BBM", sequence, data)),
            #[cfg(feature = "ais")]
            Sentence::VDM(AisMessage { talker, sequence, data, .. }) => Some((*talker, "VDM", sequence, data)),
            #[cfg(feature = "ais")]
            Sentence::VDO(AisMessage { talker, sequence, data, .. }) => Some((*talker, "VDO", sequence, data)),
            Sentence::Encapsulated { talker, mnemonic, sequence, data, .. } => {
                Some((*talker, mnemonic.as_str(), sequence, data))
            }
            _ => None,
        }
    }
}

impl Encoder<Message> for NmeaCodec {
    type Error = Error;

    /// Encodes a message, writing nothing if it cannot be encoded.
    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let start = dst.len();
        item.tag_blocks
            .iter()
            .try_for_each(|tag_block| tag_block.encode(dst))
            .and_then(|()| self.encode(item.sentence, dst))
            .inspect_err(|_| dst.truncate(start))
    }
}

impl Encoder<Sentence> for NmeaCodec {
    type Error = Error;

    /// Encodes a sentence.
    ///
    /// A complete encapsulated message too long for one sentence, such as one reassembled by the
    /// decoder, is split into a group of sentences. Nothing is written if the sentence cannot be
    /// encoded, such as when a text field contains a character outside ISO/IEC 8859-1.
    fn encode(&mut self, item: Sentence, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let start = dst.len();
        put_sentence(&item, dst).inspect_err(|_| dst.truncate(start))?;
        if dst.len() - start > MAX_SENTENCE
            && let Some(encapsulation) = Encapsulation::from_complete(item)
        {
            dst.truncate(start);
            for message in encapsulation.messages() {
                // The fields were written successfully above, so these cannot fail.
                put_sentence(&message.sentence, dst)?;
            }
        }
        Ok(())
    }
}

/// Writes a sentence, with its checksum and line ending.
fn put_sentence(item: &Sentence, dst: &mut BytesMut) -> Result<(), Error> {
    let start = dst.len();
    match item {
        Sentence::RMC { talker, time_of_fix, position, sog, cog, variation, status, mode, nav_status } => write!(
            dst,
            "${talker}RMC,{time},{status},{lat},{long},{sog},{cog},{date},{var},{mode},{nav_status}",
            date = format_option!(time_of_fix.map(|time| { time.format("%d%m%y") }), "{}", ""),
            time = format_option!(time_of_fix.map(|time| { time.format("%H%M%S%.f") }), "{}", ""),
            lat = format_option!(position.as_ref().map(|v| &v.latitude), "{}", ","),
            long = format_option!(position.as_ref().map(|v| &v.longitude), "{}", ","),
            sog = format_option!(sog.as_ref().map(|v| v.0.get::<knot>()), "{:.2}", ""),
            cog = format_option!(cog, "{}", ""),
            var = format_option!(variation, "{}", ","),
        ),
        #[cfg(feature = "ais")]
        Sentence::ABM { talker, sequence, destination, channel, message_id, data } => {
            let (armored, padding) = into_armored(data.as_ref());
            // SAFETY: the result only contains ASCII
            let armor_text = str::from_utf8(armored.as_ref()).unwrap();
            write!(
                dst,
                "!{talker}ABM,{sequence},{destination},{channel},{message_id},{armor_text},{padding}",
                destination = format_option!(destination, "{}", ""),
                message_id = format_option!(message_id, "{}", ""),
                channel = format_option!(channel, "{}", ""),
            )
        }
        #[cfg(feature = "ais")]
        Sentence::BBM { talker, sequence, channel, message_id, data } => {
            let (armored, padding) = into_armored(data.as_ref());
            // SAFETY: the result only contains ASCII
            let armor_text = str::from_utf8(armored.as_ref()).unwrap();
            write!(
                dst,
                "!{talker}BBM,{sequence},{channel},{message_id},{armor_text},{padding}",
                message_id = format_option!(message_id, "{}", ""),
                channel = format_option!(channel, "{}", ""),
            )
        }
        #[cfg(feature = "ais")]
        Sentence::VDO(msg) => msg.encode("VDO", dst),
        #[cfg(feature = "ais")]
        Sentence::VDM(msg) => msg.encode("VDM", dst),
        Sentence::Parametric { talker, mnemonic, fields } => {
            write!(dst, "${talker}{mnemonic}").unwrap();
            for field in fields {
                dst.put_u8(b',');
                put_escaped(field, dst)?;
            }
            Ok(())
        }
        Sentence::Encapsulated { talker, mnemonic, sequence, fields, data: bits } => {
            let (armored, padding) = encapsulation::into_armored(bits.as_bitslice());
            write!(dst, "!{talker}{mnemonic},{sequence}").unwrap();
            for field in fields {
                dst.put_u8(b',');
                put_escaped(field, dst)?;
            }
            write!(dst, ",{armored},{padding}", armored = str::from_utf8(armored.as_ref()).unwrap()).unwrap();
            Ok(())
        }
        Sentence::Query { talker, target, mnemonic } => write!(dst, "${talker}{target}Q,{mnemonic}"),
        Sentence::Proprietary { mnemonic, data } => write!(dst, "$P{mnemonic}{data}"),
    }
    .unwrap();
    put_checksum(start + 1, dst);
    dst.put_slice(b"\r\n");
    Ok(())
}

fn put_checksum(start: usize, dst: &mut BytesMut) {
    let checksum = dst.as_ref()[start..].iter().fold(0u8, |sum, byte| sum ^ *byte);
    write!(dst, "*{checksum:02X}").expect("Failed to write checksum");
}

/// Writes text to a field, sending reserved and non-printable characters in the `^hh` hex form
/// of their ISO/IEC 8859-1 code.
fn put_escaped(text: &str, dst: &mut BytesMut) -> Result<(), Error> {
    for ch in text.chars() {
        let code =
            u8::try_from(ch).map_err(|_| Error::new(ErrorKind::InvalidInput, "Character not in ISO/IEC 8859-1"))?;
        if (0x20..0x7f).contains(&code) && !b"!$*,\\^~".contains(&code) {
            dst.put_u8(code);
        } else {
            write!(dst, "^{code:02X}").unwrap();
        }
    }
    Ok(())
}

/// Replaces the `^hh` hex forms in a field with the ISO/IEC 8859-1 characters they represent.
fn unescape(field: &str) -> Result<Cow<'_, str>, Error> {
    if !field.contains('^') {
        return Ok(Cow::Borrowed(field));
    }
    let mut text = String::with_capacity(field.len());
    let mut bytes = field.bytes();
    while let Some(ch) = bytes.next() {
        if ch == b'^' {
            let (Some(high), Some(low)) = (bytes.next(), bytes.next()) else {
                return Err(Error::new(ErrorKind::InvalidData, "Incomplete hex character"));
            };
            text.push(char::from(hex(high)? << 4 | hex(low)?));
        } else {
            text.push(char::from(ch));
        }
    }
    Ok(Cow::Owned(text))
}

impl Decoder for NmeaCodec {
    /// A decoded message, or the error that made a line invalid.
    ///
    /// Invalid lines are returned as items rather than as decoder errors, which would end a
    /// [`FramedRead`](tokio_util::codec::FramedRead) stream.
    type Item = Result<Message, Error>;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        while self.next < src.len() {
            if src[self.next] == b'\n' {
                let line = src.split_to(self.next + 1);
                trace!("decoding: {}", line.trim_ascii().escape_ascii());
                self.next = 0;
                match parse(line) {
                    Ok(Some(message)) => {
                        if let Some(message) = self.reassemble(message) {
                            return Ok(Some(Ok(message)));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => return Ok(Some(Err(e))),
                }
            } else {
                self.next += 1;
            }
        }
        Ok(None)
    }
}

fn hex(ch: u8) -> Result<u8, Error> {
    Ok(match ch {
        b'0'..=b'9' => ch - b'0',
        b'A'..=b'F' => ch - b'A' + 10,
        _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid hex character")),
    })
}
/// Validates the line contains valid characters and a correct checksum, returning the content.
fn validate(buf: &[u8]) -> Result<(char, &str), Error> {
    if buf.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, "Empty line"));
    }
    let kind = buf[0] as char;

    let mut checksum: u8 = 0;
    let mut pos = 1;
    while pos + 3 < buf.len() {
        let ch = buf[pos];
        pos += 1;
        match ch {
            b'*' => {
                let their_checksum = hex(buf[pos])? << 4 | hex(buf[pos + 1])?;
                if checksum != their_checksum {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid checksum"));
                }
                // SAFETY: we checked the buffer contains valid ASCII
                return Ok((kind, unsafe { str::from_utf8_unchecked(&buf[1..pos - 1]) }));
            }
            0x20..=0x7f => checksum ^= ch,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid character")),
        }
    }
    Err(Error::new(ErrorKind::InvalidData, "No checksum found"))
}

#[expect(clippy::too_many_lines, reason = "one parse, each kind of sentence in turn")]
fn parse(mut buf: BytesMut) -> Result<Option<Message>, Error> {
    let mut tag_blocks = vec![];
    while buf.first() == Some(&b'\\') {
        let len = buf[1..]
            .iter()
            .position(|&ch| ch == b'\\')
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Unterminated tag block"))?;
        tag_blocks.push(TagBlock::decode(&buf.split_to(len + 2))?);
    }
    let (kind, data) = validate(buf.as_ref())?;

    // Minimum is ttnnn
    if data.len() < 5 {
        return Err(Error::new(ErrorKind::InvalidData, "Sentence too short"));
    }
    let talker = Talker::try_from(&data[0..=1])?;
    let sentence = match kind {
        '$' => {
            // Proprietary format: Pnnn..
            if let Some(data) = data.strip_prefix('P') {
                if data.len() < 3 {
                    return Err(Error::new(ErrorKind::InvalidData, "Proprietary sentence too short"));
                }
                let (mnemonic, data) = data.split_at(3);
                Sentence::Proprietary { mnemonic: mnemonic.to_string(), data: data.to_string() }
            } else if &data[4..=4] == "Q" {
                // Query format: ttddQ,nnn
                if data.len() != 9 {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid query"));
                }
                let target = Talker::try_from(&data[2..=3])?;
                let mnemonic = data[6..=8].to_string();
                Sentence::Query { talker, target, mnemonic }
            } else {
                let mut fields = data[2..].split(',');
                let mnemonic = fields.next().unwrap();

                if mnemonic == "RMC" {
                    parse_rmc(talker, &fields.collect::<Vec<_>>())?
                } else {
                    let fields = fields.map(|field| unescape(field).map(Cow::into_owned)).collect::<Result<_, _>>()?;
                    Sentence::Parametric { talker, mnemonic: mnemonic.to_string(), fields }
                }
            }
        }
        '!' => {
            // Format: mnemonic,x1,x2,x3,c--c,x4
            // Where:
            // * x1: total number of sentences
            // * x2: sentence number
            // * x3: sequence id
            // * x4: fill bits
            // and c--c is the armored data preceded by optional application fields
            let fields = data[2..].split(',').collect::<Vec<&str>>();
            if fields.len() < 6 {
                return Err(Error::new(ErrorKind::InvalidData, "Invalid encapsulation"));
            }
            let mnemonic = fields[0].to_string();
            let x1 = fields[1];
            let x2 = fields[2];
            let x3 = fields[3];
            // The armored data is always the field before the fill bits, which come last.
            let armor = fields[fields.len() - 2];
            let x4 = fields[fields.len() - 1];

            // Extract the encapsulation header (x1, x2, x3)
            let total = u8::from_str(x1).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid total"))?;
            let number = u8::from_str(x2).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid sequence number"))?;
            let id = {
                if x3.is_empty() {
                    None
                } else {
                    Some(u8::from_str(x3).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid fill bits"))?)
                }
            };
            let sequence = Sequence { total, number, id };

            // Extract the armored data.
            let fill_bits = u8::from_str(x4).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid fill bits"))?;
            let bits = encapsulation::from_armored(armor, fill_bits)?;

            let fields = &fields[4..fields.len() - 2];
            match mnemonic.as_str() {
                #[cfg(feature = "ais")]
                "VDM" | "VDO" => {
                    // Format: VDM,x1,x2,x3,a,c--c,x4 where a is the AIS channel (same for VDO)
                    let [channel] = fields else {
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid VDM/VDO"));
                    };
                    let channel = parse_optional(channel, AisChannel::try_from)?;
                    let msg = AisMessage { talker, sequence, channel, data: bits };
                    if mnemonic == "VDM" { Sentence::VDM(msg) } else { Sentence::VDO(msg) }
                }
                #[cfg(feature = "ais")]
                "ABM" => {
                    // Format: ABM,x1,x2,x3,xxxxxxxxx,x,x.x,c--c,x4
                    // where the fields are the destination MMSI, AIS channel and message ID
                    let [destination, channel, message_id] = fields else {
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid ABM"));
                    };
                    Sentence::ABM {
                        talker,
                        sequence,
                        destination: parse_optional(destination, parse_mmsi)?,
                        channel: parse_optional(channel, TransmitChannel::try_from)?,
                        message_id: parse_optional(message_id, parse_message_id)?,
                        data: bits,
                    }
                }
                #[cfg(feature = "ais")]
                "BBM" => {
                    // Format: BBM,x1,x2,x3,x,x.x,c--c,x4 where the fields are the AIS channel and message ID
                    let [channel, message_id] = fields else {
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid BBM"));
                    };
                    Sentence::BBM {
                        talker,
                        sequence,
                        channel: parse_optional(channel, TransmitChannel::try_from)?,
                        message_id: parse_optional(message_id, parse_message_id)?,
                        data: bits,
                    }
                }
                _ => Sentence::Encapsulated {
                    talker,
                    mnemonic,
                    sequence,
                    fields: fields
                        .iter()
                        .map(|field| unescape(field).map(Cow::into_owned))
                        .collect::<Result<_, _>>()?,
                    data: bits,
                },
            }
        }
        _ => {
            return Err(Error::new(ErrorKind::InvalidData, "Unknown sentence type"));
        }
    };
    Ok(Some(Message { tag_blocks, sentence }))
}

/// Parses a field that may be empty, returning `None` if it is.
#[cfg(feature = "ais")]
fn parse_optional<'a, T>(field: &'a str, parse: impl FnOnce(&'a str) -> Result<T, Error>) -> Result<Option<T>, Error> {
    if field.is_empty() { Ok(None) } else { parse(field).map(Some) }
}

#[cfg(feature = "ais")]
fn parse_mmsi(field: &str) -> Result<MMSI, Error> {
    u32::from_str(field).map(MMSI).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid MMSI"))
}

#[cfg(feature = "ais")]
fn parse_message_id(field: &str) -> Result<u8, Error> {
    u8::from_str(field).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid message ID"))
}

fn parse_rmc(talker: Talker, fields: &[&str]) -> Result<Sentence, Error> {
    if fields.len() < 12 {
        return Err(Error::new(ErrorKind::InvalidData, "Insufficient fields in RMC"));
    }
    let time = fields[0];
    let status = fields[1].try_into()?;
    let latitude = fields[2];
    let latitude_ord = fields[3];
    let longitude = fields[4];
    let longitude_ord = fields[5];
    let sog = SpeedOverGround::parse_str(fields[6])?;
    let cog = CourseOverGround::parse_str(fields[7])?;
    let date = fields[8];
    let variation = fields[9];
    let variation_ord = fields[10];
    let mode = fields[11].try_into()?;
    // If NavigationalStatus is missing default to NotValid
    let nav_status = if fields.len() > 12 { fields[12].try_into()? } else { NavigationalStatus::NotValid };

    let time_of_fix = if date.is_empty() || time.is_empty() {
        None
    } else {
        let date = NaiveDate::parse_from_str(date, "%d%m%y")
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid date"))?;
        let time = NaiveTime::parse_from_str(time, "%H%M%S%.f")
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid time"))?;
        Some(date.and_time(time).and_utc())
    };
    let position = Position::parse_str(latitude, latitude_ord, longitude, longitude_ord)?;
    let variation = MagneticVariation::parse_str(variation, variation_ord)?;

    Ok(Sentence::RMC { talker, status, mode, nav_status, time_of_fix, position, sog, cog, variation })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoder() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::with_capacity(1024);
        codec.encode(
            Message {
                tag_blocks: vec![],
                sentence: Sentence::Parametric { talker: Talker::GNSS, mnemonic: "RMC".to_string(), fields: vec![] },
            },
            &mut buf,
        )?;
        assert_eq!(buf.as_ref(), b"$GNRMC*55\r\n");
        Ok(())
    }

    #[test]
    fn rmc_course() -> Result<(), Error> {
        use uom::si::angle::degree;
        use uom::si::f64::Angle;

        let mut buf = BytesMut::new();
        let sentence = Sentence::RMC {
            talker: Talker::GNSS,
            time_of_fix: None,
            position: None,
            sog: None,
            cog: Some(CourseOverGround(Angle::new::<degree>(273.6))),
            variation: None,
            status: PositioningSystemStatus::Valid,
            mode: PositioningSystemMode::Autonomous,
            nav_status: NavigationalStatus::NotValid,
        };
        NmeaCodec::new().encode(sentence, &mut buf)?;
        assert!(buf.starts_with(b"$GNRMC,,A,,,,,,273.6,"));
        Ok(())
    }

    #[test]
    fn latitude_longitude() {
        use crate::types::{Latitude, Longitude};
        use uom::si::angle::degree;
        use uom::si::f64::Angle;

        let latitude = |degrees| Latitude(Angle::new::<degree>(degrees)).to_string();
        let longitude = |degrees| Longitude(Angle::new::<degree>(degrees)).to_string();
        assert_eq!((latitude(47.61), longitude(-122.4)), ("4736.6000,N".to_string(), "12224.0000,W".to_string()));
        // Degrees and minutes are padded with zeros.
        assert_eq!(
            (latitude(-(5.0 + 5.0 / 60.0)), longitude(27.0 + 5.0 / 60.0)),
            ("0505.0000,S".to_string(), "02705.0000,E".to_string())
        );
        // Minutes that round up carry into the degrees.
        assert_eq!(
            (latitude(47.999_999_99), longitude(-179.999_999_99)),
            ("4800.0000,N".to_string(), "18000.0000,W".to_string())
        );
        // A position that rounds to zero is north and east.
        assert_eq!((latitude(-1e-9), longitude(-1e-9)), ("0000.0000,N".to_string(), "00000.0000,E".to_string()));

        // What is written is read back.
        let position = Position::parse_str("0505.0000", "S", "02705.0000", "E").unwrap().unwrap();
        assert_eq!(
            (position.latitude.to_string(), position.longitude.to_string()),
            ("0505.0000,S".to_string(), "02705.0000,E".to_string())
        );
    }

    #[test]
    fn course_over_ground() {
        use uom::si::angle::degree;
        use uom::si::f64::Angle;

        let cog = |degrees| CourseOverGround(Angle::new::<degree>(degrees)).to_string();
        assert_eq!(cog(273.6), "273.6");
        assert_eq!(cog(5.0), "5.0");
        // A course that rounds up to 360 is 0, and negative courses are from 0 to 360.
        assert_eq!(cog(359.96), "0.0");
        assert_eq!(cog(-10.0), "350.0");
    }

    #[test]
    fn magnetic_variation() {
        use uom::si::angle::degree;
        use uom::si::f64::Angle;

        assert_eq!(MagneticVariation(Angle::new::<degree>(-15.0)).to_string(), "15.0,W");
        assert_eq!(MagneticVariation(Angle::new::<degree>(2.34)).to_string(), "2.3,E");

        // A variation that rounds to zero is not west.
        assert_eq!(MagneticVariation(Angle::new::<degree>(-0.04)).to_string(), "0.0,E");
        assert_eq!(MagneticVariation(Angle::new::<degree>(-0.05)).to_string(), "0.1,W");

        let parse = |variation, ord| MagneticVariation::parse_str(variation, ord).map(|v| v.map(|v| v.to_string()));
        assert_eq!(parse("15.0", "W").unwrap().as_deref(), Some("15.0,W"));
        assert_eq!(parse("2.3", "E").unwrap().as_deref(), Some("2.3,E"));
        assert_eq!(parse("", "").unwrap(), None);
        assert!(parse("15.0", "N").is_err());
    }

    #[test]
    fn decoder_tag_blocks() -> Result<(), Error> {
        use tag_block::Tag;

        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::with_capacity(1024);
        buf.put_slice(b"\\s:AI0001,c:1767225600000*01\\!AIyyy,1,1,1,ccc,0*3F\r\n");
        buf.put_slice(b"\\s:AI0001*40\\\\c:1767225600000*6D\\!AIyyy,1,1,1,ccc,0*3F\r\n");
        buf.put_slice(b"\\s:AI0001*00\\!AIyyy,1,1,1,ccc,0*3F\r\n");
        buf.put_slice(b"\\s:unterminated*00!AIyyy,1,1,1,ccc,0*3F\r\n");
        buf.put_slice(b"$GNxxx*71\r\n");

        // The tag block is decoded along with the sentence after it.
        let source = Tag::Source("AI0001".to_string());
        let time = Tag::Time("2026-01-01T00:00:00Z".parse().unwrap());
        let msg = codec.decode(&mut buf)?.unwrap()?;
        assert_eq!(msg.tag_blocks, vec![TagBlock { tags: vec![source.clone(), time.clone()] }]);
        assert!(matches!(msg.sentence, Sentence::Encapsulated { talker: Talker::AIS, .. }));

        // Multiple tag blocks are decoded in order.
        let msg = codec.decode(&mut buf)?.unwrap()?;
        assert_eq!(msg.tag_blocks, vec![TagBlock { tags: vec![source] }, TagBlock { tags: vec![time] }]);
        assert!(matches!(msg.sentence, Sentence::Encapsulated { talker: Talker::AIS, .. }));

        // A tag block with a bad checksum or without a closing '\\' is invalid, but the next
        // sentence can be decoded.
        assert!(decodes_invalid(&mut codec, &mut buf));
        assert!(decodes_invalid(&mut codec, &mut buf));
        let msg = codec.decode(&mut buf)?.unwrap()?;
        assert!(matches!(msg.sentence, Sentence::Parametric { talker: Talker::GNSS, .. }));
        Ok(())
    }

    #[test]
    fn encoder_tag_blocks() -> Result<(), Error> {
        use tag_block::{Group, Tag};

        let tag_blocks = vec![
            TagBlock { tags: vec![Tag::Source("AI0001".to_string())] },
            TagBlock { tags: vec![Tag::Group(Group { number: 2, total: 3, id: 1234 })] },
        ];
        let sentence = Sentence::Parametric { talker: Talker::GNSS, mnemonic: "xxx".to_string(), fields: vec![] };
        let lines = encode_lines(&mut NmeaCodec::new(), Message { tag_blocks: tag_blocks.clone(), sentence })?;
        assert_eq!(lines, vec!["\\s:AI0001*40\\\\g:2-3-1234*58\\$GNxxx*71\r\n"]);

        let msg = NmeaCodec::new().decode(&mut BytesMut::from(lines[0].as_str()))?.unwrap()?;
        assert_eq!(msg.tag_blocks, tag_blocks);
        Ok(())
    }

    #[test]
    fn escaped_fields() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let fields = vec!["a,b".to_string(), "^".to_string(), "é".to_string()];
        let sentence =
            Sentence::Parametric { talker: Talker::GNSS, mnemonic: "xxx".to_string(), fields: fields.clone() };
        let lines = encode_lines(&mut codec, Message { tag_blocks: vec![], sentence })?;
        assert_eq!(lines, vec!["$GNxxx,a^2Cb,^5E,^E9*7D\r\n"]);

        let mut buf = BytesMut::from(lines[0].as_str());
        buf.put_slice(b"!AIyyy,1,1,1,a^2Cb,ccc,0*3F\r\n");
        buf.put_slice(b"$GNxxx,a^2*50\r\n");
        let Sentence::Parametric { fields: decoded, .. } = codec.decode(&mut buf)?.unwrap()?.sentence else {
            panic!("Unexpected sentence");
        };
        assert_eq!(decoded, fields);
        let Sentence::Encapsulated { fields: decoded, .. } = codec.decode(&mut buf)?.unwrap()?.sentence else {
            panic!("Unexpected sentence");
        };
        assert_eq!(decoded, vec!["a,b"]);
        // An incomplete hex form is invalid.
        assert!(decodes_invalid(&mut codec, &mut buf));

        // A character outside ISO/IEC 8859-1 cannot be sent, and nothing is written.
        let tag_blocks = vec![TagBlock { tags: vec![tag_block::Tag::Source("AI0001".to_string())] }];
        let sentence =
            Sentence::Parametric { talker: Talker::GNSS, mnemonic: "xxx".to_string(), fields: vec!["€".to_string()] };
        let mut buf = BytesMut::from("$GNxxx*71\r\n");
        assert!(codec.encode(Message { tag_blocks, sentence }, &mut buf).is_err());
        assert_eq!(&buf[..], b"$GNxxx*71\r\n");
        Ok(())
    }

    #[test]
    fn decoder() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::with_capacity(1024);
        assert!(codec.decode(&mut buf)?.is_none());

        buf.put_slice(b"$GNxxx*71\r\n");
        buf.put_slice(b"!AIyyy,1,1,1,ccc,0*3F\r\n");
        buf.put_slice(b"$GPzzz");

        // Decode the GNSS sentence.
        let msg = codec.decode(&mut buf)?.unwrap()?;
        assert_eq!(msg.tag_blocks, [] as [tag_block::TagBlock; 0]);
        if let Sentence::Parametric { talker, mnemonic, fields } = msg.sentence {
            assert_eq!(talker, Talker::GNSS);
            assert_eq!(mnemonic, "xxx");
            assert_eq!(fields, [] as [std::string::String; 0]);
        } else {
            panic!("Unexpected sentence");
        }

        // Decode the VDM sentence.
        let msg = codec.decode(&mut buf)?.unwrap()?;
        assert_eq!(msg.tag_blocks, [] as [tag_block::TagBlock; 0]);
        if let Sentence::Encapsulated { talker, mnemonic, .. } = msg.sentence {
            assert_eq!(talker, Talker::AIS);
            assert_eq!(mnemonic, "yyy");
        } else {
            panic!("Unexpected sentence");
        }

        // Partial sentence should return None and the buffer should contain the remainder.
        assert!(codec.decode(&mut buf)?.is_none());
        assert_eq!(buf.as_ref(), b"$GPzzz");

        // .. until \n is seen
        buf.put_slice(b"*6D");
        assert!(codec.decode(&mut buf)?.is_none());
        buf.put_slice(b"\r\n");
        assert!(matches!(codec.decode(&mut buf)?, Some(Ok(_))));
        Ok(())
    }

    /// Without AIS support, VDM sentences are decoded as generic encapsulated sentences.
    #[cfg(not(feature = "ais"))]
    #[test]
    fn dearmor() -> Result<(), Error> {
        use bitvec::field::BitField;

        // A type 21 AtoN report from a StationKeeper shore station set up for testing.
        let mut buf = BytesMut::from("!AIVDM,1,1,,A,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*53\r\n");
        let msg = NmeaCodec::new().decode(&mut buf)?.unwrap()?;
        let Sentence::Encapsulated { talker, mnemonic, sequence, fields, data } = msg.sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!(talker, Talker::AIS);
        assert_eq!(mnemonic, "VDM");
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, None));
        assert_eq!(fields, vec!["A"]);
        // 46 armored characters less 4 fill bits.
        assert_eq!(data.len(), 272);
        assert_eq!(data[..6].load_be::<u8>(), 21);
        Ok(())
    }

    #[test]
    fn dearmor_invalid() {
        let mut buf = BytesMut::from("!AIVDM,1,1,,A,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,6*51\r\n");
        assert!(decodes_invalid(&mut NmeaCodec::new(), &mut buf));
        let mut buf = BytesMut::from("!AIVDM,1,1,,A,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT34000000xP10,4*19\r\n");
        assert!(decodes_invalid(&mut NmeaCodec::new(), &mut buf));
    }

    #[cfg(feature = "ais")]
    #[test]
    fn vdm() -> Result<(), Error> {
        use deku::DekuContainerRead;

        // A type 21 AtoN report from a StationKeeper shore station set up for testing.
        let mut buf = BytesMut::from("!AIVDM,1,1,,A,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*53\r\n");
        let msg = NmeaCodec::new().decode(&mut buf)?.unwrap()?;
        let Sentence::VDM(AisMessage { talker, sequence, channel, data }) = msg.sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!(talker, Talker::AIS);
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, None));
        assert_eq!(channel, Some(AisChannel::A));
        // 46 armored characters less 4 fill bits.
        assert_eq!(data.len(), 272);

        let (_, ais_msg) = ais_message::Message::from_bytes((data.as_raw_slice(), 0)).unwrap();
        let ais_message::Message::AtonReport(report) = ais_msg else { panic!("Unexpected message {ais_msg:?}") };
        assert_eq!(u32::from(report.source), 888_888_111);
        assert_eq!(report.name.to_string(), "STATIONKEEPER");
        Ok(())
    }

    #[cfg(feature = "ais")]
    #[test]
    fn vdo() -> Result<(), Error> {
        let mut buf = BytesMut::from("!AIVDO,1,1,,B,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*52\r\n");
        let msg = NmeaCodec::new().decode(&mut buf)?.unwrap()?;
        let Sentence::VDO(AisMessage { talker, sequence, channel, data }) = msg.sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!(talker, Talker::AIS);
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, None));
        assert_eq!(channel, Some(AisChannel::B));
        assert_eq!(data.len(), 272);
        Ok(())
    }

    /// Encodes a group of sentences and decodes them again, checking they produce a single message.
    #[cfg(feature = "ais")]
    fn round_trip(sentences: impl IntoIterator<Item = Sentence>) -> Result<(String, Sentence), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::new();
        for sentence in sentences {
            codec.encode(sentence, &mut buf)?;
        }
        let text = String::from_utf8(buf.to_vec()).unwrap();
        let msg = codec.decode(&mut buf)?.unwrap()?;
        assert!(buf.is_empty());
        assert!(codec.decode(&mut buf)?.is_none());
        Ok((text, msg.sentence))
    }

    #[cfg(feature = "ais")]
    #[test]
    fn abm() -> Result<(), Error> {
        use bitvec::bitvec;

        let data = bitvec![u8, Msb0; 1, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 1];
        let (text, sentence) = round_trip([Sentence::ABM {
            talker: Talker::AIS,
            sequence: Sequence { total: 1, number: 1, id: None },
            destination: Some(MMSI(338_087_471)),
            // The channel to transmit on is numbered, with 1 for A.
            channel: Some(TransmitChannel::A),
            message_id: Some(6),
            data: data.clone(),
        }])?;
        assert!(text.starts_with("!AIABM,1,1,,338087471,1,6,"), "{text}");
        let Sentence::ABM { talker, sequence, destination, channel, message_id, data: decoded } = sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!(talker, Talker::AIS);
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, None));
        assert_eq!(destination.map(|mmsi| mmsi.0), Some(338_087_471));
        assert_eq!(channel, Some(TransmitChannel::A));
        assert_eq!(message_id, Some(6));
        assert_eq!(decoded, data);

        // Subsequent sentences in a group omit the destination, channel and message ID.
        let (text, sentence) = round_trip([
            Sentence::ABM {
                talker: Talker::AIS,
                sequence: Sequence { total: 2, number: 1, id: Some(3) },
                destination: Some(MMSI(338_087_471)),
                channel: Some(TransmitChannel::A),
                message_id: Some(6),
                data: data.clone(),
            },
            Sentence::ABM {
                talker: Talker::AIS,
                sequence: Sequence { total: 2, number: 2, id: Some(3) },
                destination: None,
                channel: None,
                message_id: None,
                data: data.clone(),
            },
        ])?;
        assert!(text.starts_with("!AIABM,2,1,3,338087471,1,6,"), "{text}");
        assert!(text.contains("\r\n!AIABM,2,2,3,,,,"), "{text}");
        let Sentence::ABM { sequence, destination, channel, message_id, data: decoded, .. } = sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, Some(3)));
        assert_eq!(destination.map(|mmsi| mmsi.0), Some(338_087_471));
        assert_eq!((channel, message_id), (Some(TransmitChannel::A), Some(6)));
        assert_eq!(decoded, data.iter().chain(data.iter()).collect::<BitVec<u8, Msb0>>());
        Ok(())
    }

    #[cfg(feature = "ais")]
    #[test]
    fn bbm() -> Result<(), Error> {
        use bitvec::bitvec;

        let data = bitvec![u8, Msb0; 1, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 1];
        let (text, sentence) = round_trip([Sentence::BBM {
            talker: Talker::AIS,
            sequence: Sequence { total: 1, number: 1, id: None },
            // The channel to transmit on is numbered, with 2 for B.
            channel: Some(TransmitChannel::B),
            message_id: Some(8),
            data: data.clone(),
        }])?;
        assert!(text.starts_with("!AIBBM,1,1,,2,8,"), "{text}");
        let Sentence::BBM { talker, sequence, channel, message_id, data: decoded } = sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!(talker, Talker::AIS);
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, None));
        assert_eq!(channel, Some(TransmitChannel::B));
        assert_eq!(message_id, Some(8));
        assert_eq!(decoded, data);

        // Subsequent sentences in a group omit the channel and message ID.
        let (text, sentence) = round_trip([
            Sentence::BBM {
                talker: Talker::AIS,
                sequence: Sequence { total: 2, number: 1, id: Some(0) },
                channel: Some(TransmitChannel::B),
                message_id: Some(8),
                data: data.clone(),
            },
            Sentence::BBM {
                talker: Talker::AIS,
                sequence: Sequence { total: 2, number: 2, id: Some(0) },
                channel: None,
                message_id: None,
                data: data.clone(),
            },
        ])?;
        assert!(text.contains("\r\n!AIBBM,2,2,0,,,"), "{text}");
        let Sentence::BBM { sequence, channel, message_id, data: decoded, .. } = sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, Some(0)));
        assert_eq!((channel, message_id), (Some(TransmitChannel::B), Some(8)));
        assert_eq!(decoded, data.iter().chain(data.iter()).collect::<BitVec<u8, Msb0>>());
        Ok(())
    }

    #[test]
    fn transmit_channel() {
        use crate::types::TransmitChannel::{self, *};

        for (text, channel) in [("0", NoPreference), ("1", A), ("2", B), ("3", Both)] {
            assert_eq!(TransmitChannel::try_from(text).unwrap(), channel);
            assert_eq!(channel.to_string(), text);
        }
        assert!(TransmitChannel::try_from("A").is_err());
    }

    #[cfg(feature = "ais")]
    #[test]
    fn abm_bbm_invalid() {
        for content in [
            // ABM missing the message ID field.
            "AIABM,1,1,,338087471,1,wK,4",
            // ABM with an invalid MMSI.
            "AIABM,1,1,,33808747x,1,6,wK,4",
            // ABM with a channel letter rather than number.
            "AIABM,1,1,,338087471,A,6,wK,4",
            // BBM with an extra field.
            "AIBBM,1,1,,2,8,x,wK,4",
            // BBM with an invalid message ID.
            "AIBBM,1,1,,2,256,wK,4",
            // BBM with a channel out of range.
            "AIBBM,1,1,,4,8,wK,4",
        ] {
            let mut buf = BytesMut::from(sentence(content).as_str());
            assert!(decodes_invalid(&mut NmeaCodec::new(), &mut buf), "{content}");
        }
    }

    #[cfg(feature = "ais")]
    #[test]
    fn vdm_round_trip_no_channel() -> Result<(), Error> {
        use bitvec::bitvec;

        let data = bitvec![u8, Msb0; 1, 0, 1, 1, 0, 0];
        let sequence = Sequence { total: 1, number: 1, id: None };
        let (text, sentence) = round_trip([Sentence::VDM(AisMessage {
            talker: Talker::AIS,
            sequence,
            channel: None,
            data: data.clone(),
        })])?;
        assert!(text.starts_with("!AIVDM,1,1,,,d,0*"), "{text}");
        let Sentence::VDM(AisMessage { channel, data: decoded, .. }) = sentence else { panic!("Unexpected sentence") };
        assert_eq!(channel, None);
        assert_eq!(decoded, data);
        Ok(())
    }

    #[cfg(feature = "ais")]
    #[test]
    fn vdm_no_channel() -> Result<(), Error> {
        let mut buf = BytesMut::from("!AIVDM,1,1,,,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*12\r\n");
        let msg = NmeaCodec::new().decode(&mut buf)?.unwrap()?;
        let Sentence::VDM(AisMessage { channel, .. }) = msg.sentence else { panic!("Unexpected sentence") };
        assert_eq!(channel, None);
        Ok(())
    }

    #[cfg(feature = "ais")]
    #[test]
    fn vdm_invalid() {
        // Channel is not a single character.
        let mut buf = BytesMut::from("!AIVDM,1,1,,AB,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*11\r\n");
        assert!(decodes_invalid(&mut NmeaCodec::new(), &mut buf));
        // Too many fields.
        let mut buf = BytesMut::from("!AIVDM,1,1,,A,x,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*07\r\n");
        assert!(decodes_invalid(&mut NmeaCodec::new(), &mut buf));
    }

    // A type 5 message in two sentences, from gpsd's "AIVDM/AIVDO protocol decoding"
    // (www/AIVDM.adoc).
    const PART_1: &str = "!AIVDM,2,1,3,B,55P5TL01VIaAL@7WKO@mBplU@<PDhh000000001S;AJ::4A80?4i@E53,0*3E\r\n";
    const PART_2: &str = "!AIVDM,2,2,3,B,1@0000000000000,2*55\r\n";
    // A type 21 AtoN report from a StationKeeper shore station set up for testing.
    const SINGLE: &str = "!AIVDM,1,1,,A,E=?eK;hab0b4WW5RR`2a0000000C@OBPvT340000002P10,4*53\r\n";

    /// Formats an encapsulated sentence, adding the checksum.
    fn sentence(content: &str) -> String {
        let checksum = content.bytes().fold(0u8, |sum, byte| sum ^ byte);
        format!("!{content}*{checksum:02X}\r\n")
    }

    /// Decodes all complete messages in the buffer, returning their sequence and data length.
    /// A decoded message's total sentences, sequence and data length.
    type Decoded = (u8, u8, Option<u8>, usize);

    fn decode_all(codec: &mut NmeaCodec, buf: &mut BytesMut) -> Result<Vec<Decoded>, Error> {
        let mut result = Vec::new();
        while let Some(msg) = codec.decode(buf)? {
            let mut msg = msg?;
            let (_, _, sequence, data) = msg.sentence.encapsulation_mut().unwrap();
            result.push((sequence.total, sequence.number, sequence.id, data.len()));
        }
        Ok(result)
    }

    #[test]
    fn framed_read_continues_after_invalid_line() {
        use futures::StreamExt;
        use tokio_util::codec::FramedRead;

        // A sentence with a bad checksum between two valid ones.
        let input = format!("{SINGLE}!AIVDM,1,1,,A,E=?,0*00\r\n{SINGLE}");
        let items: Vec<_> = futures::executor::block_on(FramedRead::new(input.as_bytes(), NmeaCodec::new()).collect());
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], Ok(Ok(_))));
        assert!(matches!(&items[1], Ok(Err(e)) if e.to_string() == "Invalid checksum"));
        assert!(matches!(items[2], Ok(Ok(_))));
    }

    /// Whether the next line is decoded as invalid.
    fn decodes_invalid(codec: &mut NmeaCodec, buf: &mut BytesMut) -> bool {
        matches!(codec.decode(buf), Ok(Some(Err(_))))
    }

    #[test]
    fn reassemble() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::from(PART_1);
        // Nothing is returned until the last sentence arrives.
        assert!(codec.decode(&mut buf)?.is_none());
        assert!(buf.is_empty());
        buf.put_slice(PART_2.as_bytes());
        buf.put_slice(SINGLE.as_bytes());
        // 56 + 15 armored characters less 2 fill bits, then the following single sentence.
        assert_eq!(decode_all(&mut codec, &mut buf)?, vec![(1, 1, Some(3), 424), (1, 1, None, 272)]);
        assert!(codec.fragments.is_empty());
        Ok(())
    }

    #[test]
    fn reassemble_interleaved() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::new();
        buf.put_slice(sentence("AIVDM,2,1,1,A,wwww,0").as_bytes());
        buf.put_slice(sentence("AIVDM,3,1,2,A,ww,0").as_bytes());
        buf.put_slice(sentence("AIVDM,3,2,2,A,ww,0").as_bytes());
        buf.put_slice(sentence("AIVDM,2,2,1,A,w,2").as_bytes());
        buf.put_slice(sentence("AIVDM,3,3,2,A,w,0").as_bytes());
        assert_eq!(decode_all(&mut codec, &mut buf)?, vec![(1, 1, Some(1), 28), (1, 1, Some(2), 30)]);
        Ok(())
    }

    #[test]
    fn reassemble_discards_incomplete() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::new();
        // Missing the first sentence.
        buf.put_slice(PART_2.as_bytes());
        // Missing the middle sentence.
        buf.put_slice(sentence("AIVDM,3,1,4,A,ww,0").as_bytes());
        buf.put_slice(sentence("AIVDM,3,3,4,A,ww,0").as_bytes());
        // Restarted group: only the second attempt is returned.
        buf.put_slice(PART_1.as_bytes());
        buf.put_slice(PART_1.as_bytes());
        buf.put_slice(PART_2.as_bytes());
        // Total changes part way through.
        buf.put_slice(sentence("AIVDM,2,1,5,A,ww,0").as_bytes());
        buf.put_slice(sentence("AIVDM,3,2,5,A,ww,0").as_bytes());
        buf.put_slice(SINGLE.as_bytes());
        assert_eq!(decode_all(&mut codec, &mut buf)?, vec![(1, 1, Some(3), 424), (1, 1, None, 272)]);
        assert!(codec.fragments.is_empty());
        Ok(())
    }

    #[test]
    fn reassemble_expires_incomplete() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::new();
        // The next sentence arrives within the lifetime, after 9 others.
        buf.put_slice(sentence("AIVDM,2,1,3,A,ww,0").as_bytes());
        buf.put_slice(SINGLE.repeat(9).as_bytes());
        buf.put_slice(sentence("AIVDM,2,2,3,A,w,0").as_bytes());
        let decoded = decode_all(&mut codec, &mut buf)?;
        assert_eq!(decoded.last(), Some(&(1, 1, Some(3), 18)));

        // The second sentence of a group is lost, then a group reusing its sequence id loses its
        // first sentence: the remaining sentences are not joined.
        buf.put_slice(sentence("AIVDM,2,1,3,A,ww,0").as_bytes());
        buf.put_slice(SINGLE.repeat(10).as_bytes());
        buf.put_slice(sentence("AIVDM,2,2,3,A,w,0").as_bytes());
        let decoded = decode_all(&mut codec, &mut buf)?;
        assert_eq!(decoded, vec![(1, 1, None, 272); 10]);
        assert!(codec.fragments.is_empty());
        Ok(())
    }

    #[test]
    fn reassemble_limits_incomplete() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::new();
        // First sentences that are never completed, with different mnemonics.
        for n in 0..100 {
            buf.put_slice(sentence(&format!("AIX{n:02},2,1,1,A,ww,0")).as_bytes());
        }
        assert_eq!(decode_all(&mut codec, &mut buf)?, [] as [(u8, u8, std::option::Option<u8>, usize); 0]);
        assert_eq!(codec.fragments.len(), usize::try_from(FRAGMENT_LIFETIME).unwrap() + 1);
        Ok(())
    }

    /// Encodes a message and checks each sentence fits, returning the sentences.
    fn encode_lines(codec: &mut NmeaCodec, message: Message) -> Result<Vec<String>, Error> {
        let mut buf = BytesMut::new();
        codec.encode(message, &mut buf)?;
        let lines: Vec<String> =
            String::from_utf8(buf.to_vec()).unwrap().split_inclusive("\r\n").map(String::from).collect();
        for line in &lines {
            assert!(line.len() <= MAX_SENTENCE, "{line}");
        }
        Ok(lines)
    }

    #[test]
    fn reassembled_encodes_as_group() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::from(format!("{PART_1}{PART_2}").as_str());
        let mut message = codec.decode(&mut buf)?.unwrap()?;
        let (_, _, _, data) = message.sentence.encapsulation_mut().unwrap();
        let data = data.clone();

        // The 424 bits do not fit in one sentence, so they are sent as a group with the same id.
        let lines = encode_lines(&mut codec, message)?;
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("!AIVDM,2,1,3,B,") && lines[1].starts_with("!AIVDM,2,2,3,,"), "{lines:?}");

        // Decoding the group gives the same message.
        let mut buf = BytesMut::from(lines.concat().as_str());
        let mut decoded = codec.decode(&mut buf)?.unwrap()?;
        let (_, _, sequence, decoded_data) = decoded.sentence.encapsulation_mut().unwrap();
        assert_eq!((sequence.total, sequence.id), (1, Some(3)));
        assert_eq!(*decoded_data, data);
        Ok(())
    }

    #[test]
    fn long_encapsulated_encodes_as_group() -> Result<(), Error> {
        let sentence = Sentence::Encapsulated {
            talker: Talker::AIS,
            mnemonic: "XYZ".to_string(),
            sequence: Sequence { total: 1, number: 1, id: Some(5) },
            fields: vec!["f1".to_string()],
            data: BitVec::repeat(true, 600),
        };
        let lines = encode_lines(&mut NmeaCodec::new(), Message { tag_blocks: vec![], sentence })?;
        assert_eq!(lines.len(), 2);
        // The field is left empty in the second sentence.
        assert!(lines[0].starts_with("!AIXYZ,2,1,5,f1,") && lines[1].starts_with("!AIXYZ,2,2,5,,"), "{lines:?}");
        Ok(())
    }

    #[test]
    fn without_reassembly() -> Result<(), Error> {
        let mut codec = NmeaCodec::without_reassembly();
        let mut buf = BytesMut::from(format!("{PART_1}{PART_2}").as_str());
        // Each sentence is returned with its own sequence number, and nothing is kept.
        assert_eq!(decode_all(&mut codec, &mut buf)?, vec![(2, 1, Some(3), 336), (2, 2, Some(3), 88)]);
        assert!(codec.fragments.is_empty());

        // The sentences are encoded as they were received.
        let mut buf = BytesMut::from(format!("{PART_1}{PART_2}").as_str());
        let mut encoded = BytesMut::new();
        while let Some(message) = codec.decode(&mut buf)? {
            codec.encode(message?, &mut encoded)?;
        }
        assert_eq!(encoded.as_ref(), format!("{PART_1}{PART_2}").as_bytes());
        Ok(())
    }

    #[test]
    fn reassemble_keeps_first_fields() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::new();
        buf.put_slice(sentence("AIXYZ,2,1,7,f1,f2,ww,0").as_bytes());
        buf.put_slice(sentence("AIXYZ,2,2,7,,,w,0").as_bytes());
        let msg = codec.decode(&mut buf)?.unwrap()?;
        let Sentence::Encapsulated { mnemonic, sequence, fields, data, .. } = msg.sentence else {
            panic!("Unexpected sentence")
        };
        assert_eq!(mnemonic, "XYZ");
        assert_eq!((sequence.total, sequence.number, sequence.id), (1, 1, Some(7)));
        assert_eq!(fields, vec!["f1", "f2"]);
        assert_eq!(data.len(), 18);
        Ok(())
    }

    #[cfg(feature = "ais")]
    #[test]
    fn reassemble_vdm() -> Result<(), Error> {
        use deku::DekuContainerRead;

        let mut buf = BytesMut::from(format!("{PART_1}{PART_2}").as_str());
        let msg = NmeaCodec::new().decode(&mut buf)?.unwrap()?;
        let Sentence::VDM(AisMessage { channel, data, .. }) = msg.sentence else { panic!("Unexpected sentence") };
        assert_eq!(channel, Some(AisChannel::B));
        let (_, ais_msg) = ais_message::Message::from_bytes((data.as_raw_slice(), 0)).unwrap();
        let ais_message::Message::StaticData(report) = ais_msg else { panic!("Unexpected message {ais_msg:?}") };
        assert_eq!(u32::from(report.source), 369_190_000);
        assert_eq!(report.call_sign.to_string(), "WDA9674");
        assert_eq!(report.name.to_string(), "MT.MITCHELL");
        assert_eq!(report.destination.to_string(), "SEATTLE");
        Ok(())
    }

    #[test]
    fn proprietary() -> Result<(), Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::from("$Pxxxabc*48\r\n");
        let msg = codec.decode(&mut buf)?.unwrap()?;
        if let Sentence::Proprietary { mnemonic, data } = msg.sentence {
            assert_eq!(mnemonic, "xxx");
            assert_eq!(data, "abc");
        } else {
            panic!("Unexpected sentence");
        }

        buf.clear();
        let msg = Message {
            tag_blocks: vec![],
            sentence: Sentence::Proprietary { mnemonic: "yyy".to_string(), data: "abcdef".to_string() },
        };
        codec.encode(msg, &mut buf)?;
        assert_eq!(buf.as_ref(), b"$Pyyyabcdef*2E\r\n");
        Ok(())
    }
}
