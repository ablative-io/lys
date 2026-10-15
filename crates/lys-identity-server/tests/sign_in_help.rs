//! Signed-out help exposes only the configured administrator's name and email.

use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

#[tokio::test]
async fn signed_out_help_names_only_the_configured_administrator()
-> Result<(), Box<dyn std::error::Error>> {
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, "member"])?))
            .await?;
    let (status, answer) = service.get("/sign-in/help", None).await?;
    assert_eq!(status, 200);
    assert_eq!(
        answer,
        json!({"administrator": {
            "name": seeded.people[0].display_name, "email": null,
        }})
    );
    Ok(())
}

#[tokio::test]
async fn an_issuer_error_return_is_a_readable_page_without_a_code()
-> Result<(), Box<dyn std::error::Error>> {
    let service = Service::start().await?;
    let answer = reqwest::Client::new()
        .get(format!(
            "{}/callback?error=access_denied&error_description=Permission%20was%20not%20given",
            service.base
        ))
        .header(reqwest::header::ACCEPT, "text/html")
        .send()
        .await?;
    assert_eq!(answer.status().as_u16(), 400);
    let page = answer.text().await?;
    assert!(page.contains("Sign-in was cancelled"));
    assert!(page.contains("Try again"));
    assert!(!page.contains("missing field"));
    Ok(())
}
