#![cfg(test)]

use super::{Kind, parse};

#[test]
fn reordering_entries_does_not_change_their_operation_ids() {
    let first = parse(
        br#"{"agents":[{"display_name":"Gypsy"},{"display_name":"Apollo"}]}"#,
        "loader-a",
    )
    .unwrap();
    let second = parse(
        br#"{"agents":[{"display_name":"Apollo"},{"display_name":"Gypsy"}]}"#,
        "loader-a",
    )
    .unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0], second[1]);
    assert_eq!(first[1], second[0]);
    assert_ne!(first[0].operation, first[1].operation);
}

#[test]
fn canonical_content_ignores_whitespace_and_nested_member_order() {
    let first = parse(br#"{"apps":[{"id":"cambium","name":"Cambium","redirects":[],"schema":{"kinds":{},"version":1}}]}"#, "loader-a").unwrap();
    let second = parse(br#"{ "apps": [{"schema":{"version":1,"kinds":{}},"redirects":[],"name":"Cambium","id":"cambium"}] }"#, "loader-a").unwrap();
    assert_eq!(first, second);
    assert_eq!(first[0].kind, Kind::App);
}

#[test]
fn account_and_changed_content_both_change_the_operation() {
    let document = br#"{"agents":[{"display_name":"Gypsy"}]}"#;
    let first = parse(document, "loader-a").unwrap();
    let other_account = parse(document, "loader-b").unwrap();
    let changed = parse(br#"{"agents":[{"display_name":"Chippy"}]}"#, "loader-a").unwrap();
    assert_ne!(first[0].operation, other_account[0].operation);
    assert_ne!(first[0].operation, changed[0].operation);
}

#[test]
fn duplicate_names_and_injected_operations_are_refused_before_execution() {
    let duplicate = parse(
        br#"{"agents":[{"display_name":"Gypsy"},{"display_name":"Gypsy"}]}"#,
        "loader-a",
    )
    .unwrap_err();
    assert_eq!(duplicate.entry, "agents/Gypsy");
    assert_eq!(duplicate.reason, "entry name is repeated");
    let injected = parse(
        br#"{"agents":[{"display_name":"Gypsy","operation":"op-not-mine"}]}"#,
        "loader-a",
    )
    .unwrap_err();
    assert_eq!(injected.entry, "agents/Gypsy");
    assert_eq!(injected.reason, "operation ids are derived, not supplied");
}

#[test]
fn unknown_sections_and_request_fields_are_not_silently_ignored() {
    assert!(parse(br#"{"operator":"secret"}"#, "loader-a").is_err());
    let refused = parse(
        br#"{"agents":[{"display_name":"Gypsy","admin":true}]}"#,
        "loader-a",
    )
    .unwrap_err();
    assert_eq!(refused.entry, "agents/Gypsy");
    assert_eq!(refused.reason, "unknown request member");
}

#[test]
fn malformed_input_never_appears_in_the_error() {
    let refused = parse(b"credential-secret-material", "loader-a").unwrap_err();
    assert!(!refused.to_string().contains("credential-secret-material"));
}
