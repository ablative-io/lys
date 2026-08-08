//! Output emission for every subcommand: human lines or one JSON object.
//!
//! # This is a second copy of `crates/lys/src/commands/output.rs`, and the
//! duplication is forced rather than chosen
//!
//! `crates/lys` has **no library target** — it is `[[bin]]` and nothing else —
//! so not one line of its plumbing is reachable from another crate. Combined
//! with `BUILD-PLAN.md` §2.5, which rules that the anchor CLI must not be
//! subcommands on the published `lys` binary, there is no arrangement in which
//! these two emitters are one emitter. Giving `lys` a library target would fix
//! the duplication by turning a binary's internals into a published crate's
//! semver-bound public API, which is a worse trade.
//!
//! What that costs is real and is worth naming: **two copies of an output
//! contract can drift, and a consumer parsing `{"ok":…}` from both binaries is
//! the party that finds out.** The contract is kept identical here on purpose,
//! and the invariants below are copied verbatim so a reader comparing the two
//! files is comparing prose as well as code.
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
//! - Error strings are the CLI's existing messages. `AnchorError::NotAdmitted`
//!   carries nothing by construction, and JSON mode reformats what is printed
//!   — it never adds detail a refusal deliberately withheld.

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
