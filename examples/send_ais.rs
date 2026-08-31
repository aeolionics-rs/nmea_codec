//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

use ais_message::Message::ScheduledPositionReport;
use ais_message::message::PositionReport;
use ais_message::types::*;
use bytes::BytesMut;
use deku::DekuContainerWrite;
use nmea_codec::NmeaCodec;
use nmea_codec::encapsulation::Fields;
use nmea_codec::types::{AisChannel, Talker};
use tokio_util::codec::Encoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ais_msg = ScheduledPositionReport(PositionReport {
        repeat: RepeatCount::Twice,
        source: MMSI::new(127)?,
        status: NavigationalStatus::UnderWay,
        rate_of_turn: RateOfTurn(5),
        speed_over_ground: SpeedOverGround(0b1001100100),
        position: StationPosition::new(5.0 + 5.0 / 60.0, 27.0 + 5.0 / 60.0, PositionAccuracy::Low),
        course_over_ground: CourseOverGround(0b001110111111),
        heading: Heading(0b101011111),
        timestamp: Timestamp::Second(53),
        manoeuvre: SpecialManoeuvre::NotAvailable,
        transmit_power: TransmitPower::High,
        raim: RaimFlag::NotInUse,
        state: CommunicationState {
            sync: 0b00,
            slot_time_out: 0b001,
            sub_message: 0b01_111001_000100,
        },
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
    // println!("{}", str::from_utf8(buf.as_ref()).unwrap());
    assert_eq!(buf.as_ref(), b"!AIVDM,1,1,,A,1P000Oh1IT1svTP2r:43grwb05q4,0*71\r\n");
    Ok(())
}
