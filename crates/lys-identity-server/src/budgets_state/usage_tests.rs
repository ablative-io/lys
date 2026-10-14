//! Usage-call counts and account-window reads follow indexed records, not budget crossings.

use std::error::Error;

use lys_runner::tracking::Unavailable;
use lys_runner::tracking_budget::PlanWindow;

use super::{CallSeen, Usage};
use crate::budgets_state::{Held, Leaf};

type TestResult = Result<(), Box<dyn Error>>;
const NOW: i64 = 1_790_000_000_000;
const RESET: u64 = 1_790_000_000_100;

fn call(event: &str, agent: &str) -> Usage {
    Usage {
        event: event.to_owned(),
        agent: agent.to_owned(),
        at_ms: 10,
        session: Some("session".to_owned()),
        call: Some(Box::new(CallSeen {
            id: event.to_owned(),
            model: Some("claude-opus-5-5".to_owned()),
            input_tokens: Some(58_795),
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            run: "0123456789abcdef0123456789abcdef".to_owned(),
            record: None,
        })),
        ..Usage::default()
    }
}

fn windows(event: &str, at_ms: i64, readings: &[(u64, u64, u64)]) -> Usage {
    Usage {
        event: event.to_owned(),
        agent: "agent".to_owned(),
        session: Some("session".to_owned()),
        account: Some("account".to_owned()),
        at_ms,
        plan_windows: Some(
            readings
                .iter()
                .map(|(duration, percent, reset)| PlanWindow {
                    duration_minutes: *duration,
                    used_percent: (*percent).into(),
                    resets_at_ms: *reset,
                })
                .collect(),
        ),
        ..Usage::default()
    }
}

#[test]
fn reported_calls_are_counted_without_any_budget_crossing_and_survive_reopen() -> TestResult {
    let mut held = Held::default();
    for usage in [
        call("one", "agent"),
        call("two", "agent"),
        call("other", "other-agent"),
        Usage {
            event: "status".to_owned(),
            agent: "agent".to_owned(),
            at_ms: 10,
            native_snapshot: true,
            ..Usage::default()
        },
    ] {
        held.hold(Leaf::Used(usage))?;
    }
    held.hold(Leaf::Used(call("one", "agent")))?;
    assert_eq!(held.reported_calls("agent")?, 2);
    assert_eq!(held.reported_calls("other-agent")?, 1);
    assert_eq!(held.reported_calls("never-reported")?, 0);
    assert!(held.crossings.of_agent("agent").is_empty());
    let restored = Held::decode(&held.encode()?)?;
    assert_eq!(restored.reported_calls("agent")?, 2);
    assert!(restored.crossings.of_agent("agent").is_empty());
    Ok(())
}

#[test]
fn an_incomplete_call_is_counted_even_when_its_response_reported_no_figures() -> TestResult {
    let mut held = Held::default();
    let mut incomplete = call("lost", "agent");
    incomplete.call.as_mut().ok_or("call absent")?.input_tokens = None;
    incomplete.unavailable.push(Unavailable {
        figure: "tokens".to_owned(),
        reason: "proxy call was lost before the response ended".to_owned(),
    });
    held.hold(Leaf::Used(incomplete))?;
    assert_eq!(held.reported_calls("agent")?, 1);
    Ok(())
}

#[test]
fn a_call_count_walks_every_index_page_without_counting_another_agents_calls() -> TestResult {
    let mut held = Held::default();
    for index in 0..260 {
        held.hold(Leaf::Used(call(&format!("call-{index}"), "agent")))?;
    }
    held.hold(Leaf::Used(call("other", "other-agent")))?;
    assert_eq!(held.reported_calls("agent")?, 260);
    assert_eq!(held.reported_calls("other-agent")?, 1);
    Ok(())
}

#[test]
fn a_missing_indexed_call_is_named_instead_of_returning_a_smaller_count() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("one", "agent")))?;
    held.uses.clear();
    assert_eq!(
        held.reported_calls("agent").err().as_deref(),
        Some("call index names missing record 0")
    );
    Ok(())
}

#[test]
fn an_indexed_record_that_no_longer_carries_a_call_names_the_inconsistency() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(call("one", "agent")))?;
    held.uses[0].call = None;
    assert_eq!(
        held.reported_calls("agent").err().as_deref(),
        Some("call index names record 0 without a call for agent agent")
    );
    Ok(())
}

#[test]
fn latest_windows_are_selected_by_account_and_duration_without_erasing_an_omitted_duration()
-> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(windows(
        "both",
        NOW - 2,
        &[(300, 37, RESET), (10_080, 49, RESET)],
    )))?;
    held.hold(Leaf::Used(windows(
        "weekly-only",
        NOW - 1,
        &[(10_080, 61, RESET)],
    )))?;
    let accounts = held.account_windows("agent", NOW)?;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account, "account");
    assert_eq!(accounts[0].at_ms, NOW - 1);
    assert_eq!(
        accounts[0].windows,
        vec![
            PlanWindow {
                duration_minutes: 300,
                used_percent: 37.into(),
                resets_at_ms: RESET
            },
            PlanWindow {
                duration_minutes: 10_080,
                used_percent: 61.into(),
                resets_at_ms: RESET
            },
        ]
    );
    Ok(())
}

#[test]
fn an_explicit_gap_keeps_the_previous_window_until_its_own_reset_and_not_after() -> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(windows(
        "known",
        NOW - 2,
        &[(300, 37, u64::try_from(NOW)?)],
    )))?;
    let mut gap = windows("gap", NOW - 1, &[]);
    gap.unavailable.push(Unavailable {
        figure: "plan_windows".to_owned(),
        reason: "provider_header_absent".to_owned(),
    });
    held.hold(Leaf::Used(gap))?;
    assert_eq!(
        held.account_windows("agent", NOW - 1)?[0].windows[0].used_percent,
        37.into()
    );
    assert!(held.account_windows("agent", NOW)?.is_empty());
    assert!(
        Held::decode(&held.encode()?)?
            .account_windows("agent", NOW)?
            .is_empty()
    );
    Ok(())
}

#[test]
fn a_future_window_reading_cannot_replace_the_accounts_last_report_at_the_observation_time()
-> TestResult {
    let mut held = Held::default();
    held.hold(Leaf::Used(windows("known", NOW - 2, &[(300, 37, RESET)])))?;
    held.hold(Leaf::Used(windows("future", NOW + 1, &[(300, 71, RESET)])))?;
    let accounts = held.account_windows("agent", NOW)?;
    assert_eq!(accounts[0].at_ms, NOW - 2);
    assert_eq!(accounts[0].windows[0].used_percent, 37.into());
    Ok(())
}

#[test]
fn a_window_with_no_paying_account_names_the_missing_attribution() -> TestResult {
    let mut held = Held::default();
    let mut unknown = windows("unknown-account", NOW - 2, &[(300, 37, RESET)]);
    unknown.account = None;
    held.hold(Leaf::Used(unknown))?;
    assert_eq!(
        held.account_windows("agent", NOW).err().as_deref(),
        Some("the reported plan window names no account")
    );
    Ok(())
}

#[test]
fn a_plan_observation_before_the_epoch_is_named_and_never_guessed() {
    assert!(Held::default().account_windows("agent", -1).is_err());
}
