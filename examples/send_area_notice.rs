//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

use ais_asm::area_notice::{AreaNotice, Description, Position, Radius, Scale, SubArea};
use ais_asm::{Addressed, Broadcast, Linkage};
use ais_message::message::{AddressedApplicationMessage, BroadcastApplicationMessage};
use ais_message::types::*;
use ais_types::{Minutes, MonthDayHourMinute};
use bytes::BytesMut;
use deku::DekuContainerWrite;
use nmea_codec::NmeaCodec;
use nmea_codec::ais::IntoABM;
use nmea_codec::encapsulation::Fields;
use nmea_codec::types::{AisChannel, Talker};
use tokio_util::codec::Encoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let notice = area_notice();
    let ais_msg = ais_message::Message::BroadcastBinaryMessage(BroadcastApplicationMessage {
        repeat: Default::default(),
        source: MMSI::new(3669999)?,
        message: Broadcast::AreaNotice(notice),
    });
    let encapsulation = nmea_codec::encapsulation::Encapsulation {
        talker: Talker::AIS,
        sequence: None,
        fields: Fields::VDM { channel: Some(AisChannel::A) },
        bits: ais_msg.to_bits()?,
    };

    let mut encoder = NmeaCodec::new();
    let mut buf = BytesMut::with_capacity(1024);
    encoder.encode(encapsulation, &mut buf)?;
    assert_eq!(buf.as_ref(), b"!AIVDM,1,1,,A,803Owsh0EWm0gEe`2l0=v:>i=W9L000001giAn9dq;Wl0000,3*3E\r\n");

    let notice = area_notice();
    let ais_msg = AddressedApplicationMessage {
        repeat: Default::default(),
        source: MMSI::new(3669999)?,
        destination: MMSI::new(999999999)?,
        sequence: Default::default(),
        retransmitted: false,
        message: Addressed::AreaNotice(notice),
    };
    let encapsulation = ais_msg.to_abm(Talker::ECDIS, Some(0), Some(AisChannel::A));

    buf.clear();
    encoder.encode(encapsulation, &mut buf)?;
    println!("{}", str::from_utf8(buf.as_ref())?);
    Ok(())
}

fn area_notice() -> AreaNotice {
    let notice = AreaNotice {
        linkage: Linkage(501),
        description: Description::MammalsInArea_ReduceSpeed,
        start: MonthDayHourMinute { month: 7, day: 21, hour: 13, minute: 45 },
        duration: Minutes(1440),
        subareas: vec![
            SubArea::Circle {
                scale: Scale(0),
                position: Position::new(42.386320965494704, -70.91003783738283, 4),
                radius: Radius(0),
            },
            SubArea::Circle {
                scale: Scale(0),
                position: Position::new(42.386320965494704, -70.91003783738283, 4),
                radius: Radius(4000),
            },
        ],
    };
    notice
}
