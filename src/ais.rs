//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! [`Sentence`] structures used for AIS communication.
//!
use crate::encapsulation::{Sequence, into_armored};
use crate::types::{AisChannel, Talker};
use bitvec::order::Msb0;
use bitvec::prelude::BitVec;
use bytes::BytesMut;
use show_option::format_option;
use std::fmt::Write;

#[derive(Clone)]
pub struct AisMessage {
    pub talker: Talker,
    pub sequence: Sequence,
    pub channel: Option<AisChannel>,
    pub data: BitVec<u8, Msb0>,
}

impl AisMessage {
    pub fn encode(&self, id: &'static str, dst: &mut BytesMut) -> std::fmt::Result {
        let (armored, padding) = into_armored(self.data.as_ref());
        // SAFETY: the result only contains ASCII
        let armor_text = str::from_utf8(armored.as_ref()).unwrap();
        write!(
            dst,
            "!{talker}{id},{sequence},{channel},{armor_text},{padding}",
            talker = self.talker,
            sequence = self.sequence,
            channel = format_option!(self.channel, "{}", ",,"),
        )
    }
}
