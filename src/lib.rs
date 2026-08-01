//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! A codec for NMEA 1083 sentences.
/// A [`Decoder`] and [`Encoder`] for NMEA 0183 messages.
///
/// [`Decoder`]: Decoder
/// [`Encoder`]: Encoder
use show_option::prelude::*;

use bitvec::bitvec;
use bitvec::prelude::{BitVec, Msb0};
use std::fmt::Write;
use std::io::ErrorKind;
use std::str::FromStr;

pub mod ais;
pub mod encapsulation;
pub mod types;

use ais::AisMessage;
use bytes::{BufMut, BytesMut};
use chrono::{DateTime, Utc};
use tokio_util::codec::{Decoder, Encoder};
use types::{CourseOverGround, MagneticVariation, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, SpeedOverGround, Talker};
use uom::si::angle::degree;
use uom::si::velocity::knot;

/// A [`Decoder`] and [`Encoder`] for NMEA 0183 messages.
///
/// [`Decoder`]: Decoder
/// [`Encoder`]: Encoder
pub struct NmeaCodec {
    next: usize,
}

impl NmeaCodec {
    /// Creates a new codec.
    pub fn new() -> Self {
        Self { next: 0 }
    }
}

/// A NMEA 0183 message.
///
/// **Message format:** \[tag_block\] \<sentence\> \r\n
#[derive(Clone)]
pub struct Message {
    pub tag_block: Option<TagBlock>,
    pub sentence: Sentence,
}

/// A block of tags containing additional metadata about the message.
#[derive(Clone)]
pub struct TagBlock {}

/// A NMEA 0183 sentence.
#[derive(Clone)]
pub enum Sentence {
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
    VDM(AisMessage),
    VDO(AisMessage),
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
        /// The raw fields in the sentence.
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
        /// The number of messages in this sequence.
        total: u8,
        /// The number of this message in the sequence.
        sequence: u8,
        /// An optional identifier for a sequence of messages.
        sequence_id: Option<u8>,
        /// Additional data fields sent with the message.
        fields: Vec<String>,
        /// The binary data that was encapsulated.
        bits: BitVec<u8, Msb0>,
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

impl Encoder<Message> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if let Some(tag_block) = item.tag_block {
            self.encode(tag_block, dst)?;
        }
        self.encode(item.sentence, dst)?;
        Ok(())
    }
}

impl Encoder<TagBlock> for NmeaCodec {
    type Error = std::io::Error;
    fn encode(&mut self, _tag_block: TagBlock, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Encoder<Sentence> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Sentence, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let start = dst.len();
        match item {
            Sentence::RMC {
                talker,
                time_of_fix,
                position,
                sog,
                cog,
                variation,
                status,
                mode,
                nav_status,
            } => write!(
                dst,
                "${talker}RMC,{time},{status},{lat},{long},{sog},{cog},{date},{var},{mode},{nav_status}",
                date = format_option!(time_of_fix.map(|time| { time.format("%d%m%y") }), "{}", ""),
                time = format_option!(time_of_fix.map(|time| { time.format("%H%M%S%.f") }), "{}", ""),
                lat = format_option!(position.as_ref().map(|v| &v.latitude), "{}", ","),
                long = format_option!(position.as_ref().map(|v| &v.longitude), "{}", ","),
                sog = format_option!(sog.as_ref().map(|v| v.0.get::<knot>()), "{}", ""),
                cog = format_option!(cog.as_ref().map(|v| v.0.get::<degree>()), "{:.0}", ""),
                var = format_option!(variation, "{}", ","),
            ),
            Sentence::VDO(msg) => msg.encode("VDO", dst),
            Sentence::VDM(msg) => msg.encode("VDM", dst),
            Sentence::Parametric { talker, mnemonic, .. } => write!(dst, "${}{}", talker, mnemonic),
            Sentence::Encapsulated { talker, mnemonic, .. } => write!(dst, "!{}{}", talker, mnemonic),
            Sentence::Query { talker, target, mnemonic } => write!(dst, "${talker}{target}Q,{mnemonic}"),
            Sentence::Proprietary { mnemonic, data } => write!(dst, "$P{mnemonic}{data}"),
        }
        .expect("Failed to encode sentence");
        put_checksum(start + 1, dst);
        dst.put_slice(b"\r\n");
        Ok(())
    }
}

fn put_checksum(start: usize, dst: &mut BytesMut) {
    let checksum = dst.as_ref()[start..].iter().fold(0u8, |sum, byte| sum ^ *byte);
    write!(dst, "*{checksum:02X}").expect("Failed to write checksum")
}

impl Decoder for NmeaCodec {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        while self.next < src.len() {
            if src[self.next] == b'\n' {
                let line = src.split_to(self.next + 1);
                self.next = 0;
                return parse(line);
            }
            self.next += 1;
        }
        Ok(None)
    }
}

fn hex(ch: u8) -> Result<u8, std::io::Error> {
    Ok(match ch {
        b'0'..=b'9' => ch - b'0',
        b'A'..=b'F' => ch - b'A' + 10,
        _ => return Err(std::io::Error::new(ErrorKind::InvalidData, "Invalid hex character")),
    })
}
/// Validates the line contains valid characters and a correct checksum, returning the content.
fn validate(buf: &[u8]) -> Result<(char, &str), std::io::Error> {
    if buf.is_empty() {
        return Err(std::io::Error::new(ErrorKind::InvalidData, "Empty line"));
    }
    let kind = buf[0] as char;

    let mut checksum: u8 = 0;
    let mut pos = 1;
    while pos < buf.len() - 3 {
        let ch = buf[pos];
        pos += 1;
        match ch {
            b'*' => {
                let their_checksum = hex(buf[pos])? << 4 | hex(buf[pos + 1])?;
                if checksum != their_checksum {
                    return Err(std::io::Error::new(ErrorKind::InvalidData, "Invalid checksum"));
                }
                // SAFETY: we checked the buffer contains valid ASCII
                return Ok((kind, unsafe { str::from_utf8_unchecked(&buf[1..pos - 1]) }));
            }
            0x20..=0x7f => checksum = checksum ^ ch,
            _ => return Err(std::io::Error::new(ErrorKind::InvalidData, "Invalid character")),
        }
    }
    Err(std::io::Error::new(ErrorKind::InvalidData, "No checksum found"))
}

fn parse(buf: BytesMut) -> Result<Option<Message>, std::io::Error> {
    let (kind, data) = validate(buf.as_ref())?;
    let tag_block = if kind == '\\' { todo!() } else { None };

    // Minimum is ttnnn
    if data.len() < 5 {
        return Err(std::io::Error::new(ErrorKind::InvalidData, "Sentence too short"));
    }
    let talker = Talker::try_from(&data[0..=1])?;
    let sentence = match kind {
        '$' => {
            // Proprietary format: Pnnn..
            if data.starts_with('P') {
                if data.len() < 4 {
                    return Err(std::io::Error::new(ErrorKind::InvalidData, "Proprietary sentence too short"));
                }
                let (mnemonic, data) = data[1..].split_at(3);
                Sentence::Proprietary {
                    mnemonic: mnemonic.to_string(),
                    data: data.to_string(),
                }
            } else if &data[4..=4] == "Q" {
                // Query format: ttddQ,nnn
                if data.len() != 9 {
                    return Err(std::io::Error::new(ErrorKind::InvalidData, "Invalid query"));
                }
                let target = Talker::try_from(&data[2..=3])?;
                let mnemonic = data[6..=8].to_string();
                Sentence::Query { talker, target, mnemonic }
            } else {
                let mut fields = data[2..].split(',');
                let mnemonic = fields.next().unwrap().to_string();
                let fields = fields.map(|field| field.to_string()).collect();

                match mnemonic {
                    _ => Sentence::Parametric {
                        talker,
                        mnemonic: mnemonic.to_string(),
                        fields,
                    },
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
                return Err(std::io::Error::new(ErrorKind::InvalidData, "Invalid encapsulation"));
            }
            let mnemonic = fields[0].to_string();
            let x1 = fields[1];
            let x2 = fields[2];
            let x3 = fields[3];
            let _armor = fields[fields.len() - 2]; // "the encapsulation data field shall always be the second to the last data field in the sentence"
            let x4 = fields[fields.len() - 1];

            // Extract the encapsulation header (x1, x2, x3)
            let total = u8::from_str(x1).map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "Invalid total"))?;
            let sequence = u8::from_str(x2).map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "Invalid sequence number"))?;
            let sequence_id = {
                if !x3.is_empty() {
                    Some(u8::from_str(x3).map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "Invalid fill bits"))?)
                } else {
                    None
                }
            };

            // Extract the armored data.
            let _fill_bits = u8::from_str(x4).map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "Invalid fill bits"))?;
            let bits = bitvec![u8, Msb0; 0, 1];

            let fields = fields[4..fields.len() - 2].iter().map(|field| field.to_string()).collect();
            match mnemonic {
                _ => Sentence::Encapsulated {
                    talker,
                    mnemonic,
                    total,
                    sequence,
                    sequence_id,
                    fields,
                    bits,
                },
            }
        }
        _ => {
            return Err(std::io::Error::new(ErrorKind::InvalidData, "Unknown sentence type"));
        }
    };
    Ok(Some(Message { tag_block, sentence }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoder() -> Result<(), std::io::Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::with_capacity(1024);
        codec.encode(
            Message {
                tag_block: None,
                sentence: Sentence::Parametric {
                    talker: Talker::GNSS,
                    mnemonic: "RMC".to_string(),
                    fields: vec![],
                },
            },
            &mut buf,
        )?;
        assert_eq!(buf.as_ref(), b"$GNRMC*55\r\n");
        Ok(())
    }

    #[test]
    fn decoder() -> Result<(), std::io::Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::with_capacity(1024);
        assert!(codec.decode(&mut buf)?.is_none());

        buf.put_slice(b"$GNxxx*71\r\n");
        buf.put_slice(b"!AIyyy,1,1,1,ccc,0*3F\r\n");
        buf.put_slice(b"$GPzzz");

        // Decode the GNSS sentence.
        let msg = codec.decode(&mut buf)?.unwrap();
        assert!(msg.tag_block.is_none());
        if let Sentence::Parametric { talker, mnemonic, fields } = msg.sentence {
            assert_eq!(talker, Talker::GNSS);
            assert_eq!(mnemonic, "xxx");
            assert!(fields.is_empty())
        } else {
            panic!("Unexpected sentence");
        }

        // Decode the VDM sentence.
        let msg = codec.decode(&mut buf)?.unwrap();
        assert!(msg.tag_block.is_none());
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
        assert!(codec.decode(&mut buf)?.is_some());
        Ok(())
    }

    #[test]
    fn proprietary() -> Result<(), std::io::Error> {
        let mut codec = NmeaCodec::new();
        let mut buf = BytesMut::from("$Pxxxabc*48\r\n");
        let msg = codec.decode(&mut buf)?.unwrap();
        if let Sentence::Proprietary { mnemonic, data } = msg.sentence {
            assert_eq!(mnemonic, "xxx");
            assert_eq!(data, "abc");
        } else {
            panic!("Unexpected sentence");
        }

        buf.clear();
        let msg = Message {
            tag_block: None,
            sentence: Sentence::Proprietary {
                mnemonic: "yyy".to_string(),
                data: "abcdef".to_string(),
            },
        };
        codec.encode(msg, &mut buf)?;
        assert_eq!(buf.as_ref(), b"$Pyyyabcdef*2E\r\n");
        Ok(())
    }
}
