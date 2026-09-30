//! Context gaps replace earlier figures without growing with report count.

use std::collections::BTreeSet;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Held, Leaf, Measure, Usage};

use super::{Availability, FUTURE, NO_CONTEXT, NO_SESSION};

fn usage(event: &str, session: Option<&str>, figure: Option<u64>, at_ms: i64) -> Usage {
    Usage {
        event: event.to_owned(),
        agent: "agent".to_owned(),
        at_ms,
        session: session.map(str::to_owned),
        context_percent: figure,
        ..Usage::default()
    }
}

fn limit() -> Limit {
    Limit {
        unit: Measure::ContextPercent,
        amount: 80.into(),
        period: None,
        act: Act::Tell,
        zone: None,
    }
}

fn agents() -> BTreeSet<String> {
    BTreeSet::from(["agent".to_owned()])
}

#[test]
fn a_missing_session_reading_hides_the_maximum_until_that_session_reports_again()
-> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("one", Some("first"), Some(90), 1))?;
    held.keep(&usage("two", Some("second"), Some(60), 2))?;
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX).figure,
        Some(90.into())
    );
    held.keep(&usage("three", Some("first"), None, 3))?;
    let gap = held.used(&limit(), &agents(), i64::MAX);
    assert_eq!(gap.figure, None);
    assert_eq!(gap.unavailable.as_deref(), Some(NO_CONTEXT));
    held.keep(&usage("four", Some("first"), Some(40), 4))?;
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX).figure,
        Some(60.into())
    );
    assert_eq!(held.used(&limit(), &agents(), i64::MAX).unavailable, None);
    Ok(())
}

#[test]
fn an_older_reading_cannot_replace_a_gap_and_equal_instants_keep_the_last_report()
-> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("one", Some("session"), None, 3))?;
    held.keep(&usage("two", Some("session"), Some(70), 2))?;
    assert_eq!(held.used(&limit(), &agents(), i64::MAX).figure, None);
    held.keep(&usage("three", Some("session"), Some(50), 3))?;
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX).figure,
        Some(50.into())
    );
    held.keep(&usage("four", None, None, 4))?;
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX)
            .unavailable
            .as_deref(),
        Some(NO_SESSION)
    );
    Ok(())
}

#[test]
fn availability_is_restored_from_existing_usage_without_adding_snapshot_fields()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(usage("one", Some("session"), Some(60), 1)))?;
    held.hold(Leaf::Used(usage("two", Some("session"), None, 2)))?;
    let encoded = held.encode()?;
    let restored = Held::decode(&encoded)?;
    assert_eq!(restored.encode()?, encoded);
    assert_eq!(
        restored
            .context_availability
            .used(&limit(), &agents(), i64::MAX)
            .figure,
        None
    );
    assert_eq!(
        restored
            .context_availability
            .used(&limit(), &agents(), i64::MAX)
            .unavailable
            .as_deref(),
        Some(NO_CONTEXT)
    );
    assert_eq!(restored, held);
    Ok(())
}

#[test]
fn repeated_reports_keep_one_reading_per_session() -> Result<(), String> {
    let mut held = Availability::default();
    for index in 0..4096 {
        held.keep(&usage(&index.to_string(), Some("session"), Some(60), index))?;
    }
    assert_eq!(held.agents.len(), 1);
    let agent = held.agents.get("agent").ok_or("missing agent")?;
    assert_eq!(agent.sessions.len(), 1);
    assert_eq!(agent.figures.len(), 1);
    assert_eq!(agent.figures.get(&60), Some(&1));
    assert!(agent.missing.is_empty());
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX).figure,
        Some(60.into())
    );
    Ok(())
}

#[test]
fn an_unmeasured_crossing_needs_a_reason_and_cannot_request_another_action()
-> Result<(), serde_json::Error> {
    let mut crossing = serde_json::json!({
        "operation": "operation", "holder": {"kind": "agent", "id": "agent"},
        "measure": "context_percent", "version": 1, "limit": 80,
        "act": "stop", "agent": "agent", "session": "session", "text": null, "at_ms": 1
    });
    let absent: crate::budgets_crossing::Crossing = serde_json::from_value(crossing.clone())?;
    assert!(absent.checked().is_err());
    crossing["unavailable"] = serde_json::json!(NO_CONTEXT);
    let stopped: crate::budgets_crossing::Crossing = serde_json::from_value(crossing.clone())?;
    assert!(stopped.checked().is_ok());
    crossing["act"] = serde_json::json!("notice");
    let notice: crate::budgets_crossing::Crossing = serde_json::from_value(crossing.clone())?;
    assert!(notice.checked().is_err());
    crossing["act"] = serde_json::json!("stop");
    crossing["figure"] = serde_json::json!(0);
    let conflicting: crate::budgets_crossing::Crossing = serde_json::from_value(crossing)?;
    assert!(conflicting.checked().is_err());
    Ok(())
}

#[test]
fn future_reports_name_a_gap_until_the_observation_time_reaches_them() -> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("one", Some("session"), Some(60), 1))?;
    held.keep(&usage("two", Some("session"), Some(10), 3))?;
    let future = held.used(&limit(), &agents(), 2);
    assert_eq!(future.figure, None);
    assert_eq!(future.unavailable.as_deref(), Some(FUTURE));
    let current = held.used(&limit(), &agents(), 3);
    assert_eq!(current.figure, Some(10.into()));
    assert_eq!(current.unavailable, None);
    Ok(())
}
