#![cfg(test)]
//! A person's own account (DIRECTORY-101 R3): `GET /me/account` says whether
//! the account has a password and which providers it signs in with; an
//! account with no password sets one and changes its email with no current
//! password, the signed-in session being the only proof there is, and an
//! account with a password is asked for it every time.

use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use identity_contract::lys_identity_server::sign_in_providers::SignInProvidersSettings;
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Call the issuer's administration API as the installation's key does.
async fn issuer_api(rauthy: &FakeRauthy, path: &str, body: &Value) -> Result<Value> {
    Ok(reqwest::Client::new()
        .post(format!("{}{path}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(body)
        .send()
        .await?
        .json()
        .await?)
}

/// The service over an issuer holding Google as a provider and Gail's account,
/// bound to Google with no password, and Gail's session.
async fn table() -> Result<(Service, FakeRauthy, String)> {
    let rauthy = FakeRauthy::start().await?;
    let google = issuer_api(
        &rauthy,
        "/providers/create",
        &json!({"name": "Google", "enabled": true, "client_id": "google-client"}),
    )
    .await?;
    let gail = issuer_api(
        &rauthy,
        "/users",
        &json!({
            "email": "gail@example.test", "given_name": "Gail", "language": "en", "roles": [],
            "enabled": true, "email_verified": true, "user_values": {},
            "auth_provider_id": google["id"], "federation_uid": "google-gail",
        }),
    )
    .await?;
    let subject = gail["id"]
        .as_str()
        .ok_or("the issuer names the account")?
        .to_owned();
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, cookie) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        |_| {},
        |config| {
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "bea-subject"],
            )?;
            let sessions = crate::session::Sessions::open(
                config.sessions_file.clone().ok_or("session file missing")?,
                600,
                false,
            )?;
            Ok(sessions.begin(Actor::new(
                LoginBinding::new(&config.issuer, &subject)?,
                Provenance::new(AuthMethod::Oidc, crate::session::now()),
            ))?)
        },
    )
    .await?;
    Ok((service, rauthy, cookie))
}

#[tokio::test]
async fn a_google_account_with_no_password_sets_one_without_a_current_then_needs_it() -> Result {
    let (service, _rauthy, gail) = table().await?;
    let (status, shown) = service.get("/me/account", Some(&gail)).await?;
    assert_eq!(status, 200, "{shown}");
    assert_eq!(shown["password"], false, "{shown}");
    assert_eq!(shown["providers"], json!(["google"]), "{shown}");

    let (status, changed) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email": "gail@lovelace.test"}),
        )
        .await?;
    assert_eq!(
        status, 200,
        "an account with no password changes its email with none: {changed}"
    );
    assert_eq!(changed["email"], "gail@lovelace.test");

    let (status, set) = service
        .post(
            "/me/account/password",
            Some(&gail),
            &json!({"password": "Gail-First-Password-77"}),
        )
        .await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["password"], true, "{set}");
    assert_eq!(set["providers"], json!(["google"]), "{set}");

    let (status, again) = service
        .post(
            "/me/account/password",
            Some(&gail),
            &json!({"password": "Gail-Second-Password-88"}),
        )
        .await?;
    assert_eq!(
        (status, again["refusal"].clone()),
        (400, json!("AccountRefused")),
        "{again}"
    );
    let (status, email) = service
        .post(
            "/me/account/email",
            Some(&gail),
            &json!({"email": "gail@example.test"}),
        )
        .await?;
    assert_eq!(
        (status, email["refusal"].clone()),
        (400, json!("AccountRefused")),
        "{email}"
    );
    Ok(())
}
