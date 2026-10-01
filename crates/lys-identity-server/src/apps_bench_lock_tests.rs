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
    super::with_engine(&Mutex::new(()), &grants, &setup, |_| {
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
    let result = super::with_engine(&Mutex::new(()), &grants, &setup, |_| {
        setup.model_revision.store(2, Ordering::Release);
        Ok(())
    });
    let error = result.err().ok_or("stale bench answer was admitted")?;
    assert_eq!(error.name(), "apps_unavailable");
    Ok(())
}

#[test]
fn concurrent_bench_questions_refuse_without_waiting_and_retry_after_release()
-> Result<(), Box<dyn Error>> {
    let grants: Mutex<Option<GrantState>> = Mutex::new(None);
    let setup = setup()?;
    let asking = Mutex::new(());
    super::with_engine(&asking, &grants, &setup, |_| {
        let error = super::with_engine(&asking, &grants, &setup, |_| Ok(()))
            .err()
            .ok_or_else(|| super::unavailable("overlapping scratch questions admitted"))?;
        assert_eq!(error.name(), "apps_unavailable");
        Ok(())
    })?;
    super::with_engine(&asking, &grants, &setup, |_| Ok(()))?;
    Ok(())
}

#[test]
fn a_failed_question_releases_both_locks_and_preserves_the_refusal() -> Result<(), Box<dyn Error>> {
    let grants: Mutex<Option<GrantState>> = Mutex::new(None);
    let setup = setup()?;
    let asking = Mutex::new(());
    let result: Result<(), _> = super::with_engine(&asking, &grants, &setup, |_| {
        Err(super::unavailable("engine call failed"))
    });
    let error = result.err().ok_or("failed question answered")?;
    assert!(error.to_string().contains("engine call failed"));
    assert!(asking.try_lock().is_ok());
    assert!(grants.try_lock().is_ok());
    super::with_engine(&asking, &grants, &setup, |_| Ok(()))?;
    Ok(())
}

#[test]
fn opening_live_grants_during_a_question_invalidates_its_snapshot() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let setup = setup()?;
    let log = temporary.path().join("grants");
    lys_log_store::FileLeafStore::create(&log, "bench-test")?;
    let live = lys_identity::grants::Grants::open(
        Box::new(move || lys_log_store::FileLeafStore::open(&log)),
        lys_core::Ed25519Identity::load_or_generate(&temporary.path().join("key"))?,
        crate::spicedb::Relationships::Memory(lys_identity::grants::MemoryRelationships::default()),
        setup.model()?,
        lys_identity::PersonId::generate()?,
    )?;
    let grants: Mutex<Option<GrantState>> = Mutex::new(None);
    let result = super::with_engine(&Mutex::new(()), &grants, &setup, |_| {
        *grants
            .lock()
            .map_err(|error| super::unavailable(error.to_string()))? = Some(live);
        Ok(())
    });
    let error = result.err().ok_or("changed grants were admitted")?;
    assert_eq!(error.name(), "apps_unavailable");
    Ok(())
}
