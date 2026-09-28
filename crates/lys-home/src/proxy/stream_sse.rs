//! Server-sent events framing: events and their data read out of a byte
//! stream as it passes, in the order the bytes came.
//!
//! Invariants:
//! - The framer observes and never changes a byte: the client receives what
//!   the upstream sent, whatever is read from it here, so byte order,
//!   encoding and anything after the last event pass untouched.
//! - A line ends at `\n`, `\r\n` or a lone `\r`, and may be split across
//!   chunks at any byte; an event ends at a blank line; its `data` lines join
//!   with `\n`. Comment lines (`:`) and fields other than `event` and `data`
//!   are passed over.
//! - A line that is not UTF-8 marks the stream malformed rather than being
//!   replaced, and a stream that stops inside a line or an event is pending,
//!   so neither is ever read as a stream that ended whole.

/// One event: its `event` field when it had one, and its data lines joined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseEvent {
    /// The `event` field.
    pub event: Option<String>,
    /// The `data` lines, joined with `\n`.
    pub data: String,
}

/// The framer's state between chunks.
#[derive(Debug, Default)]
pub struct SseFramer {
    line: Vec<u8>,
    after_cr: bool,
    event: Option<String>,
    data: Option<String>,
    malformed: bool,
}

impl SseFramer {
    /// A framer at the start of a stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the next bytes; every event they complete is pushed to `out`.
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<SseEvent>) {
        for &b in bytes {
            if self.after_cr {
                self.after_cr = false;
                if b == b'\n' {
                    continue;
                }
            }
            match b {
                b'\n' => self.end_line(out),
                b'\r' => {
                    self.after_cr = true;
                    self.end_line(out);
                }
                other => self.line.push(other),
            }
        }
    }

    /// Whether a line was not UTF-8.
    #[must_use]
    pub const fn malformed(&self) -> bool {
        self.malformed
    }

    /// Whether the bytes so far stop inside a line or an event.
    #[must_use]
    pub fn pending(&self) -> bool {
        !self.line.is_empty() || self.data.is_some() || self.event.is_some()
    }

    fn end_line(&mut self, out: &mut Vec<SseEvent>) {
        let line = std::mem::take(&mut self.line);
        if line.is_empty() {
            match self.data.take() {
                Some(data) => out.push(SseEvent {
                    event: self.event.take(),
                    data,
                }),
                None => self.event = None,
            }
            return;
        }
        let Ok(text) = String::from_utf8(line) else {
            self.malformed = true;
            return;
        };
        if text.starts_with(':') {
            return;
        }
        let (field, value) = text.split_once(':').map_or((text.as_str(), ""), |(f, v)| {
            (f, v.strip_prefix(' ').unwrap_or(v))
        });
        match field {
            "data" => match &mut self.data {
                Some(data) => {
                    data.push('\n');
                    data.push_str(value);
                }
                None => self.data = Some(value.to_owned()),
            },
            "event" => self.event = Some(value.to_owned()),
            _ => {}
        }
    }
}
