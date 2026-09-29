//! What the runner measures of a session's harness, and the adapters that
//! read it from the harness's own stream.
//!
//! A tracked session is started with its [`Tracking`]: the harness, the
//! adapter and the harness version it was measured against, its isolated
//! configuration home, and the context window its profile declares. A
//! version the adapter was not measured against is refused
//! `tracking_contract_unsupported` before anything runs, and the executable
//! actually launched and the version it reports are recorded, never
//! assumed.
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

use crate::error::RunnerError;
use crate::rotation::Move;
use crate::tracking_store::{Body, Coverage, Pending, SourceState, Totals};

/// The version of every record the runner writes of a session's figures.
pub const RECORD_VERSION: u32 = 1;

/// The Claude Code adapter: its session JSONL.
pub const CLAUDE_ADAPTER: &str = "claude-code-jsonl/1";

/// The Codex adapter: its rollout JSONL.
pub const CODEX_ADAPTER: &str = "codex-rollout/1";

/// Each adapter and the harness versions it was measured against.
pub const MEASURED: [(&str, &[&str]); 2] = [
    (CLAUDE_ADAPTER, &["2.1.281", "2.1.283"]),
    (CODEX_ADAPTER, &["0.156.0"]),
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
    /// The harness version the profile declares.
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
    /// The tracking, refused by name: an adapter not the harness's, a
    /// version it was not measured against, a window of no tokens, a home
    /// that is not absolute, or a pre-tool hook the harness has not.
    pub fn checked(&self) -> Result<(), RunnerError> {
        if self.adapter != self.harness.adapter() {
            return Err(unsupported(format!(
                "adapter `{}` does not read this harness; `{}` does",
                self.adapter,
                self.harness.adapter()
            )));
        }
        measured(&self.adapter, &self.version)?;
        if self.context_window == 0 {
            return Err(RunnerError::refused(
                "window_invalid",
                "a declared context window holds at least one token",
            ));
        }
        if !self.config_home.starts_with('/') {
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

/// The first word of `text` spelled as a version, digits and dots.
pub fn version_in(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .find(|word| {
            word.split('.').count() >= 3
                && word
                    .split('.')
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_owned)
}

/// The figures a record measures; each is null when its source does not
/// carry it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Figures {
    /// Input tokens not read from a cache.
    pub input_tokens: Option<u64>,
    /// Output tokens.
    pub output_tokens: Option<u64>,
    /// Tokens written to a cache.
    pub cache_creation_tokens: Option<u64>,
    /// Tokens read from a cache.
    pub cache_read_tokens: Option<u64>,
    /// The context in use, in tokens.
    pub context_tokens: Option<u64>,
    /// Running time, in milliseconds.
    pub running_ms: Option<u64>,
}

/// A figure that is null, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unavailable {
    /// The figure.
    pub figure: String,
    /// Why, by name.
    pub reason: String,
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
pub fn instant(value: &Value) -> Option<u64> {
    let text = value.get("timestamp")?.as_str()?;
    let stamp: jiff::Timestamp = text.parse().ok()?;
    u64::try_from(stamp.as_millisecond()).ok()
}

/// The unsigned integer at `key` in `value`.
pub fn count(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

/// A figure null because `reason`, pushed when `figure` is null.
pub fn note(figure: &str, present: Option<u64>, reason: &str, notes: &mut Vec<Unavailable>) {
    if present.is_none() {
        notes.push(Unavailable {
            figure: figure.to_owned(),
            reason: reason.to_owned(),
        });
    }
}

/// The context in use a usage object says: input, cache writes and cache
/// reads together, when all three are there.
pub fn context_of(usage: &Value) -> Option<u64> {
    let input = count(usage, "input_tokens")?;
    let created = count(usage, "cache_creation_input_tokens")?;
    let read = count(usage, "cache_read_input_tokens")?;
    input.checked_add(created)?.checked_add(read)
}

/// The figures of a Claude Code `message.usage` object, and each missing.
pub fn claude_figures(usage: &Value) -> (Figures, Vec<Unavailable>) {
    let figures = Figures {
        input_tokens: count(usage, "input_tokens"),
        output_tokens: count(usage, "output_tokens"),
        cache_creation_tokens: count(usage, "cache_creation_input_tokens"),
        cache_read_tokens: count(usage, "cache_read_input_tokens"),
        context_tokens: context_of(usage),
        running_ms: None,
    };
    let mut notes = Vec::new();
    for (figure, value) in [
        ("input_tokens", figures.input_tokens),
        ("output_tokens", figures.output_tokens),
        ("cache_creation_tokens", figures.cache_creation_tokens),
        ("cache_read_tokens", figures.cache_read_tokens),
        ("context_tokens", figures.context_tokens),
    ] {
        note(figure, value, "field_absent", &mut notes);
    }
    note(
        "running_ms",
        None,
        "measured_from_the_session_process",
        &mut notes,
    );
    (figures, notes)
}

/// The figures a Claude Code status line's input carries: snapshots of the
/// context in use and the running time, each null when it is.
pub fn status_figures(input: &Value) -> (Figures, Vec<Unavailable>) {
    let window = input.get("context_window");
    let usage = window
        .and_then(|window| window.get("current_usage"))
        .filter(|usage| usage.is_object());
    let running_ms = input
        .get("cost")
        .and_then(|cost| count(cost, "total_duration_ms"));
    let mut notes = Vec::new();
    let figures = if let Some(usage) = usage {
        Figures {
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            context_tokens: context_of(usage),
            running_ms,
        }
    } else {
        notes.push(Unavailable {
            figure: "context_tokens".to_owned(),
            reason: "status_current_usage_null".to_owned(),
        });
        Figures {
            running_ms,
            ..Figures::default()
        }
    };
    if usage.is_some() {
        note(
            "context_tokens",
            figures.context_tokens,
            "field_absent",
            &mut notes,
        );
    }
    note("running_ms", running_ms, "field_absent", &mut notes);
    for figure in [
        "input_tokens",
        "output_tokens",
        "cache_creation_tokens",
        "cache_read_tokens",
    ] {
        notes.push(Unavailable {
            figure: figure.to_owned(),
            reason: "a_snapshot_carries_no_spend".to_owned(),
        });
    }
    (figures, notes)
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

/// What a source line is read with: whose it is and how it is attributed.
#[derive(Debug, Clone, Copy)]
pub struct Reading<'a> {
    /// The runner.
    pub runner: &'a str,
    /// The session.
    pub session: &'a str,
    /// How it is tracked.
    pub tracking: &'a Tracking,
    /// The rotation evidence.
    pub accounts: Accounts<'a>,
    /// Now, for a record whose source names no instant.
    pub now: u64,
}

/// A record's source: its identity, offset, turn and instant.
struct Origin {
    id: String,
    offset: Option<u64>,
    turn: Option<String>,
    observed_at: Option<u64>,
}

impl Reading<'_> {
    fn record(
        &self,
        source: &SourceState,
        origin: Origin,
        measure: Measure,
        (figures, mut unavailable): (Figures, Vec<Unavailable>),
        model: Option<String>,
    ) -> Body {
        let observed_at = origin.observed_at.unwrap_or_else(|| {
            unavailable.push(Unavailable {
                figure: "observed_at".to_owned(),
                reason: "the_source_names_no_instant_so_the_reading_instant_stands".to_owned(),
            });
            self.now
        });
        let (account, account_unknown) = self.accounts.at(observed_at);
        Body::Usage(UsageRecord {
            version: RECORD_VERSION,
            id: origin.id,
            runner: self.runner.to_owned(),
            session: self.session.to_owned(),
            generation: source.generation,
            offset: origin.offset,
            turn: origin.turn,
            observed_at,
            measure,
            figures,
            unavailable,
            adapter: self.tracking.adapter.clone(),
            model,
            account,
            account_unknown,
            context_window: self.tracking.context_window,
            profile_version: self.tracking.profile_version,
        })
    }

    /// The response `source` holds pending, now shown whole, as one spend
    /// record; none when none is pending.
    pub fn flush(&self, source: &mut SourceState) -> Option<Body> {
        let pending = source.pending.take()?;
        let usage: Value = serde_json::from_str(&pending.usage).unwrap_or(Value::Null);
        let origin = Origin {
            id: format!("{CLAUDE_ADAPTER}:{}:{}", source.bound, pending.key),
            offset: Some(pending.offset),
            turn: pending.turn,
            observed_at: pending.observed_at,
        };
        Some(self.record(
            source,
            origin,
            Measure::Spend,
            claude_figures(&usage),
            pending.model,
        ))
    }

    /// Read one Claude Code record at `offset`: a prompt begins a turn and
    /// shows the response before it whole; a response is held until the
    /// next record with another message and request id shows it whole.
    pub fn claude(&self, source: &mut SourceState, offset: u64, record: &Value) -> Vec<Body> {
        let kind = record.get("type").and_then(Value::as_str);
        let mut bodies = Vec::new();
        if kind == Some("user") && is_prompt(record) {
            bodies.extend(self.flush(source));
            source.turn = record
                .get("promptId")
                .or_else(|| record.get("uuid"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            return bodies;
        }
        if kind != Some("assistant") {
            return bodies;
        }
        let Some(message) = record.get("message") else {
            return bodies;
        };
        let model = message.get("model").and_then(Value::as_str);
        let Some(usage) = message.get("usage").filter(|usage| usage.is_object()) else {
            return bodies;
        };
        if model == Some("<synthetic>") {
            return bodies;
        }
        let id = message.get("id").and_then(Value::as_str).unwrap_or("");
        let request = record
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or("");
        let key = format!("{id}|{request}");
        if let Some(pending) = source.pending.as_mut().filter(|held| held.key == key) {
            pending.usage = usage.to_string();
            return bodies;
        }
        bodies.extend(self.flush(source));
        source.pending = Some(Pending {
            key,
            usage: usage.to_string(),
            model: model.map(str::to_owned),
            observed_at: instant(record),
            offset,
            turn: source.turn.clone(),
        });
        bodies
    }

    /// Read one Codex rollout record at `offset`: a turn's start names the
    /// turn, and a `token_count` whose totals rose is spend by the rise; a
    /// repeated total adds nothing and a fall is a reset, said by name.
    pub fn codex(&self, source: &mut SourceState, offset: u64, record: &Value) -> Vec<Body> {
        let payload = record.get("payload");
        let event = payload
            .and_then(|payload| payload.get("type"))
            .and_then(Value::as_str);
        if record.get("type").and_then(Value::as_str) != Some("event_msg") {
            return Vec::new();
        }
        if event == Some("task_started") {
            source.turn = payload
                .and_then(|payload| payload.get("turn_id"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            return Vec::new();
        }
        let info = payload.and_then(|payload| payload.get("info"));
        let Some(totals) = info
            .filter(|_| event == Some("token_count"))
            .and_then(|info| info.get("total_token_usage"))
            .and_then(Totals::of)
        else {
            return Vec::new();
        };
        let before = source.totals.replace(totals.clone()).unwrap_or_default();
        let Some(rise) = totals.since(&before) else {
            return vec![Body::Coverage(Coverage::of(
                "source_generation",
                source,
                Some(offset),
                "the rollout's totals fell: a reset, counted from here".to_owned(),
            ))];
        };
        if rise == Totals::default() {
            return Vec::new();
        }
        let context = info
            .and_then(|info| info.get("last_token_usage"))
            .and_then(|last| count(last, "total_tokens"));
        let origin = Origin {
            id: format!(
                "{CODEX_ADAPTER}:{}:{}:{offset}",
                source.bound, source.generation
            ),
            offset: Some(offset),
            turn: source.turn.clone(),
            observed_at: instant(record),
        };
        vec![self.record(source, origin, Measure::Spend, rise.figures(context), None)]
    }

    /// A status line's snapshot, kept as record `id`; none when it repeats
    /// the last one kept.
    pub fn status(&self, source: &mut SourceState, input: &Value, id: String) -> Option<Body> {
        let (figures, unavailable) = status_figures(input);
        if source.snapshot.as_ref() == Some(&figures) {
            return None;
        }
        source.snapshot = Some(figures.clone());
        let model = input
            .get("model")
            .and_then(|model| model.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let origin = Origin {
            id,
            offset: None,
            turn: source.turn.clone(),
            observed_at: Some(self.now),
        };
        Some(self.record(
            source,
            origin,
            Measure::Snapshot,
            (figures, unavailable),
            model,
        ))
    }
}
