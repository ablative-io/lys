#![cfg(test)]
//! `/sign-in/providers` names Lys's own address (DIRECTORY-101 R5): the
//! origin of the configured `redirect_url`, the address every provider is
//! registered with, in both its answers, so a sign-in page opened at any
//! other address can say so before a person tries.

use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{GRANT_MODEL, Service};
use identity_contract::lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::json;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The origin of the configured redirect address: scheme, host and port.
fn origin_of(redirect_url: &str) -> Result<String> {
    Ok(reqwest::Url::parse(redirect_url)?
        .origin()
        .ascii_serialization())
}

#[tokio::test]
async fn the_empty_answer_names_lyss_own_origin() -> Result {
    let (service, redirect_url) =
        Service::start_with(|config| Ok(config.redirect_url.clone())).await?;
    let (status, answer) = service.get("/sign-in/providers", None).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["providers"], json!([]), "{answer}");
    assert_eq!(
        answer["origin"],
        origin_of(&redirect_url)?.as_str(),
        "{answer}"
    );
    Ok(())
}

#[tokio::test]
async fn the_listed_answer_names_lyss_own_origin() -> Result {
    let rauthy = FakeRauthy::start().await?;
    reqwest::Client::new()
        .post(format!("{}/providers/create", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&json!({"name": "Google", "enabled": true, "client_id": "google-client"}))
        .send()
        .await?
        .error_for_status()?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, redirect_url) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        |_| {},
        |config| Ok(config.redirect_url.clone()),
    )
    .await?;
    let (status, answer) = service.get("/sign-in/providers", None).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["providers"][0]["name"], "Google", "{answer}");
    assert_eq!(
        answer["origin"],
        origin_of(&redirect_url)?.as_str(),
        "{answer}"
    );
    Ok(())
}
