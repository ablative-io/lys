#![cfg(test)]
//! Approval metadata travels with the immutable draft and request digest.

use lys_pass::drafts::ApprovedDraft;
use serde_json::json;

struct NoProductCalls;

impl lys_pass::drafts::Executor for NoProductCalls {
    type Error = std::io::Error;

    fn receipt(&self, _: &str) -> Result<Option<lys_pass::drafts::Execution>, Self::Error> {
        Err(std::io::Error::other("product was reached"))
    }

    fn execute_and_record(
        &mut self,
        _: &ApprovedDraft,
    ) -> Result<lys_pass::drafts::Execution, Self::Error> {
        Err(std::io::Error::other("product was reached"))
    }
}

#[test]
fn approved_draft_round_trips_every_approver_and_mode_with_its_request()
-> Result<(), Box<dyn std::error::Error>> {
    let value = json!({"id":"draft-7", "app":"liminal", "grant":"grant-7",
        "target":{"kind":"liminal.service","id":"seat-7","action":"stop"},
        "words":"stop seat-7", "request_digest":lys_pass::drafts::request_digest("stop seat-7"),
        "approved_by":["person-1","person-2"], "mode":"by_two"});
    let draft: ApprovedDraft = serde_json::from_value(value.clone())?;
    assert_eq!(serde_json::to_value(draft)?, value);
    Ok(())
}

#[test]
fn invalid_approval_evidence_is_refused_before_any_product_call()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, people) in [
        ("outright", vec!["person-1"]),
        ("by_draft", vec![]),
        ("by_two", vec!["person-1"]),
        ("by_two", vec!["person-1", "person-1"]),
        ("by_draft", vec![""]),
    ] {
        let draft: ApprovedDraft = serde_json::from_value(json!({
            "id":"draft-7", "app":"liminal", "grant":"grant-7",
            "target":{"kind":"liminal.service","id":"seat-7","action":"stop"},
            "words":"stop seat-7", "request_digest":lys_pass::drafts::request_digest("stop seat-7"),
            "approved_by":people, "mode":mode,
        }))?;
        assert!(
            matches!(
                lys_pass::drafts::execute_once(&draft, &mut NoProductCalls),
                Err(lys_pass::drafts::ExecuteError::Contract(_))
            ),
            "{mode}: {:?}",
            draft.approved_by
        );
    }
    Ok(())
}
