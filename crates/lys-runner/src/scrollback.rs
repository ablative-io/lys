//! A session's scrollback: the last bytes of its output, up to a size in
//! bytes, each byte at a cursor that never changes.
//!
//! A cursor is the count of bytes the session has output before it, so the
//! same cursor names the same byte for the session's whole life. When the
//! scrollback is full the oldest bytes go first; a read from a cursor older
//! than the oldest kept is refused `cursor_expired`, naming the oldest.

use std::collections::VecDeque;

use crate::error::RunnerError;

/// The last bytes of a session's output.
#[derive(Debug, Clone)]
pub struct Scrollback {
    bytes: VecDeque<u8>,
    oldest: u64,
    limit: usize,
}

impl Scrollback {
    /// An empty scrollback keeping at most `limit` bytes; a runner refuses
    /// a limit of none before any session is kept ([`crate::Sessions::open`]).
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: VecDeque::new(),
            oldest: 0,
            limit,
        }
    }

    /// The oldest cursor kept.
    pub fn oldest(&self) -> u64 {
        self.oldest
    }

    /// The cursor after the last byte.
    pub fn end(&self) -> u64 {
        self.oldest + self.bytes.len() as u64
    }

    /// Keep `chunk`, letting the oldest bytes go past the limit.
    pub fn push(&mut self, chunk: &[u8]) {
        self.bytes.extend(chunk);
        let over = self.bytes.len().saturating_sub(self.limit);
        if over > 0 {
            self.bytes.drain(..over);
            self.oldest += over as u64;
        }
    }

    /// The bytes from `cursor` to the end, refused `cursor_expired` when the
    /// cursor is older than the oldest kept.
    pub fn from(&self, cursor: u64) -> Result<Vec<u8>, RunnerError> {
        if cursor < self.oldest {
            return Err(self.expired(cursor));
        }
        let skip = usize::try_from(cursor - self.oldest).unwrap_or(usize::MAX);
        Ok(self.bytes.iter().skip(skip).copied().collect())
    }

    /// The refusal for a read from `cursor`, older than the oldest kept.
    pub fn expired(&self, cursor: u64) -> RunnerError {
        RunnerError::Refused {
            refusal: "cursor_expired".to_owned(),
            words: format!(
                "cursor {cursor} is older than the scrollback keeps; the oldest cursor held is {}",
                self.oldest
            ),
            oldest: Some(self.oldest),
        }
    }

    /// The cursor the last `lines` lines begin at. A newline that ends the
    /// output does not begin a line of its own.
    pub fn last_lines(&self, lines: u32) -> u64 {
        if lines == 0 {
            return self.end();
        }
        let mut end = self.bytes.len();
        if self.bytes.back() == Some(&b'\n') {
            end -= 1;
        }
        let mut seen = 0;
        for index in (0..end).rev() {
            if self.bytes.get(index) == Some(&b'\n') {
                seen += 1;
                if seen == lines {
                    return self.oldest + index as u64 + 1;
                }
            }
        }
        self.oldest
    }

    /// The cursor the last `bytes` bytes begin at.
    pub fn last_bytes(&self, bytes: u64) -> u64 {
        self.end().saturating_sub(bytes).max(self.oldest)
    }
}

/// How many bytes at the end of `bytes` begin a character they do not
/// finish, so a read stops before them and gives them whole next time.
pub fn unfinished(bytes: &[u8]) -> usize {
    for back in 1..=bytes.len().min(4) {
        let byte = bytes[bytes.len() - back];
        if byte & 0b1100_0000 == 0b1000_0000 {
            continue;
        }
        let needs = match byte {
            0b1100_0000..=0b1101_1111 => 2,
            0b1110_0000..=0b1110_1111 => 3,
            0b1111_0000..=0b1111_0111 => 4,
            _ => 1,
        };
        return if needs > back { back } else { 0 };
    }
    0
}

/// `bytes` as text, whole characters only: the unfinished character at the
/// end is left for the next read, and the count of bytes given is answered.
pub fn whole_text(bytes: &[u8]) -> (String, usize) {
    let given = bytes.len() - unfinished(bytes);
    (String::from_utf8_lossy(&bytes[..given]).into_owned(), given)
}
