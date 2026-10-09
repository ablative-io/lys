#![cfg(test)]
//! Products use the same refusal bytes and authorization vectors.

use lys_pass::{Refusal, conformance};

#[test]
fn refusal_bytes_are_one_shape() -> Result<(), Box<dyn std::error::Error>> {
    let expected = include_str!("../fixtures/refusals/missing-grant.json").trim();
    let refusal = Refusal::new(
        "sample",
        "sample.file",
        "restricted",
        "read",
        "https://issuer.example",
    )?;
    assert_eq!(serde_json::to_string(&refusal)?, expected);
    assert_eq!(serde_json::from_str::<Refusal>(expected)?, refusal);
    Ok(())
}

#[test]
fn shipped_conformance_vectors_cover_authorization_and_refusal() {
    assert_eq!(conformance::cases().len(), 10);
    assert!(
        conformance::cases()
            .iter()
            .any(|case| case.name == "deliberate-unreachable")
    );
    assert!(
        conformance::cases()
            .iter()
            .any(|case| case.name == "restricted-child")
    );
}

#[test]
fn consumer_gate_names_all_missing_predicates() {
    let report = conformance::run(|case| {
        Ok::<_, std::convert::Infallible>(conformance::Outcome::Allowed(case.name.to_owned()))
    });
    assert_eq!(report.run, 10);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failures.len(), 10);
    assert!(report.require_conformant().is_err());
}
