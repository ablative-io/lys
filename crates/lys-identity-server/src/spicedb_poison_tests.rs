use super::{GrantError, SpiceDb};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::sync::{Arc, Mutex};

#[test]
fn poisoned_app_kinds_cannot_produce_or_replace_a_permission_schema() -> Result<(), Box<dyn Error>>
{
    let engine = SpiceDb {
        endpoint: "127.0.0.1:1".to_owned(),
        key: Arc::from("fixture"),
        mirror: "fixture".to_owned(),
        relations: BTreeMap::new(),
        app_kinds: Mutex::default(),
        scope: None,
    };
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = engine
            .app_kinds
            .lock()
            .expect("fixture lock poisoned before injection");
        assert!(held.is_empty());
        panic!("app kind state failure");
    }));
    assert!(poisoned.is_err());
    for refusal in [
        engine.schema(&BTreeSet::new()).map(drop),
        engine.set_app_kinds(&BTreeMap::new()),
        engine.write_schema(&BTreeSet::new()),
        engine.schema_writer().map(drop),
    ] {
        match refusal {
            Err(GrantError::PermissionEngineUnavailable { reason }) => {
                assert!(reason.contains("app kinds unavailable"));
            }
            other => {
                return Err(format!("poisoned app kinds did not refuse schema: {other:?}").into());
            }
        }
    }
    Ok(())
}
