//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! [`Sentence`] structures used for AIS communication.
//!
use crate::encapsulation::{Armored, Encapsulation, Sequence};
use crate::types::{AisChannel, Talker};
use crate::{Message, Sentence};
use bitvec::order::Msb0;
use bitvec::prelude::{BitSlice, BitVec};
use bitvec::slice::ChunksExact;
use bytes::BytesMut;
use deku::DekuContainerWrite;
use show_option::format_option;
use std::fmt::Write;

#[derive(Clone)]
pub struct AisMessage {
    pub talker_id: Talker,
    pub channel: Option<AisChannel>,
    pub message: Encapsulation,
}

impl AisMessage {
    pub fn new(talker_id: Talker, sequence: Sequence, channel: Option<AisChannel>, data: &BitSlice<u8, Msb0>) -> Self {
        let data = Armored::from_bits(data);
        Self {
            talker_id,
            channel,
            message: Encapsulation { sequence, data },
        }
    }
    pub fn encode(&self, id: &'static str, dst: &mut BytesMut) -> std::fmt::Result {
        write!(
            dst,
            "!{talker}{id},{sequence},{channel},{data}",
            talker = self.talker_id,
            sequence = self.message.sequence,
            channel = format_option!(self.channel, "{}", ",,"),
            data = self.message.data,
        )
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
        AisMessageSequence { talker, id, channel, bits }
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