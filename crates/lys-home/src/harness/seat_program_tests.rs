use super::launch_fields::{InstructionsMode, LaunchFields};
use super::{rendering, rendering_launch};
use serde_json::{Value, json};
use std::error::Error;

fn spec() -> Value {
    json!({"executable": "/opt/seat/host", "executable_sha256": "a".repeat(64),
        "arguments": ["run", "--config", "{seat_file}", "", " literal "],
        "seat_file": "/opt/seat/seat.json", "seat_file_sha256": "b".repeat(64),
        "adapter": "acp/1"})
}

fn fields(seat: Value) -> Result<LaunchFields, serde_json::Error> {
    serde_json::from_value(json!({
        "identity": {"agent": "agent", "session": "session", "machine": "machine", "version": 1},
        "harness": {"name": "Seat host", "program": "/opt/seat/host", "package": "seat 1",
            "description": {"models": {"minimum": 0, "maximum": 0, "further_encoding": {"kind": "array"}},
                "permissions": {"modes": [], "rule_forms": []},
                "mcp": {"transports": [], "working_directory": false, "handle_variables": false, "channel_policies": []},
                "rendering_contract": "seat-program/v1", "seat_program": seat}},
        "instructions": "", "models": [], "mcp_servers": [], "skills": [], "plugins": [],
        "model_proxy": "http://localhost:8411/anthropic", "run": "0123456789abcdef0123456789abcdef"
    }))
}

#[test]
fn seat_program_contract_is_registered() {
    assert!(rendering::registered("seat-program/v1"));
}

#[test]
fn seat_program_argv_is_exactly_the_reviewed_literals() -> Result<(), Box<dyn Error>> {
    let fields = fields(spec())?;
    let rendered = rendering::render(&fields, &[], &json!({}), &[])?;
    let launch = rendering_launch::render(
        "seat-program/v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Keep,
    )?;
    assert_eq!(launch.program, "/opt/seat/host");
    assert_eq!(
        launch.arguments,
        ["run", "--config", "/opt/seat/seat.json", "", " literal "]
    );
    assert!(launch.files.is_empty());
    assert!(launch.argument_files.is_empty());
    Ok(())
}

#[test]
fn seat_program_malformed_fields_are_refused_by_name() -> Result<(), Box<dyn Error>> {
    let mut bad = Vec::new();
    for field in [
        "executable",
        "executable_sha256",
        "arguments",
        "seat_file",
        "seat_file_sha256",
        "adapter",
    ] {
        let mut value = spec();
        value
            .as_object_mut()
            .ok_or("spec is not an object")?
            .remove(field);
        bad.push((field, value));
    }
    for field in ["executable", "seat_file"] {
        let mut value = spec();
        value[field] = json!("relative/path");
        bad.push((field, value));
    }
    for arguments in [
        json!(["run"]),
        json!(["{seat_file}", "{seat_file}"]),
        json!(["{seat_file}", 42]),
    ] {
        let mut value = spec();
        value["arguments"] = arguments;
        bad.push(("arguments", value));
    }
    for field in ["confinement", "CLAUDE_CONFIG_DIR", "flags"] {
        let mut value = spec();
        value[field] = json!("native");
        bad.push((field, value));
    }
    for (field, value) in bad {
        let error = match fields(value) {
            Err(error) => error.to_string(),
            Ok(fields) => rendering::render(&fields, &[], &json!({}), &[])
                .err()
                .ok_or("malformed seat accepted")?
                .to_string(),
        };
        assert!(error.contains(field), "{field}: {error}");
    }
    Ok(())
}
