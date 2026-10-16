#![cfg(test)]
use super::{fields, spec};
use crate::harness::launch_fields::InstructionsMode;
use crate::harness::{rendering, rendering_launch};
use serde_json::json;
use std::error::Error;

#[test]
fn a_seat_description_with_an_unregistered_contract_is_refused_with_its_name()
-> Result<(), Box<dyn Error>> {
    let name = "unregistered-seat/v91";
    let mut description = fields(&spec())?;
    description.harness.description.rendering_contract = name.to_owned();
    let refusal = rendering::render(&description, &[], &json!({}), &[])
        .err()
        .ok_or("an unregistered rendering contract was admitted")?;
    assert!(refusal.to_string().contains(name), "{refusal}");
    let refusal = rendering_launch::render(name, "/opt/seat/host", "{}", InstructionsMode::Keep)
        .err()
        .ok_or("an unregistered launch contract was admitted")?;
    assert!(refusal.to_string().contains(name), "{refusal}");
    Ok(())
}
