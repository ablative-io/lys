//! Retained records are neither copied on writes nor visited by indexed reads.

use std::error::Error;

use serde_json::json;

use super::{ProvisioningStore, Review, SkillText, Version, index_work};

type TestResult = Result<(), Box<dyn Error>>;

fn version(agent: usize, number: u32, reviewed: bool) -> Result<Version, Box<dyn Error>> {
    Ok(serde_json::from_value(json!({
        "number": number, "operation": format!("record-{agent}-{number}"),
        "set_by": "operator", "set_at": 1,
        "reviewed": reviewed.then(|| json!({
            "operation": format!("review-{agent}-{number}"), "by": "reviewer", "at": 2
        })),
        "settings": {
            "model_access": [], "tools": [], "skills": [], "instructions": "", "note": "",
            "mcp_servers": [{"name": format!("server-{agent}-{number}"), "url": "https://tools.example.test/mcp"}]
        }
    }))?)
}

fn fixture() -> Result<(tempfile::TempDir, ProvisioningStore), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let profiles = (0..16)
        .map(|agent| {
            let versions = (1..=8)
                .map(|number| version(agent, number, true))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"agent": format!("agent-{agent}"), "versions": versions}))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let skills: Vec<_> = (0..16)
        .map(|number| {
            json!({
                "name": format!("skill-{number}"), "text": "retained text",
                "len": 13, "sha256": format!("digest-{number}")
            })
        })
        .collect();
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"profiles": profiles, "skills": skills}))?,
    )?;
    Ok((directory, ProvisioningStore::open(&path)?))
}

#[test]
fn provisioning_writes_do_not_copy_retained_profiles_or_skill_texts() -> TestResult {
    let (directory, mut store) = fixture()?;
    let before = index_work();
    store.set("agent-0", 8, version(0, 9, true)?)?;
    assert_eq!(
        store
            .latest_reviewed("agent-0")
            .ok_or("reviewed version missing")?
            .number,
        9
    );
    assert_eq!(
        index_work().0 - before.0,
        0,
        "approval copied retained profiles"
    );
    assert_eq!(
        index_work().1 - before.1,
        0,
        "approval copied retained skill texts"
    );
    store.set("agent-0", 9, version(0, 10, false)?)?;
    assert_eq!(
        store
            .latest_reviewed("agent-0")
            .ok_or("reviewed version missing")?
            .number,
        9
    );
    store.review(
        "agent-0",
        10,
        Review {
            operation: "review-0-10".to_owned(),
            by: "reviewer".to_owned(),
            at: 2,
        },
    )?;
    assert_eq!(
        store
            .latest_reviewed("agent-0")
            .ok_or("reviewed version missing")?
            .number,
        10
    );
    store.keep_skill(SkillText {
        name: "new-skill".to_owned(),
        text: "new text".to_owned(),
        len: 8,
        sha256: "new-digest".to_owned(),
    })?;
    assert_eq!(
        index_work().0 - before.0,
        0,
        "a write copied retained profiles"
    );
    assert_eq!(
        index_work().1 - before.1,
        0,
        "a write copied retained skill texts"
    );
    assert_eq!(
        index_work().2 - before.2,
        0,
        "a write rebuilt retained version history"
    );
    let reopened = ProvisioningStore::open(&directory.path().join("profiles.json"))?;
    assert_eq!(reopened.profiles(), store.profiles());
    assert_eq!(reopened.skills(), store.skills());
    assert!(reopened.named("record-0-10").is_some());
    assert!(reopened.skill("new-skill", "new-digest").is_some());
    Ok(())
}

#[test]
fn shared_indexes_clone_without_copying_and_keep_their_recorded_view() -> TestResult {
    let (directory, mut store) = fixture()?;
    let before = index_work();
    let shared = std::sync::Arc::clone(&store.indexes);
    let another = std::sync::Arc::clone(&shared);
    assert!(std::sync::Arc::ptr_eq(&store.indexes, &shared));
    assert!(std::sync::Arc::ptr_eq(&shared, &another));
    assert_eq!(index_work(), before);
    store.set("agent-0", 8, version(0, 9, true)?)?;
    assert!(store.named("record-0-9").is_some());
    assert!(!shared.operations.contains_key("record-0-9"));
    assert!(!std::sync::Arc::ptr_eq(&store.indexes, &shared));
    assert_eq!(
        index_work(),
        before,
        "a shared-index update copied retained records or rescanned history"
    );
    assert!(directory.path().join("profiles.json").exists());
    Ok(())
}

#[test]
fn provisioning_named_operation_does_not_visit_retained_history() -> TestResult {
    let (directory, store) = fixture()?;
    let before = index_work().2;
    let (agent, version) = store.named("record-15-8").ok_or("operation missing")?;
    assert_eq!(agent, "agent-15");
    assert_eq!(version.number, 8);
    assert!(store.named("missing-operation").is_none());
    assert_eq!(
        index_work().2 - before,
        0,
        "operation lookup scanned retained versions"
    );
    assert!(directory.path().join("profiles.json").exists());
    Ok(())
}

#[test]
fn a_numbered_profile_version_uses_the_index_after_open_and_write() -> TestResult {
    let (directory, mut store) = fixture()?;
    let before = index_work().2;
    assert_eq!(
        store.version("agent-15", 8).map(|version| version.number),
        Some(8)
    );
    assert!(store.version("agent-15", 99).is_none());
    assert!(store.version("missing", 1).is_none());
    assert_eq!(
        index_work().2 - before,
        0,
        "numbered version lookup scanned history"
    );
    store.set("agent-0", 8, version(0, 9, true)?)?;
    let reopened = ProvisioningStore::open(&directory.path().join("profiles.json"))?;
    let before = index_work().2;
    assert_eq!(store.version("agent-0", 9), reopened.version("agent-0", 9));
    assert_eq!(
        store.version("agent-0", 9).map(|version| version.number),
        Some(9)
    );
    assert_eq!(index_work().2 - before, 0);
    Ok(())
}

#[test]
fn provisioning_administrator_declaration_does_not_visit_retained_history() -> TestResult {
    let (directory, store) = fixture()?;
    let before = index_work().2;
    let server = store
        .declared_server("server-15-8")
        .ok_or("declaration missing")?;
    assert_eq!(server.url, "https://tools.example.test/mcp");
    assert!(store.declared_server("missing-server").is_none());
    assert_eq!(
        index_work().2 - before,
        0,
        "administrator declaration scanned retained versions"
    );
    assert!(directory.path().join("profiles.json").exists());
    Ok(())
}

#[test]
fn reviewing_an_earlier_declaration_keeps_profile_and_version_precedence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let mut early = version(0, 1, false)?;
    early.settings.mcp_servers[0].name = "shared".to_owned();
    early.settings.mcp_servers[0].url = "https://early.example.test/mcp".to_owned();
    let mut late = version(1, 1, true)?;
    late.settings.mcp_servers[0].name = "shared".to_owned();
    late.settings.mcp_servers[0].url = "https://late.example.test/mcp".to_owned();
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"profiles": [
            {"agent": "agent-0", "versions": [early]},
            {"agent": "agent-1", "versions": [late]}
        ]}))?,
    )?;
    let mut store = ProvisioningStore::open(&path)?;
    assert_eq!(
        store
            .declared_server("shared")
            .ok_or("late declaration missing")?
            .url,
        "https://late.example.test/mcp"
    );
    store.review(
        "agent-0",
        1,
        Review {
            operation: "review-0-1".to_owned(),
            by: "reviewer".to_owned(),
            at: 2,
        },
    )?;
    assert_eq!(
        store
            .declared_server("shared")
            .ok_or("early declaration missing")?
            .url,
        "https://early.example.test/mcp"
    );
    let bytes = std::fs::read(&path)?;
    store.review(
        "agent-0",
        1,
        Review {
            operation: "review-0-1".to_owned(),
            by: "reviewer".to_owned(),
            at: 3,
        },
    )?;
    assert_eq!(std::fs::read(&path)?, bytes);
    assert!(matches!(
        store.review(
            "agent-1",
            1,
            Review {
                operation: "review-0-1".to_owned(),
                by: "reviewer".to_owned(),
                at: 3,
            }
        ),
        Err(crate::error::ServerError::ProvisioningReused { .. })
    ));
    let reopened = ProvisioningStore::open(&path)?;
    assert_eq!(
        reopened.declared_server("shared"),
        store.declared_server("shared")
    );
    Ok(())
}

#[test]
fn failed_writes_rebuild_only_durable_records_and_retry_updates_reads() -> TestResult {
    let (directory, mut store) = fixture()?;
    let path = directory.path().join("profiles.json");
    let blocked = path.with_extension("writing");
    std::fs::create_dir(&blocked)?;
    assert!(matches!(
        store.set("agent-0", 8, version(0, 9, true)?),
        Err(crate::error::ServerError::ProvisioningUnavailable { .. })
    ));
    assert!(store.named("record-0-9").is_none());
    assert!(store.declared_server("server-0-9").is_none());
    assert_eq!(
        store.profile("agent-0").ok_or("profile missing")?.latest(),
        8
    );
    std::fs::remove_dir(&blocked)?;
    store.set("agent-0", 8, version(0, 9, true)?)?;
    assert!(store.named("record-0-9").is_some());
    assert!(store.declared_server("server-0-9").is_some());
    let reopened = ProvisioningStore::open(&path)?;
    assert_eq!(reopened.profiles(), store.profiles());
    assert_eq!(reopened.skills(), store.skills());
    Ok(())
}

#[test]
fn uncertain_reload_replaces_every_derived_lookup_from_durable_records() -> TestResult {
    let (directory, mut store) = fixture()?;
    let path = directory.path().join("profiles.json");
    let replacement = version(20, 1, true)?;
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "profiles": [{"agent": "replacement", "versions": [replacement]}],
            "skills": [{"name": "replacement", "text": "new", "len": 3, "sha256": "new-digest"}]
        }))?,
    )?;
    let bytes = std::fs::read(&path)?;
    store.uncertain = true;
    store.settle()?;
    assert!(store.named("record-15-8").is_none());
    assert!(store.declared_server("server-15-8").is_none());
    assert!(store.profile("agent-0").is_none());
    assert!(store.skill("skill-0", "digest-0").is_none());
    assert_eq!(
        store.named("record-20-1").ok_or("replacement missing")?.0,
        "replacement"
    );
    assert!(store.declared_server("server-20-1").is_some());
    assert!(store.skill("replacement", "new-digest").is_some());
    assert_eq!(
        store.pins(&["replacement".to_owned()])?[0].sha256,
        "new-digest"
    );
    assert_eq!(std::fs::read(&path)?, bytes);
    Ok(())
}
