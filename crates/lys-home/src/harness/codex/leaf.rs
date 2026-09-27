//! The `lys.translation` side leaf (HOME-009 R6): the durable link from a
//! Codex fork back to the session, which a Codex compaction cannot erase.
//!
//! Invariants:
//!
//! - One leaf per translation, appended through
//!   [`Session::append_beside`] after the rollout and the account are both
//!   written and synced, and never when a write before it failed. The head
//!   does not move and no earlier byte of the session file changes, so the
//!   Claude Code render of the session is unchanged.
//! - The data holds exactly `harness`, `codex_version`, `thread`, `head`,
//!   `head_hash`, `rollout` (relative to `--out`), `rollout_sha256` and
//!   `account_sha256`: no absolute path and no content.
//! - Being a custom entry off the path under the head, a leaf is listed lost
//!   by the next translation of the same head.

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::Session;
use crate::record::entries::{CUSTOM_TRANSLATION, EntryBody};

/// What a `lys.translation` entry carries in `custom.data`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationData {
    /// The harness translated to: `codex`.
    pub harness: String,
    /// The Codex version the rollout shape was measured against.
    pub codex_version: String,
    /// The Codex thread id.
    pub thread: String,
    /// The head entry's id.
    pub head: String,
    /// The session head hash before the leaf was appended.
    pub head_hash: String,
    /// The rollout's path relative to `--out`, `/`-separated.
    pub rollout: String,
    /// The SHA-256 of the rollout's bytes.
    pub rollout_sha256: String,
    /// The SHA-256 of the account's bytes.
    pub account_sha256: String,
}

/// Append the side leaf beside the context path; returns its entry id.
pub(crate) fn append_leaf(
    session: &mut Session,
    data: &TranslationData,
) -> Result<String, HomeError> {
    let value = serde_json::to_value(data).map_err(|source| HomeError::Json {
        context: "the translation leaf's data could not be serialised",
        source,
    })?;
    session.append_beside(EntryBody::Custom {
        custom_type: CUSTOM_TRANSLATION.to_owned(),
        data: Some(value),
    })
}
