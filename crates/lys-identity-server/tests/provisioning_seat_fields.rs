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

#[tokio::test]
async fn provisioning_all_six_seat_fields_survive_review_and_get() -> TestResult {
    let (service, cookie, path) = table().await?;
    let mut body = profile("claude-code")?;
    body["hooks"] = json!({"Stop":[{"command":"seat-stop", "matcher":"*", "timeout":5}]});
    body["status_line"] = json!("seat-status");
    body["settings"] = json!({"theme":"dark", "nested":{"a":1}});
    body["session"] = json!({"resume":"019c6e27-e55b-73d1-87d8-4e01f1f75043",
        "context_window":200_000, "config_dir":"/tmp/chosen-config", "sensitive":false});
    let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
    assert_eq!(status, 200, "{answer}");
    let review = json!({"operation":OperationId::generate()?.to_string()});
    let (status, answer) = service
        .post(&format!("{path}/1/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert!(answer["profile"]["reviewed_by"].is_string());
    let store = ProvisioningStore::open(&service.dir.path().join("provisioning.json"))?;
    let agent = path.split('/').nth(2).ok_or("missing agent")?;
    let version = store
        .profile(agent)
        .ok_or("missing profile")?
        .versions
        .first()
        .ok_or("missing version")?;
    let stored = serde_json::to_value(&version.settings)?;
    for field in [
        "hooks",
        "status_line",
        "settings",
        "session",
        "resume",
        "context_window",
        "config_dir",
    ] {
        assert_eq!(answer["profile"][field], body[field]);
        assert_eq!(stored[field], body[field]);
    }
    Ok(())
}

#[tokio::test]
async fn provisioning_openapi_lists_optional_seat_fields() -> TestResult {
    let (service, cookie, _) = table().await?;
    let (status, document) = service.get("/openapi.json", Some(&cookie)).await?;
    assert_eq!(status, 200, "{document}");
    for (schema, fields) in [
        (
            "ProvisioningSetBody",
            &[
                "hooks",
                "status_line",
                "settings",
                "resume",
                "context_window",
                "config_dir",
            ][..],
        ),
        (
            "ProvisioningVersionView",
            &[
                "hooks",
                "status_line",
                "settings",
                "resume",
                "context_window",
                "config_dir",
            ][..],
        ),
        (
            "SessionSettings",
            &["resume", "context_window", "config_dir"][..],
        ),
    ] {
        let shape = &document["components"]["schemas"][schema];
        let properties = shape["properties"]
            .as_object()
            .ok_or("missing schema properties")?;
        for &field in fields {
            assert!(properties.contains_key(field), "{schema}.{field}");
            assert!(
                !shape["required"]
                    .as_array()
                    .is_some_and(|required| required.iter().any(|name| name == field)),
                "{schema}.{field} is required"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn provisioning_omitted_seat_fields_remain_absent_after_reopen() -> TestResult {
    let (service, cookie, path) = table().await?;
    let (status, answer) = service
        .post(&path, Some(&cookie), &profile("claude-code")?)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    let store = ProvisioningStore::open(&service.dir.path().join("provisioning.json"))?;
    let agent = path.split('/').nth(2).ok_or("missing agent")?;
    let version = store
        .profile(agent)
        .ok_or("missing profile")?
        .versions
        .first()
        .ok_or("missing version")?;
    let stored = serde_json::to_value(&version.settings)?;
    for field in [
        "hooks",
        "status_line",
        "settings",
        "session",
        "resume",
        "context_window",
        "config_dir",
    ] {
        assert!(answer["profile"][field].is_null(), "{field}");
        assert!(stored.get(field).is_none(), "{field}");
    }
    Ok(())
}

#[tokio::test]
async fn provisioning_profile_seat_fields_survive_review_get_and_reopen() -> TestResult {
    let (service, cookie, path) = table().await?;
    let mut body = profile("claude-code")?;
    let seat = json!({"hooks":{"Stop":[{"command":"seat-stop", "matcher":"*", "timeout":5}]},
        "status_line":"seat-status", "settings":{"theme":"dark", "nested":{"a":1}},
        "resume":"019c6e27-e55b-73d1-87d8-4e01f1f75043",
        "context_window":200_000, "config_dir":"/tmp/chosen-config"});
    body.as_object_mut()
        .ok_or("profile is not an object")?
        .extend(seat.as_object().ok_or("seat is not an object")?.clone());
    let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
    assert_eq!(status, 200, "{answer}");
    let review = json!({"operation":OperationId::generate()?.to_string()});
    let (status, answer) = service
        .post(&format!("{path}/1/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert!(answer["profile"]["reviewed_by"].is_string());
    assert!(answer["profile"]["session"].is_null());
    let store = ProvisioningStore::open(&service.dir.path().join("provisioning.json"))?;
    let agent = path.split('/').nth(2).ok_or("missing agent")?;
    let version = store.version(agent, 1).ok_or("missing version")?;
    assert!(version.reviewed.is_some());
    let stored = serde_json::to_value(&version.settings)?;
    for field in lys_home::harness::launch_fields::SeatFields::NAMES {
        assert_eq!(answer["profile"][field], seat[field], "{field}");
        assert_eq!(stored[field], seat[field], "{field}");
    }
    Ok(())
}

#[test]
fn provisioning_old_shape_stored_version_has_six_absent_profile_fields() -> TestResult {
    let fixture = br#"{
        "profiles":[{"agent":"old-agent", "versions":[{
            "number":1, "operation":"old-operation", "set_by":"person", "set_at":1,
            "settings":{"model_access":[], "tools":[], "skills":[], "mcp_servers":[],
                "instructions":"", "note":"",
                "session":{"requires_controls":false, "compact":"/compact", "sensitive":false}}
        }]}]
    }"#;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("provisioning.json");
    std::fs::write(&file, fixture)?;
    let store = ProvisioningStore::open(&file)?;
    let settings = &store
        .version("old-agent", 1)
        .ok_or("old version absent")?
        .settings;
    assert_eq!(settings.hooks, None);
    assert_eq!(settings.status_line, None);
    assert_eq!(settings.settings, None);
    assert_eq!(settings.resume, None);
    assert_eq!(settings.context_window, None);
    assert_eq!(settings.config_dir, None);
    let session = settings.session.as_ref().ok_or("old session absent")?;
    assert_eq!(session.resume, None);
    assert_eq!(session.context_window, None);
    assert_eq!(session.config_dir, None);
    assert_eq!(std::fs::read(&file)?, fixture);
    Ok(())
}

#[tokio::test]
async fn provisioning_profile_session_values_are_validated_and_conflicts_refused() -> TestResult {
    let (service, cookie, path) = table().await?;
    for (field, value) in [
        ("resume", json!("not-an-id")),
        ("context_window", json!(0)),
        ("config_dir", json!("relative/config")),
    ] {
        let mut body = profile("codex")?;
        body[field] = value;
        let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
        assert_eq!(status, 400, "{answer}");
        assert!(
            answer["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(field)),
            "{answer}"
        );
    }
    for (field, profile_value, session_value) in [
        (
            "resume",
            json!("019c6e27-e55b-73d1-87d8-4e01f1f75043"),
            json!("019c6e27-e55b-73d1-87d8-4e01f1f75044"),
        ),
        ("context_window", json!(200_000), json!(100_000)),
        (
            "config_dir",
            json!("/tmp/profile-config"),
            json!("/tmp/session-config"),
        ),
    ] {
        let mut body = profile("codex")?;
        body[field] = profile_value;
        body["session"] = json!({field:session_value});
        let (status, answer) = service.post(&path, Some(&cookie), &body).await?;
        assert_eq!(status, 400, "{answer}");
        assert!(
            answer["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(field) && reason.contains("conflict")),
            "{answer}"
        );
    }
    Ok(())
}
