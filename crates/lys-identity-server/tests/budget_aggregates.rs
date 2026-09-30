//! Aggregates keep missing sources, exact levels and distinct accounts visible.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity_server::budgets_enforce::reached;
use lys_identity_server::budgets_limits::Limit;
use lys_identity_server::budgets_state::{Act, Held, Leaf, Length, Measure, Usage};
use lys_identity_server::budgets_usage::{Used, figure};
use lys_runner::tracking::Unavailable;
use lys_runner::tracking_budget::PlanWindow;
use serde_json::Number;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const NOW: i64 = 1_790_000_000_000;

fn limit(unit: Measure) -> Limit {
    Limit {
        unit,
        amount: 50.into(),
        period: Some(Length::Week),
        act: Act::Stop,
        zone: None,
    }
}

fn report(
    event: &str,
    agent: &str,
    session: &str,
    account: &str,
    percent: &str,
) -> TestResult<Usage> {
    Ok(Usage {
        event: event.to_owned(),
        agent: agent.to_owned(),
        session: Some(session.to_owned()),
        account: Some(account.to_owned()),
        at_ms: NOW,
        plan_windows: Some(vec![PlanWindow {
            duration_minutes: 10_080,
            used_percent: percent.parse()?,
            resets_at_ms: u64::try_from(NOW)? + 604_800_000,
        }]),
        ..Usage::default()
    })
}

fn used(held: &Held, unit: Measure, agents: &[&str]) -> TestResult<Used> {
    Ok(figure(
        held,
        &limit(unit),
        &agents
            .iter()
            .map(|agent| (*agent).to_owned())
            .collect::<BTreeSet<_>>(),
        "UTC",
        NOW,
        None,
    )?)
}

#[test]
fn shared_accounts_are_deduplicated_and_distinct_accounts_use_the_highest_exact_level() -> TestResult
{
    let mut held = Held::default();
    for report in [
        report("one", "a", "s1", "shared", "49")?,
        report("two", "b", "s2", "shared", "50")?,
        report("three", "a", "s3", "other", "50.00000000000000001")?,
    ] {
        held.hold(Leaf::Used(report))?;
    }
    let used = used(&held, Measure::PlanPercent, &["a", "b"])?;
    assert_eq!(used.figure, Some("50.00000000000000001".parse()?));
    assert_eq!(used.unavailable, None);
    assert_eq!(
        serde_json::to_value(&used)?
            .as_object()
            .ok_or("no used properties")?
            .len(),
        5
    );
    Ok(())
}

#[test]
fn fractional_levels_do_not_round_up_into_a_stop() -> TestResult {
    let below: Number = "49.999999999999999999999999999999999999".parse()?;
    assert!(!reached(Measure::PlanPercent, &below, &50.into())?);
    assert!(reached(Measure::PlanPercent, &"5e1".parse()?, &50.into())?);
    assert_eq!(
        lys_runner::tracking_budget::compare(&"0.0000001".parse()?, &"1e-7".parse()?)?,
        std::cmp::Ordering::Equal
    );
    Ok(())
}

#[test]
fn one_unknown_dollar_session_is_not_hidden_by_another_known_session() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(Usage {
        event: "known".to_owned(),
        agent: "a".to_owned(),
        session: Some("claude".to_owned()),
        at_ms: NOW,
        dollars_micros: Some(400_000_000),
        native_snapshot: true,
        ..Usage::default()
    }))?;
    held.hold(Leaf::Used(Usage {
        event: "unknown".to_owned(),
        agent: "a".to_owned(),
        session: Some("codex".to_owned()),
        at_ms: NOW,
        native_snapshot: true,
        unavailable: vec![Unavailable {
            figure: "dollars_micros".to_owned(),
            reason: "Codex reports dollars only through its app-server; Lys does not read it yet"
                .to_owned(),
        }],
        ..Usage::default()
    }))?;
    let used = used(&held, Measure::Dollars, &["a"])?;
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("Codex reports dollars only through its app-server; Lys does not read it yet")
    );
    Ok(())
}

#[test]
fn a_counter_reset_keeps_the_period_unavailable_after_later_known_deltas() -> TestResult {
    let mut held = Held::default();
    for (event, amount, gaps) in [
        ("first", Some(100), Vec::new()),
        (
            "reset",
            None,
            vec![Unavailable {
                figure: "dollars_micros".to_owned(),
                reason: "reported_session_cost_reset".to_owned(),
            }],
        ),
        ("later", Some(5), Vec::new()),
    ] {
        held.hold(Leaf::Used(Usage {
            event: event.to_owned(),
            agent: "a".to_owned(),
            at_ms: NOW,
            dollars_micros: amount,
            native_snapshot: true,
            unavailable: gaps,
            ..Usage::default()
        }))?;
    }
    let used = used(&held, Measure::Dollars, &["a"])?;
    assert_eq!(used.figure, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("reported_session_cost_reset")
    );
    Ok(())
}

#[test]
fn a_missing_cost_report_recovers_when_the_cumulative_source_catches_up() -> TestResult {
    let mut held = Held::default();
    for (event, amount) in [
        ("first", Some(400_000_000)),
        ("missing", None),
        ("caught-up", Some(100_000_000)),
    ] {
        held.hold(Leaf::Used(Usage {
            event: event.to_owned(),
            agent: "a".to_owned(),
            at_ms: NOW,
            dollars_micros: amount,
            native_snapshot: true,
            ..Usage::default()
        }))?;
    }
    assert_eq!(
        used(&held, Measure::Dollars, &["a"])?.figure,
        Some(500.into())
    );
    Ok(())
}

#[test]
fn spend_overflow_is_named_instead_of_saturating() -> TestResult {
    let mut held = Held::default();
    for (event, tokens) in [("first", u64::MAX), ("second", 1)] {
        held.hold(Leaf::Used(Usage {
            event: event.to_owned(),
            agent: "a".to_owned(),
            at_ms: NOW,
            tokens,
            ..Usage::default()
        }))?;
    }
    let refused = figure(
        &held,
        &limit(Measure::Tokens),
        &BTreeSet::from(["a".to_owned()]),
        "UTC",
        NOW,
        None,
    )
    .err()
    .ok_or("overflow was accepted")?;
    assert_eq!(refused, "budget spend overflows its reported unit");
    Ok(())
}

#[test]
fn an_expired_account_leaves_other_live_accounts_visible() -> TestResult {
    let mut held = Held::default();
    let mut expired = report("expired", "a", "s1", "old-account", "99")?;
    expired.plan_windows.as_mut().ok_or("no windows")?[0].resets_at_ms = u64::try_from(NOW)?;
    held.hold(Leaf::Used(expired))?;
    held.hold(Leaf::Used(report("live", "b", "s2", "live-account", "49")?))?;
    assert_eq!(
        used(&held, Measure::PlanPercent, &["a", "b"])?.figure,
        Some(49.into())
    );
    assert_eq!(
        lys_runner::tracking_budget::percent(&serde_json::Value::Number(
            "100.00000000000000001".parse()?
        )),
        None
    );
    Ok(())
}
