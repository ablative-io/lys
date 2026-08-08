#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// The `ok` key is inserted by the emitter, not by any command.
///
/// The second party is the module docs of `crates/lys/src/commands/output.rs`,
/// which state the contract this file is a copy of: *"Every JSON object carries
/// `ok`. Success is `{"ok":true,...}`"*. That sentence was written for a
/// different binary and cannot be edited by changing this one, so a drift here
/// disagrees with something outside this crate.
#[test]
fn success_object_carries_ok_true_without_the_command_saying_so() {
    let mut emit = Emitter::new(true);
    emit.field("origin", "origin", "example.com/anchor");
    let value = emit.into_value().unwrap();
    assert_eq!(value["ok"], Value::Bool(true));
    assert_eq!(value["origin"], Value::String("example.com/anchor".into()));
}

/// Human mode produces no object at all — `--json` is additive, never a rewrite
/// of the human form.
#[test]
fn human_mode_produces_no_object() {
    let mut emit = Emitter::new(false);
    emit.field("origin", "origin", "example.com/anchor");
    assert!(emit.into_value().is_none());
    assert!(!Emitter::new(false).is_json());
    assert!(Emitter::new(true).is_json());
}

/// A failure a machine consumer receives is parseable, and says `ok: false`.
///
/// Asserted on the same shape the success path uses, because the whole point of
/// the rule is that a consumer branching on `ok` never has to parse prose — and
/// a failure object that omitted `ok` would break exactly the consumer that did
/// the right thing.
#[test]
fn failure_object_is_ok_false_with_the_message_verbatim() {
    let message = "the anchor's admission policy did not admit this submission";
    let mut fields = Map::new();
    fields.insert("ok".to_string(), Value::Bool(false));
    fields.insert("error".to_string(), Value::String(message.to_string()));
    let rendered = Value::Object(fields).to_string();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["ok"], Value::Bool(false));
    assert_eq!(parsed["error"], Value::String(message.to_string()));
}
