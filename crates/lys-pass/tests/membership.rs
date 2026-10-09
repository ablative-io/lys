#![cfg(test)]
//! The shared channel membership contract (ACCESS-006 R1): one versioned
//! request and decision every product decodes the same way, and vectors
//! every product and Lys itself run.

use lys_pass::membership::{
    self, CONTRACT_VERSION, GrantLog, MembershipDecision, MembershipRequest, Subject, Verdict,
};
use lys_pass::membership_conformance::{self, Expected};
use lys_pass::rights::Resource;

fn resource(kind: &str, id: &str) -> Resource {
    Resource {
        kind: kind.to_owned(),
        id: id.to_owned(),
    }
}

fn request() -> MembershipRequest {
    MembershipRequest {
        contract: CONTRACT_VERSION,
        log: GrantLog {
            identity: "lys-grants".to_owned(),
            epoch: 1,
        },
        workspace: resource("sample.workspace", "ward"),
        subject: Subject {
            id: "person-1".to_owned(),
            kind: "person".to_owned(),
        },
        resource: resource("sample.channel", "ward-a"),
        action: "read".to_owned(),
        at_least: 0,
    }
}

#[test]
fn request_names_are_stable_json() -> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string(&request())?;
    assert_eq!(
        text,
        concat!(
            r#"{"contract":1,"log":{"identity":"lys-grants","epoch":1},"#,
            r#""workspace":{"kind":"sample.workspace","id":"ward"},"#,
            r#""subject":{"id":"person-1","kind":"person"},"#,
            r#""resource":{"kind":"sample.channel","id":"ward-a"},"#,
            r#""action":"read","at_least":0}"#
        )
    );
    assert_eq!(serde_json::from_str::<MembershipRequest>(&text)?, request());
    Ok(())
}

#[test]
fn request_refuses_unknown_and_missing_members() {
    let extra = r#"{"contract":1,"log":{"identity":"l","epoch":1},"workspace":{"kind":"a.w","id":"w"},"subject":{"id":"person-1","kind":"person"},"resource":{"kind":"a.c","id":"c"},"action":"read","at_least":0,"label":"Ward A"}"#;
    assert!(serde_json::from_str::<MembershipRequest>(extra).is_err());
    let follow = extra.replace(r#","label":"Ward A""#, r#","follow":true"#);
    assert!(serde_json::from_str::<MembershipRequest>(&follow).is_err());
    let no_floor = extra.replace(r#","at_least":0,"label":"Ward A""#, "");
    assert!(serde_json::from_str::<MembershipRequest>(&no_floor).is_err());
    let log_extra = extra
        .replace(r#""epoch":1}"#, r#""epoch":1,"cursor":"x"}"#)
        .replace(r#","label":"Ward A""#, "");
    assert!(serde_json::from_str::<MembershipRequest>(&log_extra).is_err());
}

#[test]
fn decision_verdicts_are_tagged_and_closed() -> Result<(), Box<dyn std::error::Error>> {
    let allowed = MembershipDecision {
        contract: CONTRACT_VERSION,
        log: request().log,
        request: request(),
        revision: Some(41),
        verdict: Verdict::Allowed {
            grant: "g1".to_owned(),
            path: vec!["g1".to_owned(), "g0".to_owned()],
            scope: resource("sample.channel", "ward-a"),
        },
    };
    let text = serde_json::to_string(&allowed)?;
    assert!(text.contains(r#""verdict":{"outcome":"allowed","grant":"g1""#));
    assert!(text.contains(r#""revision":41"#));
    assert_eq!(serde_json::from_str::<MembershipDecision>(&text)?, allowed);
    let widened = text.replace(
        r#""outcome":"allowed","#,
        r#""outcome":"allowed","post":true,"#,
    );
    assert!(serde_json::from_str::<MembershipDecision>(&widened).is_err());
    let unknown = text.replace(r#""outcome":"allowed""#, r#""outcome":"member""#);
    assert!(serde_json::from_str::<MembershipDecision>(&unknown).is_err());
    Ok(())
}

#[test]
fn shape_refusals_are_named_before_any_lookup() {
    let mut future = request();
    future.contract = CONTRACT_VERSION + 1;
    assert_eq!(
        membership::validate(&future, &request().log).map_err(|refused| refused.name),
        Err(membership::CONTRACT_UNSUPPORTED)
    );
    let mut foreign = request();
    foreign.log.identity = "another-log".to_owned();
    assert_eq!(
        membership::validate(&foreign, &request().log).map_err(|refused| refused.name),
        Err(membership::LOG_MISMATCH)
    );
    let mut reset = request();
    reset.log.epoch = 2;
    assert_eq!(
        membership::validate(&reset, &request().log).map_err(|refused| refused.name),
        Err(membership::LOG_MISMATCH)
    );
    let edits: [fn(&mut MembershipRequest); 5] = [
        |r: &mut MembershipRequest| r.action = String::new(),
        |r: &mut MembershipRequest| r.subject.id = "person\n1".to_owned(),
        |r: &mut MembershipRequest| r.resource.kind = String::new(),
        |r: &mut MembershipRequest| r.workspace.id = String::new(),
        |r: &mut MembershipRequest| r.subject.kind = String::new(),
    ];
    for broken in edits {
        let mut malformed = request();
        broken(&mut malformed);
        assert_eq!(
            membership::validate(&malformed, &request().log).map_err(|refused| refused.name),
            Err(membership::REQUEST_MALFORMED)
        );
    }
    assert_eq!(membership::validate(&request(), &request().log), Ok(()));
}

#[test]
fn refusal_before_lookup_names_no_revision() {
    let mut future = request();
    future.contract = 9;
    let decision = membership::refused_before_lookup(
        &future,
        &request().log,
        membership::CONTRACT_UNSUPPORTED,
        "contract 9 is not served",
    );
    assert_eq!(decision.revision, None);
    assert_eq!(decision.request, future);
    assert_eq!(decision.contract, CONTRACT_VERSION);
    assert_eq!(
        decision.verdict,
        Verdict::Refused {
            refusal: membership::CONTRACT_UNSUPPORTED.to_owned(),
            reason: "contract 9 is not served".to_owned(),
        }
    );
}

#[test]
fn shipped_vectors_parse_and_count() -> Result<(), Box<dyn std::error::Error>> {
    let world = membership_conformance::world()?;
    let cases = membership_conformance::cases()?;
    assert_eq!(cases.len(), membership_conformance::CASE_COUNT);
    assert_eq!(cases.len(), 20);
    let names: std::collections::BTreeSet<_> =
        cases.iter().map(|case| case.name.as_str()).collect();
    assert_eq!(names.len(), cases.len(), "vector names are unique");
    for required in [
        "reader-reads-own-channel",
        "reader-cannot-post",
        "reader-cannot-read-sibling",
        "steward-reaches-child-through-workspace",
        "steward-not-into-restricted-child",
        "responsible-person-gives-ai-nothing",
        "unknown-contract",
        "foreign-log",
        "minimum-revision-ahead",
    ] {
        assert!(names.contains(required), "{required} is shipped");
    }
    for case in &cases {
        let label = &case.request.subject.id;
        assert!(
            world.subjects.iter().any(|subject| &subject.label == label),
            "{}: subject {label} is in the world",
            case.name
        );
        if let Expected::Allowed { grant, .. } = &case.expected {
            assert!(world.grants.iter().any(|held| &held.label == grant));
        }
    }
    Ok(())
}

#[test]
fn consumer_gate_names_every_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let report = membership_conformance::run(|case| {
        Ok::<_, std::convert::Infallible>(MembershipDecision {
            contract: CONTRACT_VERSION,
            log: case.request.log.clone(),
            request: case.request.clone(),
            revision: Some(1),
            verdict: Verdict::Allowed {
                grant: "open-room".to_owned(),
                path: Vec::new(),
                scope: case.request.workspace.clone(),
            },
        })
    })?;
    assert_eq!(report.run, membership_conformance::CASE_COUNT);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failures.len(), report.run);
    assert!(report.require_conformant().is_err());
    Ok(())
}

#[test]
fn consumer_gate_accepts_the_expected_decisions() -> Result<(), Box<dyn std::error::Error>> {
    let report = membership_conformance::run(|case| {
        Ok::<_, std::convert::Infallible>(membership_conformance::expected_decision(case, 7))
    })?;
    assert_eq!(report.passed, report.run);
    assert!(report.require_conformant().is_ok());
    Ok(())
}

#[test]
fn uncorrelated_answer_is_a_failure() -> Result<(), Box<dyn std::error::Error>> {
    let report = membership_conformance::run(|case| {
        let mut decision = membership_conformance::expected_decision(case, 7);
        decision.request.action = "post".to_owned();
        Ok::<_, std::convert::Infallible>(decision)
    })?;
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.case == "reader-reads-own-channel")
    );
    Ok(())
}
