//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Stream simulated RMC sentences over HTTPS.
//!
//! The server uses a self-signed certificate for localhost so clients must skip verification, e.g.
//! `curl -k https://localhost:10443/nmea0183`.
//!
//! Clients that accept `text/event-stream` receive each sentence as a Server-Sent Event, e.g.
//! `curl -k -H 'Accept: text/event-stream' https://localhost:10443/nmea0183`.

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, header};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use bytes::{Bytes, BytesMut};
use chrono::SubsecRound;
use futures::{Stream, StreamExt};
use nmea_codec::types::{CourseOverGround, Latitude, Longitude, MagneticVariation, NavigationalStatus, Position, PositioningSystemMode, PositioningSystemStatus, SpeedOverGround, Talker};
use nmea_codec::{Message, NmeaCodec, Sentence};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::time::{Duration, Interval, interval};
use tokio_util::codec::Encoder;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};
use uom::si::velocity::knot;

const DEFAULT_ADDR: &str = "127.0.0.1:10443";

/// How often a sentence is sent.
const INTERVAL: Duration = Duration::from_millis(100);

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Generate a self-signed certificate for localhost.
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let config = RustlsConfig::from_pem(certified.cert.pem().into_bytes(), certified.signing_key.serialize_pem().into_bytes()).await?;

    let app = Router::new().route("/nmea0183", get(nmea0183));
    let addr: SocketAddr = DEFAULT_ADDR.parse()?;
    tracing::info!("Streaming from https://localhost:{}/nmea0183", addr.port());
    axum_server::bind_rustls(addr, config).serve(app.into_make_service()).await?;
    Ok(())
}

/// Stream RMC sentences from a simulated vessel until the client disconnects.
///
/// Sentences are sent as Server-Sent Events if the client accepts `text/event-stream`, otherwise as plain text.
async fn nmea0183(headers: HeaderMap) -> Response {
    tracing::info!("GET nmea0183");
    let sentences = sentences();
    if accepts_event_stream(&headers) {
        // SSE data cannot contain line breaks so each event carries the sentence without its CR LF.
        let events = sentences.map(|sentence| {
            let text = std::str::from_utf8(&sentence).expect("NMEA sentences should be ASCII");
            Ok::<Event, Infallible>(Event::default().data(text.trim_end()))
        });
        Sse::new(events).into_response()
    } else {
        let body = Body::from_stream(sentences.map(Ok::<Bytes, Infallible>));
        ([(header::CONTENT_TYPE, "text/plain; charset=us-ascii")], body).into_response()
    }
}

/// Whether the request's `Accept` header includes `text/event-stream`.
fn accepts_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::ACCEPT)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|media_range| media_range.split(';').next().is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("text/event-stream")))
}

/// An endless stream of encoded RMC sentences, one per interval.
fn sentences() -> impl Stream<Item = Bytes> {
    let vessel = Vessel {
        latitude: 47.0 + 36.6 / 60.0,
        longitude: -(122.0 + 23.0 / 60.0),
        speed: 5.0,
        course: 273.6,
        interval: interval(INTERVAL),
        codec: NmeaCodec::new(),
    };
    futures::stream::unfold(vessel, |mut vessel| async move {
        vessel.interval.tick().await;
        let sentence = vessel.rmc();
        vessel.advance();
        Some((sentence, vessel))
    })
}

/// A vessel moving at constant speed and course.
struct Vessel {
    /// Latitude in degrees.
    latitude: f64,
    /// Longitude in degrees.
    longitude: f64,
    /// Speed over ground in knots.
    speed: f64,
    /// Course over ground in degrees true.
    course: f64,
    interval: Interval,
    codec: NmeaCodec,
}

impl Vessel {
    /// Encode an RMC sentence for the vessel's current position.
    fn rmc(&mut self) -> Bytes {
        let sentence = Sentence::RMC {
            talker: Talker::GNSS,
            time_of_fix: Some(chrono::Utc::now().trunc_subsecs(2)),
            position: Some(Position {
                latitude: Latitude(Angle::new::<degree>(self.latitude)),
                longitude: Longitude(Angle::new::<degree>(self.longitude)),
            }),
            sog: Some(SpeedOverGround(Velocity::new::<knot>(self.speed))),
            cog: Some(CourseOverGround(Angle::new::<degree>(self.course))),
            variation: Some(MagneticVariation(Angle::new::<degree>(-15.0))),
            status: PositioningSystemStatus::Valid,
            mode: PositioningSystemMode::Autonomous,
            nav_status: NavigationalStatus::NotValid,
        };
        let mut buf = BytesMut::with_capacity(128);
        self.codec.encode(Message { tag_blocks: vec![], sentence }, &mut buf).expect("RMC sentence should encode");
        buf.freeze()
    }

    /// Move the vessel along its course for one interval.
    fn advance(&mut self) {
        // A nautical mile is one minute of latitude.
        let distance = self.speed * INTERVAL.as_secs_f64() / 3600.0 / 60.0;
        let course = self.course.to_radians();
        self.latitude += distance * course.cos();
        self.longitude += distance * course.sin() / self.latitude.to_radians().cos();
    }
}
