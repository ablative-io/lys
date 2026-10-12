//! A reviewed seat's optional launch settings survive provisioning and restart.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::provisioning_store::ProvisioningStore;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn profile(contract: &str) -> Result<Value, Box<dyn Error>> {
    let catalogue: Value = serde_json::from_str(if contract == "codex" {
        include_str!("../../../docs/harness/catalogue/codex.json")
    } else {
        include_str!("../../../docs/harness/catalogue/claude-code.json")
    })?;
    Ok(json!({
        "operation": OperationId::generate()?.to_string(), "from_version": 0,
        "model_access": ["model-a"], "tools": [], "skills": [], "mcp_servers": [],
        "instructions": "", "note": "",
        "harness": {"name": "fixture", "program": "/opt/fixture/harness",
            "package": "fixture", "description": catalogue["description"]}
    }))
}

async fn table() -> Result<(Service, String, String), Box<dyn Error>> {
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
    Ok((service, cookie, path))
}

#[tokio::test]
async fn provisioning_session_launch_fields_round_trip_and_reopen() -> TestResult {
    for harness in ["claude-code", "codex"] {
        let (service, cookie, path) = table().await?;
        let mut body = profile(harness)?;
        body["session"] = json!({"resume": "019c6e27-e55b-73d1-87d8-4e01f1f75043",
            "context_window": 200_000, "config_dir": "/tmp/chosen-config", "sensitive": false});
        let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
        assert_eq!(status, 200, "{answer}");
        let (status, answer) = service.get(&path, Some(&cookie)).await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["profile"]["session"], body["session"]);
        let store = ProvisioningStore::open(&service.dir.path().join("provisioning.json"))?;
        let agent = path.split('/').nth(2).ok_or("missing agent")?;
        let kept = store.profile(agent).ok_or("profile absent")?;
        assert_eq!(
            serde_json::to_value(&kept.versions[0].settings.session)?,
            body["session"]
        );
    }
    Ok(())
}

#[tokio::test]
async fn provisioning_session_values_are_refused_by_name() -> TestResult {
    let (service, cookie, path) = table().await?;
    for (field, value) in [
        ("resume", json!("not-an-id")),
        ("context_window", json!(0)),
        ("config_dir", json!("relative/config")),
    ] {
        let mut body = profile("codex")?;
        body["session"] = json!({field: value});
        let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
        assert_eq!(status, 400, "{answer}");
        assert!(
            answer["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(field)),
            "{answer}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn provisioning_seat_commands_and_settings_round_trip() -> TestResult {
    let mut failures = Vec::new();
    for (field, value) in [
        (
            "hooks",
            json!({"Stop":[{"command":"seat-stop", "timeout":5}]}),
        ),
        ("status_line", json!("seat-status")),
        ("settings", json!({"theme":"dark", "nested":{"a":1}})),
    ] {
        let (service, cookie, path) = table().await?;
        let mut body = profile("claude-code")?;
        body[field] = value.clone();
        let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
        if status != 200 {
            failures.push(format!("{field}: {status} {answer}"));
            continue;
        }
        let (status, answer) = service.get(&path, Some(&cookie)).await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["profile"][field], value);
        let store = ProvisioningStore::open(&service.dir.path().join("provisioning.json"))?;
        let agent = path.split('/').nth(2).ok_or("missing agent")?;
        let kept = store.profile(agent).ok_or("profile absent")?;
        assert_eq!(
            serde_json::to_value(&kept.versions[0].settings)?[field],
            value
        );
    }
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[tokio::test]
async fn provisioning_seat_field_refusals_name_the_governed_value() -> TestResult {
    let (service, cookie, path) = table().await?;
    for (harness, field, value, named) in [
        (
            "codex",
            "hooks",
            json!({"Stop":[{"command":"seat-stop"}]}),
            "hooks",
        ),
        ("codex", "status_line", json!("seat-status"), "status_line"),
        ("claude-code", "settings", json!({"env":{}}), "env"),
        (
            "claude-code",
            "hooks",
            json!({"UnknownEvent":[]}),
            "UnknownEvent",
        ),
    ] {
        let mut body = profile(harness)?;
        body[field] = value;
        let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
        assert_eq!(status, 400, "{answer}");
        assert!(
            answer["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(named)),
            "{answer}"
        );
    }
    Ok(())
}
