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
//! - The body is scanned once as it passes. The complete top-level metadata
//!   value is kept until the request ends, then parsed once and released.

use serde_json::Value;

use crate::record::safe_component;

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
/// value. Strings, escapes and nesting are
/// followed so a `metadata` key inside a message is never mistaken for the
/// top-level one.
#[derive(Debug, Default)]
pub struct KeyScanner {
    depth: u32,
    in_string: bool,
    escaped: bool,
    expecting_key: bool,
    in_key: bool,
    key_offset: usize,
    key_matches: bool,
    metadata_key: bool,
    capturing: bool,
    captured: Vec<u8>,
    resolved: Option<Link>,
    done: bool,
}

impl KeyScanner {
    /// A scanner at the start of a body.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gate: scans the request frame until the complete metadata value is kept.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.done {
                break;
            }
            self.step(b);
        }
    }

    fn step(&mut self, b: u8) {
        if self.capturing {
            self.captured.push(b);
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
                    self.key_offset = 0;
                    self.key_matches = true;
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
                self.metadata_key = self.key_matches && self.key_offset == b"metadata".len();
            }
        } else {
            self.key_byte(b);
        }
    }

    fn key_byte(&mut self, b: u8) {
        // Compare the field name without retaining unrelated keys.
        if self.in_key && self.key_matches {
            self.key_matches = b"metadata".get(self.key_offset) == Some(&b);
            self.key_offset += usize::from(self.key_matches);
        }
    }

    fn stop_capture(&mut self) {
        self.capturing = false;
        // The byte that ended the value (`,` or `}`) was kept; it is not the value's.
        self.captured.pop();
        self.done = true;
    }

    /// Gate: parses a completed metadata value once, keeping only its link.
    #[must_use]
    pub fn link(&mut self) -> Link {
        if !self.done {
            return Link::Unlinked;
        }
        if self.resolved.is_none() {
            self.resolved = Some(
                serde_json::from_slice::<Value>(&self.captured)
                    .map_or(Link::Unlinked, |metadata| link_of_metadata(&metadata)),
            );
            self.captured = Vec::new();
        }
        self.resolved.clone().unwrap_or(Link::Unlinked)
    }
}
