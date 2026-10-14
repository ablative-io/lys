#![cfg(test)]
//! A product's refresh grant survives the provider and session store reopening.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::apps::{BEA, NOTES, login, registered};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use serde_json::Value;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const BACK: &str = "https://app.example.test/signed-in";
const VERIFIER: &str = "refresh-restart-verifier-0123456789-abcdefghij";

async fn token(service: &Service, form: &[(&str, &str)]) -> Result<Value, Box<dyn Error>> {
    let response = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!(
                    "{NOTES}:{}",
                    identity_contract::app_custody::secret()
                ))
            ),
        )
        .form(form)
        .send()
        .await?;
    let status = response.status();
    let body: Value = response.json().await?;
    assert_eq!(status, 200, "refusal: {}", body["refusal"]);
    Ok(body)
}

#[tokio::test]
async fn a_refresh_issued_before_restart_renews_without_signing_in_again() -> TestResult {
    let broker = identity_contract::app_custody::start().await?;
    let (mut service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds: CODE_SECONDS,
                pass_seconds: PASS_SECONDS,
                rights_bytes: None,
            });
        },
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            seed_configured(config, [ADMINISTRATOR, BEA])?;
            Ok(())
        },
    )
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &cookie, NOTES).await?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}/oauth/authorize", service.base))
        .header(reqwest::header::COOKIE, &cookie)
        .query(&[
            ("client_id", NOTES),
            ("redirect_uri", BACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), 303);
    let back = reqwest::Url::parse(response.headers()[reqwest::header::LOCATION].to_str()?)?;
    let code = back
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("no authorization code")?;
    let issued = token(
        &service,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", BACK),
            ("code_verifier", VERIFIER),
        ],
    )
    .await?;
    let refresh = issued["refresh_token"].as_str().ok_or("no refresh token")?;
    let access = issued["access_token"].as_str().ok_or("no access token")?;
    let file = service.dir.path().join("provider.tokens.json");
    let stored = std::fs::read_to_string(&file)?;
    assert!(!stored.contains(refresh), "a refresh secret was stored");
    assert!(!stored.contains(access), "a bearer secret was stored");
    assert_eq!(
        std::fs::metadata(&file)?.permissions().mode() & 0o777,
        0o600
    );
    service.restart().await?;
    let renewed = token(
        &service,
        &[("grant_type", "refresh_token"), ("refresh_token", refresh)],
    )
    .await?;
    assert!(renewed["refresh_token"].as_str() == Some(refresh));
    assert!(renewed["access_token"].is_string());
    Ok(())
}
