//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Open concurrent HTTP/1.1 streams to the `https_server` example and print the sentences received.
//!
//! HTTP/1.1 cannot multiplex so each stream uses its own connection, e.g. `https_client 4` opens four connections.
//! Pass `--sse` to request the sentences as Server-Sent Events, e.g. `https_client 4 --sse`.

use bytes::Bytes;
use futures::stream::BoxStream;
use futures::{Stream, StreamExt, TryStreamExt};
use nmea_codec::NmeaCodec;
use reqwest::header;
use std::io;
use tokio::task::JoinSet;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

const DEFAULT_URL: &str = "https://localhost:10443/nmea0183";

const EVENT_STREAM: &str = "text/event-stream";

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let mut args = std::env::args().skip(1);
    let (Some(streams), sse) = (args.next(), args.next()) else {
        eprintln!("USAGE: https_client STREAMS [--sse]");
        return Ok(());
    };
    let streams: usize = streams.parse()?;
    let sse = match sse.as_deref() {
        None => false,
        Some("--sse") => true,
        Some(arg) => return Err(format!("unexpected argument {arg}").into()),
    };

    // The server uses a self-signed certificate so skip verification.
    let client = reqwest::Client::builder().http1_only().tls_danger_accept_invalid_certs(true).build()?;

    let mut tasks = JoinSet::new();
    for id in 0..streams {
        tasks.spawn(stream(id, client.clone(), sse));
    }
    while let Some(result) = tasks.join_next().await {
        if let Err(e) = result? {
            tracing::error!("{e}");
        }
    }
    Ok(())
}

/// Request a stream of sentences and print each one until the server closes it.
///
/// If `sse` is set the sentences are requested as Server-Sent Events.
async fn stream(id: usize, client: reqwest::Client, sse: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut request = client.get(DEFAULT_URL);
    if sse {
        request = request.header(header::ACCEPT, EVENT_STREAM);
    }
    let response = request.send().await?.error_for_status()?;
    // The server decides the format so use the content type it returned.
    let is_event_stream = response.headers().get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).is_some_and(|value| value.starts_with(EVENT_STREAM));
    tracing::info!("stream {id} connected using {:?}{}", response.version(), if is_event_stream { " with events" } else { "" });

    let bytes = response.bytes_stream().map_err(io::Error::other);
    let bytes: BoxStream<'static, io::Result<Bytes>> = if is_event_stream { event_data(bytes).boxed() } else { bytes.boxed() };
    let mut messages = FramedRead::new(StreamReader::new(bytes), NmeaCodec::new());
    while let Some(message) = messages.next().await {
        match message? {
            Ok(message) => println!("{id}: {:?}", message.sentence),
            Err(e) => tracing::warn!("stream {id} skipping invalid sentence: {e}"),
        }
    }
    tracing::info!("stream {id} closed");
    Ok(())
}

/// Convert a Server-Sent Event stream into the sentences carried in each event's data, terminated with CR LF.
///
/// The server sends one sentence per event so each `data` line is treated as a sentence and other fields are ignored.
fn event_data(bytes: impl Stream<Item = io::Result<Bytes>> + Send + 'static) -> impl Stream<Item = io::Result<Bytes>> + Send + 'static {
    FramedRead::new(StreamReader::new(bytes), LinesCodec::new()).map_err(io::Error::other).try_filter_map(|line| async move {
        Ok(line.strip_prefix("data:").map(|data| {
            let sentence = data.strip_prefix(' ').unwrap_or(data);
            Bytes::from(format!("{sentence}\r\n"))
        }))
    })
}
