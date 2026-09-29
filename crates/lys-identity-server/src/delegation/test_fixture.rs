#![cfg(test)]
//! An app folded through the real app log projection, plus pure consent inputs.

use axum::http::{HeaderMap, HeaderValue, header};
use base64::{Engine, engine::general_purpose::STANDARD};
use lys_identity::PersonId;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::apps_state::{App, Approved, By, Client, Held, Line, Registered};

use super::{Binding, Current, Operation, Owner, Request};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const CLIENT: &str = "fixture_notes";
pub(super) const SECRET: &str = "fixture-secret";
pub(super) const REDIRECT: &str = "https://app.example.test/return";

pub(super) fn app() -> TestResult<App> {
    let mut held = Held::default();
    held.hold(Line::Registered(Registered {
        operation: "register".to_owned(),
        app: CLIENT.to_owned(),
        name: "Fixture".to_owned(),
        redirects: vec![REDIRECT.to_owned()],
        schema: json!({"kinds": {}}),
        service_account: None,
        by: By::Start,
        at: 1,
    }))?;
    held.hold(Line::Approved(Approved {
        operation: "approve".to_owned(),
        app: CLIENT.to_owned(),
        client: Client {
            client_id: CLIENT.to_owned(),
            secret_sha256: format!("{:x}", Sha256::digest(SECRET.as_bytes())),
        },
        binding: None,
        by: By::Start,
        at: 2,
    }))?;
    held.app(CLIENT)
        .cloned()
        .ok_or_else(|| "app missing".into())
}

pub(super) fn basic(client: &str, secret: &str) -> TestResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Basic {}",
            STANDARD.encode(format!("{client}:{secret}"))
        ))?,
    );
    Ok(headers)
}

pub(super) fn owner(byte: u8) -> Owner {
    Owner {
        person: PersonId::from_bytes([byte; 16]),
        session: format!("{byte:032x}"),
    }
}

pub(super) fn request() -> TestResult<Request> {
    Ok(Request {
        consent: "consent-1".to_owned(),
        owner: owner(1),
        binding: Binding::current(&app()?)?,
        operations: [Operation::SelfIdentity, Operation::VisibleIdentities]
            .into_iter()
            .collect(),
        issued_at: 10,
        expires_at: 100,
    })
}

pub(super) fn current(request: &Request) -> Current<'_> {
    Current {
        owner: &request.owner,
        active: true,
        session_ends_at: 100,
        binding: Some(&request.binding),
        at: 20,
    }
}
