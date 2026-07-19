use bytes::BytesMut;
use itu1371::Message::ScheduledPositionReport;
use itu1371::{CommunicationState, CourseOverGround, Heading, MMSI, NavigationalStatus, PositionAccuracy, RaimFlag, RateOfTurn, RepeatCount, ShipPositionReport, SpecialManoeuvre, SpeedOverGround, StationPosition, Timestamp, TransmitPower};
use nmea_codec::types::{AisChannel, Talker};
use nmea_codec::{IntoVDM, NmeaCodec};
use tokio_util::codec::Encoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ais_msg = ScheduledPositionReport(ShipPositionReport {
        repeat: RepeatCount(2),
        source: MMSI(127),
        status: NavigationalStatus::UnderWay,
        rate_of_turn: RateOfTurn(5),
        speed_over_ground: SpeedOverGround(0b1001100100),
        position: StationPosition {
            accuracy: PositionAccuracy::Low,
            longitude: 0b0000111101111111010010010000,
            latitude: 0b000001011101000101000010000,
        },
        course_over_ground: CourseOverGround(0b001110111111),
        heading: Heading(0b101011111),
        timestamp: Timestamp(0b110101),
        manoeuvre: SpecialManoeuvre::NotAvailable,
        transmit_power: TransmitPower::High,
        raim: RaimFlag::NotInUse,
        state: CommunicationState {
            sync: 0b00,
            slot_time_out: 0b001,
            sub_message: 0b01_111001_000100,
        },
    });

    let mut encoder = NmeaCodec::new();
    let mut buf = BytesMut::with_capacity(1024);
    let msg = ais_msg.into_vdm(Talker::AIS, None, Some(AisChannel::A)).messages().next().unwrap();
    encoder.encode(msg, &mut buf)?;
    assert_eq!(buf.as_ref(), b"!AIVDM,1,1,,A,1P000Oh1IT1svTP2r:43grwb05q4,0*71");
    Ok(())
}
