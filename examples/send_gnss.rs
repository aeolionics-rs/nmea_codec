use futures::sink::SinkExt;
use nmea_codec::{Latitude, Longitude, Message, NmeaCodec, Position, Rmc, Sentence};
use tokio::io;
use tokio_util::codec::FramedWrite;
use uom::si::angle::degree;
use uom::si::f64::Angle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let encoder = NmeaCodec::with_crlf();
    let mut writer = FramedWrite::new(io::stdout(), encoder);

    let sentence = Sentence::RMC(Rmc::default());
    writer.send(Message { sentence }).await?;

    let sentence = Sentence::RMC(Rmc {
        time_of_fix: Some(chrono::Utc::now()),
        position: Some(Position{
            latitude: Latitude(Angle::new::<degree>(45.5)),
            longitude: Longitude(Angle::new::<degree>(-45.5)),
        }),
        ..Default::default()
    });
    writer.send(Message { sentence }).await?;
    Ok(())
}
