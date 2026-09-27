//! A screen service's word for a person: admitted once, for the one request
//! it signed, from a service the broker trusts, fresh, and for a person.

use lys_core::Ed25519Identity;
use lys_secrets::{
    OnBehalf, PRESENTATION_SKEW_MS, SecretsError, ServiceKey, ServiceWindow, new_operation_id,
    request_digest, to_hex,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

#[test]
fn a_trusted_service_speaks_for_a_person_once_for_the_request_it_signed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("identity.key"))?;
    let stranger = Ed25519Identity::load_or_generate(&dir.path().join("stranger.key"))?;
    let trusted = [ServiceKey {
        name: "identity".to_owned(),
        public_key: to_hex(&key.public_key_bytes()),
    }];
    let now = 1_000_000;
    let request = request_digest("GET", "/_lys/secrets", &[])?;
    let mut window = ServiceWindow::new();

    let asked = OnBehalf::sign(
        "identity",
        "person-dana",
        &new_operation_id()?,
        now,
        request,
        &key,
    )?;
    assert_eq!(window.admit(&trusted, &asked, now)?, "person-dana");
    assert_eq!(
        refusal(window.admit(&trusted, &asked, now)),
        "ServiceReplayed"
    );

    let other = request_digest("GET", "/_lys/audit", &[])?;
    let mut moved = OnBehalf::sign(
        "identity",
        "person-dana",
        &new_operation_id()?,
        now,
        request,
        &key,
    )?;
    moved.request = other;
    assert_eq!(
        refusal(window.admit(&trusted, &moved, now)),
        "ServiceSignatureInvalid"
    );

    let forged = OnBehalf::sign(
        "identity",
        "person-dana",
        &new_operation_id()?,
        now,
        request,
        &stranger,
    )?;
    assert_eq!(
        refusal(window.admit(&trusted, &forged, now)),
        "ServiceSignatureInvalid"
    );

    let unknown = OnBehalf::sign(
        "elsewhere",
        "person-dana",
        &new_operation_id()?,
        now,
        request,
        &key,
    )?;
    assert_eq!(
        refusal(window.admit(&trusted, &unknown, now)),
        "ServiceUnknown"
    );

    let stale = OnBehalf::sign(
        "identity",
        "person-dana",
        &new_operation_id()?,
        now - PRESENTATION_SKEW_MS - 1,
        request,
        &key,
    )?;
    assert_eq!(
        refusal(window.admit(&trusted, &stale, now)),
        "PresentationStale"
    );

    let agent = OnBehalf::sign(
        "identity",
        "agent-bot",
        &new_operation_id()?,
        now,
        request,
        &key,
    )?;
    assert_eq!(
        refusal(window.admit(&trusted, &agent, now)),
        "ServicePersonInvalid"
    );

    let wire = asked.to_wire();
    let read = OnBehalf::from_wire([&wire[0], &wire[1], &wire[2], &wire[3], &wire[4]], request)?;
    let mut fresh = ServiceWindow::new();
    assert_eq!(fresh.admit(&trusted, &read, now)?, "person-dana");
    Ok(())
}

#[test]
fn the_broker_admits_what_the_identity_service_signs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("identity.key"))?;
    let trusted = [ServiceKey {
        name: "identity".to_owned(),
        public_key: to_hex(&key.public_key_bytes()),
    }];
    let now = 1_000_000;
    let body = br#"{"secret":"token","scope":"team:accounts"}"#;
    let wire = lys_identity_server::secrets_sign::headers(
        &lys_identity_server::secrets_sign::Asked {
            service: "identity",
            person: "person-dana",
            method: "POST",
            path: "/_lys/scope",
            body,
            signed_at_ms: now,
        },
        &key,
    )?;
    let asked = OnBehalf::from_wire(
        [&wire[0], &wire[1], &wire[2], &wire[3], &wire[4]],
        request_digest("POST", "/_lys/scope", body)?,
    )?;
    assert_eq!(
        ServiceWindow::new().admit(&trusted, &asked, now)?,
        "person-dana"
    );
    let elsewhere = OnBehalf::from_wire(
        [&wire[0], &wire[1], &wire[2], &wire[3], &wire[4]],
        request_digest("POST", "/_lys/recipients", body)?,
    )?;
    assert_eq!(
        refusal(ServiceWindow::new().admit(&trusted, &elsewhere, now)),
        "ServiceSignatureInvalid"
    );
    Ok(())
}
