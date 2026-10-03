//! Only the recording reader decodes admitted content codings; the raw spool stays encoded.

use std::collections::VecDeque;
use std::io::{self, BufRead, Read};

use flate2::bufread::MultiGzDecoder;
use flate2::{Decompress, FlushDecompress, Status};
use hyper::header::{GetAll, HeaderValue};
use serde_json::Value;

use super::stream::StreamReader;
use crate::record::call::Api;

const CHUNK: usize = 8192;

/// A response-side reader; unsupported or absent codings retain the raw reader.
#[derive(Debug)]
enum Coding {
    Plain(Box<StreamReader>),
    Compressed(Box<Decoded>),
}

/// A response-side reader with one recording-only decoding path.
#[derive(Debug)]
pub(super) struct Reader {
    coding: Coding,
}

impl Reader {
    pub(super) fn for_api(api: Api, encodings: GetAll<'_, HeaderValue>) -> Self {
        let reader = StreamReader::for_api(api);
        let mut values = encodings.into_iter();
        let encoding = values.next().map(HeaderValue::as_bytes);
        // Multiple codings are not admitted: none of their bytes are guessed at.
        if values.next().is_none()
            && let Some(encoding) = encoding
        {
            let encoding = encoding.trim_ascii();
            let codec = if encoding.eq_ignore_ascii_case(b"gzip") {
                Some(Codec::Gzip(Box::new(MultiGzDecoder::new(
                    Pending::default(),
                ))))
            } else if encoding.eq_ignore_ascii_case(b"deflate") {
                Some(Codec::Deflate(Box::new(Inflater {
                    codec: Decompress::new(true),
                    input: Pending::default(),
                    ended: false,
                })))
            } else {
                None
            };
            if let Some(codec) = codec {
                return Self {
                    coding: Coding::Compressed(Box::new(Decoded {
                        codec,
                        reader,
                        ended: false,
                        failure: None,
                    })),
                };
            }
        }
        Self {
            coding: Coding::Plain(Box::new(reader)),
        }
    }

    pub(super) fn feed(&mut self, bytes: &[u8]) -> io::Result<()> {
        match &mut self.coding {
            Coding::Plain(reader) => {
                reader.feed(bytes);
                Ok(())
            }
            Coding::Compressed(decoder) => decoder.feed(bytes),
        }
    }

    /// The provider's id for the message, as the stream reader read it.
    pub(super) fn message_id(&self) -> Option<&str> {
        match &self.coding {
            Coding::Plain(reader) => reader.message_id(),
            Coding::Compressed(decoder) => decoder.reader.message_id(),
        }
    }

    pub(super) fn finish(self) -> io::Result<Option<Vec<Value>>> {
        match self.coding {
            Coding::Plain(reader) => Ok((*reader).finish()),
            Coding::Compressed(mut decoder) => {
                decoder.codec.input().finished = true;
                decoder.drain()?;
                if !decoder.ended {
                    return Err(invalid("compressed response ended before its trailer"));
                }
                Ok(decoder.reader.finish())
            }
        }
    }
}

#[derive(Debug, Default)]
struct Pending {
    bytes: VecDeque<u8>,
    finished: bool,
}

impl BufRead for Pending {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.bytes.is_empty() && !self.finished {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        Ok(self.bytes.as_slices().0)
    }

    fn consume(&mut self, amount: usize) {
        drop(self.bytes.drain(..amount));
    }
}

impl Read for Pending {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if into.is_empty() {
            return Ok(0);
        }
        let bytes = self.fill_buf()?;
        let amount = bytes.len().min(into.len());
        into[..amount].copy_from_slice(&bytes[..amount]);
        self.consume(amount);
        Ok(amount)
    }
}

#[derive(Debug)]
enum Codec {
    Gzip(Box<MultiGzDecoder<Pending>>),
    Deflate(Box<Inflater>),
}

impl Codec {
    fn input(&mut self) -> &mut Pending {
        match self {
            Self::Gzip(codec) => codec.get_mut(),
            Self::Deflate(codec) => &mut codec.input,
        }
    }

    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Gzip(codec) => codec.read(output),
            Self::Deflate(codec) => codec.read(output),
        }
    }
}

#[derive(Debug)]
struct Inflater {
    codec: Decompress,
    input: Pending,
    ended: bool,
}

impl Read for Inflater {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.ended {
            return Ok(0);
        }
        loop {
            let input_before = self.codec.total_in();
            let output_before = self.codec.total_out();
            let flush = if self.input.finished {
                FlushDecompress::Finish
            } else {
                FlushDecompress::None
            };
            // Empty input can still release the codec's buffered decoded bytes.
            let status = self
                .codec
                .decompress(self.input.bytes.as_slices().0, output, flush)
                .map_err(|error| {
                    invalid(format!("deflate response could not be decoded: {error}"))
                })?;
            let consumed = usize::try_from(self.codec.total_in() - input_before)
                .map_err(|error| invalid(format!("deflate input count is too large: {error}")))?;
            let produced = usize::try_from(self.codec.total_out() - output_before)
                .map_err(|error| invalid(format!("deflate output count is too large: {error}")))?;
            self.input.consume(consumed);
            self.ended = status == Status::StreamEnd;
            if produced > 0 || self.ended {
                return Ok(produced);
            }
            if consumed == 0 {
                return if self.input.finished {
                    Err(invalid(
                        "deflate response ended before its compression trailer",
                    ))
                } else {
                    Err(io::ErrorKind::WouldBlock.into())
                };
            }
        }
    }
}

#[derive(Debug)]
struct Decoded {
    codec: Codec,
    reader: StreamReader,
    ended: bool,
    failure: Option<(io::ErrorKind, String)>,
}

impl Decoded {
    fn feed(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.check_failure()?;
        for chunk in bytes.chunks(CHUNK) {
            if self.ended {
                return self.fail(invalid("bytes follow the compression trailer"));
            }
            self.codec.input().bytes.extend(chunk);
            self.drain()?;
        }
        Ok(())
    }

    fn drain(&mut self) -> io::Result<()> {
        self.check_failure()?;
        let mut output = [0; CHUNK];
        while !self.ended {
            match self.codec.read(&mut output) {
                Ok(0) => {
                    if !self.codec.input().bytes.is_empty() {
                        return self.fail(invalid("bytes follow the compression trailer"));
                    }
                    self.ended = true;
                }
                Ok(amount) => {
                    self.reader.feed(&output[..amount]);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return self.fail(error),
            }
        }
        Ok(())
    }

    fn check_failure(&self) -> io::Result<()> {
        match &self.failure {
            Some((kind, reason)) => Err(io::Error::new(*kind, reason.clone())),
            None => Ok(()),
        }
    }

    fn fail(&mut self, error: io::Error) -> io::Result<()> {
        self.failure = Some((error.kind(), error.to_string()));
        self.codec.input().bytes.clear();
        Err(error)
    }
}

fn invalid(reason: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.into())
}

#[cfg(test)]
#[path = "decode_bound_tests.rs"]
mod tests;
