//! The apps' sealed snapshot, lys-apps-state/v1, held to the bytes today's
//! code writes before the connector line (DIRECTORY-080 R1, box 12.0). The
//! record holds the app `lys` and an app of each standing: pending, approved
//! with sign-in settings, declined and retired. The fixture was written once
//! by the code at main 6e575de1 and is never regenerated: today's writer
//! writes it, today's reader reads it back to the same record, and one
//! changed byte is refused or read as another record.

use std::error::Error;

use lys_identity_server::apps_binding::{Binding, Registrar};
use lys_identity_server::apps_state::{
    Applied, Approved, By, Client, Decided, Held, Line, LysRecorded, Placed, Proposed, Registered,
    SignInSet, Standing,
};
use lys_identity_server::read_views::Login;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/apps-state-v1.json");

fn login(subject: &str) -> Login {
    Login {
        provider: "https://issuer.test".to_owned(),
        subject: subject.to_owned(),
    }
}

fn person() -> By {
    By::Person {
        login: login("ada"),
    }
}

fn operator() -> By {
    By::Operator {
        login: login("owner"),
    }
}

fn op(byte: u8) -> String {
    let pair = format!("{byte:02x}");
    format!("op-{}", pair.repeat(16))
}

fn schema() -> serde_json::Value {
    json!({"kinds": {"notes_page": {"relations": {"reader": ["read"]}, "parents": ["notes_book"]}}})
}

fn registered(operation: u8, app: &str, service_account: Option<&str>) -> Line {
    Line::Registered(Registered {
        operation: op(operation),
        app: app.to_owned(),
        name: app.to_uppercase(),
        redirects: vec![format!("https://{app}.example.test/back")],
        schema: schema(),
        service_account: service_account.map(ToOwned::to_owned),
        by: person(),
        at: u64::from(operation),
    })
}

fn approved(operation: u8, app: &str, binding: Option<Binding>) -> Line {
    Line::Approved(Approved {
        operation: op(operation),
        app: app.to_owned(),
        client: Client {
            client_id: app.to_owned(),
            secret_sha256: "cd".repeat(32),
        },
        binding,
        by: operator(),
        at: u64::from(operation),
    })
}

fn decided(operation: u8, app: &str) -> Decided {
    Decided {
        operation: op(operation),
        app: app.to_owned(),
        reason: "fixture".to_owned(),
        by: person(),
        at: u64::from(operation),
    }
}

/// The lines the record is folded from, in order.
fn lines() -> Vec<Line> {
    let account = "op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a";
    vec![
        Line::Lys(LysRecorded {
            operation: op(1),
            version: 3,
            schema: schema(),
            at: 1,
        }),
        Line::Registrar(Registrar {
            operation: op(2),
            service_account: account.to_owned(),
            secret_sha256: "ab".repeat(32),
            by: operator(),
            at: 2,
        }),
        registered(3, "drafts", None),
        registered(4, "notes", Some(account)),
        approved(
            5,
            "notes",
            Some(Binding {
                service_account: account.to_owned(),
                app: "notes".to_owned(),
                bound_by: operator(),
                at: 5,
            }),
        ),
        Line::SignInSet(SignInSet {
            operation: op(5),
            app: "notes".to_owned(),
            redirects: vec!["https://notes.example.test/back".to_owned()],
            profile: true,
            by: operator(),
            at: 5,
        }),
        Line::Proposed(Proposed {
            operation: op(6),
            app: "notes".to_owned(),
            replaces: 1,
            schema: schema(),
            by: person(),
            at: 6,
        }),
        Line::Applied(Applied {
            operation: op(7),
            app: "notes".to_owned(),
            version: 2,
            schema: schema(),
            proposal: Some(op(6)),
            by: operator(),
            at: 7,
        }),
        Line::Proposed(Proposed {
            operation: op(8),
            app: "notes".to_owned(),
            replaces: 2,
            schema: schema(),
            by: person(),
            at: 8,
        }),
        Line::Placed(Placed {
            operation: op(9),
            app: "notes".to_owned(),
            child_kind: "notes_page".to_owned(),
            child_id: "first".to_owned(),
            parent_kind: "notes_book".to_owned(),
            parent_id: "shelf".to_owned(),
            by: person(),
            at: 9,
        }),
        registered(10, "diary", None),
        Line::Declined(decided(11, "diary")),
        registered(12, "ledger", None),
        approved(13, "ledger", None),
        Line::Retired(decided(14, "ledger")),
    ]
}

fn held() -> Result<Held, Box<dyn Error>> {
    let mut held = Held::default();
    for line in lines() {
        held.hold(line)?;
    }
    Ok(held)
}

/// The standing of each app in `held`, by id.
fn standings(held: &Held) -> Vec<(String, Standing)> {
    held.apps
        .iter()
        .map(|app| (app.registered.app.clone(), app.standing()))
        .collect()
}

#[test]
fn todays_writer_writes_the_apps_snapshot_fixture() -> TestResult {
    let written = String::from_utf8(held()?.encode()?)?;
    assert_eq!(FIXTURE.trim_end(), written);
    Ok(())
}

#[test]
fn todays_reader_reads_the_fixture_as_the_same_record_of_each_standing() -> TestResult {
    let read = Held::decode(FIXTURE.trim_end().as_bytes())?;
    let held = held()?;
    assert_eq!(read.apps, held.apps);
    assert_eq!(read.placements, held.placements);
    assert_eq!(read.registrars, held.registrars);
    assert_eq!(
        standings(&read),
        [
            ("lys".to_owned(), Standing::Approved),
            ("drafts".to_owned(), Standing::Pending),
            ("notes".to_owned(), Standing::Approved),
            ("diary".to_owned(), Standing::Declined),
            ("ledger".to_owned(), Standing::Retired),
        ]
    );
    let notes = read.app("notes").ok_or("notes is not held")?;
    assert!(notes.sign_in.is_some(), "notes keeps its sign-in settings");
    assert_eq!(notes.versions.len(), 2);
    assert!(notes.pending.is_some());
    assert_eq!(read.encode()?, FIXTURE.trim_end().as_bytes());
    Ok(())
}

#[test]
fn one_changed_byte_of_the_fixture_is_refused_or_read_as_another_record() -> TestResult {
    let held = held()?;
    let fixture = FIXTURE.trim_end();
    let renamed = fixture.replacen("\"DIARY\"", "\"DIARZ\"", 1);
    assert_ne!(renamed, fixture, "the fixture names DIARY");
    assert_ne!(Held::decode(renamed.as_bytes())?.apps, held.apps);
    let reformatted = fixture.replacen("lys-apps-state/v1", "lys-apps-state/v2", 1);
    assert_ne!(reformatted, fixture, "the fixture names its format");
    let refused = Held::decode(reformatted.as_bytes());
    assert_eq!(
        refused.err().as_deref(),
        Some("apps state is in format lys-apps-state/v2, not lys-apps-state/v1")
    );
    Ok(())
}
