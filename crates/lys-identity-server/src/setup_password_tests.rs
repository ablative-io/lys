use super::set_password_only;
use crate::sign_in_providers::{SignInProviders, SignInProvidersSettings};
use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use serde_json::json;

#[tokio::test]
async fn recovery_sets_only_password_and_keeps_disabled_unverified_and_other_factors()
-> Result<(), Box<dyn std::error::Error>> {
    let issuer = FakeRauthy::start().await?;
    let made: serde_json::Value = reqwest::Client::new()
        .post(format!("{}/users", issuer.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&json!({
            "email":"administrator@example.test", "given_name":"Administrator", "family_name":null,
            "language":"en", "roles":[], "groups":null, "enabled":false,"email_verified":false,
            "user_values":{},"auth_provider_id":"google", "federation_uid":"federated-user",
            "webauthn_user_id":"passkey-owner"
        }))
        .send()
        .await?
        .json()
        .await?;
    let id = made["id"].as_str().ok_or("fixture account id missing")?;
    let api = SignInProviders::open(
        &SignInProvidersSettings {
            api: issuer.api().to_owned(),
            api_key_file: issuer.api_key_file(),
        },
        None,
    )?;
    set_password_only(&api, None, id, "A-new-password-1234").await?;
    let users = issuer.users()?;
    assert_eq!(users.len(), 1);
    for field in [
        "enabled",
        "email_verified",
        "auth_provider_id",
        "federation_uid",
        "webauthn_user_id",
    ] {
        assert_eq!(users[0][field], made[field], "recovery changed {field}");
    }
    Ok(())
}
