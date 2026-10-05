//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Print the sentences read from a serial port, such as an AIS receiver, decoding AIS messages.

use futures::StreamExt;
use nmea_codec::{NmeaCodec, Sentence};
use tokio_serial::SerialStream;
use tokio_util::codec::Decoder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(port) = std::env::args().nth(1) else {
        eprintln!("USAGE: serial PORT");
        return Ok(());
    };
    let stream = SerialStream::open(&tokio_serial::new(port, 38400))?;
    let mut framed = NmeaCodec::new().framed(stream);

    while let Some(frame) = framed.next().await {
        match frame? {
            Ok(frame) => {
                println!("{:?}", frame.sentence);
                // VDM and VDO carry AIS messages received from other stations and sent by this one.
                if let Sentence::VDM(msg) | Sentence::VDO(msg) = &frame.sentence {
                    match ais_message::Message::from_bits(&msg.data) {
                        Ok(ais_msg) => println!("{ais_msg:#?}"),
                        // An AIS message type that cannot be decoded is skipped.
                        Err(e) => eprintln!("skipping undecodable AIS message: {e}"),
                    }
                }
            }
            Err(e) => eprintln!("skipping invalid sentence: {e}"),
        }
    }
    Ok(())
}
