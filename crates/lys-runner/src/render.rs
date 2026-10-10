//! Rendering words at the moment of delivery (AGENTS-001 R3): `{{vars.key}}`
//! and `{{vars.key | fallback}}` from the session's then the agent's
//! variables, `{{goals}}` as the recipient's unfinished goals with deadline
//! and time left, `{{time_left}}` and `{{deadline}}` for the goal a reminder
//! belongs to, and the numbers the slot carries by name. No other
//! substitution exists; a value is never scanned for placeholders, so text
//! holding `{{` is rendered as text; a missing key with no fallback renders
//! empty and is named, so the receipt says what was not there.
//!
//! The result names every revision that contributed, so a receipt written
//! before the delivery says exactly which words, template, variables and
//! goals the recipient was sent.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One variable as rendering reads it: its value and the revision of the
/// map it was read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    /// The value, as JSON; a string renders as itself, anything else as its JSON.
    pub value: serde_json::Value,
}

/// One scope's variables and the revision they were read at.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    /// The scope, for the receipt: `session <id>` or `agent <id>`.
    pub name: String,
    /// The revision the map was read at.
    pub revision: u64,
    /// The variables, by name; an expired key is absent here already.
    pub values: BTreeMap<String, Variable>,
}

/// One unfinished goal, as `{{goals}}` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalLine {
    /// The goal's id, for the receipt.
    pub id: String,
    /// Its kind: goal, expectation or deliverable.
    pub kind: String,
    /// Its words.
    pub words: String,
    /// Its deadline, in seconds since the Unix epoch, when it has one.
    pub deadline: Option<u64>,
}

/// Everything a rendering reads, at the moment it is read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inputs {
    /// The session's variables, asked first.
    pub session: Option<Scope>,
    /// The agent's variables, asked when the session has no such key.
    pub agent: Option<Scope>,
    /// The recipient's unfinished goals, with the revision of the goals log
    /// they were read at (its leaf count).
    pub goals: Vec<GoalLine>,
    /// The goals log's revision the goals were read at, when they were read.
    pub goals_revision: Option<u64>,
    /// The deadline of the goal a reminder belongs to, in seconds since the Unix epoch.
    pub deadline: Option<u64>,
    /// The numbers and names the slot carries, by placeholder name:
    /// `context_percent`, `message`, `text` and the like.
    pub numbers: BTreeMap<String, String>,
    /// Now, in seconds since the Unix epoch.
    pub now: u64,
}

/// A revision that contributed to a rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contributed {
    /// What: `variables` or `goals`.
    pub kind: String,
    /// Which: the scope's name or `goals`.
    pub key: String,
    /// The revision read.
    pub revision: u64,
}

/// What a rendering produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Rendered {
    /// The text, every placeholder replaced.
    pub text: String,
    /// Each placeholder that named a key no scope held and carried no
    /// fallback, in the order met; each rendered empty.
    pub missing: Vec<String>,
    /// Every revision read, in the order first used.
    pub contributed: Vec<Contributed>,
}

/// Render `template` from `inputs`.
#[must_use]
pub fn render(template: &str, inputs: &Inputs) -> Rendered {
    let mut out = String::with_capacity(template.len());
    let mut missing = Vec::new();
    let mut contributed: Vec<Contributed> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            // An opening with no closing is text.
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let inner = after[..close].trim();
        match placeholder(inner, inputs, &mut contributed) {
            Some(value) => out.push_str(&value),
            None => missing.push(inner.to_owned()),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    Rendered {
        text: out,
        missing,
        contributed,
    }
}

fn note(contributed: &mut Vec<Contributed>, kind: &str, key: &str, revision: u64) {
    if !contributed
        .iter()
        .any(|held| held.kind == kind && held.key == key)
    {
        contributed.push(Contributed {
            kind: kind.to_owned(),
            key: key.to_owned(),
            revision,
        });
    }
}

/// The value of one placeholder, or none when it names a key nobody holds
/// and carries no fallback.
fn placeholder(inner: &str, inputs: &Inputs, contributed: &mut Vec<Contributed>) -> Option<String> {
    let (name, fallback) = match inner.split_once('|') {
        Some((name, fallback)) => (name.trim(), Some(fallback.trim())),
        None => (inner, None),
    };
    if let Some(key) = name.strip_prefix("vars.") {
        for scope in [inputs.session.as_ref(), inputs.agent.as_ref()]
            .into_iter()
            .flatten()
        {
            if let Some(variable) = scope.values.get(key) {
                note(contributed, "variables", &scope.name, scope.revision);
                return Some(shown(&variable.value));
            }
        }
        return fallback.map(str::to_owned);
    }
    let value = match name {
        "goals" => {
            if let Some(revision) = inputs.goals_revision {
                note(contributed, "goals", "goals", revision);
            }
            Some(goals_text(&inputs.goals, inputs.now))
        }
        "time_left" => inputs
            .deadline
            .map(|deadline| time_left(deadline, inputs.now)),
        "deadline" => inputs.deadline.map(|deadline| stamp(deadline)),
        other => inputs.numbers.get(other).cloned(),
    };
    value.or_else(|| fallback.map(str::to_owned))
}

/// A value as words: a string as itself, anything else as its JSON.
fn shown(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The unfinished goals as one line each, or that there are none.
fn goals_text(goals: &[GoalLine], now: u64) -> String {
    if goals.is_empty() {
        return "No open goal.".to_owned();
    }
    let lines: Vec<String> = goals
        .iter()
        .map(|goal| match goal.deadline {
            Some(deadline) => format!(
                "{}: {} ({})",
                capitalised(&goal.kind),
                goal.words,
                time_left(deadline, now)
            ),
            None => format!("{}: {}", capitalised(&goal.kind), goal.words),
        })
        .collect();
    lines.join(" ")
}

fn capitalised(word: &str) -> String {
    let mut characters = word.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

/// The time left to `deadline` at `now`, or how long past it.
#[must_use]
pub fn time_left(deadline: u64, now: u64) -> String {
    if deadline >= now {
        format!("{} left", span(deadline - now))
    } else {
        format!("{} past the deadline", span(now - deadline))
    }
}

/// `seconds` as hours and minutes, or seconds under a minute.
#[must_use]
pub fn span(seconds: u64) -> String {
    let (hours, minutes) = (seconds / 3600, seconds % 3600 / 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds} seconds"),
        (0, _) => format!("{minutes} minutes"),
        _ => format!("{hours} hours {minutes} minutes"),
    }
}

/// `at` as an RFC 3339 instant in UTC, or the number when it is out of range.
#[must_use]
pub fn stamp(at: u64) -> String {
    i64::try_from(at)
        .ok()
        .and_then(|seconds| jiff::Timestamp::from_second(seconds).ok())
        .map_or_else(|| at.to_string(), |instant| instant.to_string())
}
