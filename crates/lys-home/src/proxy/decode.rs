//! Only the recording reader decodes admitted content codings; the raw spool stays encoded.

use std::io::{self, Write};

use flate2::write::MultiGzDecoder;
use flate2::{Decompress, FlushDecompress, Status};
use hyper::header::{GetAll, HeaderValue};
use serde_json::Value;

use super::stream::StreamReader;
use crate::record::call::Api;

#[derive(Debug)]
struct Feed(StreamReader);

impl Write for Feed {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.feed(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A response-side reader; unsupported or absent codings retain the raw reader.
#[derive(Debug)]
enum Coding {
    Plain(Box<StreamReader>),
    Gzip(Box<MultiGzDecoder<Feed>>),
    Deflate(Box<Inflater>),
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
            if encoding.eq_ignore_ascii_case(b"gzip") {
                return Self {
                    coding: Coding::Gzip(Box::new(MultiGzDecoder::new(Feed(reader)))),
                };
            }
            if encoding.eq_ignore_ascii_case(b"deflate") {
                return Self {
                    coding: Coding::Deflate(Box::new(Inflater {
                        codec: Decompress::new(true),
                        reader,
                        ended: false,
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
            Coding::Gzip(decoder) => decoder.write_all(bytes),
            Coding::Deflate(decoder) => decoder.decode(bytes, FlushDecompress::None),
        }
    }

    pub(super) fn finish(self) -> io::Result<Option<Vec<Value>>> {
        match self.coding {
            Coding::Plain(reader) => Ok((*reader).finish()),
            Coding::Gzip(decoder) => Ok((*decoder).finish()?.0.finish()),
            Coding::Deflate(mut decoder) => {
                decoder.decode(&[], FlushDecompress::Finish)?;
                if !decoder.ended {
                    return Err(invalid(
                        "deflate response ended before its compression trailer",
                    ));
                }
                Ok(decoder.reader.finish())
            }
        }
    }
}

#[derive(Debug)]
struct Inflater {
    codec: Decompress,
    reader: StreamReader,
    ended: bool,
}

impl Inflater {
    fn decode(&mut self, mut bytes: &[u8], flush: FlushDecompress) -> io::Result<()> {
        let mut output = [0; 8192];
        loop {
            if self.ended {
                return if bytes.is_empty() {
                    Ok(())
                } else {
                    Err(invalid("bytes follow the deflate compression trailer"))
                };
            }
            let input_before = self.codec.total_in();
            let output_before = self.codec.total_out();
            let status = self
                .codec
                .decompress(bytes, &mut output, flush)
                .map_err(|error| {
                    invalid(format!("deflate response could not be decoded: {error}"))
                })?;
            let consumed = usize::try_from(self.codec.total_in() - input_before)
                .map_err(|error| invalid(format!("deflate input count is too large: {error}")))?;
            let produced = usize::try_from(self.codec.total_out() - output_before)
                .map_err(|error| invalid(format!("deflate output count is too large: {error}")))?;
            self.reader.feed(&output[..produced]);
            bytes = &bytes[consumed..];
            self.ended = status == Status::StreamEnd;
            if self.ended {
                continue;
            }
            if consumed == 0 && produced == 0 {
                return if bytes.is_empty() {
                    Ok(())
                } else {
                    Err(invalid("deflate response could not consume its next bytes"))
                };
            }
            if bytes.is_empty() && produced < output.len() {
                return Ok(());
            }
        }
    }
}

fn invalid(reason: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.into())
}
