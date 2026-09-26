//! The seed prompt of a carried user message, and the argument that passes
//! it on the template's launch line (HOME-006 R6).
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
//! every part that is not text left out, as `seed_left_out` counts them; a
//! part with no type, a text part with no text, content that is neither a
//! string nor a list, and a `lys.forked_from` whose `coordinate_carried`
//! and `carried` disagree are each refused by name),
//! with no trailing newline. The render report names the seed file and
//! prints no launch line: only the template's render (`render-launch`)
//! prints one, with `"$(cat '<seed>')"` appended as the resumed session's
//! first prompt, and never runs it (ADR-007). The seed's text never enters
//! the rendered JSONL, the loss
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
/// otherwise its text parts in order joined by newlines. A message whose
/// content is neither, a part with no `type` string, and a text part with no
/// `text` string are each refused by name rather than read as empty.
pub(crate) fn text_of(entry: &Entry) -> Result<String, HomeError> {
    let shape = |reason| HomeError::BodyShape {
        api: "seed",
        reason,
    };
    let EntryBody::Message { message } = &entry.body else {
        return Err(shape("the carried entry is not a message"));
    };
    match message.get("content") {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Array(parts)) => {
            let mut texts = Vec::new();
            for part in parts {
                let kind = part
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| shape("a part of the carried message has no type"))?;
                if kind != "text" {
                    continue;
                }
                let text = part
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| shape("a text part of the carried message has no text"))?;
                texts.push(text);
            }
            Ok(texts.join("\n"))
        }
        _ => Err(shape(
            "the carried message's content is neither a string nor a list of parts",
        )),
    }
}

/// The `sessions/` directory a session file stands in; a file with no
/// directory would resolve against lys-home's own working directory, which
/// is never the home's, and is refused by name.
pub(crate) fn sessions_dir_of(file: &Path) -> Result<&Path, HomeError> {
    match file.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => Ok(dir),
        _ => Err(HomeError::NotAbsolute {
            what: "the session file",
            shape: "without a directory",
            path: file.to_path_buf(),
        }),
    }
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
    let carried = match (forked.coordinate_carried, forked.carried) {
        (false, None) => return Ok(None),
        (true, Some(carried)) => carried,
        _ => {
            return Err(HomeError::EntryShape {
                session: session.header().id.clone(),
                id: entry.id().to_owned(),
                custom_type: CUSTOM_FORKED_FROM.to_owned(),
                source: None,
            });
        }
    };
    safe_component("session id", &forked.parent_session)?;
    let sessions = sessions_dir_of(session.file())?;
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
