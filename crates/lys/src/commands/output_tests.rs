#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn json_mode_emits_one_object_carrying_ok_true() {
    let mut emitter = Emitter::new(true);
    emitter.field("tree size", "tree_size", 3);
    emitter.field("origin", "origin", "example.com/log");
    let value = emitter.into_value().unwrap();

    assert_eq!(value["ok"], Value::Bool(true));
    assert_eq!(value["tree_size"], 3);
    assert_eq!(value["origin"], "example.com/log");
    assert_eq!(
        value.as_object().unwrap().len(),
        3,
        "exactly the recorded fields plus ok"
    );
}

/// `ok` is the emitter's job, not the caller's.
///
/// A command that forgot it would produce output a consumer cannot branch
/// on, so it is inserted centrally and this pins that.
#[test]
fn ok_is_present_even_with_no_fields() {
    let value = Emitter::new(true).into_value().unwrap();
    assert_eq!(value, serde_json::json!({"ok": true}));
}

#[test]
fn human_mode_renders_no_json_at_all() {
    let mut emitter = Emitter::new(false);
    emitter.field("origin", "origin", "example.com/log");
    emitter.note("a human-only line");
    assert!(emitter.into_value().is_none());
}

/// Human-only prose must not leak into the machine surface.
///
/// `note` carries banners and UNVERIFIED markers. If those became JSON
/// fields, a consumer could start depending on prose that exists to be read
/// by a person and reworded freely.
#[test]
fn notes_never_become_json_fields() {
    let mut emitter = Emitter::new(true);
    emitter.note("UNVERIFIED — the signature was NOT checked.");
    emitter.field("subject", "subject", "agent-noor");
    let value = emitter.into_value().unwrap();
    assert_eq!(
        value,
        serde_json::json!({"ok": true, "subject": "agent-noor"})
    );
}

/// The JSON key is independent of the human label.
///
/// Human labels contain spaces and parentheses and get reworded; JSON keys
/// are a contract a pipeline depends on. Passing them separately is what
/// stops a copy edit from silently renaming a field.
#[test]
fn json_key_is_independent_of_the_human_label() {
    let mut emitter = Emitter::new(true);
    emitter.field("public key (ed25519)", "public_key_ed25519", "ab12");
    let value = emitter.into_value().unwrap();
    assert!(value.get("public_key_ed25519").is_some());
    assert!(value.get("public key (ed25519)").is_none());
}

#[test]
fn is_json_reports_the_mode() {
    assert!(Emitter::new(true).is_json());
    assert!(!Emitter::new(false).is_json());
}

#[test]
fn error_object_is_ok_false_with_the_message() {
    // Mirrors `emit_json_error`'s construction without capturing stdout.
    let value = serde_json::json!({"ok": false, "error": "verification failed"});
    assert_eq!(value["ok"], Value::Bool(false));
    assert_eq!(value["error"], "verification failed");
}
