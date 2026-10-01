use super::{AppStore, ORIGIN, SCHEMA_READS};
use crate::apps_bench_scratch::memory::MemoryStore;
use crate::apps_state::{Applied, Approved, By, Client, Decided, Line, Registered};
use lys_core::Ed25519Identity;
use serde_json::json;
use std::error::Error;
use std::sync::Arc;

#[test]
fn an_append_reparses_only_its_app_and_reopen_rebuilds_the_cache() -> Result<(), Box<dyn Error>> {
    let leaves = MemoryStore::empty();
    let shared = Arc::clone(&leaves);
    let key = Arc::new(Ed25519Identity::ephemeral());
    let mut store = AppStore::over(
        Box::new(move || MemoryStore::open(Arc::clone(&shared), ORIGIN)),
        Arc::clone(&key),
    )?;
    let schema = |id: &str| json!({"kinds":{format!("{id}.doc"):{"actions":["read"],"relations":{"viewer":["read"]}}}});
    for index in 0..32 {
        let id = format!("app_{index:03}");
        store.keep(Line::Registered(Registered {
            operation: format!("register-{id}"),
            app: id.clone(),
            name: id.clone(),
            redirects: vec![],
            schema: schema(&id),
            service_account: None,
            by: By::Start,
            at: 1,
        }))?;
        store.keep(Line::Approved(Approved {
            operation: format!("approve-{id}"),
            app: id.clone(),
            client: Client {
                client_id: id,
                secret_sha256: String::new(),
            },
            binding: None,
            by: By::Start,
            at: 1,
        }))?;
    }
    let before = SCHEMA_READS.with(std::cell::Cell::get);
    store.keep(Line::Applied(Applied {
        operation: "update".to_owned(),
        app: "app_017".to_owned(),
        version: 2,
        schema: schema("app_017"),
        proposal: None,
        by: By::Start,
        at: 2,
    }))?;
    assert_eq!(SCHEMA_READS.with(std::cell::Cell::get) - before, 1);
    store.keep(Line::Retired(Decided {
        operation: "retire".to_owned(),
        app: "app_017".to_owned(),
        reason: "finished".to_owned(),
        by: By::Start,
        at: 3,
    }))?;
    assert_eq!(SCHEMA_READS.with(std::cell::Cell::get) - before, 1);
    assert!(store.schema("app_017").is_none());
    assert!(store.schema("app_018").is_some());
    let reopened = AppStore::over(
        Box::new(move || MemoryStore::open(Arc::clone(&leaves), ORIGIN)),
        key,
    )?;
    assert_eq!(reopened.schemas, store.schemas);
    assert_eq!(reopened.model_revision(), store.model_revision());
    Ok(())
}
