//! AGENTS-004 R4: the versioned owner store migrates an actual installed
//! `lys-runner-sessions/v3` directory, commits atomically, refuses by name
//! and recovers from a bounded projection and tail.
//!
//! Every case reads its figures back from the directory: file bytes before
//! and after, the counts the store reports, the refusal name a call returns.
//! Nothing waits on a timer.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use lys_runner::error::RunnerError;
use lys_runner::peer::{Leader, StartIdentity};
use lys_runner::protocol::{Ended, EndedHow};
use lys_runner::seat_owner::store::{
    Applied, Cursors, Custody, FORMAT, Holder, Intent, Lease, MAX_CREDENTIAL_REFERENCES,
    MAX_MEMBER_BYTES, MAX_TAIL, OwnerRecord, OwnerStore,
};
use lys_runner::state::{Kept, StateFile};

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/seat_owner_installed_v3.json");

/// The file `replace` writes beside the projection before renaming it over.
const PROJECTION_BESIDE: &str = "seat-owners.writing";

/// Lays the captured old install out in `dir`: the v3 sessions record byte
/// for byte, and in the operations store's place a directory holding one
/// file per uncertain control id, so any attempt to open the operations
/// journal as a file fails loudly.
fn install_fixture(dir: &Path) -> Result<(Vec<u8>, Vec<String>), Box<dyn Error>> {
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE)?;
    let sessions = serde_json::to_vec_pretty(&fixture["sessions.json"])?;
    fs::write(dir.join("sessions.json"), &sessions)?;
    let uncertain: Vec<String> = serde_json::from_value(fixture["uncertain_control_ids"].clone())?;
    let operations = dir.join("operations.v2.journal");
    fs::create_dir(&operations)?;
    for id in &uncertain {
        fs::write(operations.join(id), b"uncertain\n")?;
    }
    Ok((sessions, uncertain))
}

fn leader(pid: u32) -> Leader {
    Leader {
        pid,
        start: StartIdentity(format!("boot-{pid}")),
    }
}

fn hex_id(n: u64) -> String {
    format!("{n:032x}")
}

fn record(session: &str, pid: u32) -> OwnerRecord {
    OwnerRecord {
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
    }
}

fn establish(session: &str, pid: u32) -> Intent {
    Intent::Establish {
        record: Box::new(record(session, pid)),
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

fn custody(session: &str, custody: Custody) -> Intent {
    Intent::Custody {
        session: session.to_owned(),
        custody,
    }
}

fn refusal<T: std::fmt::Debug>(result: Result<T, RunnerError>) -> String {
    match result {
        Ok(value) => panic!("expected a refusal, got {value:?}"),
        Err(error) => error.name(),
    }
}

fn read_installed(dir: &Path) -> Result<Kept, Box<dyn Error>> {
    // The reader takes the runner lock while it lives; it is dropped before
    // the store is opened, as a runner reads first and holds nothing twice.
    let state = StateFile::open(dir)?;
    let kept = state.read()?;
    drop(state);
    Ok(kept)
}

fn written_checkpoint(dir: &Path) -> Result<u64, Box<dyn Error>> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("seat-owners.json"))?)?;
    Ok(value["checkpoint"]
        .as_u64()
        .ok_or("the projection names no checkpoint")?)
}

#[test]
fn seat_owner_migrates_installed_v3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (sessions_before, uncertain) = install_fixture(dir.path())?;
    let operations_before: BTreeMap<String, Vec<u8>> = uncertain
        .iter()
        .map(|id| {
            let path = dir.path().join("operations.v2.journal").join(id);
            fs::read(&path).map(|bytes| (id.clone(), bytes))
        })
        .collect::<Result<_, _>>()?;
    assert!(
        !dir.path().join("seat-owners.json").exists(),
        "an old install has no owner file; that absence is how it is told"
    );

    let installed = read_installed(dir.path())?;
    assert_eq!(installed.format, lys_runner::state::FORMAT);
    assert_eq!(installed.sessions.len(), 2);

    let migrated = OwnerStore::migrate_installed(dir.path(), &installed)?;
    assert_eq!(migrated.installed_format, lys_runner::state::FORMAT);
    assert_eq!(
        migrated.live_manual, 1,
        "the live manual session is counted"
    );
    assert_eq!(migrated.ended, 1, "the ended session is counted");
    assert_eq!(
        migrated.owners, 0,
        "manual state is never promoted to a seat"
    );
    assert!(!migrated.already_versioned);

    // The installed bytes are exactly as they were: ended session, manual
    // meaning, the responsible person.
    assert_eq!(fs::read(dir.path().join("sessions.json"))?, sessions_before);
    let reread = read_installed(dir.path())?;
    assert_eq!(reread, installed);
    assert_eq!(
        reread
            .responsible
            .get("manual-live-7c1f")
            .map(String::as_str),
        Some("tom")
    );
    assert!(reread.sessions[1].ended.is_some());

    // The operations store was never opened: every id is where it was, and
    // none was copied into the owner record.
    for (id, bytes) in &operations_before {
        let path = dir.path().join("operations.v2.journal").join(id);
        assert_eq!(&fs::read(&path)?, bytes, "{id} stays where it was");
    }
    let projection = fs::read_to_string(dir.path().join("seat-owners.json"))?;
    for id in &uncertain {
        assert!(
            !projection.contains(id),
            "{id} is not the owner store's to hold"
        );
    }

    // The new record has its explicit version and reopens as empty.
    let value: serde_json::Value = serde_json::from_str(&projection)?;
    assert_eq!(value["format"], FORMAT);
    assert_eq!(value["checkpoint"], 0);
    let store = OwnerStore::open(dir.path())?;
    assert!(store.owners().is_empty());
    assert_eq!(store.counts().record_visits, 0);

    // A second upgrade over a versioned directory writes nothing.
    let mtime = fs::metadata(dir.path().join("seat-owners.json"))?.modified()?;
    let again = OwnerStore::migrate_installed(dir.path(), &installed)?;
    assert!(again.already_versioned);
    assert_eq!(
        fs::metadata(dir.path().join("seat-owners.json"))?.modified()?,
        mtime
    );

    // An installed record of another version is refused by name, never read
    // as empty, and nothing is written.
    let mut other = installed.clone();
    other.format = "lys-runner-sessions/v2".to_owned();
    let fresh = tempfile::tempdir()?;
    assert_eq!(
        refusal(OwnerStore::migrate_installed(fresh.path(), &other)),
        "seat_owner_format_unknown"
    );
    assert!(!fresh.path().join("seat-owners.json").exists());
    Ok(())
}

#[test]
fn seat_owner_migration_is_atomic() -> TestResult {
    let dir = tempfile::tempdir()?;
    install_fixture(dir.path())?;
    let installed = read_installed(dir.path())?;

    // Exit at the projection write barrier: the file written beside the
    // projection by an exit mid-write is not the projection. The upgrade
    // replaces whole, so the leftover goes and the installed state was
    // authoritative all along.
    fs::write(dir.path().join(PROJECTION_BESIDE), b"{\"half\":")?;
    let migrated = OwnerStore::migrate_installed(dir.path(), &installed)?;
    assert_eq!(migrated.owners, 0);
    assert!(!dir.path().join(PROJECTION_BESIDE).exists());

    // The first custody intent. Its append reply is lost: the same id is
    // sent again and the store answers the same sequence, with one line.
    let mut store = OwnerStore::open(dir.path())?;
    let first = store.record(&hex_id(1), &establish("seat-session-1", 5_001))?;
    assert_eq!(first, Applied::Recorded { seq: 1 });
    let resent = store.record(&hex_id(1), &establish("seat-session-1", 5_001))?;
    assert_eq!(resent, Applied::Already { seq: 1 });
    assert_eq!(store.counts().journal_appends, 1);
    let journal = fs::read_to_string(dir.path().join("seat-owners.journal"))?;
    assert_eq!(
        journal.lines().count(),
        1,
        "an uncertain append never becomes two"
    );

    // A different id is a different intent: the same establish under a new
    // id is refused because the owner is held, not applied twice.
    assert_eq!(
        refusal(store.record(&hex_id(2), &establish("seat-session-1", 5_001))),
        "seat_owner_held"
    );
    assert_eq!(store.owners().len(), 1, "exactly one committed owner");

    // Exit at the checkpoint barrier: the old checkpoint stays readable until
    // its replacement is durable. A leftover beside a projection whose
    // checkpoint is 0, plus the tail, reopens to the same one owner.
    fs::write(dir.path().join(PROJECTION_BESIDE), b"{\"half\":")?;
    let reopened = OwnerStore::open(dir.path())?;
    assert_eq!(reopened.owners().len(), 1);
    assert_eq!(reopened.last_seq(), 1);
    assert_eq!(
        reopened
            .owner("seat-session-1")
            .map(|owner| owner.lease.generation),
        Some(1)
    );

    // Exit mid-append: a torn trailing line is not skipped and not read as
    // nothing. It is refused by name and the committed files are untouched.
    let projection_before = fs::read(dir.path().join("seat-owners.json"))?;
    let mut torn = fs::read(dir.path().join("seat-owners.journal"))?;
    torn.extend_from_slice(b"{\"seq\":2,\"intent\":\"");
    fs::write(dir.path().join("seat-owners.journal"), &torn)?;
    assert_eq!(
        refusal(OwnerStore::open(dir.path())),
        "seat_owner_record_invalid"
    );
    assert_eq!(
        fs::read(dir.path().join("seat-owners.json"))?,
        projection_before
    );
    assert_eq!(fs::read(dir.path().join("seat-owners.journal"))?, torn);
    Ok(())
}

#[test]
fn seat_owner_store_refuses_named_errors() -> TestResult {
    // Malformed version: another version is named, a missing one is invalid.
    let dir = tempfile::tempdir()?;
    fs::write(
        dir.path().join("seat-owners.json"),
        b"{\"format\":\"lys-runner-seat-owners/v0\",\"checkpoint\":0,\"owners\":{}}",
    )?;
    assert_eq!(
        refusal(OwnerStore::open(dir.path())),
        "seat_owner_format_unknown"
    );
    let dir = tempfile::tempdir()?;
    fs::write(
        dir.path().join("seat-owners.json"),
        b"{\"checkpoint\":0,\"owners\":{}}",
    )?;
    assert_eq!(
        refusal(OwnerStore::open(dir.path())),
        "seat_owner_record_invalid"
    );

    // Oversized custody member: an exit whose reason is over the member
    // bound is refused before anything is appended; the owner stays as it
    // was. A handover whose intent is not an id is invalid by name.
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    store.record(&hex_id(10), &establish("s", 5_002))?;
    let oversized = custody("s", exited(&hex_id(11), Some("r".repeat(MAX_MEMBER_BYTES))));
    assert_eq!(
        refusal(store.record(&hex_id(11), &oversized)),
        "seat_owner_bound_exceeded"
    );
    let bad_handover = custody(
        "s",
        Custody::HandingOver {
            intent: "not an id".to_owned(),
            successor: leader(5_003),
            since: 1,
        },
    );
    assert_eq!(
        refusal(store.record(&hex_id(12), &bad_handover)),
        "seat_owner_record_invalid"
    );
    assert_eq!(
        store.owner("s").map(|owner| owner.custody.clone()),
        Some(Custody::Owned)
    );
    assert_eq!(store.counts().journal_appends, 1);

    // Credential references: a value, an empty name, an oversized name and
    // too many are each refused by name, without the bytes, with no partial
    // success.
    let with_references = |references: Vec<String>| Intent::Establish {
        record: Box::new(OwnerRecord {
            credential_references: references,
            ..record("u", 5_005)
        }),
    };
    let error = match store.record(
        &hex_id(13),
        &with_references(vec!["token=sk-live-0000".to_owned()]),
    ) {
        Ok(applied) => panic!("a value was accepted as a reference: {applied:?}"),
        Err(error) => error,
    };
    assert_eq!(error.name(), "seat_owner_record_invalid");
    assert!(
        !error.to_string().contains("sk-live"),
        "the refusal carries no credential bytes: {error}"
    );
    for references in [
        vec![String::new()],
        vec!["n".repeat(MAX_MEMBER_BYTES)],
        (0..=MAX_CREDENTIAL_REFERENCES)
            .map(|n| format!("reference-{n}"))
            .collect(),
    ] {
        assert_eq!(
            refusal(store.record(&hex_id(14), &with_references(references))),
            "seat_owner_record_invalid"
        );
    }
    assert!(store.owner("u").is_none(), "no partial success");
    assert_eq!(store.counts().journal_appends, 1);
    Ok(())
}

#[test]
fn seat_owner_store_refuses_a_durable_write_by_name() -> TestResult {
    // A journal that cannot be read is refused on open.
    let dir = tempfile::tempdir()?;
    fs::create_dir(dir.path().join("seat-owners.journal"))?;
    assert_eq!(
        refusal(OwnerStore::open(dir.path())),
        "seat_owner_store_unavailable"
    );

    // An append that cannot be made durable is refused by name and the
    // projection in memory does not move.
    let dir = tempfile::tempdir()?;
    let mut store = OwnerStore::open(dir.path())?;
    store.record(&hex_id(20), &establish("v", 5_006))?;
    let journal = dir.path().join("seat-owners.journal");
    fs::remove_file(&journal)?;
    fs::create_dir(&journal)?;
    let stopping = custody(
        "v",
        Custody::Stopping {
            intent: hex_id(21),
            since: 2,
        },
    );
    let name = refusal(store.record(&hex_id(21), &stopping));
    assert!(
        name == "seat_owner_append_refused" || name == "seat_owner_store_unavailable",
        "a refused append is named for its stage, got {name}"
    );
    assert_eq!(
        store.owner("v").map(|owner| owner.custody.clone()),
        Some(Custody::Owned)
    );
    assert_eq!(store.last_seq(), 1);

    // A checkpoint that cannot be written is refused by name and the
    // projection file is what it was.
    fs::remove_dir(&journal)?;
    let projection = dir.path().join("seat-owners.json");
    fs::create_dir(dir.path().join(PROJECTION_BESIDE))?;
    assert_eq!(refusal(store.checkpoint()), "seat_owner_checkpoint_refused");
    assert!(!projection.exists());
    Ok(())
}

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
