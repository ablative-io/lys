//! Context readings survive unreported calls with their original provenance.

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
fn a_report_without_context_keeps_the_sessions_last_reading_and_only_a_never_read_session_is_missing()
-> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("one", Some("first"), Some(90), 1))?;
    held.keep(&usage("two", Some("second"), Some(60), 2))?;
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX).figure,
        Some(90.into())
    );
    // An incomplete call reports no context: the first session's 90 stands.
    held.keep(&usage("three", Some("first"), None, 3))?;
    let kept = held.used(&limit(), &agents(), i64::MAX);
    assert_eq!(kept.figure, Some(90.into()));
    assert_eq!(kept.unavailable, None);
    assert_eq!(held.reading("agent", "first"), Some((1, Some(90))));
    held.keep(&usage("four", Some("first"), Some(40), 4))?;
    assert_eq!(
        held.used(&limit(), &agents(), i64::MAX).figure,
        Some(60.into())
    );
    // A session that has never reported a context is the gap it always was.
    held.keep(&usage("five", Some("third"), None, 5))?;
    let gap = held.used(&limit(), &agents(), i64::MAX);
    assert_eq!(gap.figure, None);
    assert_eq!(gap.unavailable.as_deref(), Some(NO_CONTEXT));
    held.keep(&usage("six", Some("third"), Some(10), 6))?;
    assert_eq!(held.used(&limit(), &agents(), i64::MAX).unavailable, None);
    Ok(())
}

#[test]
fn a_late_reading_restores_a_never_read_session_and_equal_instants_keep_the_last_reading()
-> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("one", Some("session"), None, 3))?;
    held.keep(&usage("two", Some("session"), Some(70), 2))?;
    let restored = held.used(&limit(), &agents(), i64::MAX);
    assert_eq!(restored.figure, Some(70.into()));
    assert_eq!(
        restored.context.ok_or("missing provenance")?.read_at_ms,
        Some(2)
    );
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
fn replay_retains_the_last_reported_reading_through_a_gap_without_adding_snapshot_fields()
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
        Some(60.into())
    );
    assert_eq!(
        restored
            .context_availability
            .used(&limit(), &agents(), i64::MAX)
            .unavailable
            .as_deref(),
        None
    );
    let context = restored
        .context_availability
        .used(&limit(), &agents(), i64::MAX)
        .context
        .ok_or("missing restored provenance")?;
    assert_eq!(context.read_at_ms, Some(1));
    assert_eq!(context.reported_none, 0);
    assert_eq!(restored, held);
    Ok(())
}

fn call(event: &str, figure: Option<u64>, at_ms: i64) -> Usage {
    let mut usage = usage(event, Some("session"), figure, at_ms);
    usage.call = Some(Box::new(crate::budgets_state::CallSeen {
        id: event.to_owned(),
        model: Some("claude-opus-5-5".to_owned()),
        input_tokens: None,
        output_tokens: None,
        cache_creation_tokens: None,
        cache_read_tokens: None,
        run: "run".to_owned(),
        record: None,
    }));
    usage
}

#[test]
fn later_empty_calls_keep_the_read_time_and_only_calls_after_that_reading_are_counted()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", Some(5), 1)))?;
    held.hold(Leaf::Used(call("empty", None, 5)))?;
    held.hold(Leaf::Used(call("empty-too", None, 6)))?;
    held.hold(Leaf::Used(usage("snapshot", Some("session"), None, 7)))?;
    held.hold(Leaf::Used(call("empty", None, 5)))?;
    let used = held.context_availability.used(&limit(), &agents(), 7);
    assert_eq!(used.figure, Some(5.into()));
    let context = used.context.ok_or("missing provenance")?;
    assert_eq!(context.read_at_ms, Some(1));
    assert_eq!(context.reported_none, 2);
    assert_eq!(context.model.as_deref(), Some("claude-opus-5-5"));
    held.hold(Leaf::Used(call("late-reading", Some(6), 4)))?;
    let used = held.context_availability.used(&limit(), &agents(), 7);
    assert_eq!(used.figure, Some(6.into()));
    let context = used.context.ok_or("missing late provenance")?;
    assert_eq!(context.read_at_ms, Some(4));
    assert_eq!(context.reported_none, 2);
    held.hold(Leaf::Used(call("fresh", Some(7), 8)))?;
    held.hold(Leaf::Used(call("older-empty", None, 2)))?;
    let context = held
        .context_availability
        .used(&limit(), &agents(), 8)
        .context
        .ok_or("missing fresh provenance")?;
    assert_eq!(context.read_at_ms, Some(8));
    assert_eq!(context.reported_none, 0);
    let restored = Held::decode(&held.encode()?)?;
    assert_eq!(restored, held);
    Ok(())
}

#[test]
fn a_never_read_models_undeclared_window_keeps_its_model_and_named_reason() -> Result<(), String> {
    let mut held = Availability::default();
    let mut unknown = call("empty", None, 1);
    unknown.unavailable.push(lys_runner::tracking::Unavailable {
        figure: "context_percent".to_owned(),
        reason: "no context window is declared for model claude-opus-5-5".to_owned(),
    });
    held.keep(&unknown)?;
    let used = held.used(&limit(), &agents(), 1);
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("no context window is declared for model claude-opus-5-5")
    );
    let context = used.context.ok_or("missing undeclared provenance")?;
    assert_eq!(context.read_at_ms, None);
    assert_eq!(context.model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(context.reported_none, 1);
    Ok(())
}

#[test]
fn ending_a_session_retains_other_sessions_latest_empty_call_for_unnamed_report_ordering()
-> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("read", Some("live"), Some(5), 1))?;
    held.keep(&usage("other", Some("ended"), Some(10), 2))?;
    held.keep(&usage("unnamed", None, None, 3))?;
    held.keep(&usage("gap", Some("live"), None, 4))?;
    held.remove("agent", "ended")?;
    assert_eq!(held.used(&limit(), &agents(), 4).figure, Some(5.into()));
    let future = held.used(&limit(), &agents(), 3);
    assert_eq!(future.figure, None);
    assert_eq!(future.unavailable.as_deref(), Some(FUTURE));
    Ok(())
}

#[test]
fn empty_calls_preserve_the_winning_context_source_and_time_through_ties_and_replacement()
-> Result<(), String> {
    let mut held = Availability::default();
    for (event, session, at, figure, source, expected, declared, read_at) in [
        (
            "first",
            "one",
            1,
            Some(60),
            Some("profile"),
            60,
            "profile",
            1,
        ),
        ("tie", "two", 2, Some(60), Some("model"), 60, "profile", 1),
        ("lower", "one", 3, Some(40), Some("profile"), 60, "model", 2),
        ("gap", "two", 4, None, None, 60, "model", 2),
        (
            "restore",
            "two",
            5,
            Some(20),
            Some("model"),
            40,
            "profile",
            3,
        ),
        ("older", "two", 1, Some(90), Some("model"), 40, "profile", 3),
    ] {
        let mut report = usage(event, Some(session), figure, at);
        report.context_source = source.map(str::to_owned);
        held.keep(&report)?;
        let used = held.used(&limit(), &agents(), 10);
        assert_eq!(used.figure, Some(expected.into()), "{event}");
        assert_eq!(used.source.as_deref(), Some(declared), "{event}");
        assert_eq!(
            used.context.ok_or("missing winning provenance")?.read_at_ms,
            Some(read_at),
            "{event}"
        );
    }
    held.remove("agent", "one")?;
    let used = held.used(&limit(), &agents(), 10);
    assert_eq!(used.figure, Some(20.into()));
    assert_eq!(used.source.as_deref(), Some("model"));
    held.remove("agent", "two")?;
    let used = held.used(&limit(), &agents(), 10);
    assert_eq!(used.figure, None);
    assert_eq!(used.source, None);
    assert_eq!(used.context, None);
    Ok(())
}

#[test]
fn equal_time_empty_calls_follow_arrival_order_and_a_new_reading_resets_their_count()
-> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&call("read", Some(60), 5))?;
    held.keep(&call("empty", None, 5))?;
    let used = held.used(&limit(), &agents(), 5);
    assert_eq!(used.figure, Some(60.into()));
    let context = used.context.ok_or("missing same-time provenance")?;
    assert_eq!(context.read_at_ms, Some(5));
    assert_eq!(context.reported_none, 1);
    held.keep(&call("new-reading", Some(40), 5))?;
    let used = held.used(&limit(), &agents(), 5);
    assert_eq!(used.figure, Some(40.into()));
    assert_eq!(
        used.context
            .ok_or("missing replaced provenance")?
            .reported_none,
        0
    );
    held.keep(&call("older", Some(90), 4))?;
    assert_eq!(held.used(&limit(), &agents(), 5).figure, Some(40.into()));
    Ok(())
}

#[test]
fn live_context_retains_its_own_read_time_and_empty_call_count_without_a_closed_sessions_maximum()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::runtime_state::{Report, Reported};
    use crate::runtime_store::RuntimeStore;
    use std::sync::Arc;
    let dir = tempfile::tempdir()?;
    let key = Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("key"),
    )?);
    let mut runtime = RuntimeStore::open(&dir.path().join("runtime"), key)?;
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
        session: "session".to_owned(),
        ..first
    })?;
    let sessions = runtime.agents_with_sessions(&agents())?;
    let mut held = Availability::default();
    held.keep(&usage("old", Some("old"), Some(95), 1))?;
    held.keep(&call("read", Some(5), 2))?;
    held.keep(&call("empty", None, 3))?;
    let used = held.used_live(&limit(), &agents(), 3, Some(&sessions));
    assert_eq!(used.figure, Some(5.into()));
    assert_eq!(used.unavailable, None);
    let context = used.context.ok_or("missing live provenance")?;
    assert_eq!(context.read_at_ms, Some(2));
    assert_eq!(context.reported_none, 1);
    assert_eq!(context.model.as_deref(), Some("claude-opus-5-5"));
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

#[test]
fn a_confirmed_end_removes_its_reading_and_gap_from_the_heap() -> Result<(), String> {
    let mut held = Availability::default();
    held.keep(&usage("one", Some("old"), Some(95), 1))?;
    held.keep(&usage("two", Some("live"), Some(40), 2))?;
    held.remove("agent", "old")?;
    let agent = held.agents.get("agent").ok_or("missing agent")?;
    assert_eq!(agent.sessions.len(), 1);
    assert!(!agent.figures.contains_key(&95));
    assert_eq!(held.used(&limit(), &agents(), 3).figure, Some(40.into()));
    held.keep(&usage("three", Some("old"), None, 3))?;
    held.remove("agent", "old")?;
    assert!(
        held.agents
            .get("agent")
            .ok_or("missing agent")?
            .missing
            .is_empty()
    );
    assert_eq!(held.used(&limit(), &agents(), 4).figure, Some(40.into()));
    Ok(())
}
