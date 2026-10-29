//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Support for encapsulated sentences.
//!
//! An encapsulated sentence carries binary data, such as an AIS message, armored as printable
//! characters of six bits each. Data too long for one sentence is split across a numbered group of
//! them, and any other fields a sentence has come before its data.
//!
#[cfg(feature = "ais")]
use crate::ais::AisMessage;
#[cfg(feature = "ais")]
use crate::types::{AisChannel, MMSI, TransmitChannel};
use crate::{MAX_DATA, Message, NmeaCodec, Sentence, Talker};
use bitvec::field::BitField;
use bitvec::order::Msb0;
use bitvec::prelude::{BitSlice, BitVec};
use bitvec::slice::Chunks;
use bytes::{BufMut, Bytes, BytesMut};
use show_option::format_option;
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind};
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

// ttccc,x,x,x,..,x
const ENCAPSULATION_OVERHEAD: usize = 5 + 2 + 2 + 2 + 1 + 2;
const MAX_CHUNK: usize = MAX_DATA - ENCAPSULATION_OVERHEAD;

/// Parametric fields associated with this encapsulated message.
pub enum Fields {
    /// An AIS addressed binary message.
    #[cfg(feature = "ais")]
    ABM {
        /// The MMSI of the destination AIS unit.
        destination: MMSI,
        /// The AIS channel to transmit on: 0 for no preference, 1 for A, 2 for B or 3 for both.
        channel: Option<TransmitChannel>,
        /// The AIS message type (6, 12, or 25).
        message_id: u8,
    },
    /// An AIS broadcast binary message.
    #[cfg(feature = "ais")]
    BBM {
        /// The AIS channel to transmit on: 0 for no preference, 1 for A, 2 for B or 3 for both.
        channel: Option<TransmitChannel>,
        /// The AIS message type (8, 14, or 25).
        message_id: u8,
    },
    /// An AIS message received from another station.
    #[cfg(feature = "ais")]
    VDM {
        /// The AIS channel used.
        channel: Option<AisChannel>,
    },

    /// An AIS message sent by this station.
    #[cfg(feature = "ais")]
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
    /// The message in an encapsulated sentence that holds all of it, such as one reassembled by
    /// the decoder, so it can be split into a group of sentences.
    ///
    /// Returns `None` if the sentence is not encapsulated, is part of a group, or is missing the
    /// fields of the first sentence in a group.
    pub(crate) fn from_complete(sentence: Sentence) -> Option<Self> {
        let (talker, sequence, fields, bits) = match sentence {
            #[cfg(feature = "ais")]
            Sentence::ABM {
                talker,
                sequence,
                destination: Some(destination),
                channel,
                message_id: Some(message_id),
                data,
            } => (talker, sequence, Fields::ABM { destination, channel, message_id }, data),
            #[cfg(feature = "ais")]
            Sentence::BBM { talker, sequence, channel, message_id: Some(message_id), data } => {
                (talker, sequence, Fields::BBM { channel, message_id }, data)
            }
            #[cfg(feature = "ais")]
            Sentence::VDM(AisMessage { talker, sequence, channel, data }) => {
                (talker, sequence, Fields::VDM { channel }, data)
            }
            #[cfg(feature = "ais")]
            Sentence::VDO(AisMessage { talker, sequence, channel, data }) => {
                (talker, sequence, Fields::VDO { channel }, data)
            }
            Sentence::Encapsulated { talker, mnemonic, sequence, fields, data } => {
                (talker, sequence, Fields::Unknown { mnemonic, fields }, data)
            }
            _ => return None,
        };
        (sequence.total == 1).then_some(Self { talker, sequence: sequence.id, fields, bits })
    }

    /// Returns an iterator over the individual encapsulated sentences for this message.
    pub fn messages(&self) -> impl Iterator<Item = Message> {
        // Calculate the number of characters needed for the parametric fields on the first and
        // subsequence sentences.
        let (first, others) = match &self.fields {
            #[cfg(feature = "ais")]
            Fields::ABM { .. } => (14, 3),
            #[cfg(feature = "ais")]
            Fields::BBM { .. } => (4, 2),
            #[cfg(feature = "ais")]
            Fields::VDM { channel, .. } => (channel.map_or(1, |_| 2), 1),
            #[cfg(feature = "ais")]
            Fields::VDO { channel, .. } => (channel.map_or(1, |_| 2), 1),
            Fields::Unknown { fields, .. } => {
                let count = fields.len();
                let field_data = fields.iter().fold(0, |acc, f| acc + f.len());
                (count + field_data, count)
            }
        };
        let first_size = (MAX_CHUNK - first) * 6;
        let chunk_size = (MAX_CHUNK - others) * 6;
        let (first, chunks, total) = if self.bits.len() <= first_size {
            // Everything will fit in a single sentence.
            (self.bits.as_bitslice(), None, 1)
        } else {
            let (first, rest) = self.bits.split_at(first_size);
            let chunks = rest.chunks(chunk_size);
            // Never more sentences than a u8 counts: the message would be too long to send.
            let total = u8::try_from(1 + chunks.len()).unwrap_or(u8::MAX);
            (first, Some(chunks), total)
        };
        EncapsulationIterator { outer: self, number: 1, first, chunks, total }
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
    first: &'a BitSlice<u8, Msb0>,
    chunks: Option<Chunks<'a, u8, Msb0>>,
}

impl Iterator for EncapsulationIterator<'_> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        let number = self.number;
        let total = self.total;
        if number > total {
            return None;
        }
        self.number = number + 1;

        let talker = self.outer.talker;
        let sequence = Sequence { total, number, id: self.outer.sequence };

        let sentence = if number == 1 {
            // For the first sentence, use the first bits and include the fields.
            let data = self.first.to_bitvec();
            match &self.outer.fields {
                #[cfg(feature = "ais")]
                Fields::ABM { destination, channel, message_id } => Sentence::ABM {
                    talker,
                    sequence,
                    destination: Some(*destination),
                    channel: *channel,
                    message_id: Some(*message_id),
                    data,
                },

                #[cfg(feature = "ais")]
                Fields::BBM { channel, message_id } => {
                    Sentence::BBM { talker, sequence, channel: *channel, message_id: Some(*message_id), data }
                }
                #[cfg(feature = "ais")]
                Fields::VDM { channel } => Sentence::VDM(AisMessage { talker, sequence, channel: *channel, data }),
                #[cfg(feature = "ais")]
                Fields::VDO { channel } => Sentence::VDO(AisMessage { talker, sequence, channel: *channel, data }),
                Fields::Unknown { mnemonic, fields } => Sentence::Encapsulated {
                    talker,
                    mnemonic: mnemonic.clone(),
                    sequence,
                    fields: fields.clone(),
                    data,
                },
            }
        } else {
            // For other sentences, get the next chunk of bits and omit the fields.
            let data = self.chunks.as_mut()?.next()?.to_bitvec();
            match &self.outer.fields {
                #[cfg(feature = "ais")]
                Fields::ABM { .. } => {
                    Sentence::ABM { talker, sequence, destination: None, channel: None, message_id: None, data }
                }
                #[cfg(feature = "ais")]
                Fields::BBM { .. } => Sentence::BBM { talker, sequence, channel: None, message_id: None, data },
                #[cfg(feature = "ais")]
                Fields::VDM { .. } => Sentence::VDM(AisMessage { talker, sequence, channel: None, data }),
                #[cfg(feature = "ais")]
                Fields::VDO { .. } => Sentence::VDO(AisMessage { talker, sequence, channel: None, data }),
                // The fields are left empty rather than omitted, so the data stays in its position.
                Fields::Unknown { mnemonic, fields } => Sentence::Encapsulated {
                    talker,
                    mnemonic: mnemonic.clone(),
                    sequence,
                    fields: vec![String::new(); fields.len()],
                    data,
                },
            }
        };
        Some(Message { tag_blocks: vec![], sentence })
    }
}

/// The position of this sentence in the group needed to represent the binary message.
#[derive(Clone, Debug)]
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
#[must_use]
pub fn into_armored(data: &BitSlice<u8, Msb0>) -> (Bytes, u8) {
    let mut result = BytesMut::with_capacity(data.len().div_ceil(6));
    let mut chunks = data.chunks_exact(6);
    for chunk in chunks.by_ref() {
        result.put_u8(armor(chunk.load_be()));
    }
    let remainder = chunks.remainder();
    let padding = if remainder.is_empty() {
        0u8
    } else {
        let padding = 6 - remainder.len();
        result.put_u8(armor(remainder.load_be::<u8>() << padding));
        #[expect(clippy::cast_possible_truncation, reason = "under six bits of padding")]
        let padding = padding as u8;
        padding
    };
    (result.freeze(), padding)
}

fn armor(byte: u8) -> u8 {
    match byte {
        ..0b0010_1000 => byte + 0b0011_0000,
        _ => byte + 0b0011_1000,
    }
}

/// Convert ASCII-armored data to binary bits.
///
/// `fill_bits` is the number of padding bits that were added to the last character; these are
/// discarded.
///
/// # Errors
///
/// An [`ErrorKind::InvalidData`] error for more than five fill bits, or a character that isn't
/// armored six-bit data.
pub fn from_armored(armored: &str, fill_bits: u8) -> Result<BitVec<u8, Msb0>, Error> {
    let fill_bits = fill_bits as usize;
    if fill_bits > 5 || fill_bits > armored.len() * 6 {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid fill bits"));
    }
    let mut bits = BitVec::with_capacity(armored.len() * 6);
    for ch in armored.bytes() {
        let value = dearmor(ch).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid armored character"))?;
        for i in (0..6).rev() {
            bits.push(value >> i & 1 == 1);
        }
    }
    bits.truncate(bits.len() - fill_bits);
    Ok(bits)
}

fn dearmor(ch: u8) -> Option<u8> {
    match ch {
        0x30..=0x57 => Some(ch - 0b0011_0000),
        0x60..=0x77 => Some(ch - 0b0011_1000),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::bitvec;

    #[test]
    fn armor_round_trip() {
        let bits = bitvec![u8, Msb0; 1, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 1];
        let (armored, fill_bits) = into_armored(&bits);
        let armored = str::from_utf8(armored.as_ref()).unwrap();
        assert_eq!(fill_bits, 4);
        assert_eq!(from_armored(armored, fill_bits).unwrap(), bits);
    }

    #[test]
    fn dearmor_all_characters() {
        let armored: String = (0u8..64).map(|value| armor(value) as char).collect();
        let bits = from_armored(&armored, 0).unwrap();
        assert_eq!(bits.len(), 64 * 6);
        for (value, chunk) in bits.chunks(6).enumerate() {
            assert_eq!(usize::from(chunk.load_be::<u8>()), value);
        }
    }

    #[test]
    fn dearmor_invalid() {
        assert!(from_armored("0X", 0).is_err());
        assert!(from_armored("0_", 0).is_err());
        assert!(from_armored("0x", 0).is_err());
        assert!(from_armored("00", 6).is_err());
        assert!(from_armored("", 1).is_err());
        assert!(from_armored("", 0).unwrap().is_empty());
    }
}
