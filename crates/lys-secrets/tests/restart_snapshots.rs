//! A start reads the audit log's signed snapshot and only the lines after
//! it. What the broker holds after such a start is what it holds after a
//! start from the whole log. A snapshot that cannot be believed is refused
//! by name, every line is read, and a new snapshot is written.

use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use lys_core::Ed25519Identity;
use lys_log_store::{Frontier, seal, unseal};
use lys_secrets::{
    Broker, BrokerPaths, EntryClass, Holder, IssuedHandle, LocalGrants, Presentation, Relation,
    STATE_DOMAIN, Secret, SecretRelation, SecretsError, SnapshotRefusal, Start, Used,
    new_operation_id,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Opened = Result<Broker<LocalGrants>, SecretsError>;

const START_MS: i64 = 1_800_000_000_000;
const LATER_MS: i64 = START_MS + 60_000;
const CALL: [u8; 32] = [7; 32];
const ORIGIN: &str = "lys.local/secrets-audit";
/// A count of lines no log here reaches, so no snapshot is written by count.
const NEVER: NonZeroU64 = NonZeroU64::MAX;

struct World {
    _dir: TempDir,
    paths: BrokerPaths,
    clock: Arc<AtomicI64>,
    agent: Ed25519Identity,
    holder: Holder,
}

/// The handles a busy log was written with.
struct Leases {
    spent: IssuedHandle,
    dropped: IssuedHandle,
    busy: IssuedHandle,
}

fn world() -> Result<World, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let keys = dir.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    let agent = Ed25519Identity::load_or_generate(&keys.join("agent.key"))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let paths = BrokerPaths {
        store_dir: dir.path().join("store"),
        log_dir: dir.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    Ok(World {
        _dir: dir,
        paths,
        clock: Arc::new(AtomicI64::new(START_MS)),
        agent,
        holder,
    })
}

fn clock(world: &World) -> lys_secrets::Clock {
    let shared = Arc::clone(&world.clock);
    Box::new(move || shared.load(Ordering::SeqCst))
}

fn relation(secret: &str) -> SecretRelation {
    SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: secret.to_owned(),
        granted_by: Some("person:tom".to_owned()),
    }
}

fn granted() -> LocalGrants {
    let grants = LocalGrants::new();
    grants.grant(relation("token"));
    grants.grant_as(Relation::Read, relation("notes"));
    grants
}

fn open(world: &World, every: NonZeroU64) -> Opened {
    Broker::open_every(&world.paths, granted(), clock(world), every)
}

fn snapshot(world: &World) -> PathBuf {
    world.paths.log_dir.join("snapshot.bin")
}

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

fn present(world: &World, issued: &IssuedHandle) -> Result<Presentation, SecretsError> {
    Presentation::sign(
        &issued.id,
        &new_operation_id()?,
        world.clock.load(Ordering::SeqCst),
        CALL,
        &world.agent,
    )
}

/// Uses `issued` `times` times, and answers the uses left after the last.
fn calls(
    world: &World,
    broker: &mut Broker<LocalGrants>,
    issued: &IssuedHandle,
    times: usize,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut left = 0;
    for _call in 0..times {
        let presentation = present(world, issued)?;
        match broker.use_handle(&issued.token, &presentation, Secret::len)? {
            Used::Forwarded { uses_left, .. } => left = uses_left,
            Used::Retried { outcome } => return Err(outcome.into()),
        }
    }
    Ok(left)
}

/// A new broker whose log holds a spent lease, a dropped one, one used
/// `uses` times of its hundreds, and a record read.
fn busy(world: &World, uses: usize) -> Result<Leases, Box<dyn std::error::Error>> {
    let mut broker = Broker::create(&world.paths, granted(), clock(world))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"value-one"))?;
    broker.seal_record(
        "notes",
        EntryClass::Memory,
        "person:tom",
        &Secret::from_slice(b"the meeting is at nine"),
    )?;
    let spent = broker.issue(&world.holder, "token", 1, LATER_MS)?;
    let dropped = broker.issue(&world.holder, "token", 1, LATER_MS)?;
    let busy = broker.issue(&world.holder, "token", 900, LATER_MS)?;
    calls(world, &mut broker, &spent, 1)?;
    broker.drop_handle(&dropped.id)?;
    broker.read_record("agent:noor", "notes", None)?;
    calls(world, &mut broker, &busy, uses)?;
    Ok(Leases {
        spent,
        dropped,
        busy,
    })
}

#[test]
fn a_start_after_a_snapshot_reads_only_the_lines_after_it() -> TestResult {
    let world = world()?;
    let leases = busy(&world, 150)?;

    let mut first = open(&world, NEVER)?;
    let whole = first.audit().len();
    assert!(whole > 300, "the log holds {whole} lines");
    assert_eq!(
        first.start(),
        &Start::Rebuilt {
            refusal: SnapshotRefusal::Missing,
            replayed: whole
        }
    );
    calls(&world, &mut first, &leases.busy, 3)?;
    let after = first.audit().len() - whole;
    assert_eq!(after, 6, "each use is its admission and its settlement");
    drop(first);

    let second = open(&world, NEVER)?;
    assert_eq!(
        second.start(),
        &Start::Resumed {
            size: whole,
            replayed: after
        },
        "of {} lines the start reads the {after} after the snapshot",
        whole + after
    );
    Ok(())
}

#[test]
fn a_snapshot_is_written_each_time_the_log_crosses_the_count() -> TestResult {
    let world = world()?;
    let leases = busy(&world, 2)?;
    let every = NonZeroU64::new(8).ok_or("eight is not zero")?;

    let mut first = open(&world, every)?;
    calls(&world, &mut first, &leases.busy, 21)?;
    let len = first.audit().len();
    assert_eq!(first.snapshot_failure(), None);
    drop(first);

    let second = open(&world, NEVER)?;
    assert_eq!(
        second.start(),
        &Start::Resumed {
            size: len - len % 8,
            replayed: len % 8
        }
    );
    Ok(())
}

#[test]
fn a_resumed_start_holds_what_a_start_from_the_whole_log_holds() -> TestResult {
    let world = world()?;
    let leases = busy(&world, 9)?;
    let every = NonZeroU64::new(4).ok_or("four is not zero")?;
    let mut first = open(&world, every)?;
    calls(&world, &mut first, &leases.busy, 5)?;
    first.seal("other", "person:tom", &Secret::from_slice(b"value-two"))?;
    drop(first);

    let resumed = open(&world, NEVER)?;
    assert!(
        matches!(resumed.start(), Start::Resumed { replayed, .. } if *replayed < 4),
        "{}",
        resumed.start()
    );
    let from_the_snapshot = resumed.folded()?;
    drop(resumed);

    std::fs::remove_file(snapshot(&world))?;
    let rebuilt = open(&world, NEVER)?;
    assert_eq!(rebuilt.start().refusal(), Some(&SnapshotRefusal::Missing));
    assert_eq!(rebuilt.folded()?, from_the_snapshot);
    drop(rebuilt);

    let mut again = open(&world, NEVER)?;
    assert!(
        matches!(again.start(), Start::Resumed { replayed: 0, .. }),
        "{}",
        again.start()
    );
    assert_eq!(again.folded()?, from_the_snapshot);
    let spent = present(&world, &leases.spent)?;
    assert_eq!(
        refusal(again.use_handle(&leases.spent.token, &spent, Secret::len)),
        "LeaseExhausted"
    );
    let gone = present(&world, &leases.dropped)?;
    assert_eq!(
        refusal(again.use_handle(&leases.dropped.token, &gone, Secret::len)),
        "HandleDropped"
    );
    assert_eq!(calls(&world, &mut again, &leases.busy, 1)?, 900 - 9 - 5 - 1);
    again
        .permissions()
        .revoke_as(Relation::Read, "agent:noor", "notes");
    assert_eq!(
        refusal(again.read_record("agent:noor", "notes", None)),
        "RelationRemoved"
    );
    Ok(())
}

/// A snapshot the audit key signed, of `state` at `frontier`, in place of
/// the one the broker wrote.
fn forge(
    world: &World,
    domain: &str,
    frontier: &Frontier,
    state: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let key = Ed25519Identity::load(&world.paths.audit_key)?;
    let sealed = seal(domain, ORIGIN, frontier, state, &key);
    Ok(std::fs::write(snapshot(world), sealed)?)
}

fn damaged(
    world: &World,
    name: &str,
    written: &[u8],
    len: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = Ed25519Identity::load(&world.paths.audit_key)?;
    let honest = unseal(written, STATE_DOMAIN, ORIGIN, &key.public_key_bytes())?;
    let lines = |count: u64| Frontier::from_leaves((0..count).map(u64::to_be_bytes));
    match name {
        "SnapshotMissing" => std::fs::remove_file(snapshot(world))?,
        "SnapshotMalformed" => std::fs::write(snapshot(world), &written[..written.len() / 2])?,
        "SnapshotSignatureInvalid" => {
            let mut bytes = written.to_vec();
            if let Some(last) = bytes.last_mut() {
                *last ^= 1;
            }
            std::fs::write(snapshot(world), bytes)?;
        }
        "SnapshotWrongKind" => forge(world, "another/state", honest.frontier(), honest.state())?,
        "SnapshotWrongRoot" => forge(world, STATE_DOMAIN, &lines(len), honest.state())?,
        "SnapshotBeyondLog" => forge(world, STATE_DOMAIN, &lines(len + 1), honest.state())?,
        "SnapshotStateUnreadable" => {
            forge(world, STATE_DOMAIN, honest.frontier(), b"not a state")?;
        }
        other => return Err(format!("no damage is known by the name {other}").into()),
    }
    Ok(())
}

#[test]
fn a_snapshot_that_cannot_be_believed_is_refused_by_name_and_every_line_is_read() -> TestResult {
    for name in [
        "SnapshotMissing",
        "SnapshotMalformed",
        "SnapshotSignatureInvalid",
        "SnapshotWrongKind",
        "SnapshotWrongRoot",
        "SnapshotBeyondLog",
        "SnapshotStateUnreadable",
    ] {
        let world = world()?;
        let leases = busy(&world, 12)?;
        drop(open(&world, NEVER)?);
        let honest = open(&world, NEVER)?;
        assert!(matches!(honest.start(), Start::Resumed { .. }), "{name}");
        let (held, len) = (honest.folded()?, honest.audit().len());
        drop(honest);

        let written = std::fs::read(snapshot(&world))?;
        damaged(&world, name, &written, len)?;
        let mut rebuilt = open(&world, NEVER)?;
        let Start::Rebuilt { refusal, replayed } = rebuilt.start().clone() else {
            return Err(format!("{name}: the damaged snapshot was used").into());
        };
        assert!(refusal.to_string().starts_with(name), "{name}: {refusal}");
        assert_eq!(replayed, len, "{name}");
        assert_eq!(rebuilt.folded()?, held, "{name}");
        assert_eq!(calls(&world, &mut rebuilt, &leases.busy, 1)?, 900 - 12 - 1);
        drop(rebuilt);

        assert_eq!(std::fs::read(snapshot(&world))?, written, "{name}");
        let resumed = open(&world, NEVER)?;
        assert_eq!(
            resumed.start(),
            &Start::Resumed {
                size: len,
                replayed: 2
            },
            "{name}"
        );
    }
    Ok(())
}
