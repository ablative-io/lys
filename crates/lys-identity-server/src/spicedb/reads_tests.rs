//! A read of a kind nothing was ever written of is empty, not an outage.

use super::{reads_nothing_of_a_kind_never_written, undefined_kind};
use serde_json::json;

/// What the engine's HTTP door answered on 14 October 2026 when the first
/// seat was started before any grant on a seat had been written.
const HTTP_DOOR: &str = r#"{"error":{"code":9,"message":"object definition `seat` not found","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ERROR_REASON_UNKNOWN_DEFINITION","domain":"authzed.com","metadata":{"definition_name":"seat"}}]}}"#;
const TOP_LEVEL: &str = r#"{"code":9,"message":"object definition `seat` not found","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ERROR_REASON_UNKNOWN_DEFINITION","domain":"authzed.com","metadata":{"definition_name":"seat"}}]}"#;

#[test]
fn the_undefined_kind_is_read_from_either_shape_of_the_refusal() {
    assert_eq!(undefined_kind(400, HTTP_DOOR).as_deref(), Some("seat"));
    assert_eq!(undefined_kind(400, TOP_LEVEL).as_deref(), Some("seat"));
    assert_eq!(
        undefined_kind(500, HTTP_DOOR),
        None,
        "another status is not this refusal"
    );
    assert_eq!(
        undefined_kind(400, "{\"error\":{\"code\":3,\"message\":\"bad\"}}"),
        None
    );
    assert_eq!(undefined_kind(400, "not json"), None);
}

#[test]
fn the_first_read_of_a_kind_never_written_is_empty_and_any_other_refusal_stands() {
    let seats = json!({"resourceType": "seat", "optionalResourceId": "lys-device-walk"});
    assert!(reads_nothing_of_a_kind_never_written(
        &seats, 400, HTTP_DOOR
    ));
    // Under a scope's prefix the engine names the kind as it holds it.
    let scoped = HTTP_DOOR.replace(
        "\"definition_name\":\"seat\"",
        "\"definition_name\":\"t_ab12/seat\"",
    );
    assert!(reads_nothing_of_a_kind_never_written(&seats, 400, &scoped));
    // A kind that is not the one asked for is not an empty read of it.
    let other = json!({"resourceType": "channel"});
    assert!(!reads_nothing_of_a_kind_never_written(
        &other, 400, HTTP_DOOR
    ));
    let suffix = json!({"resourceType": "eat"});
    assert!(
        !reads_nothing_of_a_kind_never_written(&suffix, 400, HTTP_DOOR),
        "`seat` is not `eat` under a prefix"
    );
    // A filter with no resource kind, or a refusal of another sort, is an outage still.
    assert!(!reads_nothing_of_a_kind_never_written(
        &json!({}),
        400,
        HTTP_DOOR
    ));
    assert!(!reads_nothing_of_a_kind_never_written(
        &seats,
        503,
        "upstream connect error"
    ));
}
