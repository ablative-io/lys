//! The content codings a stored body is decoded from.

use std::io::Read;

use super::captured::{Head, Side};

/// A content coding a stored response is decoded from: the three the
/// proxy's stream reader decodes (`proxy::decode`).
#[derive(Clone, Copy)]
pub(super) enum Coding {
    Gzip,
    Deflate,
    Brotli,
}

impl Coding {
    /// The one coding the response's head names, when it is one of the
    /// three. More than one coding, or any other, is none: none of those
    /// bytes are guessed at.
    pub(super) fn of(head: &Head) -> Option<Self> {
        Self::of_side(&head.response)
    }

    /// The one coding one side of a head names, read as [`Self::of`] reads
    /// the response's.
    pub(super) fn of_side(side: &Side) -> Option<Self> {
        let [one] = side.values.get("content-encoding")?.as_slice() else {
            return None;
        };
        let one = one.trim();
        [
            ("gzip", Self::Gzip),
            ("deflate", Self::Deflate),
            ("br", Self::Brotli),
        ]
        .into_iter()
        .find_map(|(name, coding)| one.eq_ignore_ascii_case(name).then_some(coding))
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
            Self::Brotli => "br",
        }
    }

    /// The whole of `stored`, decoded. The stored bytes stay as they came.
    pub(super) fn decode(self, stored: &[u8]) -> std::io::Result<Vec<u8>> {
        // The decoder's working buffer, not a bound on what it decodes.
        const BUFFER: usize = 8192;
        let mut bytes = Vec::new();
        match self {
            Self::Gzip => flate2::read::MultiGzDecoder::new(stored).read_to_end(&mut bytes)?,
            Self::Deflate => flate2::read::ZlibDecoder::new(stored).read_to_end(&mut bytes)?,
            Self::Brotli => {
                brotli_decompressor::Decompressor::new(stored, BUFFER).read_to_end(&mut bytes)?
            }
        };
        Ok(bytes)
    }
}
