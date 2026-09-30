//! Read only the figures the native stream reports, retaining missing-field reasons.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    /// Reported dollar spend, rounded to the nearest microdollar.
    #[serde(default)]
    pub dollars_micros: Option<u64>,
    /// Reported account windows, never added as spend.
    #[serde(default)]
    pub plan_windows: Vec<crate::tracking_budget::PlanWindow>,
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
        ..Figures::default()
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
    notes.push(Unavailable {
        figure: "dollars_micros".to_owned(),
        reason: "reported_by_the_status_line".to_owned(),
    });
    notes.push(Unavailable {
        figure: "plan_windows".to_owned(),
        reason: "reported_by_the_status_line".to_owned(),
    });
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
    let mut figures = if let Some(usage) = usage {
        Figures {
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            context_tokens: context_of(usage),
            running_ms,
            ..Figures::default()
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
    figures.dollars_micros = crate::tracking_budget::dollars(input, &mut notes);
    figures.plan_windows = crate::tracking_budget::windows(input, &mut notes);
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
