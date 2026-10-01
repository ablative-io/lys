//! History cannot multiply emergency-stop or named-operation lookup work.

use std::error::Error;

use super::{Work, count, reset};
use crate::{read_views::Login, reviews_state, service_accounts_state, stops_state};

type TestResult = Result<(), Box<dyn Error>>;

fn stop(operation: String, agent: &str) -> stops_state::Stop {
    stops_state::Stop {
        operation,
        agent: agent.to_owned(),
        by: "person".to_owned(),
        reason: "stop".to_owned(),
        at: 1,
        certificates_withdrawn: Vec::new(),
        sessions_asked: Vec::new(),
        credentials_ended: None,
        credentials_refused: None,
        done: false,
    }
}

fn login() -> Login {
    Login {
        provider: "issuer".to_owned(),
        subject: "person".to_owned(),
    }
}

#[test]
fn stop_fold_and_lookups_visit_only_the_named_records() -> TestResult {
    let mut held = stops_state::Held::default();
    reset();
    for index in 0..512 {
        held.hold(stop(format!("history-{index}"), "unrelated"))?;
    }
    assert_eq!(count(Work::Stop), 0, "new stops scanned earlier stops");
    held.hold(stop("current".to_owned(), "covered"))?;
    let mut completed = stop("current".to_owned(), "covered");
    completed.done = true;
    held.hold(completed.clone())?;
    let bytes = held.encode()?;
    let held = stops_state::Held::decode(&bytes)?;
    assert_eq!(held.encode()?, bytes);
    reset();
    assert_eq!(held.operation("current"), Some(&completed));
    assert_eq!(
        held.of_agent("covered").collect::<Vec<_>>(),
        vec![&completed]
    );
    assert!(held.operation("missing").is_none());
    assert!(count(Work::Stop) <= 2);
    Ok(())
}

#[test]
fn review_operation_and_latest_decision_do_not_scan_history() -> TestResult {
    let mut held = reviews_state::Held::default();
    for index in 0..512 {
        held.hold(reviews_state::Kept {
            operation: format!("operation-{index}"),
            grant: format!("grant-{index}"),
            kept_by: "person".to_owned(),
            note: String::new(),
            at: 1,
            revision: 1,
        })?;
    }
    let bytes = held.encode()?;
    let held = reviews_state::Held::decode(&bytes)?;
    assert_eq!(held.encode()?, bytes);
    reset();
    assert_eq!(
        held.operation("operation-511")
            .map(|kept| kept.grant.as_str()),
        Some("grant-511")
    );
    assert_eq!(
        held.last_for("grant-0").map(|kept| kept.operation.as_str()),
        Some("operation-0")
    );
    assert!(held.operation("missing").is_none());
    assert!(count(Work::Review) <= 2);
    Ok(())
}

#[test]
fn service_account_operations_do_not_scan_other_accounts() -> TestResult {
    use service_accounts_state::{Created, Held, Line, Retired};
    let mut held = Held::default();
    for index in 0..512 {
        held.hold(Line::Created(Created {
            id: format!("account-{index}"),
            owner: "person".to_owned(),
            name: "service".to_owned(),
            description: String::new(),
            by: login(),
            at: 1,
        }))?;
    }
    held.hold(Line::Retired(Retired {
        operation: "retire".to_owned(),
        account: "account-511".to_owned(),
        by: login(),
        at: 2,
    }))?;
    let bytes = held.encode()?;
    let held = Held::decode(&bytes)?;
    assert_eq!(held.encode()?, bytes);
    reset();
    assert!(held.account("account-511").is_some());
    assert!(matches!(held.operation("retire"), Some(Line::Retired(_))));
    assert!(held.operation("missing").is_none());
    assert!(count(Work::Account) <= 2);
    Ok(())
}

#[test]
fn import_refusal_duplicates_do_not_visit_refusal_history() -> TestResult {
    use service_accounts_state::{Created, Held, ImportRefused, Line};
    let mut held = Held::default();
    held.hold(Line::Created(Created {
        id: "account".to_owned(),
        owner: "person".to_owned(),
        name: "service".to_owned(),
        description: String::new(),
        by: login(),
        at: 1,
    }))?;
    for n in 0..512 {
        held.hold(Line::ImportRefused(ImportRefused {
            account: "account".to_owned(),
            operation: format!("import-{n}"),
            entry: "entry".to_owned(),
            refusal: "not_admitted".to_owned(),
            at: 1,
        }))?;
    }
    let bytes = held.encode()?;
    let mut held = Held::decode(&bytes)?;
    reset();
    let refused = ImportRefused {
        account: "account".to_owned(),
        operation: "import-511".to_owned(),
        entry: "changed".to_owned(),
        refusal: "not_admitted".to_owned(),
        at: 2,
    };
    assert!(held.hold(Line::ImportRefused(refused.clone())).is_err());
    assert_eq!(held.encode()?, bytes);
    assert!(held.operation(&refused.operation).is_none());
    held.hold(Line::ImportRefused(ImportRefused {
        refusal: "different".to_owned(),
        ..refused
    }))?;
    assert!(
        count(Work::Refusal) <= 1,
        "duplicate detection scanned refusal history"
    );
    Ok(())
}

#[test]
fn team_operations_do_not_visit_change_history() -> TestResult {
    use crate::teams_state::{Changed, Created, Held, Line};
    let mut held = Held::default();
    held.hold(Line::Created(Created {
        id: "team".to_owned(),
        owner: "person".to_owned(),
        name: "team".to_owned(),
        description: String::new(),
        by: login(),
        at: 1,
    }))?;
    for n in 0..512 {
        held.hold(Line::Added(Changed {
            operation: format!("add-{n}"),
            team: "team".to_owned(),
            member: format!("member-{n}"),
            by: login(),
            at: 1,
        }))?;
    }
    let bytes = held.encode()?;
    let mut held = Held::decode(&bytes)?;
    reset();
    assert!(matches!(held.operation("add-511"), Some(Line::Added(_))));
    assert!(held.operation("missing").is_none());
    assert!(
        held.hold(Line::Added(Changed {
            operation: "add-511".to_owned(),
            team: "team".to_owned(),
            member: "other".to_owned(),
            by: login(),
            at: 2
        }))
        .is_err()
    );
    assert_eq!(held.encode()?, bytes);
    assert!(
        count(Work::TeamOperation) <= 2,
        "operation lookup scanned team history"
    );
    Ok(())
}
