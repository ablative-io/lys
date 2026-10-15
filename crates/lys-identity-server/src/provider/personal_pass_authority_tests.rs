#![cfg(test)]
//! Personal Lys authority and app authority stay distinct through the HTTP routes.

use std::error::Error;
use std::sync::Arc;

use crate::apps_state::{Approved, By, Client, Line, Registered};
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use lys_identity::{Actor, AuthMethod, OperationId, Provenance};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

async fn post(
    state: &Arc<crate::routes::AppState>,
    pass: &str,
    path: &str,
    body: Value,
) -> TestResult<(u16, Value)> {
    let router = crate::signed_first::guarded(
        crate::routes_table::router(Arc::clone(state)),
        Arc::clone(state),
    );
    let response = crate::session_admission::guarded(router, Arc::clone(state))
        .oneshot(
            Request::post(path)
                .header(header::AUTHORIZATION, format!("Bearer {pass}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let status = response.status().as_u16();
    Ok((
        status,
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?,
    ))
}

fn pass(
    state: &crate::routes::AppState,
    app: &str,
    session: &str,
    subject: &str,
) -> TestResult<String> {
    let provider = state.provider.as_ref().ok_or("provider absent")?;
    let at = crate::session::now();
    let pass = provider.signed(&json!({
        "iss": provider.issuer(), "aud": app, "sub": subject,
        "iat": at, "exp": at + 60,
        "holder": {"id": subject, "kind": "person"}, "rights": []
    }))?;
    super::super::held(&provider.tokens)?.insert(
        crate::routes::hex(&Sha256::digest(pass.as_bytes())),
        super::super::Access {
            session_id: session.to_owned(),
            subject: subject.to_owned(),
            app: app.to_owned(),
            profile: false,
            expires_at: at + 60,
            kind: None,
        },
        at,
    )?;
    Ok(pass)
}

#[tokio::test]
async fn administrator_personal_pass_suspends_another_identity() -> TestResult {
    let (_dir, mut state, _, _, _, target) =
        crate::grant_settlement_tests::person_fixture().await?;
    let administrator = Actor::new(
        state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let person = crate::routes::with_directory(&state, |directory| {
        crate::read_api::own_person(directory.projection()?, &administrator)
    })?;
    crate::provider::tests::prepare_refresh(&mut state, administrator, person)?;
    let token = crate::provider::tests::refresh(&state)?;
    let pass = token["access_token"]
        .as_str()
        .ok_or("personal pass absent")?;
    let state = Arc::new(state);
    let (status, answer) = post(&state, pass,
        &format!("/identities/{target}/transitions"),
        json!({"operation": OperationId::generate()?.to_string(), "transition": "suspend", "reason": "authority regression"}),
    ).await?;
    assert_eq!(status, 200, "personal pass suspension refused: {answer}");
    Ok(())
}

#[tokio::test]
async fn other_app_person_pass_cannot_suspend_or_register_for_others() -> TestResult {
    let (_dir, mut state, _, _, _, target) =
        crate::grant_settlement_tests::person_fixture().await?;
    let administrator = Actor::new(
        state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let person = crate::routes::with_directory(&state, |directory| {
        crate::read_api::own_person(directory.projection()?, &administrator)
    })?;
    crate::provider::tests::prepare_refresh(&mut state, administrator, person)?;
    let session = state
        .sessions
        .live(|_| true)?
        .into_iter()
        .next()
        .ok_or("session absent")?
        .id;
    crate::apps_api::with_apps(&state, |apps, _| {
        apps.keep(Line::Registered(Registered {
            operation: "register-fixture".to_owned(), app: "fixture_app".to_owned(), name: "Fixture".to_owned(),
            redirects: vec!["https://fixture.test/callback".to_owned()],
            schema: json!({"kinds": {"fixture_app.doc": {"actions": ["read"], "relations": {"viewer": ["read"]}}}}),
            service_account: None, by: By::Start, at: 1,
        }))?;
        apps.keep(Line::Approved(Approved {
            operation: "approve-fixture".to_owned(),
            app: "fixture_app".to_owned(),
            client: Client {
                client_id: "fixture_app".to_owned(),
                secret_sha256: crate::apps_binding::sha256_hex("fixture"),
            },
            binding: None,
            by: By::Start,
            at: 2,
        }))?;
        Ok(())
    })?;
    let pass = pass(&state, "fixture_app", &session, &person.to_string())?;
    let other = crate::routes::with_directory(&state, |directory| {
        directory
            .projection()?
            .record(lys_identity::IdentityId::Agent(target))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(crate::ServerError::NoPerson)
    })?;
    let state = Arc::new(state);
    for (path, body) in [
        (
            format!("/identities/{target}/transitions"),
            json!({"operation": OperationId::generate()?.to_string(), "transition": "suspend"}),
        ),
        (
            "/agents".to_owned(),
            json!({"operation": OperationId::generate()?.to_string(), "display_name": "Foreign", "answers_to": other.to_string()}),
        ),
    ] {
        let (status, answer) = post(&state, &pass, &path, body).await?;
        assert_eq!(status, 403, "app authority escaped at {path}: {answer}");
    }
    Ok(())
}
