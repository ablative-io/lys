#![cfg(test)]
//! DIRECTORY-029 R6, CONFORMANCE 5.2 and 5.3: its virtual credentials are
//! valid reads the handle record SECRETS-002 keeps through the one seam
//! `HandleRecords`, which answers ids and validity only, and nothing the
//! check answers ever holds a credential value.

use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential, check};

/// A value the door keeps beside vc-fixture-1; the seam never answers it.
const VALUE_ONE: &str = "fixture-credential-value-1f3a";

/// The handle record: each credential's id, validity and the value kept
/// beside it, or no record at all.
struct Handles(Option<Vec<(&'static str, bool, &'static str)>>);

impl HandleRecords for Handles {
    fn handles(&self, agent: &str) -> HandleAnswer {
        match &self.0 {
            Some(held) if agent == "agent-fixture-1" => HandleAnswer::Held(
                held.iter()
                    .map(|(id, valid, _)| HeldCredential {
                        id: (*id).to_owned(),
                        valid: *valid,
                    })
                    .collect(),
            ),
            Some(_) | None => HandleAnswer::RecordMissing,
        }
    }
}

const SOURCE: &str = include_str!("../src/start/credentials.rs");

/// The lines of `SOURCE` between the line starting `open` and the next line
/// that is exactly `}`.
fn block(open: &str) -> Vec<&'static str> {
    SOURCE
        .lines()
        .skip_while(|line| !line.starts_with(open))
        .skip(1)
        .take_while(|line| *line != "}")
        .collect()
}

#[test]
fn the_seam_answers_ids_and_validity_and_never_a_value() {
    let methods: Vec<&str> = block("pub trait HandleRecords")
        .into_iter()
        .filter(|line| line.trim_start().starts_with("fn "))
        .collect();
    assert_eq!(
        methods,
        ["    fn handles(&self, agent: &str) -> HandleAnswer;"]
    );
    let fields: Vec<&str> = block("pub struct HeldCredential")
        .into_iter()
        .filter(|line| line.trim_start().starts_with("pub "))
        .collect();
    assert_eq!(fields, ["    pub id: String,", "    pub valid: bool,"]);
    let answer = block("pub enum HandleAnswer").join("\n");
    for field in ["value:", "secret:", "token:", "credential:"] {
        assert!(!answer.contains(field), "HandleAnswer has a field {field}");
    }
}

#[test]
fn a_valid_credential_passes_and_its_id_is_handed_on() {
    let handles = Handles(Some(vec![("vc-fixture-1", true, VALUE_ONE)]));
    assert_eq!(
        check(&handles, "agent-fixture-1"),
        Ok(Ok(vec!["vc-fixture-1".to_owned()]))
    );
}

#[test]
fn a_revoked_credential_is_not_valid() -> Result<(), Box<dyn std::error::Error>> {
    let handles = Handles(Some(vec![("vc-fixture-1", false, VALUE_ONE)]));
    let refusal = check(&handles, "agent-fixture-1")?.expect_err("no credential is valid");
    assert_eq!(refusal.name(), "virtual_credentials_not_valid");
    assert!(refusal.to_string().contains("agent-fixture-1"), "{refusal}");
    Ok(())
}

#[test]
fn with_no_handle_record_the_check_names_secrets_002() -> Result<(), Box<dyn std::error::Error>> {
    let refusal = check(&Handles(None), "agent-fixture-1")?.expect_err("the record is missing");
    assert_eq!(refusal.name(), "check_record_missing");
    let words = refusal.to_string();
    assert!(
        words.contains("its virtual credentials are valid"),
        "{words}"
    );
    assert!(words.contains("SECRETS-002"), "{words}");
    Ok(())
}

#[test]
fn no_output_of_the_check_holds_the_value_kept_beside_it() {
    let mut shown = 0;
    for valid in [true, false] {
        let handles = Handles(Some(vec![("vc-fixture-1", valid, VALUE_ONE)]));
        let answer = handles.handles("agent-fixture-1");
        let checked = check(&handles, "agent-fixture-1");
        let output = format!("{answer:?} {checked:?}");
        assert!(!output.contains(VALUE_ONE), "{output}");
        if let Ok(Err(refusal)) = checked {
            let words = format!("{refusal} {}", refusal.to_json());
            assert!(!words.contains(VALUE_ONE), "{words}");
        }
        shown += 1;
    }
    assert_eq!(shown, 2);
}

#[test]
fn an_answer_that_carried_a_value_is_refused_by_name() -> Result<(), Box<dyn std::error::Error>> {
    struct Carried;
    impl HandleRecords for Carried {
        fn handles(&self, _: &str) -> HandleAnswer {
            HandleAnswer::ValueInAnswer {
                record: "vc-fixture-1".to_owned(),
                field: "value".to_owned(),
            }
        }
    }
    let refusal = check(&Carried, "agent-fixture-1")?.expect_err("a value in the answer");
    assert_eq!(refusal.name(), "credential_value_in_answer");
    let words = refusal.to_string();
    assert!(
        words.contains("vc-fixture-1") && words.contains("value"),
        "{words}"
    );
    assert_eq!(Carried.handles("agent-fixture-1").refusal(), Some(refusal));
    Ok(())
}
