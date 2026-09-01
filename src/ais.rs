//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Support for AIS messages.
//! 
use crate::encapsulation::{Encapsulation, Fields, Sequence, into_armored};
use crate::types::{AisChannel, Talker};
use ais_message::message::{AddressedApplicationMessage, AddressedSafetyMessage, BroadcastApplicationMessage, BroadcastSafetyMessage};
use bitvec::order::Msb0;
use bitvec::prelude::BitVec;
use bytes::BytesMut;
use deku::DekuContainerWrite;
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

/// Trait for encapsulating an AIS addressed binary message.
pub trait IntoABM {
    fn to_abm(&self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> Encapsulation;
}

impl IntoABM for AddressedApplicationMessage {
    fn to_abm(&self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> Encapsulation {
        let destination = self.destination.into();
        let bits = self.to_bits().expect("Failed to serialize addressed application message");
        let fields = Fields::ABM { destination, channel, message_id: 6 };
        Encapsulation { talker, sequence, fields, bits }
    }
}

impl IntoABM for AddressedSafetyMessage {
    fn to_abm(&self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> Encapsulation {
        let destination = self.destination.into();
        let bits = self.to_bits().expect("Failed to serialize addressed safety message");
        let fields = Fields::ABM { destination, channel, message_id: 12 };
        Encapsulation { talker, sequence, fields, bits }
    }
}

/// Trait for encapsulating an AIS broadcast binary message.
pub trait IntoBBM {
    fn to_bbm(&self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> Encapsulation;
}

impl IntoBBM for BroadcastApplicationMessage {
    fn to_bbm(&self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> Encapsulation {
        let bits = self.to_bits().expect("Failed to serialize broadcast application message");
        let fields = Fields::BBM { channel, message_id: 8 };
        Encapsulation { talker, sequence, fields, bits }
    }
}

impl IntoBBM for BroadcastSafetyMessage {
    fn to_bbm(&self, talker: Talker, sequence: Option<u8>, channel: Option<AisChannel>) -> Encapsulation {
        let bits = self.to_bits().expect("Failed to serialize broadcast safety message");
        let fields = Fields::BBM { channel, message_id: 14 };
        Encapsulation { talker, sequence, fields, bits }
    }
}
