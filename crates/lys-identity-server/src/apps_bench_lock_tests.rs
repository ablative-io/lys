use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

use lys_identity::grants::{Action, Model, Relation};

use crate::grants::{GrantSetup, GrantState};

fn setup() -> Result<GrantSetup, Box<dyn Error>> {
    Ok(GrantSetup {
        log_dir: "unused".into(),
        log_origin: "unused".to_owned(),
        key_file: "unused".into(),
        model: RwLock::new(Model::new(
            1,
            [(Relation::new("viewer")?, [Action::new("read")?].into())],
        )?),
        spicedb: None,
        model_revision: AtomicU64::new(1),
        refresh: Mutex::new(()),
    })
}

#[test]
fn a_bench_network_call_does_not_hold_the_live_grants_lock() -> Result<(), Box<dyn Error>> {
    let grants: Mutex<Option<GrantState>> = Mutex::new(None);
    let setup = setup()?;
    super::with_engine(&grants, &setup, |_| {
        assert!(
            grants.try_lock().is_ok(),
            "bench I/O holds the live grants lock"
        );
        Ok(())
    })?;
    Ok(())
}

#[test]
fn a_bench_answer_is_refused_if_its_model_changed_during_the_call() -> Result<(), Box<dyn Error>> {
    let grants: Mutex<Option<GrantState>> = Mutex::new(None);
    let setup = setup()?;
    let result = super::with_engine(&grants, &setup, |_| {
        setup.model_revision.store(2, Ordering::Release);
        Ok(())
    });
    let error = result.err().ok_or("stale bench answer was admitted")?;
    assert_eq!(error.name(), "apps_unavailable");
    Ok(())
}
