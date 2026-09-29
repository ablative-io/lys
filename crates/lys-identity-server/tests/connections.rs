//! Connections are admitted for the administrator and show configuration without credentials.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn configured_connections_are_not_a_health_claim() -> TestResult {
    let service = Service::start().await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/connections", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["health_checked"], false);
    let connections = body["connections"].as_array().ok_or("connections absent")?;
    assert_eq!(connections.len(), 3);
    assert_eq!(connections[0]["state"], "configured");
    // The sign-in service is named on Lys's own origin, as installed; its
    // loopback address is never shown.
    assert_eq!(connections[0]["endpoint"], service.base.as_str());
    assert!(!body.to_string().contains(service.issuer.loopback()));
    assert_eq!(connections[1]["state"], "local");
    assert_eq!(connections[2]["state"], "unconfigured");
    assert!(connections[1]["endpoint"].is_null());
    assert!(connections[2]["endpoint"].is_null());
    for entry in connections {
        let object = entry.as_object().ok_or("connection is not an object")?;
        let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(keys, ["endpoint", "id", "name", "purpose", "state"]);
    }
    Ok(())
}

#[tokio::test]
async fn connections_refuse_anonymous_and_non_administrator_reads() -> TestResult {
    let service = Service::start().await?;
    let (status, _) = service.get("/connections", None).await?;
    assert_eq!(status, 401);
    let cookie = service
        .sign_in(Login {
            subject: "ordinary-person".to_owned(),
            email: "person@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/connections", Some(&cookie)).await?;
    assert_eq!(status, 403);
    assert_eq!(body["refusal"], "NotAdmitted");
    assert!(body.get("connections").is_none());
    Ok(())
}

#[tokio::test]
async fn configured_projection_is_reported_without_reading_its_key_or_connecting() -> TestResult {
    let (service, ()) = Service::start_judging(
        identity_contract::harness::GRANT_MODEL,
        Some(lys_identity_server::spicedb::SpiceDbSettings {
            endpoint: "does-not-exist.invalid:8089".to_owned(),
            key_file: std::path::PathBuf::from("/unreadable-connection-test-key"),
            mirror: "test".to_owned(),
        }),
        |_| Ok(()),
    )
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/connections", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["connections"][1]["state"], "configured");
    assert_eq!(
        body["connections"][1]["endpoint"],
        "http://does-not-exist.invalid:8089"
    );
    assert_eq!(body["health_checked"], false);
    assert!(!body.to_string().contains("unreadable-connection-test-key"));
    Ok(())
}
