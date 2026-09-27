//! The loss account written beside a Codex rollout (HOME-009 R2): what a
//! translation kept, what it changed and how, and what it lost and why, by
//! entry id and hash.
//!
//! Invariants:
//!
//! - The account is a JSON object with exactly the keys `session`, `head`,
//!   `head_hash`, `thread`, `codex_version`, `kept`, `changed` and `lost`. A
//!   kept row has exactly `entry`, `hash`, `before` and `after`; a changed
//!   row those and `how`; a lost row `entry`, `hash`, `kind` and `reason`.
//! - `hash` is the block hash of a part as the home entry holds it: the
//!   SHA-256, lowercase hex, of `serde_json::to_vec` of the part's JSON value
//!   ([`part_hash`]), the rule the Claude Code render's loss account uses.
//!   No stored block is read. A row about a whole entry rather than a part
//!   has `hash` null.
//! - The account measures the entry as the home holds it, so it cannot name
//!   a field the importer dropped before the entry was written: a
//!   `tool_use`'s `caller` field is dropped at import and recorded nowhere
//!   yet (the board 5 card "The importer records every field it drops").
//! - Rows are in walk order within each list. Nothing of the content enters
//!   a row: only ids, hashes, kind names, field names and fixed reasons.
//! - The account is written with `create_new` and synced before the
//!   translation reports; an existing path is refused by name.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::HomeError;
use crate::record::blocks::Hash;

/// A part carried whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// The entry the part is in.
    pub entry: String,
    /// The part's hash as the entry holds it.
    pub hash: Option<String>,
    /// The part's kind in the home.
    pub before: String,
    /// The item's kind in the rollout.
    pub after: String,
}

/// A part or entry carried with a field not carried or reshaped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Changed {
    /// The entry.
    pub entry: String,
    /// The part's hash, or null for a row about the whole entry.
    pub hash: Option<String>,
    /// The kind in the home.
    pub before: String,
    /// The kind in the rollout.
    pub after: String,
    /// Every field not carried or reshaped, joined by `; `.
    pub how: String,
}

/// A part or entry not carried.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lost {
    /// The entry.
    pub entry: String,
    /// The part's hash, or null for a row about the whole entry.
    pub hash: Option<String>,
    /// The kind of what was lost.
    pub kind: String,
    /// Why, naming no content.
    pub reason: String,
}

/// The three lists, filled in walk order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    /// Kept rows.
    pub kept: Vec<Kept>,
    /// Changed rows.
    pub changed: Vec<Changed>,
    /// Lost rows.
    pub lost: Vec<Lost>,
}

impl Rows {
    /// Record a part carried whole.
    pub fn keep(&mut self, entry: &str, hash: Option<String>, before: &str, after: &str) {
        self.kept.push(Kept {
            entry: entry.to_owned(),
            hash,
            before: before.to_owned(),
            after: after.to_owned(),
        });
    }

    /// Record a part or entry carried with changes.
    pub fn change(
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

    /// Record a part or entry not carried.
    pub fn lose(&mut self, entry: &str, hash: Option<String>, kind: &str, reason: &str) {
        self.lost.push(Lost {
            entry: entry.to_owned(),
            hash,
            kind: kind.to_owned(),
            reason: reason.to_owned(),
        });
    }

    /// Append another set of rows after these, each list in its order.
    pub fn extend(&mut self, other: Rows) {
        self.kept.extend(other.kept);
        self.changed.extend(other.changed);
        self.lost.extend(other.lost);
    }
}

/// The loss account beside one rollout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// The home session's id.
    pub session: String,
    /// The head entry's id.
    pub head: String,
    /// The session head hash at translation time.
    pub head_hash: String,
    /// The Codex thread id.
    pub thread: String,
    /// The Codex version the rollout shape was measured against.
    pub codex_version: String,
    /// Parts carried whole.
    pub kept: Vec<Kept>,
    /// Parts and entries carried with changes.
    pub changed: Vec<Changed>,
    /// Parts and entries not carried.
    pub lost: Vec<Lost>,
}

/// The block hash of a part as the entry holds it.
pub fn part_hash(part: &Value) -> Result<String, HomeError> {
    let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
        context: "a part could not be serialised for its hash",
        source,
    })?;
    Ok(Hash::of(&bytes).to_string())
}

/// The bytes an account is written as.
pub fn account_bytes(account: &Account) -> Result<Vec<u8>, HomeError> {
    let mut bytes = serde_json::to_vec_pretty(account).map_err(|source| HomeError::Json {
        context: "the loss account could not be serialised",
        source,
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Write bytes to a path that does not yet exist, and sync them. An
/// existing path is refused as the translation's target.
pub fn write_new(path: &Path, bytes: &[u8], what: &'static str) -> Result<(), HomeError> {
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
        Err(e) => return Err(HomeError::io(what, path, e)),
    };
    file.write_all(bytes)
        .map_err(|e| HomeError::io(what, path, e))?;
    file.sync_all().map_err(|e| HomeError::io(what, path, e))
}
