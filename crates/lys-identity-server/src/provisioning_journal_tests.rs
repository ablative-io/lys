//! Durable changes append bounded bytes and legacy profiles remain readable.

use std::error::Error;

use serde_json::json;

use super::{ProvisioningStore, Review, SkillText, Version, edits, journal};
use crate::error::ServerError;

type TestResult = Result<(), Box<dyn Error>>;

fn version(operation: &str) -> Result<Version, Box<dyn Error>> {
    Ok(serde_json::from_value(json!({
        "number": 1, "operation": operation, "set_by": "operator", "set_at": 1,
        "settings": {
            "model_access": [], "tools": [], "skills": [],
            "instructions": "retained instructions", "note": "",
            "mcp_servers": [{"name": "server", "url": "https://tools.example.test/mcp"}]
        }
    }))?)
}

fn skill(name: &str) -> SkillText {
    SkillText {
        name: name.to_owned(),
        text: "retained skill".to_owned(),
        len: 14,
        sha256: format!("digest-{name}"),
    }
}

fn fixture(profiles: usize) -> Result<(tempfile::TempDir, ProvisioningStore), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let profiles = (0..profiles)
        .map(|position| {
            Ok(json!({
                "agent": format!("agent-{position}"),
                "versions": [version(&format!("original-{position}"))?]
            }))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"profiles": profiles, "skills": [skill("original")]}))?,
    )?;
    Ok((directory, ProvisioningStore::open(&path)?))
}

#[test]
fn appending_a_version_writes_the_same_bounded_bytes_with_large_history() -> TestResult {
    let mut sizes = Vec::new();
    for profiles in [1, 256] {
        let (directory, mut store) = fixture(profiles)?;
        store.keep_skill(skill("migration"))?;
        let path = directory.path().join("profiles.json");
        let before = std::fs::read(&path)?;
        store.set("agent-0", 1, version("next-version")?)?;
        let after = std::fs::read(&path)?;
        assert!(
            after.starts_with(&before),
            "retained snapshot was rewritten"
        );
        let appended = after.len() - before.len();
        assert!(appended < 1024, "a version appended {appended} bytes");
        sizes.push(appended);
        let reopened = ProvisioningStore::open(&path)?;
        assert_eq!(reopened.profiles(), store.profiles());
        assert_eq!(reopened.skills(), store.skills());
    }
    assert_eq!(sizes[0], sizes[1], "append cost grew with retained history");
    Ok(())
}

#[test]
fn an_old_install_migrates_with_reviews_skills_retries_and_checkpoint() -> TestResult {
    let (directory, mut store) = fixture(3)?;
    let path = directory.path().join("profiles.json");
    let original = std::fs::read(&path)?;
    assert!(store.version("agent-0", 1).is_some());
    assert_eq!(
        std::fs::read(&path)?,
        original,
        "open changed an old install"
    );
    store.set("agent-0", 1, version("next-version")?)?;
    store.keep_skill(skill("new"))?;
    store.review(
        "agent-0",
        2,
        Review {
            operation: "review-next".to_owned(),
            by: "reviewer".to_owned(),
            at: 2,
        },
    )?;
    let recorded = std::fs::read(&path)?;
    assert_eq!(store.set("agent-0", 1, version("next-version")?)?, 2);
    store.keep_skill(skill("new"))?;
    store.review(
        "agent-0",
        2,
        Review {
            operation: "review-next".to_owned(),
            by: "reviewer".to_owned(),
            at: 3,
        },
    )?;
    assert_eq!(
        std::fs::read(&path)?,
        recorded,
        "a retry appended a duplicate"
    );
    assert!(matches!(
        store.set("agent-0", 0, version("late-version")?),
        Err(ServerError::ProvisioningChanged { latest: 2 })
    ));
    let mut reused = version("next-version")?;
    reused.settings.note = "different".to_owned();
    assert!(matches!(
        store.set("agent-0", 1, reused),
        Err(ServerError::ProvisioningReused { .. })
    ));
    let reopened = ProvisioningStore::open(&path)?;
    assert_eq!(reopened.profiles(), store.profiles());
    assert_eq!(reopened.skills(), store.skills());
    assert_eq!(
        reopened
            .named("next-version")
            .map(|(_, version)| version.number),
        Some(2)
    );
    assert!(reopened.declared_server("server").is_some());
    assert!(reopened.skill("original", "digest-original").is_some());
    assert!(reopened.skill("new", "digest-new").is_some());
    store.checkpoint()?;
    let checkpointed = ProvisioningStore::open(&path)?;
    assert_eq!(checkpointed.profiles(), store.profiles());
    assert_eq!(checkpointed.skills(), store.skills());
    store.set("agent-0", 2, version("after-checkpoint")?)?;
    let reopened = ProvisioningStore::open(&path)?;
    assert!(reopened.named("after-checkpoint").is_some());
    let entries = std::fs::read_dir(directory.path())?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries.len(), 1, "migration left another store file");
    Ok(())
}

#[test]
fn torn_final_changes_are_durably_truncated_and_complete_corruption_is_refused() -> TestResult {
    let (directory, mut store) = fixture(1)?;
    store.keep_skill(skill("migration"))?;
    let path = directory.path().join("profiles.json");
    let boundary = std::fs::metadata(&path)?.len();
    store.set("agent-0", 1, version("next-version")?)?;
    let complete = std::fs::read(&path)?;
    let boundary = usize::try_from(boundary)?;
    for length in [boundary + 1, boundary + 39, complete.len() - 1] {
        std::fs::write(&path, &complete[..length])?;
        let before = journal::RECOVERED_TAILS.with(std::cell::Cell::get);
        let mut reopened = ProvisioningStore::open(&path)?;
        assert!(reopened.named("next-version").is_none());
        assert_eq!(std::fs::read(&path)?, complete[..boundary]);
        assert_eq!(
            journal::RECOVERED_TAILS.with(std::cell::Cell::get) - before,
            1
        );
        reopened.set("agent-0", 1, version("next-version")?)?;
        let reopened = ProvisioningStore::open(&path)?;
        assert!(reopened.named("next-version").is_some());
    }
    for length in [9, boundary - 1] {
        std::fs::write(&path, &complete[..length])?;
        assert!(matches!(
            ProvisioningStore::open(&path),
            Err(ServerError::ProvisioningUnavailable { reason }) if reason.contains("incomplete provisioning frame")
        ));
        assert_eq!(std::fs::read(&path)?, complete[..length]);
    }
    let mut corrupt = complete;
    let last = corrupt.last_mut().ok_or("no retained frame")?;
    *last ^= 1;
    std::fs::write(&path, &corrupt)?;
    assert!(matches!(
        ProvisioningStore::open(&path),
        Err(ServerError::ProvisioningUnavailable { reason }) if reason.contains("checksum mismatch")
    ));
    assert_eq!(std::fs::read(&path)?, corrupt);
    Ok(())
}

fn seeded_changes(changes: u64) -> Result<(tempfile::TempDir, ProvisioningStore), Box<dyn Error>> {
    let (directory, store) = fixture(1)?;
    let path = directory.path().join("profiles.json");
    journal::snapshot(&path, &serde_json::to_vec(&store.kept)?)?;
    let mut bytes = std::fs::read(&path)?;
    for number in 0..changes {
        bytes.extend_from_slice(&journal::encode_change(&edits::Edit::Skill(skill(
            &format!("seed-{number}"),
        )))?);
    }
    std::fs::write(&path, &bytes)?;
    Ok((directory, ProvisioningStore::open(&path)?))
}

#[test]
fn periodic_checkpoints_bound_actual_restart_change_replay() -> TestResult {
    let every = lys_identity::SNAPSHOT_EVERY.get();
    let (directory, mut store) = seeded_changes(every - 1)?;
    assert_eq!(store.since_snapshot, every - 1);
    store.keep_skill(skill("boundary"))?;
    assert_eq!(store.since_snapshot, 0);
    store.keep_skill(skill("tail"))?;
    let before = journal::REPLAYED_CHANGES.with(std::cell::Cell::get);
    let reopened = ProvisioningStore::open(&directory.path().join("profiles.json"))?;
    let replayed = journal::REPLAYED_CHANGES.with(std::cell::Cell::get) - before;
    assert_eq!(
        replayed, 1,
        "restart replayed changes before the checkpoint"
    );
    assert!(replayed <= every);
    assert!(u64::try_from(reopened.skills().len())? > every);
    assert_eq!(reopened.profiles(), store.profiles());
    assert_eq!(reopened.skills(), store.skills());
    Ok(())
}

#[test]
fn recovering_a_torn_tail_near_the_checkpoint_boundary_folds_each_change_once() -> TestResult {
    let every = lys_identity::SNAPSHOT_EVERY.get();
    let (directory, store) = seeded_changes(every - 1)?;
    let path = directory.path().join("profiles.json");
    let complete = std::fs::read(&path)?;
    let torn = journal::encode_change(&edits::Edit::Skill(skill("torn")))?;
    let mut bytes = complete.clone();
    bytes.extend_from_slice(&torn[..torn.len() - 1]);
    std::fs::write(&path, &bytes)?;
    let before = journal::REPLAYED_CHANGES.with(std::cell::Cell::get);
    let mut reopened = ProvisioningStore::open(&path)?;
    let replayed = journal::REPLAYED_CHANGES.with(std::cell::Cell::get) - before;
    assert_eq!(
        replayed,
        every - 1,
        "recovery folded the valid prefix twice"
    );
    assert!(replayed <= every);
    assert_eq!(std::fs::read(&path)?, complete);
    assert_eq!(reopened.skills(), store.skills());
    assert!(reopened.skill("torn", "digest-torn").is_none());
    reopened.keep_skill(skill("after-recovery"))?;
    assert_eq!(reopened.since_snapshot, 0);
    let checkpointed = ProvisioningStore::open(&path)?;
    assert!(
        checkpointed
            .skill("after-recovery", "digest-after-recovery")
            .is_some()
    );
    Ok(())
}

#[test]
fn a_crash_at_the_checkpoint_boundary_cannot_append_past_the_replay_bound() -> TestResult {
    let every = lys_identity::SNAPSHOT_EVERY.get();
    let (directory, mut store) = seeded_changes(every)?;
    assert_eq!(store.since_snapshot, every);
    let path = directory.path().join("profiles.json");
    let recorded = std::fs::read(&path)?;
    let blocked = path.with_extension("writing");
    std::fs::create_dir(&blocked)?;
    assert!(matches!(
        store.keep_skill(skill("after-boundary")),
        Err(ServerError::ProvisioningUnavailable { reason }) if reason.contains("provisioning checkpoint")
    ));
    assert_eq!(
        std::fs::read(&path)?,
        recorded,
        "failed checkpoint appended another change"
    );
    assert!(
        store
            .skill("after-boundary", "digest-after-boundary")
            .is_none()
    );
    std::fs::remove_dir(&blocked)?;
    store.keep_skill(skill("after-boundary"))?;
    assert_eq!(store.since_snapshot, 1);
    let before = journal::REPLAYED_CHANGES.with(std::cell::Cell::get);
    let reopened = ProvisioningStore::open(&path)?;
    assert_eq!(
        journal::REPLAYED_CHANGES.with(std::cell::Cell::get) - before,
        1
    );
    assert_eq!(reopened.skills(), store.skills());
    Ok(())
}

#[test]
fn uncertain_recovery_cannot_answer_a_retry_until_file_and_parent_are_synced() -> TestResult {
    for parent in [false, true] {
        let (directory, mut store) = fixture(1)?;
        let path = directory.path().join("profiles.json");
        let mut writer = ProvisioningStore::open(&path)?;
        writer.set("agent-0", 1, version("next-version")?)?;
        let recorded = std::fs::read(&path)?;
        store.uncertain = true;
        journal::fail_settle_sync(parent);
        assert!(matches!(
            store.set("agent-0", 1, version("next-version")?),
            Err(ServerError::ProvisioningUnavailable { reason }) if reason.contains("settle sync failure")
        ));
        assert!(store.uncertain, "failed sync cleared uncertainty");
        assert!(
            store.named("next-version").is_none(),
            "failed sync installed recovered memory"
        );
        assert_eq!(store.set("agent-0", 1, version("next-version")?)?, 2);
        assert!(!store.uncertain);
        assert!(store.named("next-version").is_some());
        assert_eq!(
            std::fs::read(&path)?,
            recorded,
            "retry appended a duplicate"
        );
    }
    Ok(())
}
