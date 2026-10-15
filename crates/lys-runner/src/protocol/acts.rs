//! Requests retain their exact tagged wire representation.

use super::{Key, Launch, LysMcp};
use serde::{Deserialize, Serialize};

/// Harness hook tracking supplied by a PTY start, without version evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackingSpec {
    /// The harness the launch's rendering contract selected.
    pub harness: crate::tracking::Harness,
    /// Its explicit configuration home, when the launch declared one.
    #[serde(default)]
    pub config_home: Option<std::path::PathBuf>,
}

/// One act a request asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "snake_case", deny_unknown_fields)]
pub enum Act {
    /// Retain private launch inputs and fence new starts for an upgrade.
    UpgradeSessions,
    /// Cancel an upgrade before its runner is stopped.
    CancelUpgrade,
    /// Resume the launch a previous runner retained for this upgrade.
    ResumeUpgrade {
        /// The private, admitted launch and native conversation.
        saved: Box<crate::session::upgrade::UpgradeSession>,
    },
    /// Discover harness executables on this computer's login PATH.
    Programs,
    /// Start an explicitly selected, versioned managed harness channel.
    StartManaged {
        /// Existing launch inputs and the requested channel.
        managed: Box<crate::harness_control::ManagedLaunch>,
        /// The same admitted Lys MCP credential configuration as a manual start.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lys_mcp: Option<LysMcp>,
        /// Existing authoritative proxy tracking, when configured.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proxy: Option<crate::tracking_proxy::ProxyTracking>,
    },
    /// Read the exact PTY bytes without UTF-8 replacement or boundary trimming.
    ReadBytes {
        /// The session.
        session: String,
        /// The next byte cursor, or the oldest retained byte when absent.
        cursor: Option<u64>,
        /// Wait for output or an observed process exit.
        follow: bool,
    },
    /// Deliver exact terminal input, including escape sequences and binary replies.
    InputBytes {
        /// The session.
        session: String,
        /// Exact bytes; no implicit newline or character decoding.
        data: Vec<u8>,
    },
    /// Start a session in its own pseudo-terminal.
    Start {
        /// What it is started with.
        launch: Box<Launch>,
        /// The harness's hooks and configuration, omitted by older clients.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tracking: Option<TrackingSpec>,
        /// Run-only MCP credentials, omitted by older servers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lys_mcp: Option<LysMcp>,
        /// The run key and how the session is tracked through the proxy,
        /// for a run whose launch gave it the proxy's base address under
        /// that key. Absent for a run that does not go through the proxy.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proxy: Option<crate::tracking_proxy::ProxyTracking>,
    },
    /// Type text, then Enter when asked.
    Input {
        /// The session.
        session: String,
        /// The text.
        text: String,
        /// Whether Enter follows it.
        #[serde(default)]
        enter: bool,
    },
    /// Send named keys, in order.
    Keys {
        /// The session.
        session: String,
        /// The keys.
        keys: Vec<Key>,
    },
    /// Read output: from `cursor` when given, else the last `lines` lines or
    /// `bytes` bytes, else everything kept. With `follow`, the answer waits
    /// until there is output after the cursor or the session ends.
    Read {
        /// The session.
        session: String,
        /// Where to read on from.
        #[serde(default)]
        cursor: Option<u64>,
        /// How many of the last lines.
        #[serde(default)]
        lines: Option<u32>,
        /// How many of the last bytes.
        #[serde(default)]
        bytes: Option<u64>,
        /// Whether to wait for output after the cursor.
        #[serde(default)]
        follow: bool,
    },
    /// Wait until `pattern` appears in output after `cursor` (the end of the
    /// output when not given): a literal string unless `regex` asks for a
    /// regular expression by name.
    Wait {
        /// The session.
        session: String,
        /// Where new output begins.
        #[serde(default)]
        cursor: Option<u64>,
        /// The pattern.
        pattern: String,
        /// Whether the pattern is a regular expression.
        #[serde(default)]
        regex: bool,
    },
    /// Resize the terminal.
    Resize {
        /// The session.
        session: String,
        /// Width in columns.
        columns: u16,
        /// Height in rows.
        rows: u16,
    },
    /// End the session's process, answering once its exit is seen.
    End {
        /// The session.
        session: String,
    },
    /// Read current controller state without output or account history.
    ControlStatus {
        /// The explicit session whose current control is read.
        session: String,
    },
    /// Say which protocol this runner speaks and what it holds.
    Status {
        /// One session only, when named.
        #[serde(default)]
        session: Option<String>,
    },
    /// Accept an operation under the server's stable id, answering how it
    /// stands; asked again under that id it is answered, never done twice.
    Operate {
        /// The operation.
        operation: crate::operations::Operation,
    },
    /// Withdraw an accepted operation the server re-judged before its
    /// boundary, so it is never typed.
    Withdraw {
        /// The operation's id.
        operation: String,
        /// Why the server withdrew it.
        why: String,
    },
    /// How an operation stands, as the runner's record keeps it.
    Outcome {
        /// The operation's id.
        operation: String,
    },
    /// Read a managed operation's evidence without its private preparation.
    ControlReceipt {
        /// The existing operation identity.
        operation: String,
    },
    /// Read a bounded page of current receipts on one explicit session.
    ControlReceipts {
        /// The session whose receipts are requested.
        session: String,
        /// Continue after the preceding page's final operation.
        after: Option<String>,
    },
    /// Record the verified responsible person's explicit decision without replay.
    ReconcileControl {
        /// The uncertain operation.
        operation: String,
        /// The service's verified decision.
        decision: crate::operations::Reconciled,
    },
    /// Read the tracking feed after `cursor`, from its start when none is
    /// given. With `follow`, the answer waits until an entry is committed
    /// after the cursor or the caller leaves.
    Feed {
        /// Where the last page ended.
        #[serde(default)]
        cursor: Option<String>,
        /// Whether to wait for an entry.
        #[serde(default)]
        follow: bool,
    },
    /// Do `act` for `caller`, the person or agent the server verified and
    /// judged before it signed this request. A start done for a caller is
    /// owned by them; typed input and operations done for a caller are
    /// admitted to an owned session and attributed to them.
    AsCaller {
        /// The verified caller.
        caller: String,
        /// The act done for them.
        done: Box<Act>,
    },
    /// Name the folders directly inside `under`, the runner's own home
    /// folder when none is given, so a person chooses where an agent works
    /// from what this computer holds.
    Folders {
        /// The folder looked in.
        #[serde(default)]
        under: Option<String>,
    },
    /// Open this computer's own folder dialog, starting in `start` or the
    /// runner's home folder, and name the folder the person picks there.
    /// The answer waits for the person; cancelling the dialog is a refusal.
    ChooseFolder {
        /// The folder the dialog opens in.
        #[serde(default)]
        start: Option<String>,
    },
    /// Hold this connection as the live grant authority's channel: the
    /// runner answers `grant_channel`, then writes each grantable question
    /// as one line and reads the answer line to it, until the connection
    /// closes.
    GrantChannel,
    /// End every running session this runner holds, recording on each who
    /// asked and why. The first ask hangs each session up and answers once
    /// every exit is seen; `kill`, or an ask while sessions stopped by an
    /// earlier one still run, ends each proved process group at once.
    ///
    /// A `settling` ask is the server's own, sent again under a pull in
    /// force for a start that crossed it; it is never a person's second
    /// pull. A session an earlier ask already stopped is left as it is: no
    /// signal and no kill, and the answer does not wait for its exit but
    /// names it ended or running as it stands. A session not yet asked is
    /// asked as the pull says.
    StopEverything {
        /// Who pulled the cord, as the server verified them.
        by: String,
        /// Why, in their words.
        reason: String,
        /// Whether each proved process group is ended at once.
        #[serde(default)]
        kill: bool,
        /// Whether the server is settling a start under a pull in force,
        /// so a session already asked is left as that ask has it.
        #[serde(default)]
        settling: bool,
    },
    /// Say, from the runner's own knowledge, whether each session's process
    /// is alive, its harness session, whether a turn is in progress and
    /// when it last signalled.
    Liveness {
        /// One session only, when named.
        #[serde(default)]
        session: Option<String>,
    },
    /// Read a managed session's frames rendered as lines, from `cursor`, the
    /// oldest kept when absent. With `follow`, the answer waits until a line
    /// follows the cursor or the session ends. A session in a
    /// pseudo-terminal is refused `attach_pty_use_read_bytes`.
    AttachRead {
        /// The session.
        session: String,
        /// Where to read on from.
        #[serde(default)]
        cursor: Option<u64>,
        /// Whether to wait for a line after the cursor.
        #[serde(default)]
        follow: bool,
    },
    /// A command to the seat owner this runner is (AGENTS-004 R1): bind,
    /// read, move cursors, stop, or step a handover. A runner that is not
    /// an owner refuses `seat_owner_not_an_owner`.
    Owner {
        /// The command.
        command: crate::seat_owner::protocol::OwnerCommand,
    },
    /// The supervised seats this runner started owners for, from its index.
    Owned,
    /// The Claude Code plugins installed in a config folder on this machine:
    /// `config_dir` when given, else the folder a run here uses with no
    /// `CLAUDE_CONFIG_DIR` (the login's `~/.claude`), by the same rule the
    /// start's `plugin_not_installed` check uses.
    Plugins {
        /// An absolute config folder, the run's `CLAUDE_CONFIG_DIR`.
        #[serde(default)]
        config_dir: Option<String>,
    },
}

impl Act {
    /// The one session this act is about, when it names one: the act a
    /// server routes to whichever runner holds that session, the machine's
    /// own or a seat owner's. A start names none: the machine's runner
    /// starts it, and starts its owner when it is supervised.
    #[must_use]
    pub fn session(&self) -> Option<&str> {
        match self {
            Self::ReadBytes { session, .. }
            | Self::InputBytes { session, .. }
            | Self::Input { session, .. }
            | Self::Keys { session, .. }
            | Self::Read { session, .. }
            | Self::Wait { session, .. }
            | Self::Resize { session, .. }
            | Self::End { session }
            | Self::ControlStatus { session }
            | Self::ControlReceipts { session, .. }
            | Self::AttachRead { session, .. } => Some(session),
            Self::Status { session } | Self::Liveness { session } => session.as_deref(),
            Self::Operate { operation } => Some(&operation.session),
            Self::AsCaller { done, .. } => done.session(),
            _ => None,
        }
    }
}

#[path = "answers.rs"]
mod answers;
pub use answers::Answer;

#[cfg(test)]
#[path = "acts_session_tests.rs"]
mod session_tests;
