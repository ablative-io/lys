//! The apps' sealed snapshot, lys-apps-state/v1, held to the bytes today's
//! code writes before the connector line (DIRECTORY-080 R1, box 12.0). The
//! record holds the app `lys` and an app of each standing: pending, approved
//! with sign-in settings, declined and retired. An app keeps its history as
//! lines, so every kind of line and every kind of actor is a stored shape,
//! and the record holds each. The fixture was written once by the code at
//! main d759ef18 and is never regenerated: today's writer writes it, today's
//! reader reads it back to the same record, and one changed byte is refused
//! or read as another record.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity_server::apps_binding::{Binding, Registrar};
use lys_identity_server::apps_state::{
    Applied, Approved, By, Client, ClientCredentialIssued, ClientCredentialRevoked,
    ClientCredentialsEnded, Connected, Decided, Held, Line, LysRecorded, Placed, Proposed,
    Registered, SignInSet, Standing,
};
use lys_identity_server::read_views::Login;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/apps-state-v1.json");
/// The same record with client credentials kept on it (DIRECTORY-081),
/// written once with them and never regenerated.
const CREDENTIAL_FIXTURE: &str = include_str!("fixtures/apps-state-081.json");

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
        Line::ChangeDeclined(decided(15, "notes")),
        Line::Proposed(Proposed {
            operation: op(16),
            app: "notes".to_owned(),
            replaces: 2,
            schema: schema(),
            by: By::ServiceAccount {
                id: account.to_owned(),
            },
            at: 16,
        }),
    ]
}

/// Which kind `line` is. Every kind is named here, so a kind added later is
/// placed in words before it can be written.
fn kind(line: &Line) -> &'static str {
    match line {
        Line::Lys(_) => "lys",
        Line::Registered(_) => "registered",
        Line::Approved(_) => "approved",
        Line::SignInSet(_) => "sign_in_set",
        Line::Declined(_) => "declined",
        Line::Retired(_) => "retired",
        Line::Proposed(_) => "proposed",
        Line::Applied(_) => "applied",
        Line::ChangeDeclined(_) => "change_declined",
        Line::Placed(_) => "placed",
        Line::Registrar(_) => "registrar",
        Line::Connector(_) => "connector",
        Line::ClientCredentialIssued(_) => "client_credential_issued",
        Line::ClientCredentialRevoked(_) => "client_credential_revoked",
        Line::ClientCredentialsEnded(_) => "client_credentials_ended",
    }
}

/// Who `line` records as acting. The app `lys` is recorded as made by the
/// service at start; a credentials-ended line records the broker's
/// confirmation and names no actor.
fn actor(line: &Line) -> Option<By> {
    match line {
        Line::Lys(_) => Some(By::Start),
        Line::Registered(Registered { by, .. })
        | Line::Approved(Approved { by, .. })
        | Line::SignInSet(SignInSet { by, .. })
        | Line::Declined(Decided { by, .. })
        | Line::Retired(Decided { by, .. })
        | Line::Proposed(Proposed { by, .. })
        | Line::Applied(Applied { by, .. })
        | Line::ChangeDeclined(Decided { by, .. })
        | Line::Placed(Placed { by, .. })
        | Line::Registrar(Registrar { by, .. })
        | Line::Connector(Connected { by, .. })
        | Line::ClientCredentialIssued(ClientCredentialIssued { by, .. })
        | Line::ClientCredentialRevoked(ClientCredentialRevoked { by, .. }) => Some(by.clone()),
        Line::ClientCredentialsEnded(_) => None,
    }
}

/// Which kind of actor `by` is. Every kind is named here.
fn actor_kind(by: &By) -> &'static str {
    match by {
        By::Person { .. } => "person",
        By::Operator { .. } => "operator",
        By::ServiceAccount { .. } => "service_account",
        By::Start => "start",
    }
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

/// The connector line is the one kind no record here holds: it is pinned by
/// its own fixture in `apps_lines.rs` (box 12.1).
#[test]
fn the_record_holds_every_kind_of_line_and_every_kind_of_actor() {
    let lines: Vec<Line> = lines().into_iter().chain(credential_lines()).collect();
    let kinds: BTreeSet<_> = lines.iter().map(kind).collect();
    assert_eq!(
        kinds,
        BTreeSet::from([
            "lys",
            "registered",
            "approved",
            "sign_in_set",
            "declined",
            "retired",
            "proposed",
            "applied",
            "change_declined",
            "placed",
            "registrar",
            "client_credential_issued",
            "client_credential_revoked",
            "client_credentials_ended",
        ]),
        "every kind of line but the connector is in the fixtures"
    );
    let actors: BTreeSet<_> = lines
        .iter()
        .filter_map(actor)
        .map(|by| actor_kind(&by))
        .collect();
    assert_eq!(
        actors,
        BTreeSet::from(["person", "operator", "service_account", "start"]),
        "every kind of actor is in the fixture"
    );
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

/// The client credential lines kept on `notes` after the 12.0 record
/// (DIRECTORY-081): two issued, the first revoked and its ending confirmed.
fn credential_lines() -> Vec<Line> {
    let first = "0a1b2c3d4e5f6a7b";
    let issued = |operation: u8, credential_id: &str| {
        Line::ClientCredentialIssued(ClientCredentialIssued {
            operation: op(operation),
            app: "notes".to_owned(),
            credential_id: credential_id.to_owned(),
            owner: "person-custody".to_owned(),
            by: operator(),
            at: u64::from(operation),
        })
    };
    // Operations 17 to 20, after every operation the record above holds or
    // will hold (15 and 16 are box 12.0b's), so each stays one line's.
    vec![
        issued(17, first),
        issued(18, "1b2c3d4e5f6a7b8c"),
        Line::ClientCredentialRevoked(ClientCredentialRevoked {
            operation: op(19),
            app: "notes".to_owned(),
            credential_id: first.to_owned(),
            reason: "rotated".to_owned(),
            by: person(),
            at: 19,
        }),
        Line::ClientCredentialsEnded(ClientCredentialsEnded {
            operation: op(20),
            app: "notes".to_owned(),
            credential_ids: vec![first.to_owned()],
            at: 20,
        }),
    ]
}

fn held_with_credentials() -> Result<Held, Box<dyn Error>> {
    let mut held = held()?;
    for line in credential_lines() {
        held.hold(line)?;
    }
    Ok(held)
}

#[test]
fn todays_writer_and_reader_keep_the_snapshot_with_client_credentials() -> TestResult {
    let held = held_with_credentials()?;
    assert_eq!(
        String::from_utf8(held.encode()?)?,
        CREDENTIAL_FIXTURE.trim_end()
    );
    let read = Held::decode(CREDENTIAL_FIXTURE.trim_end().as_bytes())?;
    assert_eq!(read.apps, held.apps);
    let notes = read.app("notes").ok_or("notes is not held")?;
    assert_eq!(notes.live_client_credentials(), ["1b2c3d4e5f6a7b8c"]);
    assert!(notes.client_credentials_to_end().is_empty());
    Ok(())
}

#[test]
fn a_record_written_before_client_credentials_takes_them_and_folds_to_the_same_apps() -> TestResult
{
    let mut upgraded = Held::decode(FIXTURE.trim_end().as_bytes())?;
    for line in credential_lines() {
        upgraded.hold(line)?;
    }
    assert_eq!(upgraded.apps, held_with_credentials()?.apps);
    assert_eq!(upgraded.encode()?, CREDENTIAL_FIXTURE.trim_end().as_bytes());
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
