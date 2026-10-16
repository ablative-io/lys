#![cfg(test)]
use super::seat_program::SeatProgram;
use serde_json::{Value, json};
use std::error::Error;

fn spec() -> Value {
    json!({"executable":"/opt/seat/host","executable_sha256":"a".repeat(64),
        "arguments":["{seat_file}"],"seat_file":"/opt/seat/seat.json",
        "seat_file_sha256":"b".repeat(64),"adapter":"acp/1"})
}

#[test]
fn seat_program_model_wire_round_trips_both_reviewed_wires() -> Result<(), Box<dyn Error>> {
    for wire in ["openai-responses", "anthropic-messages"] {
        let mut value = spec();
        value["model_wire"] = json!(wire);
        let parsed: SeatProgram = serde_json::from_value(value.clone())?;
        assert_eq!(serde_json::to_value(parsed)?, value);
    }
    let value = spec();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<SeatProgram>(value.clone())?)?,
        value
    );
    Ok(())
}

#[test]
fn seat_program_model_wire_unknown_names_the_allowed_values() {
    let mut value = spec();
    value["model_wire"] = json!("gpt");
    let error = serde_json::from_value::<SeatProgram>(value)
        .expect_err("invalid wire admitted")
        .to_string();
    for word in ["model_wire", "openai-responses", "anthropic-messages"] {
        assert!(error.contains(word), "{error}");
    }
}
