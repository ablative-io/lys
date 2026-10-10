//! AGENTS-004 R5: the counted work of the seat-owner hot paths the store
//! serves, against the checked-in ratchet. Counts gate; durations are kept
//! beside them as evidence and never widen a limit.

use std::error::Error;
use std::time::{Duration, Instant};

use lys_runner::peer::{Leader, StartIdentity};
use lys_runner::protocol::{Ended, EndedHow};
use lys_runner::seat_owner::counts::{HotPath, Ratchet, Work};
use lys_runner::seat_owner::store::{
    Cursors, Custody, Holder, Intent, Lease, MAX_MEMBER_BYTES, OwnerRecord, OwnerStore, StoreCounts,
};

type TestResult = Result<(), Box<dyn Error>>;

const RATCHET: &str = include_str!("seat_cost_ratchet.json");

fn leader(pid: u32) -> Leader {
    Leader {
        pid,
        start: StartIdentity(format!("boot-{pid}")),
    }
}

fn hex_id(n: u64) -> String {
    format!("{n:032x}")
}

fn establish(session: &str, pid: u32) -> Intent {
    Intent::Establish {
        record: Box::new(OwnerRecord {
            seat: format!("seat-{session}"),
            session: session.to_owned(),
            conversation: format!("conversation-{session}"),
            harness: None,
            lease: Lease {
                generation: 1,
                holder: Holder::Owner { start: leader(pid) },
                taken_at: 1_760_060_000_000,
                build: "0.0.0-test".to_owned(),
            },
            custody: Custody::Owned,
            cursors: Cursors::default(),
            credential_references: vec!["seat-token".to_owned()],
            established_at: 1_760_060_000_000,
        }),
    }
}

fn custody(session: &str, custody: Custody) -> Intent {
    Intent::Custody {
        session: session.to_owned(),
        custody,
    }
}

fn exited(intent: &str, reason: Option<String>) -> Custody {
    Custody::Exited {
        intent: intent.to_owned(),
        exit: Ended {
            how: EndedHow::Exited,
            at: 1_760_060_001_000,
            status: Some(0),
            signal: None,
            reason,
            stopped: None,
        },
    }
}

fn retire(session: &str) -> Intent {
    Intent::Retire {
        session: session.to_owned(),
    }
}

/// One custody transfer at the ratchet's inputs: establish, stop, exit,
/// retire, and one duplicate readback of the exit. Returns the work.
fn transfer(store: &mut OwnerStore, n: u64, session: &str) -> Result<Work, Box<dyn Error>> {
    let before = store.counts();
    let base = n * 10;
    store.record(&hex_id(base + 1), &establish(session, 7_000))?;
    store.record(
        &hex_id(base + 2),
        &custody(
            session,
            Custody::Stopping {
                intent: hex_id(base + 2),
                since: 1,
            },
        ),
    )?;
    let exit = custody(session, exited(&hex_id(base + 3), None));
    store.record(&hex_id(base + 3), &exit)?;
    store.record(&hex_id(base + 3), &exit)?;
    store.record(&hex_id(base + 4), &retire(session))?;
    Ok(Work::between(before, store.counts(), 5))
}

/// A store holding `live` owners and `tail` more lines after its checkpoint.
fn populated(dir: &std::path::Path, live: u64, tail: u64) -> Result<(), Box<dyn Error>> {
    let mut store = OwnerStore::open(dir)?;
    for n in 0..live {
        store.record(&hex_id(1_000 + n), &establish(&format!("live-{n}"), 7_100))?;
    }
    store.checkpoint()?;
    for n in 0..tail {
        let session = format!("tail-{n}");
        store.record(&hex_id(5_000 + n * 3), &establish(&session, 7_200))?;
        let exit = hex_id(5_001 + n * 3);
        store.record(&exit, &custody(&session, exited(&exit, None)))?;
        store.record(&hex_id(5_002 + n * 3), &retire(&session))?;
    }
    Ok(())
}

fn report(path: HotPath, work: Work) -> String {
    let counters: Vec<String> = work
        .named()
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect();
    format!("{} {}", path.name(), counters.join(" "))
}

#[test]
fn seat_cost_ratchet() -> TestResult {
    let ratchet = Ratchet::parse(RATCHET)?;
    let dir = tempfile::tempdir()?;

    // Custody transfer at the fixed inputs.
    let mut store = OwnerStore::open(dir.path())?;
    let work = transfer(&mut store, 1, "transfer-1")?;
    assert_eq!(
        work.journal_appends,
        4,
        "{}",
        report(HotPath::CustodyTransfer, work)
    );
    assert_eq!(
        work.syncs, 4,
        "one sync per transition, zero for the duplicate"
    );
    ratchet.check(HotPath::CustodyTransfer, work)?;

    // Registry read: one keyed lookup moves no store counter.
    store.record(&hex_id(90), &establish("registry", 7_300))?;
    let before = store.counts();
    assert!(store.owner("registry").is_some());
    let read = Work::between(before, store.counts(), 1);
    assert_eq!(
        read,
        Work {
            calls: 1,
            ..Work::default()
        },
        "{}",
        report(HotPath::RegistryRead, read)
    );
    ratchet.check(HotPath::RegistryRead, read)?;
    drop(store);

    // Reconnect: reopening over the ratchet's owners and a full tail.
    let dir = tempfile::tempdir()?;
    let inputs = &ratchet.inputs;
    let live = inputs["owners_live"];
    let tail = inputs["tail_lines_max"] / 3;
    populated(dir.path(), live, tail)?;
    let reopened = OwnerStore::open(dir.path())?;
    let reconnect = Work::between(StoreCounts::default(), reopened.counts(), 1);
    assert_eq!(reopened.owners().len(), usize::try_from(live)?);
    ratchet.check(HotPath::Reconnect, reconnect)?;
    eprintln!("{}", report(HotPath::Reconnect, reconnect));

    // Positive control: one extra sync in the transfer raises its counter and
    // fails the ratchet by name. The control lives here, not in the store.
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    let before = store.counts();
    transfer(&mut store, 2, "transfer-2")?;
    store.checkpoint()?;
    let controlled = Work::between(before, store.counts(), 5);
    assert!(
        controlled.syncs > 4,
        "{}",
        report(HotPath::CustodyTransfer, controlled)
    );
    let refused = ratchet
        .check(HotPath::CustodyTransfer, controlled)
        .expect_err("an extra sync must fail the ratchet");
    assert_eq!(refused.name(), "seat_cost_over_ceiling");
    assert!(refused.to_string().contains("syncs"), "{refused}");
    Ok(())
}

#[test]
fn seat_cost_ratchet_reads_only_its_own_counters() {
    let renamed = RATCHET.replace("\"syncs\"", "\"fsyncs\"");
    let error = Ratchet::parse(&renamed).expect_err("a renamed counter does not read");
    assert_eq!(error.name(), "seat_cost_ratchet_invalid");
    let dropped = RATCHET.replace("\"custody_transfer\"", "\"custody_transfer_old\"");
    let error = Ratchet::parse(&dropped).expect_err("a missing path does not read");
    assert_eq!(error.name(), "seat_cost_ratchet_invalid");
    let other = RATCHET.replace("seat-cost-ratchet/v1", "seat-cost-ratchet/v0");
    let error = Ratchet::parse(&other).expect_err("another format does not read");
    assert_eq!(error.name(), "seat_cost_ratchet_invalid");
}

/// The fastest of `runs` reopenings of `dir`, with the work one of them did.
fn reopen_timed(dir: &std::path::Path, runs: u32) -> Result<(Duration, Work), Box<dyn Error>> {
    let mut fastest = Duration::MAX;
    let mut work = Work::default();
    for _ in 0..runs {
        let started = Instant::now();
        let store = OwnerStore::open(dir)?;
        let elapsed = started.elapsed();
        work = Work::between(StoreCounts::default(), store.counts(), 1);
        fastest = fastest.min(elapsed);
    }
    Ok((fastest, work))
}

#[test]
fn seat_cost_tracks_duration() -> TestResult {
    // Paired workloads: a known source of work is added (a full tail), then
    // removed (a checkpoint). Counts must order them; durations are observed
    // beside the counts, and a disagreement is reported as unqualified.
    let dir = tempfile::tempdir()?;
    populated(dir.path(), 48, 0)?;
    let (base_time, base) = reopen_timed(dir.path(), 5)?;

    let mut store = OwnerStore::open(dir.path())?;
    for n in 0..80_u64 {
        let session = format!("extra-{n}");
        store.record(&hex_id(9_000 + n * 3), &establish(&session, 7_400))?;
        let exit = hex_id(9_001 + n * 3);
        store.record(&exit, &custody(&session, exited(&exit, None)))?;
        store.record(&hex_id(9_002 + n * 3), &retire(&session))?;
    }
    drop(store);
    let (more_time, more) = reopen_timed(dir.path(), 5)?;

    let mut store = OwnerStore::open(dir.path())?;
    store.checkpoint()?;
    drop(store);
    let (less_time, less) = reopen_timed(dir.path(), 5)?;

    let evidence = format!(
        "base {} in {base_time:?}; more {} in {more_time:?}; less {} in {less_time:?}",
        report(HotPath::Reconnect, base),
        report(HotPath::Reconnect, more),
        report(HotPath::Reconnect, less)
    );
    eprintln!("{evidence}");
    assert!(
        more.record_visits > base.record_visits && more.record_visits > less.record_visits,
        "the count tracks the added work: {evidence}"
    );
    assert!(
        less.record_visits <= base.record_visits,
        "the count tracks the removed work: {evidence}"
    );
    assert!(
        more_time >= less_time,
        "unqualified: the duration did not follow the count on this machine: {evidence}"
    );
    Ok(())
}

#[test]
fn seat_cost_counts_complete_transition() -> TestResult {
    let ratchet = Ratchet::parse(RATCHET)?;
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    let session = "complete";
    let before = store.counts();

    // The full successful transition through the durable owner: establish,
    // harness, accepted delivery cursors, stop, exit, retire, and one
    // duplicate readback of the exit.
    store.record(&hex_id(1), &establish(session, 7_500))?;
    store.record(
        &hex_id(2),
        &Intent::Harness {
            session: session.to_owned(),
            start: leader(7_501),
        },
    )?;
    store.record(
        &hex_id(3),
        &Intent::Cursors {
            session: session.to_owned(),
            cursors: Cursors {
                receipt: 7,
                feed: 3,
                hook: 2,
            },
        },
    )?;
    // No guard spans the instrumented work: a second reader opened now, with
    // the first store live, reads every committed line.
    let observer = OwnerStore::open(dir.path())?;
    assert_eq!(observer.last_seq(), 3);
    assert_eq!(observer.owner(session).map(|o| o.cursors.receipt), Some(7));
    drop(observer);
    store.record(
        &hex_id(4),
        &custody(
            session,
            Custody::Stopping {
                intent: hex_id(4),
                since: 1,
            },
        ),
    )?;
    let exit = custody(session, exited(&hex_id(5), None));
    store.record(&hex_id(5), &exit)?;
    let before_duplicate = store.counts();
    store.record(&hex_id(5), &exit)?;
    let duplicate = Work::between(before_duplicate, store.counts(), 1);
    assert_eq!(
        duplicate,
        Work {
            calls: 1,
            ..Work::default()
        },
        "a duplicate readback does no durable work"
    );
    store.record(&hex_id(6), &retire(session))?;
    let work = Work::between(before, store.counts(), 7);
    eprintln!("{}", report(HotPath::CustodyTransfer, work));
    assert_eq!(work.journal_appends, 6);
    assert_eq!(work.syncs, 6, "one physical sync per transition");
    assert!(work.bytes_copied > 0, "every copied byte is reported");
    let per_intent = Work {
        calls: 1,
        bytes_copied: ratchet.ceiling(HotPath::CustodyTransfer).bytes_copied / 4,
        journal_appends: 1,
        syncs: 1,
        ..Work::default()
    };
    assert!(
        Work::between(before, store.counts(), 1).bytes_copied / 6 <= per_intent.bytes_copied,
        "each line is within the ratchet's per-intent bytes: {}",
        report(HotPath::CustodyTransfer, work)
    );

    // An excess bound is refused before acceptance, and does no durable work.
    store.record(&hex_id(7), &establish("bounded", 7_600))?;
    let before_excess = store.counts();
    let oversized = custody(
        "bounded",
        exited(&hex_id(8), Some("r".repeat(MAX_MEMBER_BYTES))),
    );
    let refused = store
        .record(&hex_id(8), &oversized)
        .expect_err("an oversized member is refused");
    assert_eq!(refused.name(), "seat_owner_bound_exceeded");
    assert_eq!(
        Work::between(before_excess, store.counts(), 1),
        Work {
            calls: 1,
            ..Work::default()
        }
    );
    Ok(())
}

#[test]
fn seat_owner_idle_is_event_driven() -> TestResult {
    use lys_runner::seat_owner::counts::Meter;
    use lys_runner::seat_owner::spawn::wait_ready;
    use std::io::{BufReader, Write};

    // The runner's wait for an owner is a blocking read on a pipe. With the
    // ready signal held closed, no counter moves; releasing it does exactly
    // one read. No sleep, watchdog or timeout stands in for the signal.
    let (reader, mut writer) = std::io::pipe()?;
    let meter = Arc::new(Meter::default());
    let (done, waited) = std::sync::mpsc::channel();
    let counted = Arc::clone(&meter);
    let waiter = std::thread::spawn(move || {
        let result = wait_ready(&mut BufReader::new(reader), &counted);
        done.send(()).expect("the test is waiting");
        result
    });
    assert_eq!(
        meter.snapshot(),
        Work::default(),
        "nothing moves while the signal is held"
    );
    assert!(
        waited.try_recv().is_err(),
        "the waiter has not returned without its signal"
    );
    let own = Leader {
        pid: std::process::id(),
        start: StartIdentity("macos:1.000001".to_owned()),
    };
    let line = lys_runner::seat_owner::protocol::ready_line(
        "0f3c9a1e5b7d4c2a8e6f1b3d5a7c9e2f",
        std::path::Path::new("/tmp/owner.sock"),
        &own,
        "0.0.0-test",
    );
    writeln!(writer, "{line}")?;
    waited.recv()?;
    let (runner, socket, announced, build) = waiter.join().map_err(|_| "the waiter panicked")??;
    assert_eq!(build, "0.0.0-test");
    assert_eq!(runner, "0f3c9a1e5b7d4c2a8e6f1b3d5a7c9e2f");
    assert_eq!(socket, std::path::PathBuf::from("/tmp/owner.sock"));
    assert_eq!(announced, own);
    let work = meter.snapshot();
    assert_eq!(work.calls, 1, "exactly the declared work: one read");
    assert_eq!(work.wakes, 0, "no wake without a signal");
    assert_eq!(work.bytes_copied, u64::try_from(line.len() + 1)?);
    Ok(())
}
