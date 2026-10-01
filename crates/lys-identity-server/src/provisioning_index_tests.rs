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
    store.review(
        "agent-0",
        10,
        Review {
            operation: "review-0-10".to_owned(),
            by: "reviewer".to_owned(),
            at: 2,
        },
    )?;
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
    let reopened = ProvisioningStore::open(&directory.path().join("profiles.json"))?;
    assert_eq!(reopened.profiles(), store.profiles());
    assert_eq!(reopened.skills(), store.skills());
    assert!(reopened.named("record-0-10").is_some());
    assert!(reopened.skill("new-skill", "new-digest").is_some());
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
