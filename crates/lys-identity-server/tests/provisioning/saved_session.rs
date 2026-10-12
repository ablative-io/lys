//! Saved session views retain the explicit allowance and its migration log.

use super::*;

#[tokio::test]
async fn instructions_mode_read_view_preserves_a_saved_session() -> TestResult {
    let heard = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let lines = std::sync::Arc::clone(&heard);
    lys_identity_server::warn_lines::install(std::sync::Arc::new(move |line| {
        match lines.lock() {
            Ok(mut lines) => lines.push(line.to_owned()),
            Err(error) => panic!("fixture log poisoned: {error}"),
        }
    }))?;
    let mut table = Table::set().await?;
    let path = table.route(1);
    let mut body = profile(&operation()?, 0, "Saved session settings.");
    let session = json!({"compact": "/compact", "message_prefix": "/message", "sensitive": true});
    body["session"] = session.clone();
    let (status, recorded) = table.service.post(&path, Some(&table.ada), &body).await?;
    assert_eq!(status, 200, "{recorded}");
    let (status, read) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{read}");
    let mut expected = session.clone();
    expected["stall_after"] = json!(300);
    assert_eq!(read["profile"]["session"], expected);
    assert_eq!(recorded["profile"]["session"], expected);
    table.service.restart().await?;
    let (status, reopened) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(reopened["profile"]["session"], expected);
    let old_path = table.service.dir.path().join("old-provisioning.json");
    let mut old_version = serde_json::to_value(version("old-session", "Saved session settings."))?;
    old_version["settings"]["session"] = session;
    let original = serde_json::to_vec_pretty(&json!({"profiles":[{
        "agent":table.seeded.people[1].agents[0].id.to_string(), "versions":[old_version]
    }]}))?;
    std::fs::write(&old_path, &original)?;
    table
        .service
        .restart_adjusted(|config| config.provisioning_file = Some(old_path.clone()))
        .await?;
    let (status, migrated) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{migrated}");
    assert_eq!(migrated["profile"]["session"], expected);
    let backups = std::fs::read_dir(table.service.dir.path())?
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
    assert_ne!(std::fs::read(&old_path)?, original);
    table.service.restart().await?;
    let (status, reopened) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(reopened["profile"]["session"], expected);
    assert_eq!(std::fs::read(backups[0].path())?, original);
    let lines = heard.lock().map_err(|error| error.to_string())?;
    let migrations = lines
        .iter()
        .filter(|line| line.contains("profile_stall_after_recorded"))
        .collect::<Vec<_>>();
    assert_eq!(migrations.len(), 2, "{lines:?}");
    assert!(migrations[0].contains("provisioning wrote session.stall_after=300"));
    assert!(migrations[1].contains("provisioning migration wrote session.stall_after=300"));
    assert!(migrations[1].contains(&backups[0].path().display().to_string()));
    Ok(())
}
