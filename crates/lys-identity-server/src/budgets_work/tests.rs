//! Unrelated history cannot multiply figure, baseline or snapshot copies.

use std::collections::BTreeSet;
use std::error::Error;

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
        let usage = crate::budgets_feed::convert("machine", "covered", &record, &held.uses)?;
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
