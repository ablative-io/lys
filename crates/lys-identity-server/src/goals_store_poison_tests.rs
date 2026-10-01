use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use axum::http::StatusCode;
use lys_core::Ed25519Identity;

use super::{GoalStore, Goals};

#[test]
fn an_interrupted_goal_store_refuses_before_running_the_requested_act() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &temporary.path().join("key"),
    )?);
    let goals = Goals::new(GoalStore::open(&temporary.path().join("goals"), key)?);
    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        if let Ok(mut store) = goals.store.lock() {
            store.uncertain = true;
            panic!("goal change interrupted");
        }
    }));
    assert!(interrupted.is_err());
    let mut acted = false;
    let refusal = goals
        .with(|_| {
            acted = true;
            Ok(())
        })
        .err()
        .ok_or("poisoned goal store admitted an act")?;
    assert!(!acted);
    assert_eq!(refusal.name(), "goals_unavailable");
    assert_eq!(refusal.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(goals.with(|_| Ok(())).is_err());
    Ok(())
}
