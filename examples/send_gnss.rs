use bytes::BytesMut;
use chrono::SubsecRound;
use nmea_codec::{Message, NmeaCodec, RMC, Sentence};
use tokio_util::codec::Encoder;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};
use uom::si::velocity::knot;
use nmea_codec::types::{CourseOverGround, Latitude, Longitude, MagneticVariation, Position, SpeedOverGround, Talker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = NmeaCodec::new();
    let sentence = Sentence::RMC(RMC::new(Talker::GNSS));
    let mut buf = BytesMut::with_capacity(1024);
    encoder.encode(Message { tag_block: None, sentence }, &mut buf)?;
    println!("{}", String::from_utf8_lossy(buf.as_ref()));

    let sentence = Sentence::RMC(RMC {
        time_of_fix: Some(chrono::Utc::now().trunc_subsecs(2)),
        position: Some(Position {
            latitude: Latitude(Angle::new::<degree>(45.5)),
            longitude: Longitude(Angle::new::<degree>(-45.5)),
        }),
        sog: Some(SpeedOverGround(Velocity::new::<knot>(1.0))),
        cog: Some(CourseOverGround(Angle::new::<degree>(273.6))),
        magnetic_variation: Some(MagneticVariation(Angle::new::<degree>(-10.0))),
        ..RMC::new(Talker::GNSS)
    });
    buf.clear();
    encoder.encode(Message { tag_block: None, sentence }, &mut buf)?;
    print!("{}", String::from_utf8_lossy(buf.as_ref()));
    Ok(())
}
