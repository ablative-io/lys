//! Profile placement and writable authority belong to a reviewed version.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::provisioning_store::{ProvisioningStore, Version};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const OLD_STORE: &str = r#"{
  "profiles": [
    {
      "agent": "agent-a",
      "versions": [
        {
          "number": 1,
          "operation": "old-operation",
          "settings": {
            "model_access": [],
            "tools": [],
            "skills": [],
            "mcp_servers": [],
            "instructions": "kept instructions",
            "note": "kept note"
          },
          "set_by": "person-a",
          "set_at": 1
        }
      ]
    }
  ]
}"#;

fn body(machine: &str, folder: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "from_version": 0,
        "model_access": [],
        "tools": [],
        "skills": [],
        "mcp_servers": [],
        "instructions": "",
        "note": "",
        "runs_on": machine,
        "writable": folder,
    }))
}

#[test]
fn an_old_store_opens_without_placement_or_writable_authority() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("provisioning.json");
    std::fs::write(&path, OLD_STORE)?;
    let mut store = ProvisioningStore::open(&path)?;
    let old = store
        .profile("agent-a")
        .and_then(|profile| profile.versions.first())
        .ok_or("old version is absent")?
        .clone();
    let settings = serde_json::to_value(&old.settings)?;
    assert!(settings.get("runs_on").is_none());
    assert!(settings.get("writable").is_none());
    assert_eq!(settings["instructions"], "kept instructions");
    assert_eq!(settings["note"], "kept note");
    assert_eq!(std::fs::read_to_string(&path)?, OLD_STORE);
    let next = Version {
        operation: "next-operation".to_owned(),
        ..old.clone()
    };
    assert_eq!(store.set("agent-a", 1, next)?, 2);
    drop(store);
    let reopened = ProvisioningStore::open(&path)?;
    let versions = &reopened
        .profile("agent-a")
        .ok_or("profile absent")?
        .versions;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0], old);
    for version in versions {
        let settings = serde_json::to_value(&version.settings)?;
        assert!(settings.get("runs_on").is_none());
        assert!(settings.get("writable").is_none());
    }
    Ok(())
}

#[test]
fn placement_and_folder_survive_a_store_reopen_without_changing_old_versions() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("provisioning.json");
    std::fs::write(&path, OLD_STORE)?;
    let mut store = ProvisioningStore::open(&path)?;
    let old = store
        .profile("agent-a")
        .ok_or("old profile absent")?
        .versions[0]
        .clone();
    let mut next = serde_json::to_value(&old)?;
    let machine = OperationId::generate()?.to_string();
    next["operation"] = json!("placed-operation");
    next["settings"]["runs_on"] = json!(machine);
    next["settings"]["writable"] = json!("/seats/workspace");
    assert_eq!(store.set("agent-a", 1, serde_json::from_value(next)?)?, 2);
    drop(store);
    let reopened = ProvisioningStore::open(&path)?;
    let versions = &reopened
        .profile("agent-a")
        .ok_or("profile absent")?
        .versions;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0], old);
    let settings = serde_json::to_value(&versions[1].settings)?;
    assert_eq!(settings["runs_on"], machine);
    assert_eq!(settings["writable"], "/seats/workspace");
    assert!(versions[1].reviewed.is_none());
    Ok(())
}

#[tokio::test]
async fn profile_routes_keep_placement_and_folder_in_the_reviewed_version() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let route = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    let machine = OperationId::generate()?.to_string();
    let given = body(&machine, "/seats/workspace")?;
    let (status, answer) = service.post(&route, Some(&cookie), &given).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["profile"]["runs_on"], machine);
    assert_eq!(answer["profile"]["writable"], "/seats/workspace");
    assert!(answer["profile"]["reviewed_by"].is_null());
    let review = json!({ "operation": OperationId::generate()?.to_string() });
    let (status, answer) = service
        .post(&format!("{route}/1/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["profile"]["runs_on"], machine);
    assert_eq!(answer["profile"]["writable"], "/seats/workspace");
    assert!(answer["profile"]["reviewed_by"].is_string());
    let (status, seen) = service.get(&route, Some(&cookie)).await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(seen["profile"], answer["profile"]);
    Ok(())
}

#[tokio::test]
async fn invalid_placement_or_folder_is_refused_before_a_version_is_written() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let route = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    let machine = OperationId::generate()?.to_string();
    for (member, invalid) in [
        ("runs_on", ""),
        ("runs_on", "not-a-machine-id"),
        ("writable", ""),
        ("writable", "relative"),
        ("writable", "/"),
        ("writable", "/seats/../outside"),
        ("writable", "/seats//workspace"),
        ("writable", "/seats/./workspace"),
        ("writable", "/seats/workspace/"),
        ("writable", "/seats/work\0space"),
    ] {
        let mut given = body(&machine, "/seats/workspace")?;
        given[member] = json!(invalid);
        let (status, answer) = service.post(&route, Some(&cookie), &given).await?;
        assert_eq!(status, 400, "{member}: {answer}");
        assert_eq!(answer["refusal"], "ProvisioningMalformed", "{answer}");
        let (status, seen) = service.get(&route, Some(&cookie)).await?;
        assert_eq!(status, 200, "{seen}");
        assert!(seen["profile"].is_null(), "{seen}");
    }
    Ok(())
}
