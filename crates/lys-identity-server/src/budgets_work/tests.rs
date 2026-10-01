//! Unrelated history cannot multiply figure, baseline or snapshot copies.

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;

use lys_runner::tracking::{
    CLAUDE_ADAPTER, Figures, Measure as NativeMeasure, RECORD_VERSION, UsageRecord,
};
use lys_runner::tracking_budget::PlanWindow;
use serde::Serialize;

use super::{Work, count, reset};
use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Held, Leaf, Length, Measure, Usage};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
const NOW: i64 = 1_790_000_000_000;

fn history(size: usize) -> TestResult<Held> {
    let mut held = Held::default();
    for index in 0..size {
        held.hold(Leaf::Used(Usage {
            event: format!("history-{index}"),
            agent: "unrelated".to_owned(),
            at_ms: NOW - 7_776_000_000,
            tokens: 1,
            ..Usage::default()
        }))?;
    }
    held.hold(Leaf::Used(Usage {
        event: "current".to_owned(),
        agent: "covered".to_owned(),
        at_ms: NOW,
        tokens: 7,
        running_ms: 5,
        reported_running_ms: Some(5),
        dollars_micros: Some(2_000_000),
        session: Some("session".to_owned()),
        account: Some("account".to_owned()),
        context_percent: Some(40),
        plan_windows: Some(vec![PlanWindow {
            duration_minutes: 10_080,
            used_percent: "49.5".parse()?,
            resets_at_ms: u64::try_from(NOW)? + 604_800_000,
        }]),
        ..Usage::default()
    }))?;
    Ok(held)
}

#[test]
fn unrelated_history_does_not_multiply_work_for_each_limit() -> TestResult {
    let agents = BTreeSet::from(["covered".to_owned()]);
    for size in [0, 512] {
        let held = history(size)?;
        reset();
        for (unit, expected) in [
            (Measure::Tokens, "7"),
            (Measure::RunningMs, "5"),
            (Measure::Dollars, "2"),
            (Measure::PlanPercent, "49.5"),
            (Measure::ContextPercent, "40"),
        ] {
            let limit = Limit {
                unit,
                amount: 100.into(),
                period: (unit != Measure::ContextPercent).then_some(Length::Week),
                act: Act::Tell,
                zone: None,
            };
            let used = crate::budgets_usage::figure(&held, &limit, &agents, "UTC", NOW, None)?;
            assert_eq!(used.figure, Some(expected.parse()?));
            assert_eq!(used.unavailable, None);
        }
        assert!(
            count(Work::Usage) <= 5,
            "{size} unrelated records caused {} usage visits",
            count(Work::Usage)
        );
    }
    Ok(())
}

#[test]
fn unrelated_history_does_not_add_running_baseline_visits() -> TestResult {
    let record = UsageRecord {
        version: RECORD_VERSION,
        id: "next".to_owned(),
        runner: "0123456789abcdef0123456789abcdef".to_owned(),
        session: "session".to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: u64::try_from(NOW)? + 1,
        measure: NativeMeasure::Snapshot,
        figures: Figures {
            running_ms: Some(9),
            ..Figures::default()
        },
        unavailable: Vec::new(),
        adapter: CLAUDE_ADAPTER.to_owned(),
        model: None,
        account: Some("account".to_owned()),
        account_unknown: None,
        context_window: 100,
        profile_version: 1,
    };
    for size in [0, 512] {
        let held = history(size)?;
        reset();
        let usage = crate::budgets_feed::convert("machine", "covered", &record, &held)?;
        assert_eq!(usage.running_ms, 4);
        assert!(
            count(Work::Running) <= 1,
            "{size} unrelated records caused {} baseline visits",
            count(Work::Running)
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct Borrowed<'a> {
    format: &'static str,
    held: &'a Held,
}

#[test]
fn snapshots_keep_their_exact_bytes_without_copying_held_history() -> TestResult {
    for size in [0, 512] {
        let held = history(size)?;
        let expected = serde_json::to_vec(&Borrowed {
            format: "lys-budgets-state/v3",
            held: &held,
        })?;
        reset();
        let bytes = held.encode()?;
        assert_eq!(bytes, expected);
        assert_eq!(Held::decode(&bytes)?, held);
        assert_eq!(
            count(Work::StateCopy),
            0,
            "snapshot copied {size} unrelated records"
        );
    }
    Ok(())
}

#[test]
fn held_copies_share_the_index_without_copying_its_records() -> TestResult {
    let held = history(512)?;
    reset();
    let copied = held.clone();
    assert_eq!(copied, held);
    assert!(Arc::ptr_eq(&held.index, &copied.index));
    assert_eq!(count(Work::StateCopy), 1);
    assert_eq!(count(Work::IndexCopy), 0);
    Ok(())
}

fn tokens(held: &Held, at_ms: i64, incoming: Option<&Usage>) -> TestResult<u64> {
    let used = crate::budgets_usage::figure(
        held,
        &Limit {
            unit: Measure::Tokens,
            amount: 100.into(),
            period: Some(Length::Week),
            act: Act::Tell,
            zone: None,
        },
        &BTreeSet::from(["covered".to_owned()]),
        "UTC",
        at_ms,
        incoming,
    )?;
    used.figure
        .and_then(|figure| figure.as_u64())
        .ok_or_else(|| "tokens unavailable".into())
}

#[test]
fn cached_totals_handle_late_records_future_records_and_clock_reversal() -> TestResult {
    let mut held = history(512)?;
    assert_eq!(tokens(&held, NOW, None)?, 7);
    reset();
    assert_eq!(tokens(&held, NOW, None)?, 7);
    assert_eq!(count(Work::Usage), 0);
    assert_eq!(count(Work::StateCopy), 0);
    for (event, at_ms, added, expected) in [("late", NOW - 1, 3, 10), ("future", NOW + 1, 2, 10)] {
        held.hold(Leaf::Used(Usage {
            event: event.to_owned(),
            agent: "covered".to_owned(),
            at_ms,
            tokens: added,
            ..Usage::default()
        }))?;
        reset();
        assert_eq!(tokens(&held, NOW, None)?, expected);
        assert_eq!(count(Work::Usage), 0);
        assert_eq!(count(Work::IndexCopy), 0);
    }
    assert_eq!(tokens(&held, NOW + 1, None)?, 12);
    assert_eq!(tokens(&held, NOW - 1, None)?, 3);
    assert_eq!(tokens(&held, NOW, None)?, 10);
    Ok(())
}

#[test]
fn incoming_totals_do_not_mutate_the_cache_or_charge_twice() -> TestResult {
    let mut held = history(512)?;
    let incoming = Usage {
        event: "incoming".to_owned(),
        agent: "covered".to_owned(),
        at_ms: NOW,
        tokens: 3,
        ..Usage::default()
    };
    assert_eq!(tokens(&held, NOW, Some(&incoming))?, 10);
    assert_eq!(tokens(&held, NOW, None)?, 7);
    held.hold(Leaf::Used(incoming.clone()))?;
    assert_eq!(tokens(&held, NOW, Some(&incoming))?, 10);
    assert_eq!(tokens(&held, NOW, None)?, 10);
    assert_eq!(held.uses.len(), 514);
    held.hold(Leaf::Used(incoming))?;
    assert_eq!(held.uses.len(), 514);
    assert_eq!(tokens(&held, NOW, None)?, 10);
    Ok(())
}

fn crossing(operation: String, agent: &str) -> crate::budgets_crossing::Crossing {
    crate::budgets_crossing::Crossing {
        operation,
        holder: crate::budgets_state::Holder {
            kind: crate::budgets_state::HolderKind::Agent,
            id: agent.to_owned(),
        },
        measure: Measure::Tokens,
        version: 1,
        limit: 10.into(),
        figure: Some(10.into()),
        unavailable: None,
        limit_index: 0,
        warning: false,
        account: None,
        act: Act::Stop,
        agent: agent.to_owned(),
        session: Some("session".to_owned()),
        text: None,
        at_ms: NOW,
    }
}

fn crossing_history() -> TestResult<Held> {
    let mut held = Held::default();
    for index in 0..512 {
        let operation = format!("old-{index}");
        held.hold(Leaf::Used(Usage {
            event: operation.clone(),
            agent: "unrelated".to_owned(),
            crossed: vec![crossing(operation.clone(), "unrelated")],
            ..Usage::default()
        }))?;
        held.hold(Leaf::Acted(crate::budgets_crossing::Acted {
            operation,
            stands: crate::budgets_crossing::Stands::Confirmed,
            words: "confirmed exit".to_owned(),
            at_ms: NOW,
            ended: None,
        }))?;
    }
    held.hold(Leaf::Used(Usage {
        event: "current".to_owned(),
        agent: "covered".to_owned(),
        crossed: vec![crossing("current".to_owned(), "covered")],
        ..Usage::default()
    }))?;
    Ok(held)
}

#[test]
fn unrelated_crossings_do_not_multiply_receipt_work_after_reopen() -> TestResult {
    let held = crossing_history()?;
    let restored = Held::decode(&held.encode()?)?;
    reset();
    let receipts = restored.crossings.of_agent("covered");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].crossing.operation, "current");
    assert!(
        count(Work::Crossing) <= 1,
        "{} unrelated crossing visits",
        count(Work::Crossing)
    );
    Ok(())
}

#[test]
fn settled_crossings_do_not_multiply_pending_work_after_reopen() -> TestResult {
    let held = crossing_history()?;
    let restored = Held::decode(&held.encode()?)?;
    reset();
    let pending = restored.crossings.unsettled();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].operation, "current");
    assert!(
        count(Work::Crossing) <= 1,
        "{} settled crossing visits",
        count(Work::Crossing)
    );
    Ok(())
}
