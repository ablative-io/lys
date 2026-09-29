//! Provisioning and skills cannot make an old reader incompatible during rollback.
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::json;

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

#[tokio::test]
async fn profile_and_skill_writes_refuse_pending_or_unreadable_upgrade_intent()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let parent = temporary.path().join("upgrade");
    std::fs::create_dir(&parent)?;
    let intent = parent.join("intent.json");
    let file = temporary.path().join("provisioning.json");
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.operator_upgrade_file = Some(intent.clone());
            config.provisioning_file = Some(file.clone());
        },
        |config| Ok(seed_configured(config, [ADMINISTRATOR, "another-person"])?),
    )
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let path = format!("/agents/{agent}/provisioning");
    let mut profile = json!({
        "operation": operation()?, "from_version": 0,
        "model_access": ["model-a"], "tools": [], "skills": [],
        "mcp_servers": [{"name": "ordinary-server", "url": "https://example.invalid/mcp"}],
        "instructions": "ordinary instructions", "note": ""
    });
    let (status, answer) = service.post(&path, Some(&cookie), &profile).await?;
    assert_eq!(status, 200, "{answer}");
    let original = std::fs::read(&file)?;
    profile["operation"] = json!(operation()?);
    profile["from_version"] = json!(1);
    profile["note"] = json!("must not be written");
    let review_path = format!("{path}/1/review");
    let review = json!({"operation": operation()?});
    let skill = json!({"name": "kept-skill", "text": "# Skill\n"});
    std::fs::write(&intent, b"pending")?;
    for unreadable in [false, true] {
        if unreadable {
            std::fs::remove_file(&intent)?;
            std::fs::remove_dir(&parent)?;
            std::fs::write(&parent, b"not a directory")?;
        }
        for (route, body) in [
            (path.as_str(), &profile),
            (review_path.as_str(), &review),
            ("/skills", &skill),
        ] {
            let (status, answer) = service.post(route, Some(&cookie), body).await?;
            assert_eq!(status, 503, "{route}: {answer}");
            assert_eq!(answer["refusal"], "ProvisioningUnavailable", "{route}");
            assert_eq!(
                std::fs::read(&file)?,
                original,
                "{route} altered old profile bytes"
            );
        }
    }
    Ok(())
}
