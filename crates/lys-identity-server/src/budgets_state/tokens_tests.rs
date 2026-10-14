//! Reported components and gaps are independent; period coverage comes from indexed calls.

use std::collections::BTreeSet;
use std::error::Error;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, CallSeen, Held, Leaf, Length, Measure, Usage};
use crate::budgets_usage::{TokenComponents, figure, start};

type TestResult = Result<(), Box<dyn Error>>;
const NOW: i64 = 1_790_000_000_000;

fn limit() -> Limit {
    Limit {
        unit: Measure::Tokens,
        amount: 50.into(),
        period: Some(Length::Day),
        act: Act::Stop,
        zone: None,
    }
}

fn call(event: &str, at_ms: i64, session: Option<&str>, values: [Option<u64>; 4]) -> Usage {
    Usage {
        event: event.to_owned(),
        agent: "agent".to_owned(),
        at_ms,
        session: session.map(str::to_owned),
        call: Some(Box::new(CallSeen {
            id: event.to_owned(),
            model: Some("claude-opus-5-5".to_owned()),
            input_tokens: values[0],
            output_tokens: values[1],
            cache_creation_tokens: values[2],
            cache_read_tokens: values[3],
            run: "0123456789abcdef0123456789abcdef".to_owned(),
            record: None,
        })),
        ..Usage::default()
    }
}

fn components(held: &Held) -> Result<TokenComponents, String> {
    held.token_components(&limit(), &BTreeSet::from(["agent".to_owned()]), "UTC", NOW)
}

#[test]
fn each_token_component_keeps_its_reported_subtotal_beside_its_own_missing_calls() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call(
        "first",
        NOW,
        Some("session"),
        [Some(10), Some(2), None, Some(100)],
    )))?;
    held.hold(Leaf::Used(call(
        "second",
        NOW,
        Some("session"),
        [None, Some(3), Some(20), Some(50)],
    )))?;
    let parts = components(&held)?;
    assert_eq!(parts.input_tokens.reported, Some(10));
    assert_eq!(parts.input_tokens.missing_calls, 1);
    assert_eq!(parts.output_tokens.reported, Some(5));
    assert_eq!(parts.output_tokens.missing_calls, 0);
    assert_eq!(parts.cache_creation_tokens.reported, Some(20));
    assert_eq!(parts.cache_creation_tokens.missing_calls, 1);
    assert_eq!(parts.cache_read_tokens.reported, Some(150));
    assert_eq!(parts.cache_read_tokens.missing_calls, 0);
    assert_eq!(parts.calls, 2);
    assert_eq!(parts.sessions, 1);
    assert_eq!(parts.calls_without_session, 0);
    Ok(())
}

#[test]
fn an_unreported_component_stays_absent_while_an_explicit_zero_stays_reported() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("missing", NOW, Some("session"), [None; 4])))?;
    held.hold(Leaf::Used(call(
        "zero",
        NOW,
        Some("session"),
        [Some(0), None, None, None],
    )))?;
    let parts = components(&held)?;
    assert_eq!(parts.input_tokens.reported, Some(0));
    assert_eq!(parts.input_tokens.missing_calls, 1);
    assert_eq!(parts.output_tokens.reported, None);
    assert_eq!(parts.output_tokens.missing_calls, 2);
    assert_eq!(parts.cache_creation_tokens.reported, None);
    assert_eq!(parts.cache_creation_tokens.missing_calls, 2);
    assert_eq!(parts.cache_read_tokens.reported, None);
    assert_eq!(parts.cache_read_tokens.missing_calls, 2);
    Ok(())
}

#[test]
fn call_coverage_counts_incomplete_calls_and_names_unnamed_sessions_without_inventing_their_number()
-> TestResult {
    let mut held = Held::default();
    for usage in [
        call("first", NOW, Some("one"), [None; 4]),
        call("second", NOW, Some("one"), [None; 4]),
        call("third", NOW, Some("two"), [None; 4]),
        call("unnamed", NOW, None, [None; 4]),
        call("empty-name", NOW, Some(""), [None; 4]),
    ] {
        held.hold(Leaf::Used(usage))?;
    }
    let parts = components(&held)?;
    assert_eq!(parts.calls, 5);
    assert_eq!(parts.sessions, 2);
    assert_eq!(parts.calls_without_session, 2);
    assert_eq!(parts.input_tokens.reported, None);
    assert_eq!(parts.input_tokens.missing_calls, 5);
    Ok(())
}

#[test]
fn period_components_select_only_the_named_agents_calls_at_or_after_the_calendar_boundary_and_not_in_the_future()
-> TestResult {
    let mut held = Held::default();
    let since = start(&limit(), "UTC", NOW)?.ok_or("day boundary absent")?;
    for (event, at) in [
        ("previous-day", since - 1),
        ("boundary", since),
        ("now", NOW),
        ("future", NOW + 1),
    ] {
        held.hold(Leaf::Used(call(event, at, Some("session"), [Some(1); 4])))?;
    }
    let mut other = call("other-agent", NOW, Some("other-session"), [Some(100); 4]);
    other.agent = "other".to_owned();
    held.hold(Leaf::Used(other))?;
    held.hold(Leaf::Used(Usage {
        event: "status".to_owned(),
        agent: "agent".to_owned(),
        at_ms: NOW,
        native_snapshot: true,
        tokens: 500,
        ..Usage::default()
    }))?;
    let parts = components(&held)?;
    assert_eq!(parts.calls, 2);
    assert_eq!(parts.sessions, 1);
    assert_eq!(parts.input_tokens.reported, Some(2));
    Ok(())
}

#[test]
fn the_declared_organisation_zone_selects_the_days_component_boundary() -> TestResult {
    let mut held = Held::default();
    let since = start(&limit(), "Australia/Melbourne", NOW)?.ok_or("day boundary absent")?;
    held.hold(Leaf::Used(call(
        "previous-day",
        since - 1,
        Some("session"),
        [Some(100); 4],
    )))?;
    held.hold(Leaf::Used(call(
        "boundary",
        since,
        Some("session"),
        [Some(3); 4],
    )))?;
    let parts = held.token_components(
        &limit(),
        &BTreeSet::from(["agent".to_owned()]),
        "Australia/Melbourne",
        NOW,
    )?;
    assert_eq!(parts.calls, 1);
    assert_eq!(parts.input_tokens.reported, Some(3));
    Ok(())
}

#[test]
fn component_reads_walk_multiple_call_pages_and_replayed_events_do_not_count_twice() -> TestResult {
    let mut held = Held::default();
    for index in 0..260 {
        held.hold(Leaf::Used(call(
            &format!("call-{index}"),
            NOW,
            Some("session"),
            [Some(1); 4],
        )))?;
    }
    held.hold(Leaf::Used(call(
        "call-0",
        NOW,
        Some("session"),
        [Some(1); 4],
    )))?;
    let parts = components(&held)?;
    assert_eq!(parts.calls, 260);
    assert_eq!(parts.input_tokens.reported, Some(260));
    assert_eq!(components(&Held::decode(&held.encode()?)?)?, parts);
    Ok(())
}

#[test]
fn a_component_subtotal_overflow_names_the_component_and_never_saturates() -> TestResult {
    for (component, name) in [
        "input_tokens",
        "output_tokens",
        "cache_creation_tokens",
        "cache_read_tokens",
    ]
    .into_iter()
    .enumerate()
    {
        let mut held = Held::default();
        let mut maximum = [None; 4];
        maximum[component] = Some(u64::MAX);
        let mut extra = [None; 4];
        extra[component] = Some(1);
        held.hold(Leaf::Used(call("maximum", NOW, Some("session"), maximum)))?;
        held.hold(Leaf::Used(call("extra", NOW, Some("session"), extra)))?;
        assert_eq!(
            components(&held).err(),
            Some(format!("reported {name} subtotal overflows"))
        );
    }
    Ok(())
}

#[test]
fn selected_agents_keep_distinct_sessions_even_when_the_session_names_match() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call(
        "agent-call",
        NOW,
        Some("same-name"),
        [Some(1); 4],
    )))?;
    let mut other = call("other-call", NOW, Some("same-name"), [Some(2); 4]);
    other.agent = "other".to_owned();
    held.hold(Leaf::Used(other))?;
    let parts = held.token_components(
        &limit(),
        &BTreeSet::from(["agent".to_owned(), "other".to_owned()]),
        "UTC",
        NOW,
    )?;
    assert_eq!(parts.calls, 2);
    assert_eq!(parts.sessions, 2);
    assert_eq!(parts.input_tokens.reported, Some(3));
    Ok(())
}

#[test]
fn serialised_component_coverage_keeps_absence_distinct_from_reported_zero() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call(
        "call",
        NOW,
        None,
        [Some(0), None, Some(7), Some(8)],
    )))?;
    let shown = serde_json::to_value(components(&held)?)?;
    assert_eq!(
        shown,
        serde_json::json!({
            "input_tokens": { "reported": 0, "missing_calls": 0 },
            "output_tokens": { "reported": null, "missing_calls": 1 },
            "cache_creation_tokens": { "reported": 7, "missing_calls": 0 },
            "cache_read_tokens": { "reported": 8, "missing_calls": 0 },
            "calls": 1, "sessions": 0, "calls_without_session": 1,
        })
    );
    Ok(())
}

#[test]
fn a_missing_indexed_call_is_named_instead_of_omitted_from_component_coverage() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("call", NOW, Some("session"), [Some(1); 4])))?;
    held.uses.clear();
    assert_eq!(
        components(&held).err().as_deref(),
        Some("token component index names missing record 0")
    );
    Ok(())
}

#[test]
fn an_empty_period_reports_no_calls_and_no_component_figure() -> TestResult {
    let parts = components(&Held::default())?;
    assert_eq!(parts.calls, 0);
    assert_eq!(parts.sessions, 0);
    assert_eq!(parts.input_tokens.reported, None);
    assert_eq!(parts.input_tokens.missing_calls, 0);
    Ok(())
}

#[test]
fn display_components_do_not_change_the_total_compared_by_budget_enforcement() -> TestResult {
    let mut held = Held::default();
    let mut usage = call(
        "call",
        NOW,
        Some("session"),
        [Some(20), Some(2), Some(1), Some(2)],
    );
    usage.tokens = 25;
    held.hold(Leaf::Used(usage))?;
    let agents = BTreeSet::from(["agent".to_owned()]);
    let enforced = figure(&held, &limit(), &agents, "UTC", NOW, None)?;
    let displayed = crate::budgets_api::current_usage(&held, &limit(), &agents, "UTC", NOW, None)?;
    assert_eq!(displayed.figure, enforced.figure);
    assert_eq!(displayed.figure, Some(25.into()));
    assert_eq!(enforced.tokens, None);
    let parts = displayed.tokens.ok_or("display components absent")?;
    assert_eq!(parts.input_tokens.reported, Some(20));
    assert_eq!(parts.output_tokens.reported, Some(2));
    assert_eq!(parts.cache_creation_tokens.reported, Some(1));
    assert_eq!(parts.cache_read_tokens.reported, Some(2));
    Ok(())
}

#[test]
fn components_refuse_a_unit_or_period_that_does_not_name_token_spend() {
    let held = Held::default();
    let agents = BTreeSet::from(["agent".to_owned()]);
    let mut wrong = limit();
    wrong.unit = Measure::Dollars;
    assert_eq!(
        held.token_components(&wrong, &agents, "UTC", NOW)
            .err()
            .as_deref(),
        Some("token components require the tokens unit")
    );
    wrong.unit = Measure::Tokens;
    wrong.period = None;
    assert_eq!(
        held.token_components(&wrong, &agents, "UTC", NOW)
            .err()
            .as_deref(),
        Some("token components require a declared period")
    );
    assert_eq!(
        held.token_components(&limit(), &agents, "UTC", -1)
            .err()
            .as_deref(),
        Some("token component observation is before the Unix epoch")
    );
}
