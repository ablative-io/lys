//! Closed runtime sessions cannot retain a context maximum or a reporting gap.

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Held, Leaf, Measure, Usage};
use crate::runtime_state::{Report, Reported};
use crate::runtime_store::RuntimeStore;
use lys_core::Ed25519Identity;

fn closed_context(figure: Option<u64>) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut runtime = RuntimeStore::open(&dir.path().join("runtime"), Arc::clone(&key))?;
    let first = Report {
        operation: "start-old".to_owned(),
        session: "old".to_owned(),
        agent: Some("agent".to_owned()),
        machine: "machine".to_owned(),
        state: Reported::Starting,
        what: String::new(),
        confirmation: String::new(),
        reported_by: "person".to_owned(),
        at: 1,
        launch: None,
    };
    runtime.report(first.clone())?;
    runtime.report(Report {
        operation: "stop-old".to_owned(),
        state: Reported::Stopped,
        at: 2,
        confirmation: "the child exited".to_owned(),
        ..first.clone()
    })?;
    runtime.report(Report {
        operation: "start-live".to_owned(),
        session: "live".to_owned(),
        ..first
    })?;
    drop(runtime);
    let mut runtime = RuntimeStore::open(&dir.path().join("runtime"), key)?;
    let agents = BTreeSet::from(["agent".to_owned()]);
    let sessions = runtime.agents_with_sessions(&agents)?;
    let mut held = Held::default();
    for (session, context_percent) in [("old", figure), ("live", Some(40))] {
        held.hold(Leaf::Used(Usage {
            event: session.to_owned(),
            agent: "agent".to_owned(),
            session: Some(session.to_owned()),
            context_percent,
            at_ms: 1,
            ..Usage::default()
        }))?;
    }
    let restored = Held::decode(&held.encode()?)?;
    let used = super::current_usage(
        &restored,
        &Limit {
            unit: Measure::ContextPercent,
            amount: 80.into(),
            period: None,
            act: Act::Stop,
            zone: None,
        },
        &agents,
        "UTC",
        2,
        Some(&sessions),
    )?;
    assert_eq!(used.figure, Some(40.into()));
    assert_eq!(used.unavailable, None);
    Ok(())
}

#[test]
fn a_closed_session_does_not_pin_the_context_maximum_after_reopen() -> Result<(), Box<dyn Error>> {
    closed_context(Some(95))
}

#[test]
fn a_closed_session_does_not_pin_an_unavailable_context_after_reopen() -> Result<(), Box<dyn Error>>
{
    closed_context(None)
}
