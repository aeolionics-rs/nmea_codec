//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Encapsulation is used to convey information when the data content is unknown or additional
//! bandwidth is needed. AIS messages, for example, are conveyed as binary data with NMEA sentences.
//!
//! The data content is broken into a sequence of encapsulation sentences, each of which contains
//! a single block of binary data, wrapped in ASCII armoring, with zero or more parametric fields.
//!
use crate::ais::AisMessage;
use crate::types::{AisChannel, MMSI};
use crate::{Message, NmeaCodec, Sentence, Talker};
use bitvec::field::BitField;
use bitvec::order::Msb0;
use bitvec::prelude::{BitSlice, BitVec};
use bitvec::slice::Chunks;
use bytes::{BufMut, Bytes, BytesMut};
use show_option::format_option;
use std::fmt::{Display, Formatter};
use tokio_util::codec::Encoder;

/// The encapsulation of a entire binary data message.
///
/// A binary message may require multiple sentences to be sent.
pub struct Encapsulation {
    /// The device sending the message.
    pub talker: Talker,
    /// An optional identifier for the sequence.
    pub sequence: Option<u8>,
    /// Sentence specific fields.
    pub fields: Fields,
    /// The binary data that was encapsulated.
    pub bits: BitVec<u8, Msb0>,
}

/// Parametric fields associated with this encapsulated message.
pub enum Fields {
    /// An AIS addressed binary message.
    ABM {
        /// The MMSI of the destination AIS unit.
        destination: MMSI,
        /// The AIS channel used.
        channel: Option<AisChannel>,
        /// The AIS message type (6, 12, or 25).
        message_id: u8,
    },
    /// An AIS broadcast binary message.
    BBM {
        /// The AIS channel used.
        channel: Option<AisChannel>,
        /// The AIS message type (8, 14, or 25).
        message_id: u8,
    },
    /// An AIS message received from another station.
    VDM {
        /// The AIS channel used.
        channel: Option<AisChannel>,
    },

    /// An AIS message sent by this station.
    VDO {
        /// The AIS channel used.
        channel: Option<AisChannel>,
    },

    /// An unknown sentence format.
    Unknown {
        /// The mnemonic code for the sentence format.
        mnemonic: String,
        /// Additional parametric fields sent with the message.
        fields: Vec<String>,
    },
}

impl Encapsulation {
    /// Returns an iterator over the individual encapsulated sentences for this message.
    pub fn messages(&self) -> impl Iterator<Item = Message> {
        let inner = self.bits.chunks(360);
        let (_, max) = inner.size_hint();
        let total = max.unwrap() as u8;
        EncapsulationIterator { outer: &self, total, number: 1, inner }
    }
}

impl Encoder<Encapsulation> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Encapsulation, dst: &mut BytesMut) -> Result<(), Self::Error> {
        for message in item.messages() {
            self.encode(message, dst)?;
        }
        Ok(())
    }
}

pub struct EncapsulationIterator<'a> {
    outer: &'a Encapsulation,
    total: u8,
    number: u8,
    inner: Chunks<'a, u8, Msb0>,
}

impl<'a> Iterator for EncapsulationIterator<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(chunk) = self.inner.next() {
            let data = chunk.to_bitvec();
            let talker = self.outer.talker.clone();
            let sequence = Sequence {
                total: self.total,
                number: self.number,
                id: self.outer.sequence.clone(),
            };
            self.number += 1;

            let sentence = match &self.outer.fields {
                Fields::ABM { destination, channel, message_id } => Sentence::ABM {
                    talker,
                    sequence,
                    destination: destination.clone(),
                    channel: channel.clone(),
                    message_id: message_id.clone(),
                    data,
                },
                Fields::BBM { channel, message_id } => Sentence::BBM {
                    talker,
                    sequence,
                    channel: channel.clone(),
                    message_id: message_id.clone(),
                    data,
                },
                Fields::VDM { channel } => Sentence::VDM(AisMessage {
                    talker,
                    sequence,
                    channel: channel.clone(),
                    data,
                }),
                Fields::VDO { channel } => Sentence::VDO(AisMessage {
                    talker,
                    sequence,
                    channel: channel.clone(),
                    data,
                }),
                Fields::Unknown { mnemonic, fields } => Sentence::Encapsulated {
                    talker,
                    mnemonic: mnemonic.clone(),
                    sequence,
                    fields: fields.clone(),
                    data,
                },
            };
            Some(Message { tag_block: None, sentence })
        } else {
            None
        }
    }
}

/// The position of this sentence in the group needed to represent the binary message.
#[derive(Clone)]
pub struct Sequence {
    /// The total number of sentences in this sequence.
    pub total: u8,

    /// The number of this sentence within the group.
    pub number: u8,

    /// An optional sequence identifier.
    ///
    /// May be omitted if the entire message can be sent in a single sentence.
    pub id: Option<u8>,
}

impl Display for Sequence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{},{}", self.total, self.number, format_option!(self.id, "{}", ""))
    }
}

/// Convert binary bits to ASCII-armored data.
pub fn into_armored(data: &BitSlice<u8, Msb0>) -> (Bytes, u8) {
    let mut result = BytesMut::with_capacity((data.len() + 5) / 6);
    let mut chunks = data.chunks_exact(6);
    while let Some(chunk) = chunks.next() {
        result.put_u8(armor(chunk.load_be()));
    }
    let remainder = chunks.remainder();
    let padding = if remainder.is_empty() {
        0u8
    } else {
        let padding = 6 - remainder.len();
        result.put_u8(armor(remainder.load_be::<u8>() << padding));
        padding as u8
    };
    (result.freeze(), padding)
}

fn armor(byte: u8) -> u8 {
    match byte {
        ..0b101000 => byte + 0b00110000,
        _ => byte + 0b00111000,
    }
}
