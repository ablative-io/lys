//! HTTP sign-in and verification shared by pass rights regressions.

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::app_custody::secret;
use identity_contract::apps::BEA;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_pass::{KeySet, VerifiedPass};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The address every fixture app's registration lists.
const BACK: &str = "https://app.example.test/signed-in";
const VERIFIER: &str = "a-pass-verifier-of-enough-length-0123456789-abcdefghij";

/// The seeded service with Lys's provider on, its signing key written.
pub(super) async fn world() -> Result<(Service, Seeded), Box<dyn Error>> {
    let broker = identity_contract::app_custody::start().await?;
    Service::start_adjusted(
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
            Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)
        },
    )
    .await
}

pub(super) fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// A code for `app`, as the browser signed in with `cookie` is handed it.
pub(super) async fn code(
    service: &Service,
    cookie: &str,
    app: &str,
) -> Result<String, Box<dyn Error>> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let answer = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", app),
            ("redirect_uri", BACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(answer.status(), 303);
    let back = answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the answer sends the browser nowhere")?
        .to_str()?;
    let back = reqwest::Url::parse(back)?;
    Ok(back
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the app is given a code")?)
}

/// The token route's answer to `app` for `form`, with the app's secret.
pub(super) async fn token(
    service: &Service,
    app: &str,
    form: &[(&str, &str)],
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Basic {}", STANDARD.encode(format!("{app}:{}", secret()))),
        )
        .form(form)
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, serde_json::from_str(&answer.text().await?)?))
}

/// The person signed in with `cookie` signs in to `app`: the token answer.
pub(super) async fn signed_in(
    service: &Service,
    cookie: &str,
    app: &str,
) -> Result<Value, Box<dyn Error>> {
    let code = code(service, cookie, app).await?;
    let (status, answer) = token(
        service,
        app,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", BACK),
            ("code_verifier", VERIFIER),
        ],
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(answer)
}

pub(super) async fn refreshed(
    service: &Service,
    app: &str,
    answer: &Value,
) -> Result<(u16, Value), Box<dyn Error>> {
    let refresh = answer["refresh_token"]
        .as_str()
        .ok_or("a refresh token is issued")?;
    token(
        service,
        app,
        &[("grant_type", "refresh_token"), ("refresh_token", refresh)],
    )
    .await
}

/// The keys Lys publishes, as a product reads them.
pub(super) async fn published(service: &Service) -> Result<KeySet, Box<dyn Error>> {
    let text = reqwest::get(format!("{}/oauth/jwks", service.base))
        .await?
        .text()
        .await?;
    Ok(KeySet::from_json(&text)?)
}

/// The answer's pass, verified by lys-pass for the audience `app`.
pub(super) async fn verified(
    service: &Service,
    answer: &Value,
    app: &str,
) -> Result<VerifiedPass, Box<dyn Error>> {
    let pass = answer["access_token"].as_str().ok_or("a pass is issued")?;
    Ok(VerifiedPass::verify(
        pass,
        &published(service).await?,
        &service.base,
        app,
        now()?,
    )?)
}
