#![cfg(test)]
//! DIRECTORY-029 R7, CONFORMANCE 5.2: the machine may reach what the profile
//! needs reads the profile's needs and the machine's egress list from the
//! network record of CONFORMANCE row 8.5, and never assumes open egress.

use lys_identity::start::egress::{EgressLists, ProfileNeeds, check};

/// What pv-fixture-1 needs.
struct Needs;

impl ProfileNeeds for Needs {
    fn needs(&self, profile_version: &str) -> Option<Vec<String>> {
        (profile_version == "pv-fixture-1")
            .then(|| vec!["model providers".to_owned(), "mcp-fixture".to_owned()])
    }
}

/// machine-fixture-1's egress list, or its absence.
struct Egress(Option<Vec<&'static str>>);

impl EgressLists for Egress {
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        let listed = self.0.as_ref()?;
        Some(if machine == "machine-fixture-1" {
            listed.iter().map(|entry| (*entry).to_owned()).collect()
        } else {
            Vec::new()
        })
    }
}

#[test]
fn a_machine_that_reaches_every_need_passes() {
    let egress = Egress(Some(vec!["model providers", "mcp-fixture"]));
    assert_eq!(
        check(&Needs, &egress, "pv-fixture-1", "machine-fixture-1"),
        Ok(())
    );
}

#[test]
fn a_destination_off_the_list_is_named() {
    let egress = Egress(Some(vec!["model providers"]));
    let refusal = check(&Needs, &egress, "pv-fixture-1", "machine-fixture-1")
        .expect_err("mcp-fixture is not reachable");
    assert_eq!(refusal.name(), "egress_not_reachable");
    assert!(refusal.to_string().contains("mcp-fixture"), "{refusal}");
}

#[test]
fn with_no_egress_record_the_check_names_row_8_5() {
    let refusal = check(&Needs, &Egress(None), "pv-fixture-1", "machine-fixture-1")
        .expect_err("the record is missing");
    assert_eq!(refusal.name(), "check_record_missing");
    let words = refusal.to_string();
    assert!(
        words.contains("the machine may reach what the profile needs"),
        "{words}"
    );
    assert!(words.contains("row 8.5"), "{words}");
}
