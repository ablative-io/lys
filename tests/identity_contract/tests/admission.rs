//! R3: the configured administrator admitted, every other caller refused by
//! name, and an email or a first visit never admitted.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::fixtures::op;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::admission::AUTHORITY;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const SHARED_EMAIL: &str = "admin@example.test";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: SHARED_EMAIL.to_owned(),
    }
}

fn person(n: u8, name: &str) -> serde_json::Value {
    json!({ "operation": op(n).to_string(), "display_name": name })
}

#[tokio::test]
async fn the_configured_administrator_is_admitted() -> TestResult {
    let service = Service::start().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, body) = service
        .post("/people", Some(&cookie), &person(1, "Ada"))
        .await?;
    assert_eq!(status, 200, "{body}");
    assert!(
        body["person"]
            .as_str()
            .is_some_and(|id| id.starts_with("person-"))
    );
    let (status, body) = service.get("/identities", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["identities"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn another_login_with_the_administrators_email_is_refused_by_name() -> TestResult {
    let service = Service::start().await?;
    let other = service.sign_in(login("someone-else")).await?;
    let (status, body) = service
        .post("/people", Some(&other), &person(1, "Mallory"))
        .await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NotAdmitted");
    let (status, body) = service.get("/identities", Some(&other)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NotAdmitted", "no read either");
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (_, body) = service.get("/identities", Some(&administrator)).await?;
    assert_eq!(
        body["identities"].as_array().map(Vec::len),
        Some(0),
        "the refused caller recorded nothing and was not registered on sight"
    );
    Ok(())
}

#[tokio::test]
async fn a_caller_without_a_session_is_refused_by_name() -> TestResult {
    let service = Service::start().await?;
    let (status, body) = service.post("/people", None, &person(1, "Ada")).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    let forged = "lys_directory_session=00";
    let (status, body) = service
        .post("/people", Some(forged), &person(1, "Ada"))
        .await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    Ok(())
}

#[tokio::test]
async fn a_first_visit_admits_nobody_and_its_answer_is_used_once() -> TestResult {
    let service = Service::start().await?;
    let back = service.issuer_answer(login(ADMINISTRATOR)).await?;
    let (status, body) = service.post("/people", None, &person(1, "Ada")).await?;
    assert_eq!(
        status, 401,
        "a visit that has not come back is not signed in: {body}"
    );
    let (status, body) = service.get(&back, None).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["authority"], AUTHORITY,
        "the authority is shown at sign-in"
    );
    assert_eq!(body["signed_in"]["subject"], ADMINISTRATOR);
    let (status, body) = service.get(&back, None).await?;
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["refusal"], "SignInStateUnknown");
    Ok(())
}
