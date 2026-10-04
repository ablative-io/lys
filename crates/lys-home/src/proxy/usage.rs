//! The per-run usage file: one line for each finished call a run made.
//!
//! # Format
//!
//! The file of the run whose key is `<key>` is `<state>/usage/<key>.jsonl`,
//! where `<state>` is the proxy's state directory (the one that holds its
//! `journal`). It is only ever appended to. Each line is one JSON object, a
//! [`UsageLine`], ended by a newline:
//!
//! - `call_id`: the proxy's id for the call, the same id as on its
//!   `lys.call` record.
//! - `run`: the run key the call's path opened with.
//! - `session`: the harness's session the call is linked to; absent when
//!   the call named none.
//! - `api`: `anthropic-messages`, `openai-chat-completions` or
//!   `openai-responses`.
//! - `model`: the model asked for; absent when the request was not readable.
//! - `usage`: the token figures the response reported, each absent when the
//!   response did not report it: `input`, `output`, `cache_creation`,
//!   `cache_read`, `reasoning`. The whole object is absent when the response
//!   reported none.
//! - `account`: the organisation or account the provider's headers named;
//!   absent when they named none.
//! - `windows`: the account windows the response's headers reported, each
//!   `duration_minutes`, `used_percent` and `resets_at_ms` (milliseconds
//!   since the Unix epoch). Empty when the headers reported none.
//! - `started_at`, `ended_at`: RFC 3339; `ended_at` is absent for a call
//!   whose length is not known (one lost in flight).
//! - `status`: how the call ended, as on its record: `complete`,
//!   `cancelled`, `partial`, `unrecorded` or `lost`.
//!
//! A call whose path carried no run key has no line in any file; its record
//! holds its figures.
//!
//! # What a reader can rely on
//!
//! - A line is written after the call's record is durable and before the
//!   journal lets the call go, and it is built from the journalled record
//!   alone. A proxy that dies between the two writes the line on its next
//!   start, from the journal's recovery.
//! - So a call's line can be written twice (the proxy died after the line
//!   and before the journal let go). The two lines are the same, byte for
//!   byte. A reader keeps one line for each `call_id`.
//! - A line that is not a whole JSON object is what a dying proxy left of a
//!   write it did not finish. A reader skips it; the call's whole line
//!   follows on a line of its own.
//! - The file and its directory are synced after each line, before the
//!   journal lets the call go: a line is not lost to a power loss.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::journal::OpenCall;
use crate::record::call::captured::Head;
/// The token figures a line carries, named here for a reader of the file.
pub use crate::record::call::captured::Tokens;
use crate::record::call::{Api, CallStatus};

/// One finished call, as its run's usage file holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageLine {
    /// The proxy's id for the call.
    pub call_id: String,
    /// The run key the call's path opened with.
    pub run: String,
    /// The harness's session the call is linked to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The api the call followed.
    pub api: Api,
    /// The model asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The token figures the response reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Tokens>,
    /// The organisation or account the provider's headers named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// The account windows the response's headers reported.
    pub windows: Vec<Window>,
    /// When the call started, RFC 3339.
    pub started_at: String,
    /// When the call ended, RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// How the call ended.
    pub status: CallStatus,
}

/// One account window a response's headers reported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// The window's length.
    pub duration_minutes: u64,
    /// How much of the window the account has used, as a percentage.
    pub used_percent: Number,
    /// When the window resets, in milliseconds since the Unix epoch.
    pub resets_at_ms: u64,
}

/// The digits of a run key: two for each of the sixteen random bytes a
/// launch mints one from (`record::fresh_id`).
const RUN_KEY_DIGITS: usize = 32;

/// Whether a path part is a run key: exactly what a Lys launch mints for a
/// run, 32 lowercase hexadecimal digits. Nothing else is one: not another
/// length, not another letter, not an upper-case digit. A key names its
/// run's usage file and says whose a call is, so nothing is taken for one
/// that a launch could not have minted.
#[must_use]
pub fn is_run_key(part: &str) -> bool {
    part.len() == RUN_KEY_DIGITS
        && part
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

impl UsageLine {
    /// The line of a journalled call whose record is prepared, when its path
    /// carried a run key.
    #[must_use]
    pub fn of(call: &OpenCall) -> Option<Self> {
        let run = call.run.as_deref().filter(|run| is_run_key(run))?;
        let record = &call.completed.as_ref()?.record;
        let head = record.head.as_ref();
        Some(Self {
            call_id: record.call_id.clone(),
            run: run.to_owned(),
            session: call.session.clone(),
            api: record.api,
            model: record.model.clone(),
            usage: record.usage.clone(),
            account: head.and_then(account),
            windows: head.map(windows).unwrap_or_default(),
            started_at: record.started_at.clone(),
            ended_at: ended_at(&record.started_at, record.duration_ms),
            status: record.status,
        })
    }
}

/// Append the call's line to its run's file under `dir`; a call with no run
/// key or no prepared record writes nothing.
pub(super) fn append(dir: &Path, call: &OpenCall) -> std::io::Result<()> {
    let Some(line) = UsageLine::of(call) else {
        return Ok(());
    };
    std::fs::create_dir_all(dir)?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(dir.join(format!("{}.jsonl", line.run)))?;
    let mut text = String::new();
    if unfinished(&mut file)? {
        // What a dying proxy left of a line is closed, so this one is whole.
        text.push('\n');
    }
    text.push_str(&serde_json::to_string(&line).map_err(std::io::Error::other)?);
    text.push('\n');
    file.write_all(text.as_bytes())?;
    // The line is durable before the journal lets the call go: a budget
    // must not read low after a power loss.
    file.sync_all()?;
    std::fs::File::open(dir)?.sync_all()
}

/// Whether the file ends part way through a line.
fn unfinished(file: &mut std::fs::File) -> std::io::Result<bool> {
    if file.metadata()?.len() == 0 {
        return Ok(false);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)?;
    Ok(last != *b"\n")
}

/// The first kept value of a header on one side.
fn first<'a>(
    values: &'a std::collections::BTreeMap<String, Vec<String>>,
    name: &str,
) -> Option<&'a str> {
    values.get(name)?.first().map(String::as_str)
}

/// The organisation Anthropic's response named, or the account a Codex
/// request named.
fn account(head: &Head) -> Option<String> {
    first(&head.response.values, "anthropic-organization-id")
        .or_else(|| first(&head.request.values, "chatgpt-account-id"))
        .map(str::to_owned)
}

/// The windows the response's headers reported: Anthropic's five hour and
/// seven day windows (a utilisation as a fraction of one, a reset in seconds
/// since the epoch), and Codex's primary and secondary windows (a percentage,
/// a length in minutes, a reset in seconds since the epoch). A window is
/// written only when every figure of it was reported and reads as a number.
fn windows(head: &Head) -> Vec<Window> {
    let reported = |name: &str| first(&head.response.values, name);
    let mut windows = Vec::new();
    for (name, duration_minutes) in [("5h", 300), ("7d", 10_080)] {
        let used = reported(&format!("anthropic-ratelimit-unified-{name}-utilization"));
        let reset = reported(&format!("anthropic-ratelimit-unified-{name}-reset"));
        if let (Some(used_percent), Some(resets_at_ms)) = (
            used.and_then(percent_of_fraction),
            reset.and_then(milliseconds),
        ) {
            windows.push(Window {
                duration_minutes,
                used_percent,
                resets_at_ms,
            });
        }
    }
    for slot in ["primary", "secondary"] {
        let minutes = reported(&format!("x-codex-{slot}-window-minutes"));
        let used = reported(&format!("x-codex-{slot}-used-percent"));
        let reset = reported(&format!("x-codex-{slot}-reset-at"));
        if let (Some(duration_minutes), Some(used_percent), Some(resets_at_ms)) = (
            minutes.and_then(|text| text.trim().parse::<u64>().ok()),
            used.and_then(|text| serde_json::from_str::<Number>(text.trim()).ok()),
            reset.and_then(milliseconds),
        ) {
            windows.push(Window {
                duration_minutes,
                used_percent,
                resets_at_ms,
            });
        }
    }
    windows
}

/// A fraction of one as a percentage.
fn percent_of_fraction(fraction: &str) -> Option<Number> {
    let fraction: f64 = fraction.trim().parse().ok()?;
    Number::from_f64(fraction * 100.0)
}

/// Seconds since the epoch as milliseconds since the epoch.
fn milliseconds(seconds: &str) -> Option<u64> {
    seconds.trim().parse::<u64>().ok()?.checked_mul(1000)
}

/// When a call that started at `started_at` and took `duration_ms` ended.
fn ended_at(started_at: &str, duration_ms: Option<u64>) -> Option<String> {
    use time::format_description::well_known::Rfc3339;
    let started = time::OffsetDateTime::parse(started_at, &Rfc3339).ok()?;
    let took = time::Duration::milliseconds(i64::try_from(duration_ms?).ok()?);
    started.checked_add(took)?.format(&Rfc3339).ok()
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
