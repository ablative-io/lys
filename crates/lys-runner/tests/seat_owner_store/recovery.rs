#![cfg(test)]
//! Live ownership remains fenced and recovery remains bounded.

use super::{TestResult, custody, establish, exited, hex_id, leader, refusal, written_checkpoint};
use lys_runner::seat_owner::store::{Cursors, Custody, Holder, Intent, MAX_TAIL, OwnerStore};

#[test]
fn seat_owner_store_fences_a_live_owner() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    store.record(&hex_id(30), &establish("w", 5_007))?;
    let stale = Intent::Lease {
        session: "w".to_owned(),
        generation: 1,
        holder: Holder::Successor {
            start: leader(5_008),
        },
        taken_at: 3,
        build: "0.0.0-test".to_owned(),
    };
    assert_eq!(
        refusal(store.record(&hex_id(31), &stale)),
        "seat_owner_generation_stale"
    );
    let harness = |pid: u32| Intent::Harness {
        session: "w".to_owned(),
        start: leader(pid),
    };
    store.record(&hex_id(32), &harness(6_000))?;
    assert_eq!(
        refusal(store.record(&hex_id(33), &harness(6_001))),
        "seat_owner_harness_mismatch"
    );
    let retire = Intent::Retire {
        session: "w".to_owned(),
    };
    assert_eq!(
        refusal(store.record(&hex_id(34), &retire)),
        "seat_owner_live"
    );
    store.record(
        &hex_id(35),
        &custody(
            "w",
            Custody::Stopping {
                intent: hex_id(35),
                since: 4,
            },
        ),
    )?;
    let adopt = custody(
        "w",
        Custody::HandingOver {
            intent: hex_id(36),
            successor: leader(5_009),
            since: 5,
        },
    );
    assert_eq!(
        refusal(store.record(&hex_id(36), &adopt)),
        "seat_owner_stop_fenced"
    );
    let cursors = |receipt: u64| Intent::Cursors {
        session: "w".to_owned(),
        cursors: Cursors {
            receipt,
            feed: 2,
            hook: 1,
        },
    };
    store.record(&hex_id(37), &cursors(4))?;
    assert_eq!(
        refusal(store.record(&hex_id(38), &cursors(3))),
        "seat_owner_cursor_regression"
    );
    store.record(&hex_id(39), &custody("w", exited(&hex_id(39), None)))?;
    assert_eq!(
        refusal(store.record(&hex_id(40), &custody("w", Custody::Owned))),
        "seat_owner_exited"
    );
    let nobody = Intent::Retire {
        session: "nobody".to_owned(),
    };
    assert_eq!(
        refusal(store.record(&hex_id(41), &nobody)),
        "seat_owner_unknown"
    );
    store.record(&hex_id(42), &retire)?;
    assert!(store.owner("w").is_none());
    assert_eq!(
        refusal(store.record("not-hex", &retire)),
        "seat_owner_intent_invalid"
    );
    Ok(())
}

#[test]
fn seat_owner_recovery_is_bounded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    // Establish, exit and retire more sessions than the tail holds; every
    // sixteenth stays live. The store checkpoints itself as the tail fills.
    let rounds = MAX_TAIL * 3;
    let mut next_id = 100_u64;
    let mut id = move || {
        next_id += 1;
        hex_id(next_id)
    };
    for n in 0..rounds {
        let session = format!("session-{n}");
        let pid = u32::try_from(10_000 + n)?;
        store.record(&id(), &establish(&session, pid))?;
        if n % 16 == 0 {
            continue;
        }
        let stop = id();
        store.record(&stop, &custody(&session, exited(&stop, None)))?;
        store.record(&id(), &Intent::Retire { session })?;
    }
    let live_count = rounds.div_ceil(16);
    let live = usize::try_from(live_count)?;
    assert_eq!(store.owners().len(), live);
    let written = store.counts();
    assert!(
        written.checkpoints >= 1,
        "the tail bound forced checkpoints"
    );
    assert_eq!(written.journal_appends, rounds + (rounds - live_count) * 2);
    assert!(
        store.last_seq() - written_checkpoint(dir.path())? <= MAX_TAIL,
        "the tail past the checkpoint is bounded"
    );

    // Reopening visits the live owners plus at most the tail, never the
    // history: the figure is read from the store, not assumed.
    let reopened = OwnerStore::open(dir.path())?;
    assert_eq!(reopened.owners().len(), live);
    assert_eq!(reopened.last_seq(), store.last_seq());
    let visits = reopened.counts().record_visits;
    let bound = live_count + MAX_TAIL + 1;
    assert!(
        visits <= bound,
        "recovery visited {visits} records; the bound is {bound} (live {live} + tail {MAX_TAIL} + 1)"
    );
    assert!(
        visits < written.journal_appends,
        "recovery read {visits} records, not the {} lines of history",
        written.journal_appends
    );
    Ok(())
}
