#![cfg(test)]
//! Membership pages (ACCESS-006 R3): the permitted resources of one subject
//! and the recipients of one channel, a bounded page at a time under the
//! same contract, log and revision as a single decision.

use lys_pass::membership::{self, CONTRACT_VERSION, GrantLog, Subject};
use lys_pass::membership_pages::{
    self, MembershipPage, PageBounds, PageOutcome, PageRow, RecipientPageRequest,
    ResourcePageRequest,
};
use lys_pass::rights::{Mode, Resource};

fn resource(kind: &str, id: &str) -> Resource {
    Resource {
        kind: kind.to_owned(),
        id: id.to_owned(),
    }
}

fn log() -> GrantLog {
    GrantLog {
        identity: "lys-grants".to_owned(),
        epoch: 0,
    }
}

fn resources() -> ResourcePageRequest {
    ResourcePageRequest {
        contract: CONTRACT_VERSION,
        log: log(),
        workspace: resource("rooms.workspace", "ward"),
        subject: Subject {
            id: "person-1".to_owned(),
            kind: "person".to_owned(),
        },
        kind: "rooms.channel".to_owned(),
        action: "read".to_owned(),
        at_least: 0,
        bounds: PageBounds {
            rows: 2,
            bytes: 4096,
        },
        after: None,
    }
}

fn recipients() -> RecipientPageRequest {
    RecipientPageRequest {
        contract: CONTRACT_VERSION,
        log: log(),
        workspace: resource("rooms.workspace", "ward"),
        resource: resource("rooms.channel", "ward-a"),
        action: "read".to_owned(),
        at_least: 0,
        bounds: PageBounds {
            rows: 2,
            bytes: 4096,
        },
        after: None,
    }
}

#[test]
fn page_requests_have_stable_names_and_refuse_unknown_members()
-> Result<(), Box<dyn std::error::Error>> {
    let text = serde_json::to_string(&resources())?;
    assert!(
        text.contains(r#""bounds":{"rows":2,"bytes":4096}"#),
        "{text}"
    );
    assert!(text.contains(r#""after":null"#), "{text}");
    assert_eq!(
        serde_json::from_str::<ResourcePageRequest>(&text)?,
        resources()
    );
    let unbounded = text.replace(r#""bounds":{"rows":2,"bytes":4096},"#, "");
    assert!(serde_json::from_str::<ResourcePageRequest>(&unbounded).is_err());
    let total = text.replace(r#""after":null"#, r#""after":null,"total":true"#);
    assert!(serde_json::from_str::<ResourcePageRequest>(&total).is_err());
    let text = serde_json::to_string(&recipients())?;
    assert_eq!(
        serde_json::from_str::<RecipientPageRequest>(&text)?,
        recipients()
    );
    Ok(())
}

#[test]
fn zero_bounds_are_refused_by_name() {
    for bounds in [
        PageBounds { rows: 0, bytes: 1 },
        PageBounds { rows: 1, bytes: 0 },
    ] {
        let mut request = resources();
        request.bounds = bounds;
        assert_eq!(
            membership_pages::validate_resources(&request, &log()).map_err(|refused| refused.name),
            Err(membership_pages::BOUND_INVALID)
        );
        let mut request = recipients();
        request.bounds = bounds;
        assert_eq!(
            membership_pages::validate_recipients(&request, &log()).map_err(|refused| refused.name),
            Err(membership_pages::BOUND_INVALID)
        );
    }
}

#[test]
fn page_requests_are_checked_as_a_decision_is() {
    let mut future = resources();
    future.contract = CONTRACT_VERSION + 1;
    assert_eq!(
        membership_pages::validate_resources(&future, &log()).map_err(|refused| refused.name),
        Err(membership::CONTRACT_UNSUPPORTED)
    );
    let mut reset = recipients();
    reset.log.epoch = 1;
    assert_eq!(
        membership_pages::validate_recipients(&reset, &log()).map_err(|refused| refused.name),
        Err(membership::LOG_MISMATCH)
    );
    let mut empty = resources();
    empty.kind = String::new();
    assert_eq!(
        membership_pages::validate_resources(&empty, &log()).map_err(|refused| refused.name),
        Err(membership::REQUEST_MALFORMED)
    );
    let mut empty_cursor = recipients();
    empty_cursor.after = Some(String::new());
    assert_eq!(
        membership_pages::validate_recipients(&empty_cursor, &log())
            .map_err(|refused| refused.name),
        Err(membership::REQUEST_MALFORMED)
    );
    assert_eq!(
        membership_pages::validate_resources(&resources(), &log()),
        Ok(())
    );
    assert_eq!(
        membership_pages::validate_recipients(&recipients(), &log()),
        Ok(())
    );
}

#[test]
fn a_page_names_its_counts_and_completion() -> Result<(), Box<dyn std::error::Error>> {
    let page = MembershipPage {
        contract: CONTRACT_VERSION,
        log: log(),
        revision: Some(12),
        outcome: PageOutcome::Page {
            rows: vec![PageRow {
                subject: Subject {
                    id: "person-1".to_owned(),
                    kind: "person".to_owned(),
                },
                resource: resource("rooms.channel", "ward-a"),
                grant: "g1".to_owned(),
                scope: resource("rooms.workspace", "ward"),
                mode: Mode::Outright,
            }],
            returned: 1,
            skipped: 3,
            complete: true,
            next: None,
        },
    };
    let text = serde_json::to_string(&page)?;
    assert!(text.contains(r#""outcome":{"outcome":"page""#), "{text}");
    assert!(
        text.contains(r#""returned":1,"skipped":3,"complete":true,"next":null"#),
        "{text}"
    );
    assert_eq!(serde_json::from_str::<MembershipPage>(&text)?, page);
    let refused = membership_pages::refused_page(&log(), membership_pages::CURSOR_FOREIGN, "x");
    assert_eq!(refused.revision, None);
    assert_eq!(
        refused.outcome,
        PageOutcome::Refused {
            refusal: membership_pages::CURSOR_FOREIGN.to_owned(),
            reason: "x".to_owned(),
        }
    );
    Ok(())
}
