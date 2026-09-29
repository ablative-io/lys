//! Registered lifecycle regression coverage.
use super::support::{TestResult, fixture_serving, inactive, stored};
use identity_contract::harness::Service;
use lys_identity::PersonId;
use serde_json::{Value, json};
use std::error::Error;

async fn issuer_account_fixture(
    active: bool,
) -> Result<
    (
        Service,
        PersonId,
        identity_contract::fake_rauthy::FakeRauthy,
        String,
    ),
    Box<dyn Error>,
> {
    use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
    use lys_identity_server::sign_in_providers::SignInProvidersSettings;

    let rauthy = FakeRauthy::start().await?;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/users", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json!({"email": "registered@example.test", "enabled": true}).to_string())
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let mut account: Value = serde_json::from_str(&response)?;
    let subject = account["id"].as_str().ok_or("issuer user id")?.to_owned();
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, person, _) = fixture_serving(&subject, active, None, Some(settings)).await?;
    rauthy.link(service.issuer.clone());
    account["password"] = json!("Original-Password-12345");
    client
        .put(format!("{}/users/{subject}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(account.to_string())
        .send()
        .await?
        .error_for_status()?;
    let cookie = service
        .sign_in_with("registered@example.test", "Original-Password-12345")
        .await?;
    Ok((service, person, rauthy, cookie))
}

async fn account_mutation(active: bool, password: bool) -> TestResult {
    let (service, person, rauthy, cookie) = issuer_account_fixture(active).await?;
    let (path, body) = if password {
        (
            "/me/account/password",
            json!({"current": "Original-Password-12345", "password": "Replacement-Password-23456"}),
        )
    } else {
        (
            "/me/account/email",
            json!({"email": "changed@example.test", "password": "Original-Password-12345"}),
        )
    };
    let requests = rauthy.request_count();
    let users = rauthy.users();
    let bytes = stored(&service)?;
    let answer = service.post(path, Some(&cookie), &body).await?;
    if active {
        assert_eq!(answer.0, 200, "{}", answer.1);
        assert!(
            rauthy.request_count() > requests,
            "Active control did not reach issuer"
        );
        let (email, secret) = if password {
            ("registered@example.test", "Replacement-Password-23456")
        } else {
            ("changed@example.test", "Original-Password-12345")
        };
        service.sign_in_with(email, secret).await?;
    } else {
        assert_eq!(
            rauthy.request_count(),
            requests,
            "Registered request reached issuer"
        );
        assert_eq!(
            rauthy.users(),
            users,
            "Registered request changed issuer account"
        );
        assert_eq!(stored(&service)?, bytes);
        inactive(&answer, person);
    }
    Ok(())
}

#[tokio::test]
async fn registered_email_change_never_reaches_issuer() -> TestResult {
    account_mutation(false, false).await
}

#[tokio::test]
async fn registered_password_change_never_reaches_issuer() -> TestResult {
    account_mutation(false, true).await
}

#[tokio::test]
async fn active_email_change_reaches_issuer_and_new_email_signs_in() -> TestResult {
    account_mutation(true, false).await
}

#[tokio::test]
async fn active_password_change_reaches_issuer_and_new_password_signs_in() -> TestResult {
    account_mutation(true, true).await
}
