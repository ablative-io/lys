//! Live crossing records answer duplicate and pending lookups without walking settled history.

use super::{Work, count, reset};
use crate::budgets_crossing::{Acted, Crossing, Crossings, Stands};
use crate::budgets_state::{Act, Holder, HolderKind, Measure};

const NOW: i64 = 1_790_000_000_000;

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
        figure: Some(7.into()),
        unavailable: None,
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

fn told(operation: &str) -> Acted {
    Acted {
        operation: operation.to_owned(),
        stands: Stands::Told,
        words: "kept".to_owned(),
        at_ms: NOW,
        ended: None,
    }
}

/// Settled crossings kept in the live state, half acted before they were held
/// and half acted after, so both index paths are exercised.
fn settled(size: usize) -> Crossings {
    let mut crossings = Crossings::default();
    for index in 0..size {
        let crossing = crossing(index);
        if index % 2 == 0 {
            crossings.acted(told(&crossing.operation));
            crossings.hold(crossing);
        } else {
            let operation = crossing.operation.clone();
            crossings.hold(crossing);
            crossings.acted(told(&operation));
        }
    }
    crossings
}

#[test]
fn settled_crossings_do_not_add_work_to_a_pending_lookup() {
    for size in [0, 128] {
        let mut crossings = settled(size);
        let pending = crossing(size);
        crossings.hold(pending.clone());
        reset();
        assert_eq!(crossings.unsettled(), vec![pending.clone()]);
        assert_eq!(crossings.unsettled_for("covered"), vec![pending]);
        assert!(
            count(Work::Crossing) <= 2,
            "{size} settled crossings caused {} pending visits",
            count(Work::Crossing)
        );
    }
}

#[test]
fn a_duplicate_crossing_does_not_scan_other_operation_ids() {
    let mut crossings = settled(128);
    let mut duplicate = crossing(127);
    duplicate.at_ms = NOW + 1;
    reset();
    crossings.hold(duplicate);
    assert_eq!(crossings.crossed.len(), 128);
    assert_eq!(crossings.crossed[127], crossing(127));
    assert!(crossings.unsettled().is_empty());
    assert!(
        count(Work::CrossingLookup) <= 1,
        "duplicate lookup visited {} crossings",
        count(Work::CrossingLookup)
    );
}
