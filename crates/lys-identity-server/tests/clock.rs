//! Supplied session time, checked failure and unchanged stored session bytes.

use std::error::Error;
use std::sync::Arc;

use identity_contract::harness::ManualClock;
use lys_core::clock::{Clock, ClockSource};
use lys_identity::{Actor, AuthMethod, LoginBinding, Provenance};
use lys_identity_server::error::ServerError;
use lys_identity_server::session::Sessions;

type TestResult = Result<(), Box<dyn Error>>;
const T0: i64 = 1_700_000_000;

fn actor() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(LoginBinding::new("https://issuer.example.test", "subject")?, Provenance::new(AuthMethod::Oidc, 90)))
}

fn source(clock: &Arc<ManualClock>) -> ClockSource {
    let provider: Arc<dyn Clock> = clock.clone();
    ClockSource::Supplied(provider)
}

#[test]
fn session_clock_is_shared_and_expiry_at_equality_refuses() -> TestResult {
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::new_with_clock(10, false, source(&clock));
    let cookie = sessions.begin(actor()?)?;
    assert_eq!(clock.reads(), 1, "begin samples once and passes time through pruning");
    let entry = sessions.session(Some(&cookie))?;
    assert_eq!(entry.started_at, u64::try_from(T0)?);
    assert_eq!(entry.ends_at, u64::try_from(T0 + 10)?);
    clock.set(T0 + 9);
    assert!(sessions.is_live(&entry.id)?);
    assert_eq!(sessions.current(Some(&cookie))?, entry.id);
    clock.set(T0 + 10);
    assert!(!sessions.is_live(&entry.id)?);
    assert!(matches!(sessions.session(Some(&cookie)), Err(ServerError::NotSignedIn)));
    assert!(sessions.live(|_| true)?.is_empty());
    let reads = clock.reads();
    assert!(!sessions.is_live("unknown")?);
    assert!(matches!(sessions.session(Some("lys_directory_session=unknown")), Err(ServerError::NotSignedIn)));
    assert_eq!(clock.reads(), reads, "missing indexed entries need no clock read");
    Ok(())
}

#[test]
fn separate_session_owners_do_not_share_time() -> TestResult {
    let first = Arc::new(ManualClock::new(T0));
    let second = Arc::new(ManualClock::new(T0 + 100));
    let a = Sessions::new_with_clock(10, false, source(&first));
    let b = Sessions::new_with_clock(10, false, source(&second));
    let ac = a.begin(actor()?)?;
    let bc = b.begin(actor()?)?;
    first.set(T0 + 10);
    assert!(matches!(a.session(Some(&ac)), Err(ServerError::NotSignedIn)));
    assert_eq!(b.session(Some(&bc))?.started_at, u64::try_from(T0 + 100)?);
    Ok(())
}

#[test]
fn clock_conversion_refuses_before_session_mutation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::open_with_clock(path.clone(), 10, false, source(&clock))?;
    for at in [T0, -1] {
        clock.set(at);
        clock.refuse(at == T0);
        assert!(matches!(sessions.begin(actor()?), Err(ServerError::ClockUnavailable { .. })));
        assert!(!path.exists(), "a refused reading cannot write a session file");
    }
    clock.refuse(false);
    clock.set(T0);
    assert!(sessions.live(|_| true)?.is_empty());
    Ok(())
}

#[test]
fn failed_clock_does_not_prune_or_rewrite_kept_sessions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::open_with_clock(path.clone(), 10, false, source(&clock))?;
    let cookie = sessions.begin(actor()?)?;
    let before = std::fs::read(&path)?;
    clock.refuse(true);
    assert!(matches!(sessions.live(|_| true), Err(ServerError::ClockUnavailable { .. })));
    assert!(matches!(sessions.end_matching(|_| true), Err(ServerError::ClockUnavailable { .. })));
    assert_eq!(std::fs::read(&path)?, before);
    clock.refuse(false);
    assert_eq!(sessions.session(Some(&cookie))?.started_at, u64::try_from(T0)?);
    Ok(())
}

#[test]
fn stored_sessions_reopen_byte_equal_under_production_defaults() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let clock = Arc::new(ManualClock::new(T0));
    let sessions = Sessions::open_with_clock(path.clone(), 4_102_444_800, false, source(&clock))?;
    let cookie = sessions.begin(actor()?)?;
    let entry = sessions.session(Some(&cookie))?;
    let before = std::fs::read(&path)?;
    drop(sessions);
    let reopened = Sessions::open(path.clone(), 600, false)?;
    assert_eq!(reopened.session(Some(&cookie))?, entry);
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}

#[test]
fn clock_failure_has_its_own_wire_tag_and_status() {
    use axum::response::IntoResponse;
    let error = ServerError::ClockUnavailable { reason: "provider read refused".to_owned() };
    assert_eq!(error.name(), "ClockUnavailable");
    assert!(error.to_string().contains("provider read refused"));
    assert_eq!(error.into_response().status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn failed_clock_refuses_before_opening_stored_sessions() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("sessions.json");
    let before = b"unreadable session bytes";
    std::fs::write(&path, before)?;
    let clock = Arc::new(ManualClock::new(-1));
    let opened = Sessions::open_with_clock(path.clone(), 10, false, source(&clock));
    assert!(matches!(opened, Err(ServerError::ClockUnavailable { .. })));
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}
