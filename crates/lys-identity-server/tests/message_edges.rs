//! Message reads authenticate before exposing integration configuration and name unavailable integration.

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::seed_configured;
use std::error::Error;

#[tokio::test]
async fn unconfigured_messages_refuse_by_name_after_sign_in() -> Result<(), Box<dyn Error>> {
    let (service, _) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR])?)).await?;
    let (status, _) = service.get("/runtime/message-edges", None).await?;
    assert_eq!(status, 401);
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "admin@example.test".to_owned(),
        })
        .await?;
    let (status, body) = service.get("/runtime/message-edges", Some(&cookie)).await?;
    assert_eq!(status, 503, "{body}");
    assert_eq!(body["refusal"], "MessageEdgesUnavailable");
    assert!(
        body["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("cambium_messages")),
        "{body}"
    );
    Ok(())
}
