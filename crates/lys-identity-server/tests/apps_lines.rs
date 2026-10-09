//! Every shape of apps line, held to the bytes written before the connector
//! kind (DIRECTORY-080 R1). The fixture was written by the code at main
//! 5215ed6c from the lines below and is never regenerated: today's reader
//! reads each fixture line as its line, and today's writer writes each line
//! as the fixture's bytes. A binding, a registrar and a service account's
//! acts are among them and stay readable for good.

use std::error::Error;

use lys_identity_server::apps_binding::{Binding, Registrar};
use lys_identity_server::apps_state::{
    Applied, Approved, By, Client, ClientCredentialIssued, ClientCredentialRevoked,
    ClientCredentialsEnded, Connected, Decided, Line, LysRecorded, Placed, Proposed, Registered,
    SignInSet,
};
use lys_identity_server::read_views::Login;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/apps-lines-5215ed6c.jsonl");
const CREDENTIAL_FIXTURE: &str = include_str!("fixtures/apps-lines-081.jsonl");

fn person() -> By {
    By::Person {
        login: Login {
            provider: "https://issuer.test".to_owned(),
            subject: "ada".to_owned(),
        },
    }
}

fn operator() -> By {
    By::Operator {
        login: Login {
            provider: "https://issuer.test".to_owned(),
            subject: "owner".to_owned(),
        },
    }
}

fn account() -> By {
    By::ServiceAccount {
        id: "op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a".to_owned(),
    }
}

fn schema() -> serde_json::Value {
    json!({"kinds": {"notes_page": {"relations": {"reader": ["read"]}, "parents": ["notes_book"]}}})
}

fn decided(operation: &str, by: By) -> Decided {
    Decided {
        operation: operation.to_owned(),
        app: "notes".to_owned(),
        reason: "fixture".to_owned(),
        by,
        at: 30,
    }
}

/// One line of every shape the apps log writes, in a fixed order.
fn lines() -> Vec<Line> {
    vec![
        Line::Lys(LysRecorded {
            operation: "op-01010101010101010101010101010101".to_owned(),
            version: 3,
            schema: schema(),
            at: 1,
        }),
        Line::Registrar(Registrar {
            operation: "op-02020202020202020202020202020202".to_owned(),
            service_account: "op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a".to_owned(),
            secret_sha256: "ab".repeat(32),
            by: operator(),
            at: 2,
        }),
        Line::Registered(Registered {
            operation: "op-03030303030303030303030303030303".to_owned(),
            app: "notes".to_owned(),
            name: "Notes".to_owned(),
            redirects: vec!["https://notes.example.test/back".to_owned()],
            schema: schema(),
            service_account: Some("op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a".to_owned()),
            by: account(),
            at: 3,
        }),
        Line::Registered(Registered {
            operation: "op-04040404040404040404040404040404".to_owned(),
            app: "diary".to_owned(),
            name: "Diary".to_owned(),
            redirects: Vec::new(),
            schema: schema(),
            service_account: None,
            by: person(),
            at: 4,
        }),
        Line::Approved(Approved {
            operation: "op-05050505050505050505050505050505".to_owned(),
            app: "notes".to_owned(),
            client: Client {
                client_id: "notes".to_owned(),
                secret_sha256: "cd".repeat(32),
            },
            binding: Some(Binding {
                service_account: "op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a".to_owned(),
                app: "notes".to_owned(),
                bound_by: person(),
                at: 5,
            }),
            by: person(),
            at: 5,
        }),
        Line::Approved(Approved {
            operation: "op-06060606060606060606060606060606".to_owned(),
            app: "diary".to_owned(),
            client: Client {
                client_id: "diary".to_owned(),
                secret_sha256: "ef".repeat(32),
            },
            binding: None,
            by: operator(),
            at: 6,
        }),
        Line::SignInSet(SignInSet {
            operation: "op-07070707070707070707070707070707".to_owned(),
            app: "notes".to_owned(),
            redirects: vec!["https://notes.example.test/back".to_owned()],
            profile: true,
            by: By::Start,
            at: 7,
        }),
        Line::Proposed(Proposed {
            operation: "op-08080808080808080808080808080808".to_owned(),
            app: "notes".to_owned(),
            replaces: 1,
            schema: schema(),
            by: account(),
            at: 8,
        }),
        Line::ChangeDeclined(decided("op-09090909090909090909090909090909", person())),
        Line::Applied(Applied {
            operation: "op-0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a".to_owned(),
            app: "notes".to_owned(),
            version: 2,
            schema: schema(),
            proposal: Some("op-08080808080808080808080808080808".to_owned()),
            by: person(),
            at: 10,
        }),
        Line::Placed(Placed {
            operation: "op-0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b".to_owned(),
            app: "notes".to_owned(),
            child_kind: "notes_page".to_owned(),
            child_id: "first".to_owned(),
            parent_kind: "notes_book".to_owned(),
            parent_id: "shelf".to_owned(),
            restricted: false,
            by: account(),
            at: 11,
        }),
        Line::Declined(decided("op-0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c", operator())),
        Line::Retired(decided("op-0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d", person())),
    ]
}

/// Which shape `line` is; every shape is named, so a new one cannot be left unpinned.
fn shape(line: &Line) -> &'static str {
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
        Line::CustodyPrepared(_) => "custody_prepared",
    }
}

/// One line of each shape an app's client credentials write
/// (DIRECTORY-081), held to `apps-lines-081.jsonl`, written once with them
/// and never regenerated.
fn credential_lines() -> Vec<Line> {
    let first = "0a1b2c3d4e5f6a7b";
    vec![
        Line::ClientCredentialIssued(ClientCredentialIssued {
            operation: "op-0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f".to_owned(),
            app: "notes".to_owned(),
            credential_id: first.to_owned(),
            owner: "person-custody".to_owned(),
            by: operator(),
            at: 15,
        }),
        Line::ClientCredentialIssued(ClientCredentialIssued {
            operation: "op-10101010101010101010101010101010".to_owned(),
            app: "notes".to_owned(),
            credential_id: "1b2c3d4e5f6a7b8c".to_owned(),
            owner: "person-custody".to_owned(),
            by: operator(),
            at: 16,
        }),
        Line::ClientCredentialRevoked(ClientCredentialRevoked {
            operation: "op-11111111111111111111111111111111".to_owned(),
            app: "notes".to_owned(),
            credential_id: first.to_owned(),
            reason: "rotated".to_owned(),
            by: person(),
            at: 17,
        }),
        Line::ClientCredentialsEnded(ClientCredentialsEnded {
            operation: "op-12121212121212121212121212121212".to_owned(),
            app: "notes".to_owned(),
            credential_ids: vec![first.to_owned()],
            at: 18,
        }),
    ]
}

#[test]
fn every_client_credential_line_shape_keeps_its_bytes_and_reads_back() -> TestResult {
    let credentials = credential_lines();
    let written = credentials
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let fixture: Vec<&str> = CREDENTIAL_FIXTURE.lines().collect();
    assert_eq!(
        fixture,
        written,
        "today's writer no longer writes the fixture's lines; it writes:\n{}",
        written.join("\n")
    );
    for (text, line) in fixture.iter().zip(&credentials) {
        let read: Line = serde_json::from_slice(text.as_bytes())?;
        assert_eq!(&read, line, "{}: today's reader", shape(line));
    }
    let connector: Line = serde_json::from_str(CONNECTOR_FIXTURE.trim_end())?;
    let shapes: std::collections::BTreeSet<_> = lines()
        .iter()
        .chain(&credentials)
        .chain(std::iter::once(&connector))
        .map(shape)
        .collect();
    assert_eq!(shapes.len(), 15, "every shape of apps line is pinned");
    Ok(())
}

/// The connector line (box 12.1), written once by the code that added it and
/// never regenerated, pinned beside the shapes written before it.
const CONNECTOR_FIXTURE: &str = include_str!("fixtures/apps-line-connector.json");

#[test]
fn the_connector_line_keeps_its_bytes_and_reads_back() -> TestResult {
    let line = Line::Connector(Connected {
        operation: "op-0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e".to_owned(),
        app: "notes".to_owned(),
        connector: "connector-c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0".to_owned(),
        approver: "person-d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1".to_owned(),
        by: operator(),
        at: 14,
    });
    let written = serde_json::to_string(&line)?;
    assert_eq!(CONNECTOR_FIXTURE.trim_end(), written);
    let read: Line = serde_json::from_str(CONNECTOR_FIXTURE.trim_end())?;
    assert_eq!(read, line);
    assert_eq!(shape(&read), "connector");
    Ok(())
}

#[test]
fn every_apps_line_shape_keeps_its_bytes_and_reads_back() -> TestResult {
    let lines = lines();
    let written = lines
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let fixture: Vec<&str> = FIXTURE.lines().collect();
    assert_eq!(
        fixture,
        written,
        "today's writer no longer writes the fixture's lines; it writes:\n{}",
        written.join("\n")
    );
    for (text, line) in fixture.iter().zip(&lines) {
        let read: Line = serde_json::from_slice(text.as_bytes())?;
        assert_eq!(&read, line, "{}: today's reader", shape(line));
    }
    let shapes: std::collections::BTreeSet<_> = lines.iter().map(shape).collect();
    assert_eq!(shapes.len(), 11, "every shape of apps line is pinned");
    Ok(())
}
