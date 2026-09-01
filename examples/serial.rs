//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use nmea_codec::NmeaCodec;
use tokio_serial::SerialStream;
use tokio_util::codec::Decoder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(port) = std::env::args().nth(1) else {
        eprintln!("USAGE: serial PORT");
        return Ok(());
    };
    let stream = SerialStream::open(&tokio_serial::new(port, 9600))?;
    let mut framed = NmeaCodec::new().framed(stream);

    while let Some(frame) = framed.next().await {
        match frame {
            Ok(frame) => println!("{:?}", frame.sentence),
            Err(e) => {
                eprintln!("{:?}", e);
                break;
            }
        }
    }
    Ok(())
}
