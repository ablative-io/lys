//! The reviewed-build index follows confirmed changes and durable recovery.

use std::error::Error;

use serde_json::{Value, json};

use super::{ProvisioningStore, Review, Version, builds};

type TestResult = Result<(), Box<dyn Error>>;
const CONTRACT: &str = "codex/template-v1";

fn version(operation: &str, program: &str, reviewed: bool) -> Result<Version, Box<dyn Error>> {
    let source: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    Ok(serde_json::from_value(json!({
        "number": 1, "operation": operation, "set_by": "operator", "set_at": 1,
        "reviewed": reviewed.then(|| json!({"operation": format!("review-{operation}"), "by": "reviewer", "at": 2})),
        "settings": {
            "model_access": ["model"], "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "",
            "harness": {"name": "Reviewed build", "program": program, "package": "build", "description": source["description"]}
        }
    }))?)
}

fn programs(store: &ProvisioningStore) -> Vec<String> {
    store
        .reviewed_builds(CONTRACT)
        .map(|build| build.program.clone())
        .collect()
}

#[test]
fn confirmed_record_and_review_extend_the_index_once_and_reopen_keeps_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let mut store = ProvisioningStore::open(&path)?;
    let before = builds::index_visits();
    let recorded = version("record", "/opt/first", false)?;
    store.set("agent", 0, recorded.clone())?;
    assert!(programs(&store).is_empty());
    let review = Review {
        operation: "review".to_owned(),
        by: "reviewer".to_owned(),
        at: 2,
    };
    store.review("agent", 1, review.clone())?;
    let bytes = std::fs::read(&path)?;
    store.review("agent", 1, review)?;
    store.set("agent", 0, recorded)?;
    assert_eq!(std::fs::read(&path)?, bytes);
    assert_eq!(programs(&store), ["/opt/first"]);
    store.set("agent", 1, version("second", "/opt/second", true)?)?;
    assert_eq!(programs(&store), ["/opt/first", "/opt/second"]);
    store.settle()?;
    assert_eq!(
        builds::index_visits() - before,
        0,
        "ordinary changes rebuilt retained history"
    );
    drop(store);
    let bytes = std::fs::read(&path)?;
    let reopened = ProvisioningStore::open(&path)?;
    assert_eq!(builds::index_visits() - before, 2);
    assert_eq!(programs(&reopened), ["/opt/first", "/opt/second"]);
    assert_eq!(std::fs::read(&path)?, bytes);
    Ok(())
}

#[test]
fn an_unconfirmed_write_never_publishes_the_proposed_build() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let mut store = ProvisioningStore::open(&path)?;
    store.set("agent", 0, version("first", "/opt/first", true)?)?;
    let blocked = path.with_extension("writing");
    std::fs::create_dir(&blocked)?;
    let proposed = version("second", "/opt/second", true)?;
    assert!(matches!(
        store.set("agent", 1, proposed.clone()),
        Err(crate::error::ServerError::ProvisioningUnavailable { .. })
    ));
    assert_eq!(programs(&store), ["/opt/first"]);
    std::fs::remove_dir(blocked)?;
    store.set("agent", 1, proposed)?;
    assert_eq!(programs(&store), ["/opt/first", "/opt/second"]);
    Ok(())
}

#[test]
fn uncertain_reload_rebuilds_the_index_from_the_durable_estate() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let mut store = ProvisioningStore::open(&path)?;
    store.set("agent", 0, version("first", "/opt/first", true)?)?;
    let recovered = version("recovered", "/opt/recovered", true)?;
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"profiles": [{"agent": "agent", "versions": [recovered]}]}))?,
    )?;
    store.uncertain = true;
    store.settle()?;
    assert_eq!(programs(&store), ["/opt/recovered"]);
    assert!(!store.uncertain);
    Ok(())
}

#[test]
fn a_failed_review_does_not_publish_an_unconfirmed_build() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiles.json");
    let mut store = ProvisioningStore::open(&path)?;
    store.set("agent", 0, version("first", "/opt/first", false)?)?;
    let blocked = path.with_extension("writing");
    std::fs::create_dir(&blocked)?;
    let review = Review {
        operation: "review".to_owned(),
        by: "reviewer".to_owned(),
        at: 2,
    };
    assert!(matches!(
        store.review("agent", 1, review.clone()),
        Err(crate::error::ServerError::ProvisioningUnavailable { .. })
    ));
    assert!(programs(&store).is_empty());
    std::fs::remove_dir(blocked)?;
    store.review("agent", 1, review)?;
    assert_eq!(programs(&store), ["/opt/first"]);
    Ok(())
}
