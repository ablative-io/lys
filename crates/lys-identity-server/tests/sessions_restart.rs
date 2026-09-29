//! A restart of the service leaves everyone signed in: a cookie from before
//! the restart still answers, starting from an install that had never kept
//! a sessions file. A session ended before the restart stays ended after it.
//! The kept file is owner-only and never holds a cookie secret.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::Service;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const ADA: &str = "ada-subject";

fn login() -> Login {
    Login {
        subject: ADA.to_owned(),
        email: "ada@example.test".to_owned(),
    }
}

fn secret(cookie: &str) -> Result<&str, Box<dyn Error>> {
    Ok(cookie
        .split_once('=')
        .map(|(_, value)| value)
        .ok_or("the cookie carries no value")?)
}

#[tokio::test]
async fn a_cookie_from_before_a_restart_still_answers() -> TestResult {
    let (mut service, _) =
        Service::start_with(|config| Ok(seed_configured(config, [ADA, "bea-subject"])?)).await?;
    let file = service.dir.path().join("sessions.json");
    assert!(!file.exists(), "the install starts with no sessions file");
    let kept = service.sign_in(login()).await?;
    let ended = service.sign_in(login()).await?;

    let written = std::fs::read_to_string(&file)?;
    for cookie in [&kept, &ended] {
        assert!(
            !written.contains(secret(cookie)?),
            "the file holds a cookie secret"
        );
    }
    assert_eq!(
        std::fs::metadata(&file)?.permissions().mode() & 0o777,
        0o600
    );

    let (status, listed) = service.get("/sessions", Some(&kept)).await?;
    assert_eq!(status, 200, "{listed}");
    let other = listed["sessions"]
        .as_array()
        .and_then(|sessions| sessions.iter().find(|session| session["current"] == false))
        .and_then(|session| session["id"].as_str())
        .ok_or("the second session is not listed")?
        .to_owned();
    let (status, body) = service
        .post(&format!("/sessions/{other}/end"), Some(&kept), &json!({}))
        .await?;
    assert_eq!(status, 200, "{body}");

    service.restart().await?;

    let (status, me) = service.get("/me", Some(&kept)).await?;
    assert_eq!(status, 200, "a cookie from before the restart: {me}");
    let (status, body) = service.get("/me", Some(&ended)).await?;
    assert_eq!(status, 401, "an ended session stays ended: {body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    Ok(())
}
