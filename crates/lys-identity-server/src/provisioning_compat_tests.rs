#![cfg(test)]
//! An ordinary pre-description profile must survive a read and write byte for byte.
use super::{ProvisioningStore, read};

#[test]
fn an_old_profile_rewritten_without_edits_keeps_its_exact_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let original = br#"{
  "profiles": [
    {
      "agent": "ordinary-agent",
      "versions": [
        {
          "number": 1,
          "operation": "recorded-operation",
          "settings": {
            "model_access": [
              "model-a"
            ],
            "tools": [],
            "skills": [],
            "mcp_servers": [
              {
                "name": "ordinary-server",
                "url": "https://example.invalid/mcp"
              }
            ],
            "instructions": "ordinary instructions",
            "note": ""
          },
          "set_by": "ordinary-person",
          "set_at": 1
        }
      ]
    }
  ]
}"#;
    let directory = tempfile::tempdir()?;
    let file = directory.path().join("provisioning.json");
    std::fs::write(&file, original)?;
    let mut store = ProvisioningStore::open(&file)?;
    let version = store
        .profile("ordinary-agent")
        .ok_or("no migrated profile")?
        .versions
        .first()
        .ok_or("no migrated version")?;
    assert_eq!(
        version.settings.instructions_mode,
        lys_home::harness::launch_fields::InstructionsMode::Append
    );
    assert_eq!(version.settings.instructions, "ordinary instructions");
    store.write(read(&file)?)?;
    assert_eq!(std::fs::read(&file)?, original);
    let reopened = ProvisioningStore::open(&file)?;
    assert_eq!(reopened.profiles(), store.profiles());
    Ok(())
}

#[test]
fn an_old_session_profile_stays_manual_and_retains_its_exact_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let original = br#"{
  "profiles": [
    {
      "agent": "ordinary-agent",
      "versions": [
        {
          "number": 1,
          "operation": "recorded-operation",
          "settings": {
            "model_access": [],
            "tools": [],
            "skills": [],
            "mcp_servers": [],
            "instructions": "",
            "note": "",
            "session": {
              "message_prefix": "legacy prefix",
              "sensitive": false
            }
          },
          "set_by": "ordinary-person",
          "set_at": 1
        }
      ]
    }
  ]
}"#;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("provisioning.json");
    std::fs::write(&path, original)?;
    let mut store = ProvisioningStore::open(&path)?;
    let version = &store
        .profile("ordinary-agent")
        .ok_or("old profile missing")?
        .versions[0];
    assert_eq!(
        serde_json::to_value(&version.settings.session)?["requires_controls"],
        serde_json::Value::Null
    );
    store.write(read(&path)?)?;
    assert_eq!(std::fs::read(&path)?, original);
    let mut explicit = version_fixture()?;
    explicit["settings"]["session"]["requires_controls"] = serde_json::json!(true);
    let version: super::Version = serde_json::from_value(explicit)?;
    store.set("ordinary-agent", 1, version)?;
    let reopened = ProvisioningStore::open(&path)?;
    let current = reopened
        .profile("ordinary-agent")
        .ok_or("updated profile missing")?
        .versions
        .last()
        .ok_or("updated version missing")?;
    assert_eq!(
        serde_json::to_value(&current.settings.session)?["requires_controls"],
        true
    );
    Ok(())
}

fn version_fixture() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(
        r#"{"number":2,"operation":"required-controls","settings":{"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":"","session":{"message_prefix":"","sensitive":false}},"set_by":"ordinary-person","set_at":2}"#,
    )?)
}
