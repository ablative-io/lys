//! Missing snapshots preserve reported account levels; unknown windows stay named gaps.

use std::collections::BTreeSet;
use std::error::Error;

use lys_runner::tracking::Unavailable;
use lys_runner::tracking_budget::PlanWindow;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Held, Leaf, Length, Measure, Usage};
use crate::budgets_usage::{Used, figure};

type TestResult = Result<(), Box<dyn Error>>;
const NOW: i64 = 1_790_000_000_000;

fn report(event: &str, account: &str, percent: Option<u64>, duration: u64) -> Usage {
    Usage {
        event: event.to_owned(),
        agent: "agent".to_owned(),
        session: Some("session".to_owned()),
        account: Some(account.to_owned()),
        at_ms: NOW - 1,
        native_snapshot: true,
        plan_windows: Some(
            percent
                .map(|percent| PlanWindow {
                    duration_minutes: duration,
                    used_percent: percent.into(),
                    resets_at_ms: 1_790_018_000_000,
                })
                .into_iter()
                .collect(),
        ),
        ..Usage::default()
    }
}

fn used(held: &Held, agents: &[&str]) -> Result<Used, String> {
    figure(
        held,
        &Limit {
            unit: Measure::PlanPercent,
            amount: 50.into(),
            period: Some(Length::FiveHour),
            act: Act::Stop,
            zone: None,
        },
        &agents
            .iter()
            .map(|agent| (*agent).to_owned())
            .collect::<BTreeSet<_>>(),
        "UTC",
        NOW,
        None,
    )
}

#[test]
fn a_snapshot_reporting_no_window_preserves_the_last_provider_reading() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(report("known", "account", Some(37), 300)))?;
    let mut gap = report("missing", "account", None, 300);
    gap.at_ms = NOW;
    gap.unavailable.push(Unavailable {
        figure: "plan_windows".to_owned(),
        reason: "field_absent".to_owned(),
    });
    held.hold(Leaf::Used(gap))?;
    let answer = used(&held, &["agent"])?;
    assert_eq!(answer.figure, Some(37.into()));
    assert_eq!(answer.since_ms, Some(NOW));
    assert_eq!(answer.unavailable, None);
    assert_eq!(answer.account.as_deref(), Some("account"));
    assert_eq!(used(&Held::decode(&held.encode()?)?, &["agent"])?, answer);
    Ok(())
}

#[test]
fn reporting_another_duration_does_not_erase_the_five_hour_reading() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(report("five-hour", "account", Some(37), 300)))?;
    let mut weekly = report("weekly", "account", Some(61), 10_080);
    weekly.at_ms = NOW;
    held.hold(Leaf::Used(weekly))?;
    assert_eq!(used(&held, &["agent"])?.figure, Some(37.into()));
    Ok(())
}

#[test]
fn a_known_account_window_survives_a_gap_from_another_session_of_that_account() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(report("known", "account", Some(37), 300)))?;
    let mut gap = report("other-session", "account", None, 300);
    gap.session = Some("other-session".to_owned());
    gap.at_ms = NOW;
    held.hold(Leaf::Used(gap))?;
    assert_eq!(used(&held, &["agent"])?.figure, Some(37.into()));
    Ok(())
}

#[test]
fn a_retained_window_expires_at_its_reported_reset_even_when_later_calls_report_none() -> TestResult
{
    let mut held = Held::default();
    let mut old = report("expired", "account", Some(37), 300);
    old.plan_windows.as_mut().ok_or("windows absent")?[0].resets_at_ms = u64::try_from(NOW)?;
    held.hold(Leaf::Used(old))?;
    let mut gap = report("missing-after-reset", "account", None, 300);
    gap.at_ms = NOW;
    held.hold(Leaf::Used(gap))?;
    let answer = used(&held, &["agent"])?;
    assert_eq!(answer.figure, None);
    assert_eq!(answer.since_ms, Some(NOW));
    assert_eq!(
        answer.unavailable,
        Some(format!("the plan window reset at {NOW}; no report since"))
    );
    Ok(())
}

#[test]
fn a_window_that_was_never_reported_names_its_reason_beside_the_known_account_level() -> TestResult
{
    let mut held = Held::default();
    held.hold(Leaf::Used(report("known", "known-account", Some(37), 300)))?;
    let mut missing = report("unknown", "unknown-account", None, 300);
    missing.session = Some("other-session".to_owned());
    missing.at_ms = NOW;
    missing.unavailable.push(Unavailable {
        figure: "plan_windows".to_owned(),
        reason: "provider_header_absent".to_owned(),
    });
    held.hold(Leaf::Used(missing))?;
    let answer = used(&held, &["agent"])?;
    assert_eq!(answer.figure, None);
    assert_eq!(
        answer.unavailable.as_deref(),
        Some(
            "300-minute plan window unreported for account unknown-account: provider_header_absent"
        )
    );
    let partial = answer.reported.ok_or("known account level absent")?;
    assert_eq!(partial.figure, 37.into());
    assert_eq!(partial.missing, 1);
    Ok(())
}

#[test]
fn a_session_rotating_accounts_is_held_against_its_current_account_not_the_previous_one()
-> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(report(
        "old-account",
        "exhausted-account",
        Some(99),
        300,
    )))?;
    let mut current = report("current-account", "available-account", Some(20), 300);
    current.at_ms = NOW;
    held.hold(Leaf::Used(current))?;
    let answer = used(&held, &["agent"])?;
    assert_eq!(answer.figure, Some(20.into()));
    assert_eq!(answer.account.as_deref(), Some("available-account"));
    Ok(())
}

#[test]
fn token_records_do_not_invent_the_providers_five_hour_quota_percentage() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(Usage {
        event: "tokens-only".to_owned(),
        agent: "agent".to_owned(),
        at_ms: NOW,
        tokens: 58_795,
        ..Usage::default()
    }))?;
    let answer = used(&held, &["agent"])?;
    assert_eq!(answer.figure, None);
    assert_eq!(answer.reported, None);
    assert_eq!(
        answer.unavailable.as_deref(),
        Some("plan window unreported for agent agent")
    );
    Ok(())
}

#[test]
fn a_missing_agents_window_keeps_the_known_level_partial_for_enforcement() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(report("known", "account", Some(37), 300)))?;
    let answer = used(&held, &["agent", "unreported"])?;
    assert_eq!(answer.figure, None);
    assert_eq!(
        answer.unavailable.as_deref(),
        Some("plan window unreported for agent unreported")
    );
    let partial = answer.reported.ok_or("known account level absent")?;
    assert_eq!(partial.figure, 37.into());
    assert_eq!(partial.missing, 1);
    Ok(())
}
