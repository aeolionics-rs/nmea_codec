//! A codec for NMEA 1083 sentences.

use show_option::prelude::*;

use std::fmt::{Display, Formatter, Write};
use bitvec::field::BitField;
use bitvec::prelude::{BitSlice, BitVec, Msb0};

pub mod types;

use bytes::{BufMut, Bytes, BytesMut};
use chrono::{DateTime, Utc};
use tokio_util::codec::Encoder;
use types::{AisChannel, CourseOverGround, MagneticVariation, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, SpeedOverGround, Talker};
use uom::si::angle::degree;
use uom::si::velocity::knot;
use bitvec::slice::ChunksExact;
use deku::DekuContainerWrite;

#[derive(Clone)]
pub struct TagBlock {}

#[derive(Clone)]
pub enum Sentence {
    RMC(RMC),
    VDM(AisMessage),
    VDO(AisMessage),
}

#[derive(Clone)]
pub struct Sequence {
    pub total: u8,
    pub item: u8,
    pub id: Option<u8>,
}
impl Default for Sequence {
    fn default() -> Self {
        Self { total: 1, item: 1, id: None }
    }
}
impl Display for Sequence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{},{}", self.total, self.item, format_option!(self.id, "{}", ""))
    }
}

#[derive(Clone)]
pub struct Armored {
    data: Bytes,
    padding: u8,
}

impl Armored {
    pub fn from_bits(data: &BitSlice<u8, Msb0>) -> Self {
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
        Self{data: result.freeze(), padding}
    }

}

fn armor(byte: u8) -> u8 {
    match byte {
        .. 0b101000 => byte + 0b00110000,
        _ => byte + 0b00111000,
    }
}

impl Display for Armored {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // SAFETY: Data can only contain ASCII characters.
        write!(f, "{},{}", unsafe { str::from_utf8_unchecked(self.data.as_ref()) }, self.padding)
    }
}

#[derive(Clone)]
pub struct Encapsulation {
    pub sequence: Sequence,
    pub data: Armored,
}

#[derive(Clone)]
pub struct Message {
    pub tag_block: Option<TagBlock>,
    pub sentence: Sentence,
}

pub struct NmeaCodec {}

impl NmeaCodec {
    /// Creates a codec for NMEA 0183/ISO 61162-1 messages.
    pub fn new() -> Self {
        Self {}
    }
}

impl Encoder<Message> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let start = dst.len();
        self.encode(item.sentence, dst)?;
        put_checksum(start + 1, dst);
        dst.put_slice(b"\r\n");
        Ok(())
    }
}

impl Encoder<Sentence> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Sentence, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Sentence::RMC(rmc) => rmc.encode(dst),
            Sentence::VDO(msg) => msg.encode("VDO", dst),
            Sentence::VDM(msg) => msg.encode("VDM", dst),
        }
        Ok(())
    }
}

fn put_checksum(start: usize, dst: &mut BytesMut) {
    let checksum = dst.as_ref()[start..].iter().fold(0u8, |sum, byte| sum ^ *byte);
    write!(dst, "*{checksum:02X}").expect("Failed to write checksum")
}

#[derive(Clone)]
pub struct RMC {
    pub talker_id: Talker,
    pub time_of_fix: Option<DateTime<Utc>>,
    pub position: Option<Position>,
    pub sog: Option<SpeedOverGround>,
    pub cog: Option<CourseOverGround>,
    pub magnetic_variation: Option<MagneticVariation>,
    pub status: PositioningSystemStatus,
    pub mode: PositioningSystemMode,
    pub navigational_status: NavigationalStatus,
}

impl RMC {
    pub fn new(talker: Talker) -> Self {
        Self {
            talker_id: talker,
            time_of_fix: None,
            position: None,
            sog: None,
            cog: None,
            magnetic_variation: None,
            status: PositioningSystemStatus::Warning,
            mode: PositioningSystemMode::NoFix,
            navigational_status: NavigationalStatus::NotValid,
        }
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        write!(
            dst,
            "${talker}RMC,{time},{status},{lat},{long},{sog},{cog},{date},{var},{mode},{nav}",
            talker = self.talker_id,
            date = format_option!(self.time_of_fix.map(|time| { time.format("%d%m%y") }), "{}", ""),
            time = format_option!(self.time_of_fix.map(|time| { time.format("%H%M%S%.f") }), "{}", ""),
            status = self.status,
            lat = format_option!(self.position.as_ref().map(|v| &v.latitude), "{}", ","),
            long = format_option!(self.position.as_ref().map(|v| &v.longitude), "{}", ","),
            sog = format_option!(self.sog.as_ref().map(|v| v.0.get::<knot>()), "{}", ""),
            cog = format_option!(self.cog.as_ref().map(|v| v.0.get::<degree>()), "{:.0}", ""),
            var = format_option!(self.magnetic_variation, "{}", ","),
            mode = self.mode,
            nav = self.navigational_status,
        )
        .expect("Failed to encode RMC")
    }
}

#[derive(Clone)]
pub struct AisMessage {
    pub talker_id: Talker,
    pub channel: Option<AisChannel>,
    pub message: Encapsulation,
}

impl AisMessage {
    pub fn new(talker_id: Talker, sequence: Sequence, channel: Option<AisChannel>, data: &BitSlice<u8, Msb0>) -> Self {
        let data = Armored::from_bits(data);
        Self{
            talker_id,
            channel,
            message: Encapsulation {
                sequence,
                data,
            },
        }
    }
    pub fn encode(&self, id: &'static str, dst: &mut BytesMut) {
        write!(
            dst,
            "!{talker}{id},{sequence},{channel},{data}",
            talker = self.talker_id,
            sequence = self.message.sequence,
            channel = format_option!(self.channel, "{}", ",,"),
            data = self.message.data,
        )
        .expect("Failed to encode AisMessage");
    }
}

pub trait IntoVDM {
    fn into_vdm(self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> AisMessageSequence;
}

impl IntoVDM for ais_rs::Message {
    fn into_vdm(self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> AisMessageSequence {
        let bits = self.to_bits().expect("");
        AisMessageSequence::new(talker, sequence, channel, bits)
    }
}

pub struct AisMessageSequence {
    talker: Talker,
    id: Option<u8>,
    channel: Option<AisChannel>,
    bits: BitVec<u8, Msb0>,
}

impl AisMessageSequence {
    pub fn new(talker: Talker, id: Option<u8>, channel: Option<AisChannel>, bits: BitVec<u8, Msb0>) -> Self {
        AisMessageSequence{ talker, id, channel, bits}
    }
    pub fn messages(&self) -> AisMessageIterator<'_> {
        AisMessageIterator::new(self.talker, self.id, self.channel, &self.bits)
    }
}

pub struct AisMessageIterator<'a> {
    talker: Talker,
    sequence_id: Option<u8>,
    channel: Option<AisChannel>,
    chunks: ChunksExact<'a, u8, Msb0>,
    size: usize,
    current: usize,
}

impl<'a> AisMessageIterator<'a> {
    pub fn new(talker: Talker, sequence_id: Option<u8>, channel: Option<AisChannel>, bits: &'a BitVec<u8, Msb0>) -> Self {
        let size = (bits.len() + (60 * 6 - 1)) / (60 * 6);
        let chunks = bits.chunks_exact(60 * 6);
        Self {
            talker,
            sequence_id,
            channel,
            chunks,
            size,
            current: 0,
        }
    }
}

impl<'a> Iterator for AisMessageIterator<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.size {
            return None;
        }
        self.current += 1;

        let sequence = Sequence {
            total: self.size as u8,
            item: self.current as u8,
            id: self.sequence_id,
        };

        let data = match self.chunks.next() {
            Some(chunk) => chunk,
            None => self.chunks.remainder(),
        };
        let ais_message = AisMessage::new(self.talker, sequence, self.channel, data);
        let sentence = Sentence::VDM(ais_message);
        Some(Message { tag_block: None, sentence })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}