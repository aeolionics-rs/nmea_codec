//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

use chrono::SubsecRound;
use futures::SinkExt;
use nmea_codec::types::{CourseOverGround, Latitude, Longitude, MagneticVariation, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, SpeedOverGround, Talker};
use nmea_codec::{Message, NmeaCodec, Sentence};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::codec::FramedWrite;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};
use uom::si::velocity::knot;

const DEFAULT_ADDR: &str = "127.0.0.1:10110";
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Open the socket we will be sending on.
    let listener = TcpListener::bind(&DEFAULT_ADDR).await?;

    // Create a broadcast channel for messages we will be sending out.
    let (tx, _rx) = tokio::sync::broadcast::channel(8);

    // Listen for incoming connections.
    let messages = tx.clone();
    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tracing::info!("New connection from {addr}");

            // Spawn a task to send messages to the client.
            let receiver = messages.subscribe();
            tokio::spawn(async move { send_messages(receiver, stream).await });
        }
    });

    loop {
        let sentence = Sentence::RMC {
            talker: Talker::GNSS,
            time_of_fix: Some(chrono::Utc::now().trunc_subsecs(2)),
            position: Some(Position {
                latitude: Latitude(Angle::new::<degree>(47.0 + 36.6 / 60.0)),
                longitude: Longitude(Angle::new::<degree>(-(122.0 + 23.0 / 60.0))),
            }),
            sog: Some(SpeedOverGround(Velocity::new::<knot>(1.0))),
            cog: Some(CourseOverGround(Angle::new::<degree>(273.6))),
            variation: Some(MagneticVariation(Angle::new::<degree>(-10.0))),
            status: PositioningSystemStatus::Valid,
            mode: PositioningSystemMode::Autonomous,
            nav_status: NavigationalStatus::NotValid,
        };
        let message = Message { tag_block: None, sentence };

        _ = tx.send(message);
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }
}

async fn send_messages(mut messages: Receiver<Message>, stream: TcpStream) -> std::io::Result<()> {
    let mut writer = FramedWrite::with_capacity(stream, NmeaCodec::new(), 128);
    loop {
        match messages.recv().await {
            Ok(msg) => _ = writer.send(msg).await?,
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
    Ok(())
}
