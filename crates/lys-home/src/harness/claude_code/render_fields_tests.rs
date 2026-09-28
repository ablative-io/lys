#![cfg(test)]
//! Gates on the render's checked readers and on the words of its refusals:
//! a field absent, a field of another type and a field present each read as
//! the reader's contract says, the one default is an absent `redacted`, and
//! the stopReason arms map three values and refuse every other by name.

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::render_fields::{array_or_string, redacted, stop_reason, string};

/// The words of a field refusal of `field` in entry `x1` of session `s`.
fn field(field: &'static str, expected: &'static str, missing: bool) -> String {
    let error = HomeError::RenderField {
        session: "s".to_owned(),
        entry: "x1".to_owned(),
        field,
        expected,
        missing,
    };
    error.to_string()
}

/// Assert `result` is a field refusal of `field` in entry `x1`, naming
/// `expected` and `missing`.
fn assert_refused<T>(result: Result<T, HomeError>, field: &str, expected: &str, missing: bool) {
    let named = match result {
        Err(HomeError::RenderField {
            entry,
            field: named_field,
            expected: named_type,
            missing: named_missing,
            ..
        }) => Some((entry, named_field, named_type, named_missing)),
        _ => None,
    };
    let wanted = ("x1".to_owned(), field, expected, missing);
    assert_eq!(named, Some(wanted));
}

#[test]
fn a_missing_field_reads_is_missing_expected_its_type() {
    let words =
        "render of session s refused entry x1: field toolCallId is missing, expected string";
    assert_eq!(field("toolCallId", "string", true), words);
}

#[test]
fn a_field_of_another_type_reads_is_not_its_type() {
    let words = "render of session s refused entry x1: field isError is not boolean";
    assert_eq!(field("isError", "boolean", false), words);
}

#[test]
fn a_missing_provider_or_api_names_the_record_that_answers_it() {
    let mut checked = 0;
    for name in ["provider", "api"] {
        let words = format!(
            "render of session s refused entry x1: field {name} is missing, expected string; the render needs a record whose assistant messages carry provider and api"
        );
        assert_eq!(field(name, "string", true), words);
        checked += 1;
    }
    assert_eq!(checked, 2);
}

#[test]
fn an_unmapped_stop_reason_names_the_value() {
    let error = HomeError::RenderStopReason {
        session: "s".to_owned(),
        entry: "x1".to_owned(),
        value: "halted".to_owned(),
    };
    let words = "render of session s refused entry x1: stopReason halted has no Claude Code value";
    assert_eq!(error.to_string(), words);
}

#[test]
fn the_string_reader_refuses_another_type_an_absence_and_a_null() {
    let read = |source: Value| string("s", "x1", &source, "text").map(str::to_owned);
    assert_refused(read(json!({"text": 3})), "text", "string", false);
    assert_refused(read(json!({})), "text", "string", true);
    assert_refused(read(json!({"text": null})), "text", "string", false);
    assert_eq!(read(json!({"text": "a"})).ok(), Some("a".to_owned()));
}

#[test]
fn the_array_or_string_reader_takes_either_unchanged_and_refuses_a_number() {
    let read = |source: Value| array_or_string("s", "x1", &source, "content").cloned();
    assert_eq!(read(json!({"content": "a"})).ok(), Some(json!("a")));
    assert_eq!(read(json!({"content": []})).ok(), Some(json!([])));
    let number = read(json!({"content": 5}));
    assert_refused(number, "content", "array or string", false);
}

#[test]
fn an_absent_redacted_reads_as_not_redacted_and_a_non_boolean_one_refuses() {
    let read = |source: Value| redacted("s", "x1", &source);
    assert_eq!(read(json!({})).ok(), Some(false));
    assert_eq!(read(json!({"redacted": true})).ok(), Some(true));
    let words = read(json!({"redacted": "yes"}));
    assert_refused(words, "redacted", "boolean", false);
}

#[test]
fn the_stop_reason_arms_map_three_values_and_refuse_every_other_by_name() {
    let read = |source: Value| stop_reason("s", "x1", &source);
    let mut mapped = Vec::new();
    for value in ["toolUse", "length", "stop"] {
        mapped.push(read(json!({"stopReason": value})).ok());
    }
    let expected = [Some("tool_use"), Some("max_tokens"), Some("end_turn")];
    assert_eq!(mapped, expected);
    let mut refused = 0;
    for value in ["error", "aborted", "halted"] {
        let named = match read(json!({"stopReason": value})) {
            Err(HomeError::RenderStopReason { entry, value, .. }) => Some((entry, value)),
            _ => None,
        };
        assert_eq!(named, Some(("x1".to_owned(), value.to_owned())));
        refused += 1;
    }
    assert_eq!(refused, 3);
    assert_refused(read(json!({})), "stopReason", "string", true);
    let number = read(json!({"stopReason": 1}));
    assert_refused(number, "stopReason", "string", false);
}
