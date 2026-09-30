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
