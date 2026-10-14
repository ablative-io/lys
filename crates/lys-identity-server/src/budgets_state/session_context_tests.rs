//! Display context follows runtime session starts and complete reported input counters.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, CallSeen, Held, Leaf, Measure, Usage};

use super::Windows;

pub(super) const MODEL: &str = "claude-opus-5-5";

pub(super) fn limit() -> Limit {
    Limit {
        unit: Measure::ContextPercent,
        amount: 80.into(),
        period: None,
        act: Act::Tell,
        zone: None,
    }
}

pub(super) fn live(sessions: &[(&str, u64)]) -> BTreeMap<String, BTreeMap<String, u64>> {
    BTreeMap::from([(
        "agent".to_owned(),
        sessions
            .iter()
            .map(|(session, start)| ((*session).to_owned(), *start))
            .collect(),
    )])
}

pub(super) fn windows() -> Windows {
    BTreeMap::from([(MODEL.to_owned(), Some(1_000_000))])
}

pub(super) fn call(event: &str, session: &str, at_ms: i64, input: Option<u64>) -> Usage {
    Usage {
        event: event.to_owned(),
        agent: "agent".to_owned(),
        session: Some(session.to_owned()),
        at_ms,
        call: Some(Box::new(CallSeen {
            id: event.to_owned(),
            model: Some(MODEL.to_owned()),
            input_tokens: input,
            output_tokens: Some(500),
            cache_creation_tokens: Some(0),
            cache_read_tokens: Some(0),
            run: "run".to_owned(),
            record: None,
        })),
        ..Usage::default()
    }
}

#[test]
fn reported_input_and_cached_input_produce_5_8795_percent_of_the_declared_million_token_window()
-> Result<(), Box<dyn std::error::Error>> {
    let mut held = Held::default();
    let mut usage = call("reading", "session", 10_000, Some(8_795));
    let counters = usage.call.as_mut().ok_or("missing call")?;
    counters.cache_creation_tokens = Some(10_000);
    counters.cache_read_tokens = Some(40_000);
    counters.output_tokens = Some(300_000);
    held.hold(Leaf::Used(usage))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)?;
    assert_eq!(
        serde_json::to_value(&used)?["figure"],
        serde_json::json!(5.8795)
    );
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.session.as_deref(), Some("session"));
    assert_eq!(context.context_tokens, Some(58_795));
    assert_eq!(context.window_tokens, Some(1_000_000));
    assert_eq!(context.read_at_ms, Some(10_000));
    assert_eq!(context.live_sessions.len(), 1);
    Ok(())
}

#[test]
fn newest_runtime_session_wins_even_when_an_older_session_reports_a_higher_percentage_later()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("new-low", "new", 20_000, Some(50_000))))?;
    held.hold(Leaf::Used(call("old-high", "old", 30_000, Some(990_000))))?;
    let used = held.session_context(
        &limit(),
        &live(&[("old", 1), ("new", 2)]),
        &windows(),
        40_000,
    )?;
    assert_eq!(used.figure, Some(5.into()));
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.session.as_deref(), Some("new"));
    assert_eq!(context.live_sessions.len(), 2);
    assert_eq!(context.live_sessions[0].session, "new");
    assert_eq!(context.live_sessions[1].figure, Some(99.into()));
    Ok(())
}

#[test]
fn newest_live_session_that_never_reported_does_not_inherit_an_older_sessions_reading()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("old-read", "old", 30_000, Some(990_000))))?;
    let used = held.session_context(
        &limit(),
        &live(&[("old", 1), ("new", 2)]),
        &windows(),
        40_000,
    )?;
    assert_eq!(used.figure, None);
    assert_eq!(used.unavailable.as_deref(), Some(super::NO_CONTEXT));
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.session.as_deref(), Some("new"));
    assert_eq!(context.read_at_ms, None);
    assert_eq!(context.context_tokens, None);
    assert_eq!(context.reported_none, 0);
    assert_eq!(context.live_sessions[1].figure, Some(99.into()));
    Ok(())
}

#[test]
fn later_incomplete_calls_keep_the_same_sessions_reading_time_and_count_without_counting_snapshots()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(50_000))))?;
    held.hold(Leaf::Used(call("empty", "session", 20_000, None)))?;
    let mut empty_cache = call("empty-cache", "session", 30_000, Some(40_000));
    empty_cache
        .call
        .as_mut()
        .ok_or("missing call")?
        .cache_read_tokens = None;
    held.hold(Leaf::Used(empty_cache))?;
    let mut snapshot = call("snapshot", "session", 40_000, None);
    snapshot.call = None;
    snapshot.native_snapshot = true;
    held.hold(Leaf::Used(snapshot))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 50_000)?;
    assert_eq!(used.figure, Some(5.into()));
    assert_eq!(used.unavailable, None);
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.read_at_ms, Some(10_000));
    assert_eq!(context.reported_none, 2);
    Ok(())
}

#[test]
fn late_reported_calls_follow_observation_order_and_equal_instants_follow_the_later_record()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("empty", "session", 30_000, None)))?;
    held.hold(Leaf::Used(call(
        "late-read",
        "session",
        20_000,
        Some(50_000),
    )))?;
    held.hold(Leaf::Used(call(
        "same-instant-read",
        "session",
        30_000,
        Some(60_000),
    )))?;
    held.hold(Leaf::Used(call(
        "same-instant-empty",
        "session",
        30_000,
        None,
    )))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 40_000)?;
    assert_eq!(used.figure, Some(6.into()));
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.read_at_ms, Some(30_000));
    assert_eq!(context.reported_none, 1);
    Ok(())
}

#[test]
fn future_indexed_calls_do_not_replace_or_add_missing_calls_to_the_observed_reading()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(50_000))))?;
    held.hold(Leaf::Used(call(
        "future-read",
        "session",
        30_000,
        Some(990_000),
    )))?;
    held.hold(Leaf::Used(call("future-empty", "session", 40_000, None)))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)?;
    assert_eq!(used.figure, Some(5.into()));
    assert_eq!(used.context.ok_or("missing context")?.reported_none, 0);
    Ok(())
}

#[test]
fn a_model_without_a_declared_window_keeps_its_name_and_raw_context_without_guessing_a_percentage()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(58_795))))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &Windows::new(), 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(used.source, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("no context window is declared for model claude-opus-5-5")
    );
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.model.as_deref(), Some(MODEL));
    assert_eq!(context.context_tokens, Some(58_795));
    assert_eq!(context.window_tokens, None);
    assert_eq!(context.read_at_ms, Some(10_000));
    Ok(())
}

#[test]
fn a_profile_sourced_call_names_the_unretained_declaration_instead_of_using_an_organisation_window()
-> Result<(), String> {
    let mut held = Held::default();
    let mut usage = call("read", "session", 10_000, Some(58_795));
    usage.context_source = Some("profile".to_owned());
    usage.context_percent = Some(90);
    held.hold(Leaf::Used(usage))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(used.source, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("the call's profile context window declaration was not retained")
    );
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.context_tokens, Some(58_795));
    assert_eq!(context.window_tokens, None);
    Ok(())
}

#[test]
fn missing_cache_counters_are_named_and_counted_without_inventing_zero() -> Result<(), String> {
    let mut held = Held::default();
    let mut usage = call("empty", "session", 10_000, Some(58_795));
    usage
        .call
        .as_mut()
        .ok_or("missing call")?
        .cache_creation_tokens = None;
    held.hold(Leaf::Used(usage))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("context counters not reported: cache_creation_tokens")
    );
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.context_tokens, None);
    assert_eq!(context.reported_none, 1);
    Ok(())
}

#[test]
fn current_declarations_recompute_raw_context_and_side_work_never_supplies_an_agent_context_reading()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(50_000))))?;
    let changed = BTreeMap::from([(MODEL.to_owned(), Some(500_000))]);
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &changed, 20_000)?;
    assert_eq!(used.figure, Some(10.into()));
    let side_work = BTreeMap::from([(MODEL.to_owned(), None)]);
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &side_work, 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("the call's model is declared as side work, which never sets an agent's context")
    );
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.read_at_ms, None);
    assert_eq!(context.context_tokens, None);
    assert_eq!(context.reported_none, 1);
    Ok(())
}

#[test]
fn context_counter_overflow_and_zero_windows_refuse_with_their_reason() -> Result<(), String> {
    let mut held = Held::default();
    let mut overflow = call("overflow", "session", 10_000, Some(u64::MAX));
    overflow
        .call
        .as_mut()
        .ok_or("missing call")?
        .cache_read_tokens = Some(1);
    held.hold(Leaf::Used(overflow))?;
    assert_eq!(
        held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)
            .err()
            .as_deref(),
        Some("reported context input and cache token counters overflow")
    );
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(50_000))))?;
    let zero = BTreeMap::from([(MODEL.to_owned(), Some(0))]);
    assert_eq!(
        held.session_context(&limit(), &live(&[("session", 1)]), &zero, 20_000)
            .err()
            .as_deref(),
        Some("model_windows must declare positive tokens")
    );
    let too_small = BTreeMap::from([(MODEL.to_owned(), Some(40_000))]);
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &too_small, 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some(
            "the call's context of 50000 tokens is larger than the window of 40000 declared for its model"
        )
    );
    Ok(())
}

#[test]
fn missing_indexed_records_refuse_but_unattributed_history_does_not_poison_live_context()
-> Result<(), String> {
    let mut held = Held::default();
    let usage = call("read", "session", 10_000, Some(50_000));
    held.index = Arc::new(crate::budgets_index::Index::from_uses(&[usage])?);
    assert_eq!(
        held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)
            .err()
            .as_deref(),
        Some("context call index names missing record 0")
    );
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "", 10_000, Some(50_000))))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(used.unavailable.as_deref(), Some(super::NO_CONTEXT));
    Ok(())
}

#[test]
fn equal_runtime_starts_choose_the_greatest_session_id_and_ended_sessions_are_not_listed()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("ended", "ended", 30_000, Some(990_000))))?;
    held.hold(Leaf::Used(call("a", "a", 10_000, Some(50_000))))?;
    held.hold(Leaf::Used(call("z", "z", 20_000, Some(60_000))))?;
    let used = held.session_context(&limit(), &live(&[("a", 1), ("z", 1)]), &windows(), 40_000)?;
    assert_eq!(used.figure, Some(6.into()));
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.session.as_deref(), Some("z"));
    assert_eq!(context.live_sessions.len(), 2);
    assert!(
        context
            .live_sessions
            .iter()
            .all(|session| session.session != "ended")
    );
    Ok(())
}

#[test]
fn context_display_replays_existing_calls_and_never_adds_fields_to_signed_state()
-> Result<(), Box<dyn std::error::Error>> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(58_795))))?;
    held.hold(Leaf::Used(call("empty", "session", 20_000, None)))?;
    let encoded = held.encode()?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 30_000)?;
    assert_eq!(held.encode()?, encoded);
    let restored = Held::decode(&encoded)?;
    assert_eq!(restored.encode()?, encoded);
    assert_eq!(
        restored.session_context(&limit(), &live(&[("session", 1)]), &windows(), 30_000)?,
        used
    );
    let snapshot: serde_json::Value = serde_json::from_slice(&encoded)?;
    assert!(snapshot["held"].get("context_availability").is_none());
    assert!(snapshot["held"].get("session_context").is_none());
    assert_eq!(used.context.ok_or("missing context")?.reported_none, 1);
    Ok(())
}

#[test]
fn incomplete_calls_page_back_to_their_own_reading_and_repeated_events_do_not_add_a_call()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("read", "session", 10_000, Some(50_000))))?;
    for index in 0..260 {
        held.hold(Leaf::Used(call(
            &format!("empty-{index}"),
            "session",
            20_000 + index,
            None,
        )))?;
    }
    held.hold(Leaf::Used(call("empty-259", "session", 20_259, None)))?;
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &windows(), 30_000)?;
    assert_eq!(used.figure, Some(5.into()));
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.read_at_ms, Some(10_000));
    assert_eq!(context.reported_none, 260);
    Ok(())
}

#[test]
fn absent_live_sessions_have_no_reading_or_invented_call_count_and_invalid_attribution_is_named()
-> Result<(), String> {
    let held = Held::default();
    let used = held.session_context(&limit(), &BTreeMap::new(), &windows(), 20_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("the agent has no live session to report context")
    );
    assert_eq!(used.context, None);
    assert_eq!(
        held.session_context(&limit(), &live(&[("", 1)]), &windows(), 20_000)
            .err()
            .as_deref(),
        Some("live context session attribution is empty or repeated")
    );
    let empty_agent =
        BTreeMap::from([(String::new(), BTreeMap::from([("session".to_owned(), 1)]))]);
    assert_eq!(
        held.session_context(&limit(), &empty_agent, &windows(), 20_000)
            .err()
            .as_deref(),
        Some("live context attribution names an empty agent")
    );
    Ok(())
}

#[test]
fn live_session_starts_cannot_overflow_or_be_ahead_of_the_observation() {
    let held = Held::default();
    assert_eq!(
        held.session_context(
            &limit(),
            &live(&[("session", u64::MAX)]),
            &windows(),
            20_000
        )
        .err()
        .as_deref(),
        Some("live context session start overflows milliseconds")
    );
    assert_eq!(
        held.session_context(&limit(), &live(&[("session", 21)]), &windows(), 20_000)
            .err()
            .as_deref(),
        Some("live context session session starts after the observation time")
    );
}
