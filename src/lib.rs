//! A codec for NMEA 1083 sentences.

pub mod ais;

use chrono::{DateTime, Utc};
use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::Encoder;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};
use uom::si::velocity::knot;

pub struct Latitude(pub Angle);
pub struct Longitude(pub Angle);
pub struct Position {
    pub latitude: Latitude,
    pub longitude: Longitude,
}

pub struct SpeedOverGround(Velocity);
pub struct CourseOverGround(Angle);
pub struct MagneticVariation(Angle);

pub enum PositioningSystemStatus {
    Valid,
    Warning,
}

pub enum PositioningSystemMode {
    NoFix,
    Autonomous,
    Differential,
    Estimated,
    Manual,
    Precise,
    FixedRTK,
    FloatRTK,
    Simulator,
}

pub enum NavigationalStatus {
    Safe,
    Caution,
    Unsafe,
    NotValid,
}

pub struct Rmc {
    pub time_of_fix: Option<DateTime<Utc>>,
    pub position: Option<Position>,
    pub sog: Option<SpeedOverGround>,
    pub cog: Option<CourseOverGround>,
    pub magnetic_variation: Option<MagneticVariation>,
    pub status: PositioningSystemStatus,
    pub mode: PositioningSystemMode,
    pub navigational_status: NavigationalStatus,
}

impl Default for Rmc {
    fn default() -> Self {
        Self {
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
}

pub struct Encapsulated {
    data: Bytes
}

pub enum Sentence {
    RMC(Rmc),
}

pub struct Message {
    pub sentence: Sentence,
}

pub struct NmeaCodec {
    with_crlf: bool,
}

impl NmeaCodec {
    /// Creates a codec for bare sentences without the CRLF separator.
    ///
    /// Sentence separation must be provided by the underlying transport such as UDP datagrams.
    pub fn new() -> Self {
        Self { with_crlf: false }
    }

    /// Creates a codec with sentences separated by CRLF.
    ///
    /// This is intended for use with streams where sepration is required, such as a serial line
    /// or TCP connection.
    pub fn with_crlf() -> Self {
        Self { with_crlf: true }
    }
}

impl Encoder<Message> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(b'$');
        let start = dst.len();
        dst.put_slice(b"xx");
        match item.sentence {
            Sentence::RMC(rmc) => self.encode(rmc, dst)?,
        }
        let checksum = dst.as_ref()[start..].iter().fold(0u8, |sum, byte| sum ^ *byte);
        dst.put_slice(format!("*{checksum:02X}").as_bytes());
        if self.with_crlf {
            dst.put_slice(b"\r\n");
        }
        Ok(())
    }
}

impl Encoder<Rmc> for NmeaCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Rmc, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_slice(b"RMC");
        write_time(item.time_of_fix, dst);
        dst.put_u8(b',');
        match item.status {
            PositioningSystemStatus::Valid => dst.put_u8(b'A'),
            PositioningSystemStatus::Warning => dst.put_u8(b'V'),
        }
        write_position(item.position, dst);
        write_sog(item.sog, dst);
        write_cog(item.cog, dst);
        write_date(item.time_of_fix, dst);
        write_magnetic_variation(item.magnetic_variation, dst);
        dst.put_u8(b',');
        dst.put_u8(match item.mode {
            PositioningSystemMode::NoFix => b'N',
            PositioningSystemMode::Autonomous => b'A',
            PositioningSystemMode::Differential => b'D',
            PositioningSystemMode::Manual => b'M',
            PositioningSystemMode::Estimated => b'E',
            PositioningSystemMode::Precise => b'P',
            PositioningSystemMode::FixedRTK => b'R',
            PositioningSystemMode::FloatRTK => b'F',
            PositioningSystemMode::Simulator => b'S',
        });
        dst.put_u8(b',');
        dst.put_u8(match item.navigational_status {
            NavigationalStatus::Safe => b'S',
            NavigationalStatus::Caution => b'C',
            NavigationalStatus::Unsafe => b'U',
            NavigationalStatus::NotValid => b'V',
        });
        Ok(())
    }
}

fn write_sog(item: Option<SpeedOverGround>, dst: &mut BytesMut) {
    if let Some(sog) = item {
        dst.put_slice(format!(",{:.2}", sog.0.get::<knot>()).as_bytes());
    } else {
        dst.put_u8(b',');
    }
}

fn write_cog(item: Option<CourseOverGround>, dst: &mut BytesMut) {
    if let Some(cog) = item {
        dst.put_slice(format!(",{:.2}", cog.0.get::<degree>()).as_bytes());
    } else {
        dst.put_u8(b',');
    }
}

fn write_magnetic_variation(item: Option<MagneticVariation>, dst: &mut BytesMut) {
    if let Some(variation) = item {
        let angle = variation.0.get::<degree>();
        dst.put_slice(format!(",{:.2},{}", angle.abs(), if angle >= 0.0 { 'E' } else { 'W' }).as_bytes());
    } else {
        dst.put_slice(b",,");
    }
}

fn write_position(item: Option<Position>, dst: &mut BytesMut) {
    if let Some(position) = item {
        let latitude = position.latitude.0.get::<degree>();
        let lat_cardinal = if latitude >= 0.0 { 'N' } else { 'S' };
        let lat_degrees = latitude.abs().trunc();
        let lat_minutes = latitude.abs().fract() * 60.0;
        let longitude = position.longitude.0.get::<degree>();
        let long_cardinal = if longitude >= 0.0 { 'E' } else { 'W' };
        let long_degrees = longitude.abs().trunc();
        let long_minutes = longitude.abs().fract() * 60.0;
        dst.put_slice(format!(",{lat_degrees:02}{lat_minutes:07.4},{lat_cardinal},{long_degrees:03}{long_minutes:07.4},{long_cardinal}",).as_bytes());
    } else {
        dst.put_slice(b",,,,")
    }
}

fn write_date(date: Option<DateTime<Utc>>, dst: &mut BytesMut) {
    if let Some(date) = date {
        _ = date.format(",%d%m%y").write_to(dst);
    } else {
        dst.put_slice(b",")
    }
}

fn write_time(time: Option<DateTime<Utc>>, dst: &mut BytesMut) {
    if let Some(time) = time {
        _ = time.format(",%H%M%S").write_to(dst);
        _ = dst.put_slice(format!(".{:02}", time.timestamp_subsec_millis() / 10).as_bytes());
    } else {
        dst.put_slice(b",")
    }
}
