#![cfg(test)]
//! Scoped guest admission (ACCESS-006 R4): whether a subject holds at least
//! one current grant within a workspace, asked under the same contract.

use lys_pass::membership::{self, CONTRACT_VERSION, GrantLog, Subject, Verdict};
use lys_pass::membership_admission::{self, AdmissionDecision, AdmissionRequest, NO_CURRENT_GRANT};
use lys_pass::rights::Resource;

fn log() -> GrantLog {
    GrantLog {
        identity: "lys-grants".to_owned(),
        epoch: 0,
    }
}

fn request() -> AdmissionRequest {
    AdmissionRequest {
        contract: CONTRACT_VERSION,
        log: log(),
        workspace: Resource {
            kind: "rooms.workspace".to_owned(),
            id: "ward".to_owned(),
        },
        subject: Subject {
            id: "agent-1".to_owned(),
            kind: "agent".to_owned(),
        },
        at_least: 0,
    }
}

#[test]
fn admission_names_are_stable_and_closed() -> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string(&request())?;
    assert_eq!(
        text,
        concat!(
            r#"{"contract":1,"log":{"identity":"lys-grants","epoch":0},"#,
            r#""workspace":{"kind":"rooms.workspace","id":"ward"},"#,
            r#""subject":{"id":"agent-1","kind":"agent"},"at_least":0}"#
        )
    );
    assert_eq!(serde_json::from_str::<AdmissionRequest>(&text)?, request());
    let responsible = text.replace(
        r#","at_least":0"#,
        r#","at_least":0,"responsible":"person-1""#,
    );
    assert!(serde_json::from_str::<AdmissionRequest>(&responsible).is_err());
    Ok(())
}

#[test]
fn admission_is_checked_as_a_decision_is() {
    let mut future = request();
    future.contract = 7;
    assert_eq!(
        membership_admission::validate(&future, &log()).map_err(|refused| refused.name),
        Err(membership::CONTRACT_UNSUPPORTED)
    );
    let mut empty = request();
    empty.subject.id = String::new();
    assert_eq!(
        membership_admission::validate(&empty, &log()).map_err(|refused| refused.name),
        Err(membership::REQUEST_MALFORMED)
    );
    assert_eq!(membership_admission::validate(&request(), &log()), Ok(()));
}

#[test]
fn no_current_grant_is_a_named_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let refused = AdmissionDecision {
        contract: CONTRACT_VERSION,
        log: log(),
        request: request(),
        revision: Some(4),
        verdict: Verdict::Refused {
            refusal: NO_CURRENT_GRANT.to_owned(),
            reason: "no current grant within the workspace".to_owned(),
        },
    };
    let text = serde_json::to_string(&refused)?;
    assert!(text.contains(r#""refusal":"membership_no_current_grant""#));
    assert_eq!(serde_json::from_str::<AdmissionDecision>(&text)?, refused);
    let before = membership_admission::refused_before_lookup(&request(), &log(), "x", "y");
    assert_eq!(before.revision, None);
    assert_eq!(before.request, request());
    Ok(())
}
