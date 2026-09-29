//! Account rotation at a usage limit.
//!
//! A session may be given an ordered list of account handles, each held by
//! the secrets broker. The handle in use is set in one environment variable
//! of the session's process, and the broker swaps it for the account's
//! credential on the way out, so no account value ever reaches the runner,
//! its answers or its record. Rotation changes only that variable: the
//! session's directory, and so where its harness keeps its session store and
//! transcripts, never changes, and a moved session resumes.
//!
//! A usage limit is the harness's own signal where it has one, an exit
//! status, so an agent quoting the words rotates nothing. A harness with no
//! such signal declares the words that mean a limit, and then quoted text can
//! trip it; that is said where the words are declared. At the list's end the
//! session is ended `accounts_exhausted`; the list never wraps round.

use serde::{Deserialize, Serialize};

use crate::error::RunnerError;

/// What says a usage limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "signal", rename_all = "snake_case", deny_unknown_fields)]
pub enum Limit {
    /// The harness exits with this status at its usage limit: its own signal.
    ExitStatus {
        /// The exit status.
        status: u32,
    },
    /// The harness has no signal of its own, so these words in its output
    /// mean a usage limit. Quoted text can trip it.
    Words {
        /// The words; any one of them, anywhere in new output, is a limit.
        words: Vec<String>,
    },
}

impl Limit {
    /// The signal's name, as a move records it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ExitStatus { .. } => "exit_status",
            Self::Words { .. } => "words",
        }
    }
}

/// How a session moves between accounts at a usage limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Rotation {
    /// The account handles, in order; the first is the session's first account.
    pub accounts: Vec<String>,
    /// The environment variable the handle in use is set in.
    pub variable: String,
    /// What says a usage limit.
    pub limit: Limit,
    /// The arguments a moved session is started again with, resuming its
    /// session record; the launch's own arguments when empty.
    #[serde(default)]
    pub resume_arguments: Vec<String>,
}

/// One move from an account to the next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Move {
    /// The handle moved from.
    pub from: String,
    /// The handle moved to; empty when the list was at its end.
    pub to: String,
    /// When, in milliseconds since the Unix epoch.
    pub at: u64,
    /// The signal that said a usage limit: `exit_status` or `words`.
    pub by: String,
}

/// A session's place in its rotation.
#[derive(Debug, Clone)]
pub struct RotationState {
    rotation: Rotation,
    index: usize,
    moves: Vec<Move>,
    tripped: bool,
}

/// Whether `name` may name an environment variable.
fn variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl RotationState {
    /// The rotation `rotation` at its first account, refused by name when it
    /// names no account, a variable no environment can hold, or no words.
    pub fn new(rotation: Rotation) -> Result<Self, RunnerError> {
        let refused = |words: &str| RunnerError::refused("rotation_invalid", words);
        if rotation.accounts.is_empty() {
            return Err(refused("a rotation names at least one account handle"));
        }
        if rotation.accounts.iter().any(String::is_empty) {
            return Err(refused("an account handle is empty"));
        }
        if !variable_name(&rotation.variable) {
            return Err(refused(
                "the rotation's variable is not an environment variable name",
            ));
        }
        let words_empty = match &rotation.limit {
            Limit::Words { words } => words.is_empty() || words.iter().any(String::is_empty),
            Limit::ExitStatus { .. } => false,
        };
        if words_empty {
            return Err(refused(
                "a rotation on words names at least one word, none empty",
            ));
        }
        Ok(Self {
            rotation,
            index: 0,
            moves: Vec::new(),
            tripped: false,
        })
    }

    /// The handle in use.
    pub fn account(&self) -> &str {
        self.rotation
            .accounts
            .get(self.index)
            .map_or("", String::as_str)
    }

    /// The environment variable the handle is set in.
    pub fn variable(&self) -> &str {
        &self.rotation.variable
    }

    /// The arguments a moved session is started again with, when the
    /// rotation names any.
    pub fn resume_arguments(&self) -> Option<&[String]> {
        (!self.rotation.resume_arguments.is_empty()).then_some(&*self.rotation.resume_arguments)
    }

    /// Every move made, in order.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Whether an exit with `status` is a usage limit: the harness's own
    /// exit status when it has that signal, or the words seen before the
    /// exit when it has none. An exit by a signal carries no status.
    pub fn limit_at_exit(&self, status: Option<u32>) -> bool {
        match &self.rotation.limit {
            Limit::ExitStatus { status: limit } => status == Some(*limit),
            Limit::Words { .. } => self.tripped,
        }
    }

    /// Whether `text` carries the declared words, for a harness with no
    /// signal of its own; the harness's own signal is never read from text.
    pub fn words_in(&self, text: &str) -> bool {
        match &self.rotation.limit {
            Limit::ExitStatus { .. } => false,
            Limit::Words { words } => words.iter().any(|word| text.contains(word.as_str())),
        }
    }

    /// The longest declared word, in bytes, so a search can reach back
    /// across the edge of what was read before.
    pub fn longest_word(&self) -> usize {
        match &self.rotation.limit {
            Limit::ExitStatus { .. } => 0,
            Limit::Words { words } => words.iter().map(String::len).max().unwrap_or(0),
        }
    }

    /// Mark the words seen, so the process's exit moves the session.
    pub fn trip(&mut self) {
        self.tripped = true;
    }

    /// Whether the words were seen and the process is being ended for them.
    pub fn tripped(&self) -> bool {
        self.tripped
    }

    /// Move to the next account at `at`, recording the move; `None` at the
    /// list's end, which is recorded too, with no account moved to.
    pub fn advance(&mut self, at: u64) -> Option<String> {
        let from = self.account().to_owned();
        let by = self.rotation.limit.name().to_owned();
        self.tripped = false;
        let next = self.rotation.accounts.get(self.index + 1).cloned();
        self.moves.push(Move {
            from,
            to: next.clone().unwrap_or_default(),
            at,
            by,
        });
        if next.is_some() {
            self.index += 1;
        }
        next
    }
}
