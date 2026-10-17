#![cfg(test)]
//! A product's refresh grant survives the provider and session store reopening.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::apps::{BEA, NOTES, login, registered};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, ManualClock, Service};
use lys_core::clock::Clock;
use lys_identity_server::Say;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use serde_json::Value;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const BACK: &str = "https://app.example.test/signed-in";
const VERIFIER: &str = "refresh-restart-verifier-0123456789-abcdefghij";

async fn token(service: &Service, form: &[(&str, &str)]) -> Result<Value, Box<dyn Error>> {
    let (status, body) = token_answer(service, form).await?;
    assert_eq!(status, 200, "refusal: {}", body["refusal"]);
    Ok(body)
}

/// The token route's status and body for `form`, as the product NOTES asks.
async fn token_answer(
    service: &Service,
    form: &[(&str, &str)],
) -> Result<(u16, Value), Box<dyn Error>> {
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
    let status = response.status().as_u16();
    let body: Value = response.json().await?;
    Ok((status, body))
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
    assert_eq!(renewed["refresh_token"].as_str(), Some(refresh));
    assert!(renewed["access_token"].is_string());
    Ok(())
}

/// The configuration both tests run with: the secrets broker the product's
/// secret is kept in, and the provider, its key beside the log.
fn provided(config: &mut lys_identity_server::Config, broker: SecretsSettings) {
    config.secrets = Some(broker);
    config.provider = Some(ProviderSettings {
        key_file: config.log_dir.with_file_name("provider.key"),
        code_seconds: CODE_SECONDS,
        pass_seconds: PASS_SECONDS,
        rights_bytes: None,
    });
}

/// The refresh token NOTES is given for the administrator's sign-in.
async fn issued_refresh(service: &Service) -> Result<String, Box<dyn Error>> {
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(service, &cookie, NOTES).await?;
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
        service,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", BACK),
            ("code_verifier", VERIFIER),
        ],
    )
    .await?;
    Ok(issued["refresh_token"]
        .as_str()
        .ok_or("no refresh token")?
        .to_owned())
}

/// DIRECTORY-099 R1: a refresh token whose Lys sign-in ended, read on the
/// service's own clock, is refused SessionEnded naming the instant it ended,
/// across a restart, and said so once in the service's log; a token Lys never
/// issued is still RefreshUnknown.
#[tokio::test]
async fn a_refresh_whose_sign_in_ended_is_refused_session_ended_naming_its_end() -> TestResult {
    let started = i64::try_from(lys_identity_server::session::now())?;
    let clock = Arc::new(ManualClock::new(started));
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let lines = Arc::clone(&heard);
    let say: Say = Arc::new(move |line: &str| match lines.lock() {
        Ok(mut lines) => lines.push(line.to_owned()),
        Err(poisoned) => panic!("refresh_restart_fixture_poisoned: {poisoned}"),
    });
    let broker = identity_contract::app_custody::start().await?;
    let source: Arc<dyn Clock> = Arc::<ManualClock>::clone(&clock);
    let (mut service, ()) = Service::start_clock_adjusted(
        source,
        move |config| {
            config.session_seconds = 60;
            let secrets = SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            };
            provided(config, secrets);
        },
        Some(Arc::clone(&say)),
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            seed_configured(config, [ADMINISTRATOR, BEA])?;
            Ok(())
        },
    )
    .await?;
    let refresh = issued_refresh(&service).await?;
    clock.set(started + 61);
    service.restart_saying(Some(Arc::clone(&say))).await?;
    let (status, refused) = token_answer(
        &service,
        &[("grant_type", "refresh_token"), ("refresh_token", &refresh)],
    )
    .await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"], "invalid_grant", "{refused}");
    assert_eq!(refused["refusal"], "SessionEnded", "{refused}");
    let ended = jiff::Timestamp::from_second(started + 60)?.to_string();
    let reason = format!(
        "SessionEnded: the Lys sign-in this refresh token stands on ended at {ended}; sign in again"
    );
    assert_eq!(refused["reason"], reason.as_str(), "{refused}");
    let said = heard
        .lock()
        .map_err(|poisoned| poisoned.to_string())?
        .clone();
    let refusals = |name: &str| {
        said.iter()
            .filter(|line| line.contains("token request refused: client_id"))
            .filter(|line| line.contains(&format!(" refusal {name} reason ")))
            .count()
    };
    assert_eq!(refusals("SessionEnded"), 1, "{said:#?}");
    assert_eq!(refusals("RefreshUnknown"), 0, "{said:#?}");
    assert!(
        said.iter().any(|line| line.ends_with(&format!(
            "error invalid_grant refusal SessionEnded reason {reason}"
        ))),
        "{said:#?}"
    );
    let (status, unknown) = token_answer(
        &service,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", "a-refresh-token-lys-never-issued"),
        ],
    )
    .await?;
    assert_eq!(status, 400, "{unknown}");
    assert_eq!(unknown["error"], "invalid_grant", "{unknown}");
    assert_eq!(unknown["refusal"], "RefreshUnknown", "{unknown}");
    Ok(())
}
