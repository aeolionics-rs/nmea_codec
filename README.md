NMEA Codec
==========

A codec for `NMEA 0183`/`IEC 61162-1` sentences,
with support for tag blocks and encapsulated messages such as sentences used for `AIS`.

The crate defines native message for well-known formats.
Others are mapped to generic types enabling support for unrecognized messages.
Messages are modeled in a Rust-native form, using the `uom` crate to improve type safety.
Conversion to text is performed during encoding and decoding.
The implementation also supports `Proprietary` and `Query` sentences.

Encodes and decodes NMEA sentences to `BytesMut` buffers using the Tokio codec framework.
The implementation does not require an async runtime but does require an allocator.

The `ais` feature enables binding to type-safe messages; if this feature is not enabled then
AIS messages are mapped to generic sentences.

# License and Contributions
Released under Apache License V2.0.
