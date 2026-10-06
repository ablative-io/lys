#![cfg(test)]
//! An app's connector line in the apps' record (DIRECTORY-080 R1, box 12.1):
//! kept beside the approval under its operation or under an operation of its
//! own, one per app, only on an approved app other than `lys`, with both ids
//! read back; sealed into the snapshot only when an app holds one.

use std::error::Error;

use serde_json::json;

use super::{
    Approved, By, Client, Connected, Decided, Held, Line, LysRecorded, Refused, Registered,
    Standing,
};

type Outcome = Result<(), Box<dyn Error>>;

const CONNECTOR: &str = "connector-c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0";
const APPROVER: &str = "person-d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1";

fn registered(held: &mut Held, app: &str) -> Result<(), String> {
    held.hold(Line::Registered(Registered {
        operation: format!("register-{app}"),
        app: app.to_owned(),
        name: app.to_owned(),
        redirects: Vec::new(),
        schema: json!({"kinds": {}}),
        service_account: None,
        by: By::Start,
        at: 1,
    }))
}

fn approved(held: &mut Held, app: &str) -> Result<(), String> {
    registered(held, app)?;
    held.hold(Line::Approved(Approved {
        operation: format!("approve-{app}"),
        app: app.to_owned(),
        client: Client {
            client_id: app.to_owned(),
            secret_sha256: "00".repeat(32),
        },
        binding: None,
        by: By::Start,
        at: 2,
    }))
}

fn connected(operation: &str, app: &str) -> Connected {
    Connected {
        operation: operation.to_owned(),
        app: app.to_owned(),
        connector: CONNECTOR.to_owned(),
        approver: APPROVER.to_owned(),
        by: By::Start,
        at: 3,
    }
}

fn decided(operation: &str, app: &str) -> Decided {
    Decided {
        operation: operation.to_owned(),
        app: app.to_owned(),
        reason: String::new(),
        by: By::Start,
        at: 3,
    }
}

#[test]
fn an_approval_keeps_its_connector_beside_it_and_the_app_holds_one() -> Outcome {
    let mut held = Held::default();
    approved(&mut held, "notes")?;
    let line = connected("approve-notes", "notes");
    held.hold(Line::Connector(line.clone()))?;
    let app = held.app("notes").ok_or("notes is not held")?;
    assert_eq!(app.connector.as_ref(), Some(&line));
    assert_eq!(app.standing(), Standing::Approved);
    assert!(
        matches!(held.operation("approve-notes"), Some(Line::Approved(_))),
        "the approval's operation still finds the approval"
    );
    let again = held.allows(&Line::Connector(connected("give-notes", "notes")));
    assert_eq!(again, Err(Refused::Exists), "an app holds one connector");
    let repeated = held.hold(Line::Connector(line));
    assert!(repeated.is_err(), "{repeated:?}");
    Ok(())
}

#[test]
fn an_app_approved_without_one_is_given_its_connector_under_its_own_operation() -> Outcome {
    let mut held = Held::default();
    approved(&mut held, "notes")?;
    assert_eq!(held.app("notes").ok_or("no notes")?.connector, None);
    held.hold(Line::Connector(connected("give-notes", "notes")))?;
    let app = held.app("notes").ok_or("no notes")?;
    assert_eq!(
        app.connector.as_ref().map(|line| line.operation.as_str()),
        Some("give-notes")
    );
    assert!(matches!(
        held.operation("give-notes"),
        Some(Line::Connector(_))
    ));
    Ok(())
}

#[test]
fn only_an_approved_app_other_than_lys_takes_a_connector() -> Outcome {
    let mut held = Held::default();
    held.hold(Line::Lys(LysRecorded {
        operation: "lys".to_owned(),
        version: 1,
        schema: json!({"kinds": {}}),
        at: 1,
    }))?;
    registered(&mut held, "pending")?;
    registered(&mut held, "declined")?;
    held.hold(Line::Declined(decided("decline", "declined")))?;
    approved(&mut held, "retired")?;
    held.hold(Line::Retired(decided("retire", "retired")))?;
    for (app, refused) in [
        ("lys", Refused::Lys),
        ("pending", Refused::Standing(Standing::Pending)),
        ("declined", Refused::Standing(Standing::Declined)),
        ("retired", Refused::Standing(Standing::Retired)),
        ("absent", Refused::Unknown),
    ] {
        let line = Line::Connector(connected(&format!("give-{app}"), app));
        assert_eq!(held.allows(&line), Err(refused), "{app}");
    }
    Ok(())
}

#[test]
fn a_connector_line_whose_ids_do_not_read_back_is_refused_by_name() -> Outcome {
    let mut held = Held::default();
    approved(&mut held, "notes")?;
    let mut line = connected("give-notes", "notes");
    line.connector = "op-c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0".to_owned();
    let refused = held.hold(Line::Connector(line.clone()));
    assert!(
        refused.as_ref().is_err_and(
            |reason| reason.starts_with("line `give-notes`:") && reason.contains("connector")
        ),
        "{refused:?}"
    );
    line.connector = CONNECTOR.to_owned();
    line.approver = "agent-d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1".to_owned();
    let refused = held.hold(Line::Connector(line));
    assert!(
        refused
            .as_ref()
            .is_err_and(|reason| reason.contains("person")),
        "{refused:?}"
    );
    assert_eq!(held.app("notes").ok_or("no notes")?.connector, None);
    Ok(())
}

#[test]
fn the_snapshot_carries_a_connector_only_for_an_app_that_holds_one() -> Outcome {
    let mut held = Held::default();
    approved(&mut held, "notes")?;
    approved(&mut held, "diary")?;
    let without = String::from_utf8(held.encode()?)?;
    assert!(!without.contains("\"connector\""), "{without}");
    held.hold(Line::Connector(connected("approve-notes", "notes")))?;
    let with = held.encode()?;
    let text = String::from_utf8(with.clone())?;
    assert_eq!(text.matches("\"connector\":{").count(), 1, "{text}");
    let read = Held::decode(&with)?;
    assert_eq!(read.apps, held.apps);
    assert_eq!(read.encode()?, with);
    Ok(())
}
