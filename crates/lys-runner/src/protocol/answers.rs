//! Typed answers to authenticated runner requests.

use super::super::{Ended, Output, StatusView};
use crate::error::RunnerError;
use serde::{Deserialize, Serialize};

/// An answer to one act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Answer {
    /// Every direct launch retained before a controlled replacement.
    UpgradeSessions {
        /// Private launch inputs; never printed by an upgrade.
        sessions: Vec<crate::session::upgrade::UpgradeSession>,
    },
    /// New starts are allowed again after an upgrade was cancelled.
    UpgradeCancelled,
    /// Harness discovery evidence from the computer asked.
    Programs {
        /// The executable paths, versions, adapters and any discovery reason.
        found: crate::dial::programs::Programs,
    },
    /// Exact bytes from a terminal, with byte cursors and observed end evidence.
    Bytes {
        /// The byte window.
        output: crate::terminal_bytes::ByteOutput,
    },
    /// The session's process is up.
    Started {
        /// A known adapter running a version that has not been qualified.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        qualification: Option<String>,
        /// The session.
        session: String,
        /// Its process id.
        pid: u32,
        /// When it started, in milliseconds since the Unix epoch.
        started_at: u64,
    },
    /// The input, keys or resize were delivered.
    Delivered {
        /// The session.
        session: String,
    },
    /// Output read.
    Output {
        /// What was read.
        output: Output,
    },
    /// The pattern appeared.
    Matched {
        /// The session.
        session: String,
        /// The text that matched.
        matched: String,
        /// The cursor just after the match.
        cursor: u64,
    },
    /// The session's process has ended.
    Ended {
        /// The session.
        session: String,
        /// Its end.
        ended: Ended,
    },
    /// What the runner is and holds.
    Status {
        /// The status.
        status: StatusView,
    },
    /// The judge's verdict on a tool call a harness asked about.
    Judged {
        /// The verdict.
        verdict: crate::refusals::Verdict,
    },
    /// A hook, status line or notice from a harness was recorded.
    Collected {
        /// What was recorded, in words.
        words: String,
    },
    /// How an operation stands.
    Operation {
        /// Its outcome.
        outcome: crate::operations::OperationOutcome,
    },
    /// Public managed delivery evidence without its retained payload.
    ControlReceipt {
        /// The operation's original identities and evidence.
        receipt: crate::operations::ControlReceipt,
    },
    /// Current managed control without output or account history.
    ControlStatus {
        /// The explicit session.
        session: String,
        /// Its current control; absent when this session has no managed channel.
        control: Option<crate::harness_control::ControlStatus>,
    },
    /// A bounded page of public control evidence.
    ControlReceipts {
        /// Continue using its cursor until the owner returns the last page.
        page: crate::operations::ControlPage,
    },
    /// A page of the tracking feed.
    Feed {
        /// The page.
        page: crate::tracking_store::FeedPage,
    },
    /// The folders directly inside one folder.
    Folders {
        /// The folder looked in, absolute.
        under: String,
        /// The name of each folder in it, in order.
        folders: Vec<String>,
    },
    /// The folder a person picked in this computer's own dialog, absolute.
    FolderChosen {
        /// The folder picked.
        folder: String,
    },
    /// The connection is held as the grant channel from here on.
    GrantChannel,
    /// A stop of everything: the sessions it ended, each recorded with who
    /// and why, and those not ended when it answered.
    StoppedEverything {
        /// Every session this act ended.
        sessions: Vec<String>,
        /// Every session asked to end that had not when the act answered.
        running: Vec<String>,
        /// Every owned seat left with its owner (AGENTS-004): stopped at
        /// the owner by a signed stop, never here.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        owned: Vec<String>,
    },
    /// The act was refused, by name.
    Refused {
        /// The refusal's name.
        refusal: String,
        /// Why, in words.
        words: String,
        /// For `cursor_expired`, the oldest cursor held.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        oldest: Option<u64>,
    },
    /// Each session's liveness, from the runner's own knowledge.
    Liveness {
        /// Every session held, or the one named.
        sessions: Vec<crate::liveness::LivenessView>,
    },
    /// A managed session's rendered lines.
    AttachLines {
        /// The lines from the cursor asked for.
        lines: Vec<crate::attach::AttachLine>,
        /// The cursor to read on from.
        cursor: u64,
        /// Whether the session has ended.
        ended: bool,
    },
    /// The owner's answer to its command.
    Owner {
        /// The answer.
        answer: crate::seat_owner::protocol::OwnerAnswer,
    },
    /// The supervised seats this runner started owners for.
    Owned {
        /// Each owned seat proved live, by session.
        owners: Vec<crate::seat_owner::sessions::OwnedSeat>,
        /// Each owner on record the kernel did not confirm, named why.
        unreachable: Vec<crate::seat_owner::recovery::Found>,
    },
    /// The plugins installed in one config folder.
    Plugins {
        /// The folder read, absolute.
        config_dir: String,
        /// Each installed plugin as name@marketplace, in order.
        installed: Vec<String>,
    },
}

impl Answer {
    /// The answer for `error`, by its name.
    #[must_use]
    pub fn refusal(error: &RunnerError) -> Self {
        let oldest = match error {
            RunnerError::Refused { oldest, .. } => *oldest,
            _ => None,
        };
        let text = error.to_string();
        let words = text
            .split_once(": ")
            .map_or(text.as_str(), |(_, words)| words)
            .to_owned();
        Self::Refused {
            refusal: error.name(),
            words,
            oldest,
        }
    }
}
