//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Support for tag blocks.
//!
//! A tag block precedes a sentence and contains additional metadata about it, such as its source
//! or the time it was received. It contains a comma separated list of `code:value` parameters
//! followed by a checksum, all between a pair of `\` delimiters:
//!
//! **Tag block format:** `\s:GP0001,n:42,c:1767225600000*60\`
//!
//! Reserved and non-printable characters in a value are sent in the `^hh` hex form of their
//! ISO/IEC 8859-1 code, so `t:a^2Cb` is the text `a,b`.
//!
use crate::{put_checksum, put_escaped, unescape, validate};
use bytes::{BufMut, BytesMut};
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind};

/// Maximum number of characters in a tag block, including the checksum but excluding the `\`
/// delimiters, as given by IEC 61162-450.
///
/// The limit is enforced when encoding; longer tag blocks are accepted when decoding.
pub const MAX_TAG_BLOCK: usize = 80;

/// A block of tags containing additional metadata about the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagBlock {
    /// The tags in the block, in the order they are sent.
    pub tags: Vec<Tag>,
}

/// A parameter within a tag block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    /// c: The UNIX time the sentence was produced, sent in milliseconds.
    ///
    /// Some descriptions of the format give seconds, but sources seen in practice send
    /// milliseconds. Any finer precision is dropped when encoding.
    Time(DateTime<Utc>),
    /// d: The identification of the destination.
    Destination(String),
    /// g: The position of the sentence within a group of related sentences.
    Group(Group),
    /// n: A count of the lines sent, used to detect missing lines.
    LineCount(u32),
    /// r: A relative time, in units defined by the source.
    RelativeTime(u64),
    /// s: The identification of the source.
    Source(String),
    /// t: A text string.
    Text(String),
    /// A parameter with an unrecognized code.
    Unknown { code: String, value: String },
}

/// The position of a sentence within a group of related sentences, sent as `number-total-id`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Group {
    /// The number of this sentence within the group, starting at 1.
    pub number: u8,
    /// The total number of sentences in the group.
    pub total: u8,
    /// An identifier common to all sentences in the group.
    pub id: u32,
}

impl TagBlock {
    /// Writes the tag block, with its checksum and delimiters.
    ///
    /// Reserved characters in text values are sent in their `^hh` hex form.
    pub(crate) fn encode(&self, dst: &mut BytesMut) -> Result<(), Error> {
        dst.put_u8(b'\\');
        let start = dst.len();
        for (i, tag) in self.tags.iter().enumerate() {
            if i > 0 {
                dst.put_u8(b',');
            }
            tag.put(dst)?;
        }
        put_checksum(start, dst);
        if dst.len() - start > MAX_TAG_BLOCK {
            return Err(Error::new(ErrorKind::InvalidInput, "Tag block too long"));
        }
        dst.put_u8(b'\\');
        Ok(())
    }

    /// Parses a tag block, including its leading and closing `\`.
    pub(crate) fn decode(block: &[u8]) -> Result<Self, Error> {
        let (_, content) = validate(block)?;
        let tags = content.split(',').map(Tag::try_from).collect::<Result<_, _>>()?;
        Ok(TagBlock { tags })
    }
}

impl Tag {
    /// Writes the tag as `code:value`, escaping any text.
    fn put(&self, dst: &mut BytesMut) -> Result<(), Error> {
        let (code, value): (&str, Cow<str>) = match self {
            Tag::Time(time) => ("c", time.timestamp_millis().to_string().into()),
            Tag::Destination(destination) => ("d", destination.into()),
            Tag::Group(group) => ("g", group.to_string().into()),
            Tag::LineCount(count) => ("n", count.to_string().into()),
            Tag::RelativeTime(time) => ("r", time.to_string().into()),
            Tag::Source(source) => ("s", source.into()),
            Tag::Text(text) => ("t", text.into()),
            Tag::Unknown { code, .. } if code.contains(':') => {
                return Err(Error::new(ErrorKind::InvalidInput, "Tag code contains ':'"));
            }
            Tag::Unknown { code, value } => (code, value.into()),
        };
        put_escaped(code, dst)?;
        dst.put_u8(b':');
        put_escaped(&value, dst)
    }
}

impl TryFrom<&str> for Tag {
    type Error = Error;

    /// Parses a tag from `code:value`, replacing any `^hh` hex forms with the characters.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let invalid = |what| move |_| Error::new(ErrorKind::InvalidData, what);
        let (code, value) =
            value.split_once(':').ok_or_else(|| Error::new(ErrorKind::InvalidData, "Tag without a code"))?;
        let (code, value) = (unescape(code)?, unescape(value)?);
        Ok(match code.as_ref() {
            "c" => {
                let millis = value.parse().map_err(invalid("Invalid tag time"))?;
                Tag::Time(
                    DateTime::from_timestamp_millis(millis)
                        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid tag time"))?,
                )
            }
            "d" => Tag::Destination(value.into_owned()),
            "g" => Tag::Group(Group::try_from(value.as_ref())?),
            "n" => Tag::LineCount(value.parse().map_err(invalid("Invalid tag line count"))?),
            "r" => Tag::RelativeTime(value.parse().map_err(invalid("Invalid tag relative time"))?),
            "s" => Tag::Source(value.into_owned()),
            "t" => Tag::Text(value.into_owned()),
            _ => Tag::Unknown { code: code.into_owned(), value: value.into_owned() },
        })
    }
}

impl TryFrom<&str> for Group {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let invalid = || Error::new(ErrorKind::InvalidData, "Invalid tag group");
        let mut parts = value.split('-');
        let (Some(number), Some(total), Some(id), None) = (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        Ok(Group {
            number: number.parse().map_err(|_| invalid())?,
            total: total.parse().map_err(|_| invalid())?,
            id: id.parse().map_err(|_| invalid())?,
        })
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.number, self.total, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(block: &TagBlock) -> Result<String, Error> {
        let mut buf = BytesMut::new();
        block.encode(&mut buf)?;
        Ok(String::from_utf8(buf.to_vec()).unwrap())
    }

    #[test]
    fn decode() -> Result<(), Error> {
        let block = TagBlock::decode(b"\\g:2-3-1234,n:42,s:AI0001,c:1767225600000*0B\\")?;
        assert_eq!(
            block.tags,
            vec![
                Tag::Group(Group { number: 2, total: 3, id: 1234 }),
                Tag::LineCount(42),
                Tag::Source("AI0001".to_string()),
                Tag::Time("2026-01-01T00:00:00Z".parse().unwrap()),
            ]
        );
        Ok(())
    }

    #[test]
    fn decode_time() -> Result<(), Error> {
        // From an IEC 61162-450 capture in the tests of fkie-cad/maritime-dissector, where the time
        // matches that of the GLL sentence following it:
        // $INGLL,5416.4774,N,01201.8283,E,084836.75,A,A*7F
        let block = TagBlock::decode(b"\\s:IN0001,n:881,c:1683881316755*4D\\")?;
        let time = "2023-05-12T08:48:36.755Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(block.tags[2], Tag::Time(time));
        assert_eq!(encoded(&block)?, "\\s:IN0001,n:881,c:1683881316755*4D\\");
        Ok(())
    }

    #[test]
    fn decode_unknown() -> Result<(), Error> {
        let block = TagBlock::decode(b"\\x:1,s:AI0001*1F\\")?;
        assert_eq!(block.tags[0], Tag::Unknown { code: "x".to_string(), value: "1".to_string() });
        assert_eq!(encoded(&block)?, "\\x:1,s:AI0001*1F\\");
        Ok(())
    }

    #[test]
    fn decode_invalid() {
        // Bad checksum.
        assert!(TagBlock::decode(b"\\s:AI0001*00\\").is_err());
        // Missing checksum.
        assert!(TagBlock::decode(b"\\s:AI0001\\").is_err());
        // Missing code.
        assert!(TagBlock::decode(b"\\AI0001*09\\").is_err());
        // Invalid values.
        assert!(TagBlock::decode(b"\\c:soon*44\\").is_err());
        assert!(TagBlock::decode(b"\\g:1-2*73\\").is_err());
        assert!(TagBlock::decode(b"\\n:-1*48\\").is_err());
    }

    #[test]
    fn encode() -> Result<(), Error> {
        let block = TagBlock {
            tags: vec![
                Tag::Source("AI0001".to_string()),
                Tag::Destination("EI0002".to_string()),
                Tag::Time("2026-01-01T00:00:00Z".parse().unwrap()),
            ],
        };
        assert_eq!(encoded(&block)?, "\\s:AI0001,d:EI0002,c:1767225600000*7D\\");
        assert_eq!(TagBlock::decode(encoded(&block)?.as_bytes())?, block);

        let block = TagBlock { tags: vec![Tag::Group(Group { number: 2, total: 3, id: 1234 })] };
        assert_eq!(encoded(&block)?, "\\g:2-3-1234*58\\");
        Ok(())
    }

    #[test]
    fn escaped() -> Result<(), Error> {
        // Reserved, non-printable and non-ASCII characters are sent in their hex form.
        let block = TagBlock { tags: vec![Tag::Text("a,b*c\\d^e\nf é".to_string())] };
        assert_eq!(encoded(&block)?, "\\t:a^2Cb^2Ac^5Cd^5Ee^0Af ^E9*60\\");
        assert_eq!(TagBlock::decode(encoded(&block)?.as_bytes())?, block);

        // An incomplete or invalid hex form is invalid.
        assert!(TagBlock::decode(b"\\t:a^2*43\\").is_err());
        assert!(TagBlock::decode(b"\\t:a^ZZ*71\\").is_err());
        Ok(())
    }

    #[test]
    fn too_long() -> Result<(), Error> {
        // Up to 80 characters can be sent, including the checksum.
        let text = |len| TagBlock { tags: vec![Tag::Text("a".repeat(len))] };
        assert_eq!(encoded(&text(75))?.len(), MAX_TAG_BLOCK + 2);
        assert!(encoded(&text(76)).is_err());

        // A longer tag block is decoded but cannot be encoded.
        let block = TagBlock::decode(
            b"\\s:AI0001,x:01234567890123456789012345678901234567890123456789012345678901234567890123456789*2E\\",
        )?;
        assert!(encoded(&block).is_err());
        Ok(())
    }

    #[test]
    fn encode_invalid() {
        // A character outside ISO/IEC 8859-1 cannot be sent.
        assert!(encoded(&TagBlock { tags: vec![Tag::Text("€".to_string())] }).is_err());
        assert!(
            encoded(&TagBlock { tags: vec![Tag::Unknown { code: "x:y".to_string(), value: "1".to_string() }] })
                .is_err()
        );
    }
}
