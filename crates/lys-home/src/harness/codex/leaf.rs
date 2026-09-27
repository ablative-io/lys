//! The side leaf a translation hangs beside the context path (HOME-009 R6).
//!
//! Once the rollout and the account are both written and synced, one
//! `lys.translation` custom entry is appended as a child of the head through
//! [`Session::append_beside`], so the head does not move, no earlier byte of
//! the session file changes and the Claude Code render of the session is
//! unchanged. Its data names the translation by hashes and ids only: the
//! harness, the Codex version, the thread, the head and its hash before the
//! append, the rollout's path relative to `--out`, and the SHA-256 of the
//! rollout and of the account. With the account, it is the durable link from
//! the Codex fork back to the session, which a Codex compaction cannot erase.

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::Session;
use crate::record::entries::{CUSTOM_TRANSLATION, EntryBody};

/// The harness a translation is for, as the leaf names it.
pub const HARNESS: &str = "codex";

/// What a `lys.translation` entry carries in `custom.data`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationData {
    /// The harness translated for: `codex`.
    pub harness: String,
    /// The Codex version the rollout shape was recorded against.
    pub codex_version: String,
    /// The Codex thread id.
    pub thread: String,
    /// The head entry's id.
    pub head: String,
    /// The session's head hash before the leaf was appended.
    pub head_hash: String,
    /// The rollout's path relative to `--out`, `/`-separated.
    pub rollout: String,
    /// The SHA-256 of the rollout's bytes.
    pub rollout_sha256: String,
    /// The SHA-256 of the account's bytes.
    pub account_sha256: String,
}

/// Append the leaf beside the context path; returns its entry id.
pub fn append_leaf(session: &mut Session, data: &TranslationData) -> Result<String, HomeError> {
    let value = serde_json::to_value(data).map_err(|source| HomeError::Json {
        context: "the translation's side leaf could not be serialised",
        source,
    })?;
    session.append_beside(EntryBody::Custom {
        custom_type: CUSTOM_TRANSLATION.to_owned(),
        data: Some(value),
    })
}
