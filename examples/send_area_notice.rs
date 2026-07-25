use ais_rs::asm;
use ais_rs::asm::area_notice::{AreaNotice, Description, Position, Radius, Scale, SubArea};
use ais_rs::asm::Linkage;
use ais_rs::message::BroadcastApplicationMessage;
use ais_rs::types::{Minutes, MonthDayHourMinute, MMSI};
use bytes::BytesMut;
use nmea_codec::types::{AisChannel, Talker};
use nmea_codec::{IntoVDM, NmeaCodec};
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
                position: Position {
                    latitude: (42.386320965494704 * 60_000.0) as i32,
                    longitude: (-70.91003783738283 * 60_000.0) as i32,
                    precision: 4,
                },
                radius: Radius(0),
            },
            SubArea::Circle {
                scale: Scale(0),
                position: Position {
                    latitude: (42.386320965494704 * 60_000.0) as i32,
                    longitude: (-70.91003783738283 * 60_000.0) as i32,
                    precision: 4,
                },
                radius: Radius(4000),
            },
        ],
    };


    let ais_msg = ais_rs::Message::BroadcastBinaryMessage(BroadcastApplicationMessage {
        repeat_count: Default::default(),
        source: MMSI::new(3669999)?,
        message: asm::Broadcast::AreaNotice(notice)
    });

    let mut encoder = NmeaCodec::new();
    let mut buf = BytesMut::with_capacity(1024);
    let msg = ais_msg.into_vdm(Talker::AIS, None, Some(AisChannel::A)).messages().next().unwrap();
    encoder.encode(msg, &mut buf)?;
    assert_eq!(buf.as_ref(), b"!AIVDM,1,1,,A,803Owsh0EWm0gEe`2l0=v:>i=W9L000001giAn9dq;Wl0000,3*3E");
    Ok(())
}
