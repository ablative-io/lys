//! A review must name the current profile before it can authorize a start.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn profile(from: u32) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "from_version": from,
        "model_access": ["model-a"],
        "tools": [],
        "skills": [],
        "mcp_servers": [],
        "instructions": "",
        "note": ""
    }))
}

#[tokio::test]
async fn a_replaced_profile_is_refused_naming_the_latest_version() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(
            config,
            [ADMINISTRATOR, "reviewer-subject"],
        )?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let path = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    for from in 0..2 {
        let (status, body) = service.post(&path, Some(&cookie), &profile(from)?).await?;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["profile"]["version"], from + 1);
    }
    let review = json!({ "operation": OperationId::generate()?.to_string() });
    let (status, body) = service
        .post(&format!("{path}/1/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["refusal"], "ProfileVersionReplaced", "{body}");
    assert!(
        body["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("version 2")),
        "{body}"
    );
    let (status, current) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{current}");
    assert_eq!(current["profile"]["reviewed_by"], Value::Null);
    let (status, reviewed) = service
        .post(&format!("{path}/2/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 200, "{reviewed}");
    assert!(reviewed["profile"]["reviewed_by"].is_string());
    Ok(())
}
