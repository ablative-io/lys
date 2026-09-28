#![cfg(test)]
//! DIRECTORY-029 R5, CONFORMANCE 5.2: the machine is allowed for the role
//! reads the role's machines from the roles card `Ink1H1Os`'s record, and no
//! machine list comes from anywhere else.

use lys_identity::start::machine_role::{HeldRole, RoleMachines, check};

/// The role machines record, or its absence; agent-fixture-1 holds
/// role-fixture-builder.
struct Roles(Option<Vec<HeldRole>>);

impl RoleMachines for Roles {
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        let held = self.0.clone()?;
        Some(if agent == "agent-fixture-1" { held } else { Vec::new() })
    }
}

fn builder() -> Roles {
    Roles(Some(vec![HeldRole {
        role: "role-fixture-builder".to_owned(),
        machines: vec!["machine-fixture-1".to_owned()],
    }]))
}

#[test]
fn a_machine_the_role_allows_passes() {
    assert_eq!(check(&builder(), "agent-fixture-1", "machine-fixture-1"), Ok(()));
}

#[test]
fn a_machine_the_role_does_not_allow_is_refused_naming_both() {
    let refusal = check(&builder(), "agent-fixture-1", "machine-fixture-2")
        .expect_err("the role does not allow the machine");
    assert_eq!(refusal.name(), "machine_not_allowed_for_role");
    let words = refusal.to_string();
    assert!(words.contains("machine-fixture-2"), "{words}");
    assert!(words.contains("role-fixture-builder"), "{words}");
}

#[test]
fn with_no_role_machines_record_the_check_names_its_card() {
    let refusal = check(&Roles(None), "agent-fixture-1", "machine-fixture-1")
        .expect_err("the record is missing");
    assert_eq!(refusal.name(), "check_record_missing");
    let words = refusal.to_string();
    assert!(words.contains("the machine is allowed for the role"), "{words}");
    assert!(words.contains("Ink1H1Os"), "{words}");
}
