#![cfg(test)]
//! Run credentials keep their refusal instead of becoming OAuth sign-in.

use std::error::Error;

use identity_contract::harness::{GRANT_MODEL, Service};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

async fn mcp(
    service: &Service,
    credential: Option<(&str, &str)>,
) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut request = reqwest::Client::new()
        .post(format!("{}/api/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "fixture", "version": "1"}}}));
    if let Some((name, value)) = credential {
        request = request.header(name, value);
    }
    Ok(request.send().await?)
}

#[tokio::test]
async fn mcp_unknown_pass_refuses_as_json_without_oauth_challenge() -> TestResult {
    let screens = tempfile::tempdir()?;
    std::fs::write(screens.path().join("index.html"), "<html></html>")?;
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.surface_dir = Some(screens.path().to_path_buf());
        },
        |_| Ok(()),
    )
    .await?;
    let response = mcp(&service, Some(("lys-agent-pass", &"a".repeat(43)))).await?;
    assert_eq!(response.status(), 401);
    assert!(!response.headers().contains_key("www-authenticate"));
    let body: Value = response.json().await?;
    assert_eq!(body["credential"], "pass", "{body}");
    assert_eq!(body["refusal"], "pass_unknown_or_ended", "{body}");
    assert_eq!(
        body["words"], "the pass is unknown or its run has ended",
        "{body}"
    );
    Ok(())
}

#[tokio::test]
async fn mcp_unverifiable_seat_refuses_without_oauth_challenge() -> TestResult {
    let screens = tempfile::tempdir()?;
    std::fs::write(screens.path().join("index.html"), "<html></html>")?;
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.surface_dir = Some(screens.path().to_path_buf());
        },
        |_| Ok(()),
    )
    .await?;
    let response = mcp(&service, Some(("lys-seat", "unverifiable"))).await?;
    assert_eq!(response.status(), 401);
    assert!(!response.headers().contains_key("www-authenticate"));
    let body: Value = response.json().await?;
    assert_eq!(body["credential"], "seat", "{body}");
    assert_eq!(body["refusal"], "seat_unverifiable", "{body}");
    assert!(
        body["words"]
            .as_str()
            .is_some_and(|words| !words.is_empty())
    );
    Ok(())
}

#[tokio::test]
async fn mcp_without_run_credential_retains_oauth_challenge() -> TestResult {
    let screens = tempfile::tempdir()?;
    std::fs::write(screens.path().join("index.html"), "<html></html>")?;
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.surface_dir = Some(screens.path().to_path_buf());
        },
        |_| Ok(()),
    )
    .await?;
    let response = mcp(&service, None).await?;
    assert_eq!(response.status(), 401);
    let challenge = response
        .headers()
        .get("www-authenticate")
        .ok_or("missing OAuth challenge")?
        .to_str()?;
    assert!(
        challenge.starts_with("Bearer resource_metadata="),
        "{challenge}"
    );
    Ok(())
}
