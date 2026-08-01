//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Sentences encapsulating binary data from other protocols (e.g. AIS).
//!
use bitvec::field::BitField;
use bitvec::order::Msb0;
use bitvec::prelude::BitSlice;
use bytes::{BufMut, Bytes, BytesMut};
use show_option::format_option;
use std::fmt::{Display, Formatter};

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
        Self { data: result.freeze(), padding }
    }
}

fn armor(byte: u8) -> u8 {
    match byte {
        ..0b101000 => byte + 0b00110000,
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