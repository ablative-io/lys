#![cfg(test)]
//! Old profiles retain their settings and preserve original bytes before migration.
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
    let original = old_session_bytes();
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("provisioning.json");
    std::fs::write(&path, original)?;
    let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let lines = std::sync::Arc::clone(&heard);
    let subscriber =
        crate::warn_lines::WarnLines::new(std::sync::Arc::new(move |line| match lines.lock() {
            Ok(mut lines) => lines.push(line.to_owned()),
            Err(error) => panic!("fixture log poisoned: {error}"),
        }));
    let mut store =
        tracing::subscriber::with_default(subscriber, || ProvisioningStore::open(&path))?;
    let version = &store
        .profile("ordinary-agent")
        .ok_or("old profile missing")?
        .versions[0];
    assert_eq!(
        serde_json::to_value(&version.settings.session)?["requires_controls"],
        serde_json::Value::Null
    );
    assert_eq!(
        version
            .settings
            .session
            .as_ref()
            .ok_or("old session missing")?
            .stall_after,
        300
    );
    let backups = std::fs::read_dir(directory.path())?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("provisioning.before-stall-")
        })
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    assert_eq!(std::fs::read(backups[0].path())?, original);
    let migrated = std::fs::read(&path)?;
    let mut expected: serde_json::Value = serde_json::from_slice(original)?;
    expected["profiles"][0]["versions"][0]["settings"]["session"]["stall_after"] = 300.into();
    assert_eq!(serde_json::to_value(read(&path)?)?, expected);
    store.write(read(&path)?)?;
    assert_ne!(migrated, original);
    assert_eq!(
        std::fs::read(&path)?,
        serde_json::to_vec_pretty(&read(&path)?)?
    );
    assert_eq!(serde_json::to_value(read(&path)?)?, expected);
    let lines = heard.lock().map_err(|error| error.to_string())?;
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("profile_stall_after_recorded"));
    assert!(lines[0].contains("provisioning migration wrote session.stall_after=300"));
    assert!(lines[0].contains(&backups[0].path().display().to_string()));
    assert_eq!(std::fs::read(backups[0].path())?, original);
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
        r#"{"number":2,"operation":"required-controls","settings":{"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":"","session":{"stall_after":300,"message_prefix":"","sensitive":false}},"set_by":"ordinary-person","set_at":2}"#,
    )?)
}

#[test]
fn provisioning_old_shape_version_keeps_all_six_seat_fields_absent()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = br#"{
        "number":1, "operation":"old-operation", "set_by":"person", "set_at":1,
        "settings":{
            "model_access":[], "tools":[], "skills":[], "mcp_servers":[],
            "instructions":"", "note":"",
            "session":{"stall_after":300,"requires_controls":false, "compact":"/compact", "sensitive":false}
        }
    }"#;
    let version: super::Version = serde_json::from_slice(fixture)?;
    assert_eq!(version.settings.hooks, None);
    assert_eq!(version.settings.status_line, None);
    assert_eq!(version.settings.settings, None);
    let session = version.settings.session.ok_or("old session absent")?;
    assert_eq!(session.resume, None);
    assert_eq!(session.context_window, None);
    assert_eq!(session.config_dir, None);
    Ok(())
}

fn old_session_bytes() -> &'static [u8] {
    br#"{
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
}"#
}
