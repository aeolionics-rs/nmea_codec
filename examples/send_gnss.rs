//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use chrono::SubsecRound;
use nmea_codec::types::{CourseOverGround, Latitude, Longitude, MagneticVariation, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, SpeedOverGround, Talker};
use nmea_codec::{NmeaCodec, Sentence};
use tokio_util::codec::Encoder;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};
use uom::si::velocity::knot;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sentence = Sentence::RMC {
        talker: Talker::GNSS,
        time_of_fix: Some(chrono::Utc::now().trunc_subsecs(2)),
        position: Some(Position {
            latitude: Latitude(Angle::new::<degree>(45.5)),
            longitude: Longitude(Angle::new::<degree>(-45.5)),
        }),
        sog: Some(SpeedOverGround(Velocity::new::<knot>(1.0))),
        cog: Some(CourseOverGround(Angle::new::<degree>(273.6))),
        variation: Some(MagneticVariation(Angle::new::<degree>(-10.0))),
        status: PositioningSystemStatus::Valid,
        mode: PositioningSystemMode::Autonomous,
        nav_status: NavigationalStatus::Safe,
    };

    let mut encoder = NmeaCodec::new();
    let mut buf = BytesMut::with_capacity(1024);
    encoder.encode(sentence, &mut buf)?;
    print!("{}", String::from_utf8_lossy(buf.as_ref()));
    Ok(())
}
