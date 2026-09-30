#![cfg(test)]
//! Snapshots borrow their state and settled operations do not add pending work.

use serde::Serialize;

use super::accounting::{NOW, TestResult, history};
use super::{Work, count, reset};
use crate::budgets_crossing::{Acted, Crossing, Crossings, Stands};
use crate::budgets_state::{Act, Held, Holder, HolderKind, Measure};

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
            "encoding copied the entire state with {size} unrelated records"
        );
    }
    Ok(())
}

fn crossing(index: usize) -> Crossing {
    Crossing {
        operation: format!("crossing-{index}"),
        holder: Holder {
            kind: HolderKind::Agent,
            id: "covered".to_owned(),
        },
        measure: Measure::Tokens,
        version: 1,
        limit: 7.into(),
        figure: 7.into(),
        limit_index: 0,
        warning: false,
        account: None,
        act: Act::Tell,
        agent: "covered".to_owned(),
        session: None,
        text: None,
        at_ms: NOW,
    }
}

fn settled(size: usize) -> Crossings {
    let crossed: Vec<_> = (0..size).map(crossing).collect();
    let acted = crossed
        .iter()
        .map(|crossing| {
            (
                crossing.operation.clone(),
                Acted {
                    operation: crossing.operation.clone(),
                    stands: Stands::Told,
                    words: "kept".to_owned(),
                    at_ms: NOW,
                    ended: None,
                },
            )
        })
        .collect();
    Crossings {
        crossed,
        acted,
        ..Crossings::default()
    }
}

#[test]
fn settled_crossings_do_not_add_work_to_a_pending_lookup() {
    for size in [0, 128] {
        let mut crossings = settled(size);
        let pending = crossing(size);
        crossings.hold(pending.clone());
        reset();
        assert_eq!(crossings.unsettled(), vec![pending]);
        assert!(
            count(Work::Pending) <= 1,
            "{size} settled crossings caused {} pending visits",
            count(Work::Pending)
        );
    }
}

#[test]
fn a_duplicate_crossing_does_not_scan_other_operation_ids() {
    let mut crossings = settled(128);
    let duplicate = crossing(127);
    reset();
    crossings.hold(duplicate);
    assert_eq!(crossings.crossed.len(), 128);
    assert!(
        count(Work::CrossingLookup) <= 1,
        "duplicate lookup visited {} crossings",
        count(Work::CrossingLookup)
    );
}
