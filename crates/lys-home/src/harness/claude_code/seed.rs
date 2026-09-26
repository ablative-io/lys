//! The seed prompt of a carried user message, and the launch line that
//! passes it (HOME-006 R6).
//!
//! A child forked at a user message does not hold that message: the cut
//! stopped at the assistant message before it, and the child's
//! `lys.forked_from` entry names it as the carried entry. When such a child
//! is rendered for Claude Code, the carried entry is read from the parent
//! session named by `parent_session`, through the read-only reader by its
//! index row, and its text is written beside the rendered file as
//! `<rendered stem>.seed.txt`: the in-band marker line naming the parent
//! session, the point and the lantern, a newline, the heading line, a
//! newline, then the message's text (its `content` when that is a string;
//! otherwise the `text` of each `text` part in order, joined by newlines,
//! every part that is not text left out, as `seed_left_out` counts them),
//! with no trailing newline. The launch line is `claude --resume '<path>'`,
//! and with a seed `claude --resume '<path>' "$(cat '<seed>')"`, the seed
//! being the resumed session's first prompt; it is printed, never run
//! (ADR-007). The seed's text never enters the rendered JSONL, the loss
//! account or the report, and a seed path that already exists is refused by
//! name before anything is written.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::HomeError;
use crate::record::entries::{CUSTOM_FORKED_FROM, Entry, EntryBody};
use crate::record::fork::ForkedFrom;
use crate::record::lantern::data_of;
use crate::record::reader::SessionReader;
use crate::record::{Session, safe_component};

/// The heading line under the marker.
pub const HEADING: &str = "The message at the coordinate, which the transcript does not hold";

/// The seed a render writes: where the child came from, and the carried
/// message's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seed {
    /// The parent session's id.
    pub parent: String,
    /// The lantern's point, the carried entry.
    pub point: String,
    /// The lantern's entry id.
    pub lantern: String,
    /// The carried message's text.
    pub text: String,
}

impl Seed {
    /// The in-band marker line.
    #[must_use]
    pub fn marker(&self) -> String {
        format!(
            "<FORKED FROM SESSION {} AT ENTRY {} BY LANTERN {}>",
            self.parent, self.point, self.lantern
        )
    }

    /// The seed file's bytes: the marker, the heading and the text, each
    /// line but the last ending in a newline.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        format!("{}\n{HEADING}\n{}", self.marker(), self.text).into_bytes()
    }
}

/// The text of a carried message: its content when that is a string,
/// otherwise its text parts in order joined by newlines.
fn text_of(entry: &Entry) -> Result<String, HomeError> {
    let EntryBody::Message { message } = &entry.body else {
        return Err(HomeError::BodyShape {
            api: "seed",
            reason: "the carried entry is not a message",
        });
    };
    Ok(match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<&str>>()
            .join("\n"),
        _ => String::new(),
    })
}

/// The seed the session's path calls for: `None` unless the path carries a
/// `lys.forked_from` entry with `coordinate_carried` true, in which case the
/// carried entry is read from the parent session beside this one.
pub fn seed_of(session: &Session, path: &[Entry]) -> Result<Option<Seed>, HomeError> {
    let Some(entry) = path
        .iter()
        .rev()
        .find(|entry| entry.is_custom(CUSTOM_FORKED_FROM))
    else {
        return Ok(None);
    };
    let forked: ForkedFrom = data_of(&session.header().id, entry, CUSTOM_FORKED_FROM)?;
    let Some(carried) = forked.carried.filter(|_| forked.coordinate_carried) else {
        return Ok(None);
    };
    safe_component("session id", &forked.parent_session)?;
    let sessions = session.file().parent().unwrap_or_else(|| Path::new("."));
    let parent = SessionReader::open(sessions.join(format!("{}.jsonl", forked.parent_session)))?;
    let text = text_of(&parent.entry(&carried)?)?;
    Ok(Some(Seed {
        parent: forked.parent_session,
        point: forked.point,
        lantern: forked.lantern,
        text,
    }))
}

/// The seed file beside a rendered file: `<stem>.seed.txt`.
#[must_use]
pub fn seed_path(rendered: &Path) -> PathBuf {
    rendered.with_extension("seed.txt")
}

/// Write the seed to a path that does not yet exist, and sync it.
pub fn write_seed(path: &Path, seed: &Seed) -> Result<(), HomeError> {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| HomeError::io("creating the seed file", path, e))?;
    file.write_all(&seed.bytes())
        .map_err(|e| HomeError::io("writing the seed file", path, e))?;
    file.sync_all()
        .map_err(|e| HomeError::io("syncing the seed file", path, e))
}

/// A path single-quoted for a POSIX shell, a quote inside written as `'\''`.
#[must_use]
pub fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

/// The seed as the launch line's first prompt: `"$(cat '<seed>')"`.
#[must_use]
pub fn seed_argument(seed: &Path) -> String {
    format!("\"$(cat {})\"", quoted(seed))
}

/// The resume-by-path launch line, with the seed as its first prompt when
/// there is one. Never run here.
#[must_use]
pub fn resume_line(rendered: &Path, seed: Option<&Path>) -> String {
    match seed {
        Some(seed) => format!(
            "claude --resume {} {}",
            quoted(rendered),
            seed_argument(seed)
        ),
        None => format!("claude --resume {}", quoted(rendered)),
    }
}
