//! A report for a session never started is refused by its runtime name.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

#[tokio::test]
async fn an_unknown_reported_session_keeps_its_refusal_name() -> Result<(), Box<dyn Error>> {
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.homes_dir = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        |config| Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?),
    )
    .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let agent = seeded.people[0].agents[0].id;
    let session = OperationId::generate()?;
    let path = format!("/agents/{agent}/runtime/sessions/{session}/reports");
    let body = json!({
        "operation": OperationId::generate()?.to_string(),
        "machine": OperationId::generate()?.to_string(),
        "state": "running",
    });
    let (status, refused) = service.post(&path, Some(&ada), &body).await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "RuntimeSessionUnknown", "{refused}");
    let (status, sessions) = service
        .get(&format!("/agents/{agent}/runtime/sessions"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{sessions}");
    assert_eq!(sessions["sessions"], json!([]));
    Ok(())
}
