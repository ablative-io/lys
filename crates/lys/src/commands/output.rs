//! Output emission for every subcommand: human lines or one JSON object.
//!
//! The CLI's flagship documented use is a CI pipeline, and a pipeline that
//! has to `grep` and `awk` a human-readable transcript is one whitespace
//! change away from silently gating on nothing. `--json` is global rather
//! than per-command so a caller never has to discover which subcommands
//! honour it — every command routes its output through [`Emitter`].
//!
//! # Invariants
//!
//! - Human mode prints exactly the lines it always printed. `--json` is
//!   additive; it does not reword the human form.
//! - JSON mode emits exactly one object on stdout, at the end. Nothing is
//!   printed incrementally, so a consumer never sees a half-written object
//!   if a command fails partway.
//! - Every JSON object carries `ok`. Success is `{"ok":true,...}` and
//!   failure is `{"ok":false,"error":"..."}` — a consumer that checks only
//!   the exit code still works, and one that checks only `ok` still works,
//!   but neither has to parse prose.
//! - Failures in JSON mode are JSON. Emitting a bare stderr line to a
//!   machine consumer is the same silent-failure shape this repo refuses
//!   elsewhere: the caller asked for parseable output and would get
//!   something it cannot parse at exactly the moment it matters most.
//! - Error strings are the CLI's existing messages, which are already
//!   collapsed to non-specific text for verification failures. JSON mode
//!   must not widen that: it reformats what is printed, never adds detail.

use serde_json::{Map, Value};

/// Accumulates a command's output and renders it in the requested form.
///
/// Construct with [`Emitter::new`], add fields as the command learns them,
/// and call [`Emitter::finish`] exactly once on the success path.
pub struct Emitter {
    json: bool,
    fields: Map<String, Value>,
}

impl Emitter {
    /// Creates an emitter for the requested output mode.
    #[must_use]
    pub fn new(json: bool) -> Self {
        Self {
            json,
            fields: Map::new(),
        }
    }

    /// Whether this emitter is producing JSON.
    #[must_use]
    pub fn is_json(&self) -> bool {
        self.json
    }

    /// Records one field.
    ///
    /// `label` is the human-readable prefix (printed as `label: value`) and
    /// `key` is the JSON object key. They are passed separately because the
    /// human labels contain spaces and parentheses that make poor JSON keys,
    /// and because renaming a human label must not silently change a JSON
    /// key that a pipeline depends on.
    pub fn field(&mut self, label: &str, key: &str, value: impl Into<Value>) {
        let value = value.into();
        if self.json {
            self.fields.insert(key.to_string(), value);
        } else {
            match &value {
                Value::String(text) => println!("{label}: {text}"),
                other => println!("{label}: {other}"),
            }
        }
    }

    /// Records a successful outcome: a bare line for a human, `key: true`
    /// for a machine.
    ///
    /// Verification commands print a standalone `... verified` line that
    /// carries no datum but is the single most important thing they say. A
    /// JSON consumer must be able to branch on it as a boolean rather than
    /// matching prose, so it becomes a field while staying a sentence for
    /// the operator.
    pub fn flag(&mut self, line: &str, key: &str) {
        if self.json {
            self.fields.insert(key.to_string(), Value::Bool(true));
        } else {
            println!("{line}");
        }
    }

    /// Prints a line in human mode only.
    ///
    /// For prose that carries no datum — banners, warnings, and the
    /// `UNVERIFIED` markers. A JSON consumer gets the corresponding boolean
    /// or field instead, so the guidance is not lost, only unformatted.
    pub fn note(&mut self, line: &str) {
        if !self.json {
            println!("{line}");
        }
    }

    /// Renders the success object, or `None` in human mode.
    ///
    /// Separated from [`Emitter::finish`] so the shape can be asserted in
    /// tests without capturing stdout — a test that scrapes stdout tends to
    /// pass for the wrong reasons.
    fn into_value(mut self) -> Option<Value> {
        if !self.json {
            return None;
        }
        // Inserted here rather than by the caller so no command can forget it.
        self.fields.insert("ok".to_string(), Value::Bool(true));
        Some(Value::Object(self.fields))
    }

    /// Emits the JSON object, if in JSON mode. No-op for human output.
    ///
    /// Call once, on the success path.
    pub fn finish(self) {
        if let Some(value) = self.into_value() {
            println!("{value}");
        }
    }
}

/// Prints a failure as JSON on stdout, for the `--json` error path.
///
/// Called from `main` when a command fails and JSON was requested. The
/// diagnostic still goes to stderr as well, so an operator watching a
/// terminal sees it in the usual place.
pub fn emit_json_error(message: &str) {
    let mut fields = Map::new();
    fields.insert("ok".to_string(), Value::Bool(false));
    fields.insert("error".to_string(), Value::String(message.to_string()));
    println!("{}", Value::Object(fields));
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
