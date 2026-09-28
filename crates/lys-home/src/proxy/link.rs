//! How a call is linked to a session: by the key the request itself carries,
//! and by nothing else.
//!
//! Claude Code names its session in the request body's `metadata.user_id`.
//! Two spellings of that value are read: the string
//! `user_<hex>_account_<uuid>_session_<uuid>`, whose last `_session_` part is
//! the session id, and a JSON object encoded as a string whose `session_id`
//! member is. Which spelling 2.1.281 sends is a measurement PROOF-PROXY.md
//! records; until it is recorded there, both are read and nothing else is.
//! A key that is not one safe path component links nothing.
//!
//! Invariants:
//! - A call is linked only by the key in its own body. No session is inferred
//!   from timing, the client's address, a working directory or an earlier
//!   call: a call without a key is [`Link::Unlinked`], whatever came before.
//! - Headers are never read here. The key is taken from the body.
//! - The body is read as it passes, by [`KeyScanner`], whose memory is
//!   bounded ([`METADATA_BUDGET`] bytes of the `metadata` value and a short
//!   key buffer) however large the body is, so the key is known on the
//!   forwarding path even when no capture slot is free.

use serde_json::Value;

use crate::record::safe_component;

/// The most bytes of the top-level `metadata` value the scanner keeps. A
/// longer value links nothing.
pub const METADATA_BUDGET: usize = 4096;

/// The longest top-level key the scanner compares; `metadata` is 8 bytes.
const KEY_BUDGET: usize = 16;

/// Which session a call belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// The call carried this session's key.
    Session(String),
    /// The call carried no key that names a session.
    Unlinked,
}

impl Link {
    /// The home session a call of this link is recorded under: the linked
    /// session's own id, or `unlinked-<day>` for the day (`YYYY-MM-DD`) the
    /// call started.
    #[must_use]
    pub fn session_id(&self, day: &str) -> String {
        match self {
            Self::Session(id) => id.clone(),
            Self::Unlinked => format!("unlinked-{day}"),
        }
    }

    /// Whether the call carried a key.
    #[must_use]
    pub const fn is_linked(&self) -> bool {
        matches!(self, Self::Session(_))
    }

    /// The link a journal record names: a session id, or none.
    #[must_use]
    pub fn from_record(session: Option<&str>) -> Self {
        session
            .filter(|id| safe_component("session key", id).is_ok())
            .map_or(Self::Unlinked, |id| Self::Session(id.to_owned()))
    }

    /// The session id a journal record keeps for this link.
    #[must_use]
    pub fn to_record(&self) -> Option<String> {
        match self {
            Self::Session(id) => Some(id.clone()),
            Self::Unlinked => None,
        }
    }
}

/// The day (`YYYY-MM-DD`) of an RFC 3339 time, as `unlinked-<day>` names it.
#[must_use]
pub fn day_of(rfc3339: &str) -> &str {
    rfc3339.get(..10).unwrap_or(rfc3339)
}

/// The session id in a `metadata.user_id` value, in either spelling read.
#[must_use]
pub fn session_key(user_id: &str) -> Option<String> {
    let candidate = if user_id.trim_start().starts_with('{') {
        let object: Value = serde_json::from_str(user_id).ok()?;
        object.get("session_id")?.as_str()?.to_owned()
    } else {
        let (_, session) = user_id.rsplit_once("_session_")?;
        session.to_owned()
    };
    safe_component("session key", &candidate)
        .is_ok()
        .then_some(candidate)
}

/// The link a parsed `metadata` value gives.
#[must_use]
pub fn link_of_metadata(metadata: &Value) -> Link {
    metadata
        .get("user_id")
        .and_then(Value::as_str)
        .and_then(session_key)
        .map_or(Link::Unlinked, Link::Session)
}

/// Reads a JSON body as it passes and keeps only its top-level `metadata`
/// value, up to [`METADATA_BUDGET`] bytes. Strings, escapes and nesting are
/// followed so a `metadata` key inside a message is never mistaken for the
/// top-level one.
#[derive(Debug, Default)]
pub struct KeyScanner {
    depth: u32,
    in_string: bool,
    escaped: bool,
    expecting_key: bool,
    in_key: bool,
    key: Vec<u8>,
    key_over: bool,
    metadata_key: bool,
    capturing: bool,
    captured: Vec<u8>,
    captured_over: bool,
    done: bool,
}

impl KeyScanner {
    /// A scanner at the start of a body.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the next bytes of the body.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.step(b);
        }
    }

    fn step(&mut self, b: u8) {
        if self.capturing {
            if self.captured.len() < METADATA_BUDGET {
                self.captured.push(b);
            } else {
                self.captured_over = true;
            }
        }
        if self.in_string {
            self.string_byte(b);
            return;
        }
        match b {
            b'"' => {
                self.in_string = true;
                if self.depth == 1 && self.expecting_key {
                    self.expecting_key = false;
                    self.in_key = true;
                    self.key.clear();
                    self.key_over = false;
                }
            }
            b'{' => {
                self.depth += 1;
                if self.depth == 1 {
                    self.expecting_key = true;
                }
            }
            b'[' => self.depth += 1,
            b'}' | b']' => {
                if self.depth == 1 && self.capturing {
                    self.stop_capture();
                }
                self.depth = self.depth.saturating_sub(1);
            }
            b',' if self.depth == 1 => {
                if self.capturing {
                    self.stop_capture();
                }
                self.expecting_key = true;
            }
            b':' if self.depth == 1 && self.metadata_key => {
                self.metadata_key = false;
                self.capturing = true;
                self.captured.clear();
            }
            _ => {}
        }
    }

    fn string_byte(&mut self, b: u8) {
        if self.escaped {
            self.escaped = false;
            self.key_byte(b);
        } else if b == b'\\' {
            self.escaped = true;
            self.key_byte(b);
        } else if b == b'"' {
            self.in_string = false;
            if self.in_key {
                self.in_key = false;
                self.metadata_key = !self.done && !self.key_over && self.key == b"metadata";
            }
        } else {
            self.key_byte(b);
        }
    }

    fn key_byte(&mut self, b: u8) {
        if !self.in_key {
            return;
        }
        if self.key.len() < KEY_BUDGET {
            self.key.push(b);
        } else {
            self.key_over = true;
        }
    }

    fn stop_capture(&mut self) {
        self.capturing = false;
        // The byte that ended the value (`,` or `}`) was kept; it is not the value's.
        self.captured.pop();
        self.done = true;
    }

    /// The link the body read so far gives: its top-level `metadata` value's
    /// key when that value was read whole within the budget, else unlinked.
    #[must_use]
    pub fn link(&self) -> Link {
        if !self.done || self.captured_over {
            return Link::Unlinked;
        }
        serde_json::from_slice::<Value>(&self.captured)
            .map_or(Link::Unlinked, |metadata| link_of_metadata(&metadata))
    }
}
