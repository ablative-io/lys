//! Effective settings are administrator-only metadata and never disclose configuration secrets or private paths.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn configuration_is_the_effective_startup_view_without_secrets() -> TestResult {
    let service = Service::start().await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/configuration", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["source"], "startup_configuration");
    assert_eq!(body["mutable_in_browser"], false);
    // The sign-in service is named on Lys's own origin, as installed; its
    // loopback address is never shown.
    assert_eq!(body["sign_in"]["provider_origin"], service.base.as_str());
    assert!(!body.to_string().contains(service.issuer.loopback()));
    assert!(body["sign_in"]["session_seconds"].as_u64().is_some());
    assert!(body["sign_in"]["secure_cookie"].as_bool().is_some());
    assert_eq!(body["permissions"]["projection"], "local");
    assert_eq!(body["secrets"]["configured"], false);
    assert_eq!(body["storage"]["directory_format"], "signed_leaf_log");
    let text = body.to_string();
    for private in [
        "client_secret",
        "key_file",
        "log_dir",
        "administrator",
        "link_audit_source",
    ] {
        assert!(!text.contains(private), "exposed {private}");
    }
    let signin = body["sign_in"]
        .as_object()
        .ok_or("sign-in metadata missing")?;
    assert_eq!(signin.len(), 3);
    Ok(())
}

#[tokio::test]
async fn configuration_refuses_anonymous_and_other_people() -> TestResult {
    let service = Service::start().await?;
    let (status, _) = service.get("/configuration", None).await?;
    assert_eq!(status, 401);
    let cookie = service
        .sign_in(Login {
            subject: "ordinary-person".to_owned(),
            email: "ordinary@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/configuration", Some(&cookie)).await?;
    assert_eq!(status, 403);
    assert_eq!(body["refusal"], "NotAdmitted");
    assert!(body.get("sign_in").is_none());
    Ok(())
}
