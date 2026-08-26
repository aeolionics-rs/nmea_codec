//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

use ais_asm::area_notice::{AreaNotice, Description, Position, Radius, Scale, SubArea};
use ais_asm::{Broadcast, Linkage};
use ais_message::message::BroadcastApplicationMessage;
use ais_message::types::*;
use ais_types::{Minutes, MonthDayHourMinute};
use bytes::BytesMut;
use nmea_codec::NmeaCodec;
use nmea_codec::ais::IntoVDM;
use nmea_codec::types::{AisChannel, Talker};
use tokio_util::codec::Encoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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


    let ais_msg = ais_message::Message::BroadcastBinaryMessage(BroadcastApplicationMessage {
        repeat: Default::default(),
        source: MMSI::new(3669999)?,
        message: Broadcast::AreaNotice(notice)
    });

    let mut encoder = NmeaCodec::new();
    let mut buf = BytesMut::with_capacity(1024);
    let msg = ais_msg.into_vdm(Talker::AIS, None, Some(AisChannel::A)).messages().next().unwrap();
    encoder.encode(msg, &mut buf)?;
    assert_eq!(buf.as_ref(), b"!AIVDM,1,1,,A,803Owsh0EWm0gEe`2l0=v:>i=W9L000001giAn9dq;Wl0000,3*3E\r\n");
    Ok(())
}
