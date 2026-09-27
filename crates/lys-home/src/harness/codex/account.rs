//! The loss account written beside a rollout (HOME-009 R2): S4's written
//! account of what a translation kept, what it changed and how, and what it
//! lost and why, by entry id and hash.
//!
//! A row's `hash` is the block hash of what it is about: the SHA-256,
//! lowercase hex, of `serde_json::to_vec` of the part (or of the tool result
//! or other whole message) as the home entry holds it, the rule the Claude
//! Code render's loss account uses, never a stored block; a row about an
//! entry rather than a part has a null hash. The account measures the entry
//! as the home holds it, so it cannot name a field the importer dropped
//! before the entry was written. It holds ids, hashes, kind names, field
//! names and fixed reason words only: never a text, an argument, an output,
//! a summary, a note, an epilogue or a seed.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::HomeError;
use crate::record::blocks::{hex_of, sync_dir};

/// A part carried whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// The entry the part is in.
    pub entry: String,
    /// The part's hash.
    pub hash: Option<String>,
    /// What the home holds it as.
    pub before: String,
    /// What the rollout carries it as.
    pub after: String,
}

/// A part or entry carried with a field not carried or reshaped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Changed {
    /// The entry.
    pub entry: String,
    /// The part's hash, or null for a row about the entry.
    pub hash: Option<String>,
    /// What the home holds it as.
    pub before: String,
    /// What the rollout carries it as.
    pub after: String,
    /// Every field not carried or reshaped, joined by `; `.
    pub how: String,
}

/// A part or entry the rollout does not carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lost {
    /// The entry.
    pub entry: String,
    /// The part's or message's hash, or null for a row about the entry.
    pub hash: Option<String>,
    /// What kind of thing was lost.
    pub kind: String,
    /// Why, in the brief's fixed words.
    pub reason: String,
}

/// The three lists, each in walk order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rows {
    /// Parts carried whole.
    pub kept: Vec<Kept>,
    /// Parts and entries carried with a change.
    pub changed: Vec<Changed>,
    /// Parts and entries not carried.
    pub lost: Vec<Lost>,
}

impl Rows {
    /// Add a kept row.
    pub fn kept(&mut self, entry: &str, hash: Option<String>, before: &str, after: &str) {
        self.kept.push(Kept {
            entry: entry.to_owned(),
            hash,
            before: before.to_owned(),
            after: after.to_owned(),
        });
    }

    /// Add a changed row.
    pub fn changed(
        &mut self,
        entry: &str,
        hash: Option<String>,
        before: &str,
        after: &str,
        how: &str,
    ) {
        self.changed.push(Changed {
            entry: entry.to_owned(),
            hash,
            before: before.to_owned(),
            after: after.to_owned(),
            how: how.to_owned(),
        });
    }

    /// Add a lost row.
    pub fn lost(&mut self, entry: &str, hash: Option<String>, kind: &str, reason: &str) {
        self.lost.push(Lost {
            entry: entry.to_owned(),
            hash,
            kind: kind.to_owned(),
            reason: reason.to_owned(),
        });
    }

    /// Kept, changed and lost rows in all.
    #[must_use]
    pub fn len(&self) -> usize {
        self.kept.len() + self.changed.len() + self.lost.len()
    }

    /// Whether no row is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The account written beside a rollout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The home session's id.
    pub session: String,
    /// The head entry's id.
    pub head: String,
    /// The session's head hash at translation time, the one the marker names.
    pub head_hash: String,
    /// The Codex thread id.
    pub thread: String,
    /// The Codex version the rollout shape was recorded against.
    pub codex_version: String,
    /// The rows.
    #[serde(flatten)]
    pub rows: Rows,
}

/// The hash of a part (or whole message) as the home entry holds it: the
/// SHA-256 of its compact JSON, the bytes `serde_json::to_vec` writes.
#[must_use]
pub fn part_hash(part: &Value) -> String {
    json_hash(part)
}

/// The SHA-256 of a value's compact JSON, serialised straight into the
/// hasher. Neither half can fail for what the account hashes (a JSON value
/// or a string, into a hasher that takes every write); were one to, the row
/// would carry serde's error words where the hex belongs, never a hash of
/// other bytes.
pub(crate) fn json_hash<T: Serialize + ?Sized>(value: &T) -> String {
    let mut hasher = Sha256::new();
    serde_json::to_writer(&mut hasher, value)
        .map_or_else(|e| e.to_string(), |()| hex_of(&hasher.finalize()))
}

/// Write bytes to a path that does not yet exist, then sync the file and
/// its directory. A path already there is refused as the translation's
/// target, naming it; a file this call created and could not fill is
/// removed before the refusal returns.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), HomeError> {
    let mut file = match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(HomeError::TranslationTargetExists {
                path: path.to_path_buf(),
            });
        }
        Err(e) => return Err(HomeError::io("creating a translation file", path, e)),
    };
    let filled = file
        .write_all(bytes)
        .map_err(|e| HomeError::io("writing a translation file", path, e))
        .and_then(|()| {
            file.sync_all()
                .map_err(|e| HomeError::io("syncing a translation file", path, e))
        });
    if let Err(e) = filled {
        drop(file);
        remove_written(&[path]);
        return Err(e);
    }
    if let Some(dir) = path.parent() {
        sync_dir(dir)?;
    }
    Ok(())
}

/// Remove files a failed translation wrote, so the retry finds no target.
/// A removal that fails leaves that file standing, and the retry's refusal
/// names it; the failure that caused the removal is the one reported.
pub(crate) fn remove_written(paths: &[&Path]) {
    for path in paths {
        if std::fs::remove_file(path).is_ok()
            && let Some(dir) = path.parent()
        {
            sync_dir(dir).ok();
        }
    }
}
