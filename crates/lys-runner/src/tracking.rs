//! What the runner measures of a session's harness, and the adapters that
//! read it from the harness's own stream.
//!
//! A tracked session is started with its [`Tracking`]: the harness, the
//! adapter and the harness version it was measured against, its isolated
//! configuration home, and the context window its profile declares. A
//! declared version the adapter was not measured against is refused
//! `tracking_contract_unsupported` before anything runs, and the executable
//! actually launched and the version it reports are recorded, never
//! assumed. Hook tracking declares neither a version nor a context window;
//! both remain unavailable in its start record.
//!
//! Figures come only from the harness's stream, never from what an agent
//! says. Claude Code's spend is each response's `message.usage`, counted
//! once per message and request id when the next record shows the response
//! complete; a partial record is never a turn. Codex's is the difference
//! between successive `token_count` totals, a repeated total adding
//! nothing. A status line's figures are snapshots of context in use and are
//! never added as spend. A figure the source does not carry stays null,
//! with the reason, and a paying account is named only where the runner's
//! rotation evidence names one at the record's own instant.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::tracking_fields::{
    Figures, Unavailable, claude_figures, context_of, count, note, status_figures,
};

use crate::error::RunnerError;
use crate::rotation::Move;
mod reading;
pub use reading::Reading;

/// The version of every record the runner writes of a session's figures.
pub const RECORD_VERSION: u32 = 1;

/// The Claude Code adapter: its session JSONL.
pub const CLAUDE_ADAPTER: &str = "claude-code-jsonl/1";

/// The Codex adapter: its rollout JSONL.
pub const CODEX_ADAPTER: &str = "codex-rollout/1";

/// Each adapter and the harness versions it was measured against.
pub const MEASURED: [(&str, &[&str]); 2] = [
    (CLAUDE_ADAPTER, &["2.1.281", "2.1.283", "2.1.285"]),
    (CODEX_ADAPTER, &["0.156.0", "0.161.0-alpha.3"]),
];

/// A harness the runner tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Harness {
    /// Claude Code: hooks, a status line and the session JSONL.
    ClaudeCode,
    /// Codex: its after-turn notify and the rollout JSONL; no pre-tool hook.
    Codex,
}

impl Harness {
    /// The one adapter that reads it.
    #[must_use]
    pub fn adapter(self) -> &'static str {
        match self {
            Self::ClaudeCode => CLAUDE_ADAPTER,
            Self::Codex => CODEX_ADAPTER,
        }
    }
}

/// How a session's harness is tracked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tracking {
    /// The harness.
    pub harness: Harness,
    /// The adapter that reads its stream.
    pub adapter: String,
    /// The harness version the profile declares; empty for unmeasured hooks.
    pub version: String,
    /// The harness's isolated configuration home: `CLAUDE_CONFIG_DIR` or
    /// `CODEX_HOME`, absolute.
    pub config_home: String,
    /// The context window the profile declares, in tokens.
    pub context_window: u64,
    /// The profile version the window was declared in.
    pub profile_version: u32,
    /// The paying account's handle, when the session does not rotate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Whether the profile requires the pre-tool policy hook.
    #[serde(default)]
    pub requires_pre_tool: bool,
}

fn unsupported(words: String) -> RunnerError {
    RunnerError::refused("tracking_contract_unsupported", words)
}

impl Tracking {
    /// Resolve a wire start's configuration without declaring measurement evidence.
    pub(crate) fn from_spec(
        spec: &crate::protocol::TrackingSpec,
        launch: &mut crate::protocol::Launch,
    ) -> Result<Self, RunnerError> {
        let variable = match spec.harness {
            Harness::ClaudeCode => "CLAUDE_CONFIG_DIR",
            Harness::Codex => "CODEX_HOME",
        };
        let resolved = crate::trust::resolve_config_home(
            variable,
            spec.config_home.as_deref(),
            &launch.environment,
        )?;
        let home = if resolved.source == "home" {
            resolved.path.join(match spec.harness {
                Harness::ClaudeCode => ".claude",
                Harness::Codex => ".codex",
            })
        } else {
            resolved.path
        };
        let config_home = home
            .to_str()
            .ok_or_else(|| {
                RunnerError::refused(
                    "tracking_home_invalid",
                    "the harness configuration home is not UTF-8",
                )
            })?
            .to_owned();
        if resolved.source != "home" {
            launch
                .environment
                .insert(variable.to_owned(), config_home.clone());
            if let Some(config) = &mut launch.config {
                config.environment_paths.remove(variable);
            }
        }
        launch.environment.insert(
            crate::trust::CONFIG_HOME_SOURCE.to_owned(),
            resolved.source.to_owned(),
        );
        Ok(Self {
            harness: spec.harness,
            adapter: spec.harness.adapter().to_owned(),
            version: String::new(),
            config_home,
            context_window: 0,
            profile_version: 0,
            account: None,
            requires_pre_tool: false,
        })
    }

    /// The tracking, refused by name: an adapter not the harness's, a
    /// declared version it was not measured against, a declared window of no tokens, a home
    /// that is not absolute, or a pre-tool hook the harness has not.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if the adapter or version is unsupported, unmeasured tracking declares measured features, the context window is empty, the home is invalid, or Codex requires an unavailable pre-tool hook.
    pub fn checked(&self) -> Result<(), RunnerError> {
        if self.adapter != self.harness.adapter() {
            return Err(unsupported(format!(
                "adapter `{}` does not read this harness; `{}` does",
                self.adapter,
                self.harness.adapter()
            )));
        }
        if self.version.is_empty() {
            if self.context_window != 0 || self.profile_version != 0 || self.requires_pre_tool {
                return Err(unsupported(
                    "unmeasured hook tracking declares no window, profile version or policy adapter".to_owned(),
                ));
            }
        } else {
            measured(&self.adapter, &self.version)?;
        }
        if !self.version.is_empty() && self.context_window == 0 {
            return Err(RunnerError::refused(
                "window_invalid",
                "a declared context window holds at least one token",
            ));
        }
        if !std::path::Path::new(&self.config_home).is_absolute()
            || self.config_home.chars().any(char::is_control)
        {
            return Err(RunnerError::refused(
                "tracking_home_invalid",
                "the harness's configuration home is an absolute path",
            ));
        }
        if self.requires_pre_tool && self.harness == Harness::Codex {
            return Err(RunnerError::refused(
                "policy_not_supported",
                "Codex has no pre-tool hook of Lys's: a profile that requires one does not run unguarded",
            ));
        }
        Ok(())
    }
}

/// Refuse `tracking_contract_unsupported` unless `adapter` was measured
/// against harness `version`.
///
/// # Errors
/// - `RunnerError::Refused` with `tracking_contract_unsupported` if the adapter is unknown or its measured versions do not contain the supplied version.
pub fn measured(adapter: &str, version: &str) -> Result<(), RunnerError> {
    let versions = MEASURED
        .iter()
        .find(|(name, _)| *name == adapter)
        .map(|(_, versions)| *versions)
        .ok_or_else(|| unsupported(format!("no adapter `{adapter}` is measured")))?;
    if versions.contains(&version) {
        Ok(())
    } else {
        Err(unsupported(format!(
            "adapter `{adapter}` was measured against {}, not {version}",
            versions.join(", ")
        )))
    }
}

/// The first semantic version, including its reported prerelease suffix.
pub fn version_in(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .find(|word| {
            let (release, suffix) = word
                .split_once('-')
                .map_or((*word, None), |(release, suffix)| (release, Some(suffix)));
            release.split('.').count() == 3
                && release
                    .split('.')
                    .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
                && suffix.is_none_or(|suffix| {
                    !suffix.is_empty()
                        && suffix.split('.').all(|part| {
                            !part.is_empty()
                                && part
                                    .bytes()
                                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                        })
                })
        })
        .map(str::to_owned)
}

/// What a record's figures are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// Spend: one response's tokens, counted once.
    Spend,
    /// A snapshot of the context in use: never added to spend.
    Snapshot,
}

/// One record of a session's figures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageRecord {
    /// [`RECORD_VERSION`].
    pub version: u32,
    /// Its stable identity, the same however often it is delivered.
    pub id: String,
    /// The runner that read it.
    pub runner: String,
    /// The session it measures.
    pub session: String,
    /// The source generation it was read from.
    pub generation: u64,
    /// The byte offset of its source record, when it came from a stream.
    pub offset: Option<u64>,
    /// The turn it belongs to, when the source names one.
    pub turn: Option<String>,
    /// When it was observed, in milliseconds since the Unix epoch.
    pub observed_at: u64,
    /// Spend or snapshot.
    pub measure: Measure,
    /// The figures.
    pub figures: Figures,
    /// Each null figure and why.
    pub unavailable: Vec<Unavailable>,
    /// The adapter that read it.
    pub adapter: String,
    /// The model named, when the source names one.
    pub model: Option<String>,
    /// The paying account's handle at the record's instant, when known.
    pub account: Option<String>,
    /// Why the paying account is not named, when it is not.
    pub account_unknown: Option<String>,
    /// The context window the profile declared.
    pub context_window: u64,
    /// The profile version that declared it.
    pub profile_version: u32,
    /// The run key of the model call this record counts, when it was read
    /// from the proxy's usage file: with `record`, where the call is kept
    /// whole on the machine it was made on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// Where the proxy keeps the call whole, when its usage line said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<RecordAt>,
}

/// Where a proxy keeps one call whole: the session of its home and the
/// entry in it, so a reader seeks to the call and never walks the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordAt {
    /// The home session the call is recorded under.
    pub session: String,
    /// The entry in that session that holds the call.
    pub entry: String,
    /// How the call ended, in the record's own word.
    pub status: String,
    /// How long the call took, when its line said when it ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// The rotation evidence a record is attributed by.
#[derive(Debug, Clone, Copy)]
pub struct Accounts<'a> {
    /// The handle in use now, when the session rotates.
    pub current: Option<&'a str>,
    /// Every move, in order.
    pub moves: &'a [Move],
    /// The declared paying account, when the session does not rotate.
    pub declared: Option<&'a str>,
}

impl Accounts<'_> {
    /// The paying account at instant `at`, or why none is named: an instant
    /// on a move is ambiguous, and so is one after the list ran out.
    #[must_use]
    pub fn at(&self, at: u64) -> (Option<String>, Option<String>) {
        let Some(current) = self.current else {
            return match self.declared {
                Some(account) => (Some(account.to_owned()), None),
                None => (None, Some("account_undeclared".to_owned())),
            };
        };
        if self.moves.iter().any(|moved| moved.at == at) {
            return (None, Some("rotation_boundary".to_owned()));
        }
        let held = match self.moves.iter().position(|moved| moved.at > at) {
            Some(0) => self.moves.first().map(|moved| moved.from.as_str()),
            Some(next) => self.moves.get(next - 1).map(|moved| moved.to.as_str()),
            None => Some(self.moves.last().map_or(current, |moved| moved.to.as_str())),
        };
        match held {
            Some(account) if !account.is_empty() => (Some(account.to_owned()), None),
            _ => (None, Some("accounts_exhausted".to_owned())),
        }
    }
}

/// The instant an RFC 3339 `timestamp` names, in milliseconds.
#[must_use]
pub fn instant(value: &Value) -> Option<u64> {
    let text = value.get("timestamp")?.as_str()?;
    let stamp: jiff::Timestamp = text.parse().ok()?;
    u64::try_from(stamp.as_millisecond()).ok()
}

/// Whether a Claude Code `user` record is a person's prompt, which begins a
/// turn, rather than a tool result or a harness note.
pub fn is_prompt(record: &Value) -> bool {
    if record.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    match record
        .get("message")
        .and_then(|message| message.get("content"))
    {
        Some(Value::String(_)) => true,
        Some(Value::Array(blocks)) => blocks
            .iter()
            .any(|block| block.get("type").and_then(Value::as_str) == Some("text")),
        _ => false,
    }
}
