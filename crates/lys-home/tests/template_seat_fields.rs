//! Optional seat slots are parsed and invalid values are refused by name.

use std::error::Error;

use lys_home::harness::claude_code::template::parse_template;
use serde_json::{Value, json};

fn template() -> Value {
    json!({"harness":"claude-code", "flags":[], "slots": {
        "transcript":{"fill":"resume-by-path", "canon":null}, "mcp":{}, "env":{},
        "secrets":{"use_only":[], "readable":[], "reader":""}, "instructions":""}})
}

#[test]
fn template_parses_every_seat_slot() -> Result<(), Box<dyn Error>> {
    let mut failures = Vec::new();
    for (field, value) in [
        (
            "hooks",
            json!({"PostToolUse":[{"matcher":"Read", "command":"check", "timeout":5}]}),
        ),
        ("status_line", json!("seat-status")),
        ("settings", json!({"theme":"dark", "nested":{"a":1}})),
        ("resume", json!("019c6e27-e55b-73d1-87d8-4e01f1f75043")),
        ("context_window", json!(200_000)),
        ("config_dir", json!("/tmp/chosen-config")),
    ] {
        let mut given = template();
        given["slots"][field] = value.clone();
        match parse_template(&serde_json::to_vec(&given)?) {
            Ok(parsed) => assert_eq!(serde_json::to_value(parsed.seat)?[field], value),
            Err(error) => failures.push(format!("{field}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[test]
fn template_refuses_invalid_seat_slots_by_name() -> Result<(), Box<dyn Error>> {
    for (field, value, named) in [
        ("hooks", json!({"UnknownEvent":[]}), "UnknownEvent"),
        ("hooks", json!({"Stop":[{"command":""}]}), "command"),
        (
            "hooks",
            json!({"Stop":[{"command":"check", "timeout":0}]}),
            "timeout",
        ),
        ("status_line", json!(""), "status_line"),
        ("settings", json!({"env":{}}), "env"),
        ("resume", json!("not-an-id"), "resume"),
        ("context_window", json!(0), "context_window"),
        ("config_dir", json!("relative/config"), "config_dir"),
    ] {
        let mut given = template();
        given["slots"][field] = value;
        let error =
            parse_template(&serde_json::to_vec(&given)?).expect_err("invalid slot accepted");
        assert!(error.to_string().contains(named), "{error}");
    }
    Ok(())
}

#[test]
fn template_seat_conversion_refusals_keep_the_underlying_error() -> Result<(), Box<dyn Error>> {
    use lys_home::harness::launch_fields::SeatFields;
    for (field, value, cause) in [
        ("context_window", json!(true), "invalid type: boolean"),
        (
            "hooks",
            json!({"Stop":[{"command":"check", "timeout":4_294_967_296_u64}]}),
            "out of range integral type conversion attempted",
        ),
    ] {
        let slots = json!({field:value});
        let error = SeatFields::parse(
            slots.as_object().ok_or("slots are not an object")?,
            "claude-code/template-v1",
        )
        .expect_err("invalid conversion accepted");
        assert!(error.to_string().contains(cause), "{error}");
        let mut given = template();
        given["slots"][field] = slots[field].clone();
        let error =
            parse_template(&serde_json::to_vec(&given)?).expect_err("invalid conversion accepted");
        assert!(
            error.to_string().contains(field) && error.to_string().contains(cause),
            "{error}"
        );
    }
    Ok(())
}
