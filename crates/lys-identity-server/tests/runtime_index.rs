//! A runtime report goes to its own session: with ten thousand sessions
//! kept, one report examines the one session it reports on and never walks
//! the others, and what is sealed in the snapshot is the same bytes the
//! sessions alone would seal.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity_server::runtime_state::{Held, Report, Reported};
use lys_identity_server::runtime_store::{ORIGIN, RuntimeStore};
use lys_log_store::{Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn Error>>;

const MACHINE: &str = "op-00000000000000000000000000000001";

/// What an in-memory store keeps, shared by every handle opened on it.
#[derive(Default)]
struct Disk {
    leaves: Vec<Vec<u8>>,
    pinned: Option<PinnedRoot>,
    snapshot: Option<Vec<u8>>,
}

/// A leaf store held in memory, so ten thousand appends cost no disk.
struct Memory {
    disk: Arc<Mutex<Disk>>,
}

impl Memory {
    fn disk(&self) -> std::sync::MutexGuard<'_, Disk> {
        self.disk.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl LeafStore for Memory {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        self.disk().leaves.len() as u64
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        Ok(usize::try_from(index)
            .ok()
            .and_then(|slot| self.disk().leaves.get(slot).cloned()))
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        let next = self.extent();
        if index != next {
            return Err(StoreError::LeafWouldLeaveGap { index, next });
        }
        self.disk().leaves.push(bytes.to_vec());
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.disk().pinned.unwrap_or(PinnedRoot {
            tree_size: 0,
            root: Frontier::new().root(),
        })
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.disk().pinned = Some(pin);
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.disk().snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.disk().snapshot = Some(bytes.to_vec());
        Ok(())
    }
}

fn operation(n: u32) -> String {
    format!("op-{n:032x}")
}

fn session(n: u32) -> String {
    format!("op-{:032x}", 1_000_000 + n)
}

fn report(operation: String, session: String, state: Reported) -> Report {
    Report {
        operation,
        session,
        agent: Some("agent-a".to_owned()),
        machine: MACHINE.to_owned(),
        state,
        what: String::new(),
        confirmation: String::new(),
        reported_by: "agent-a".to_owned(),
        at: 5,
        launch: None,
    }
}

fn opened(disk: &Arc<Mutex<Disk>>) -> Result<RuntimeStore<Memory>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("runtime.key"),
    )?);
    let shared = Arc::clone(disk);
    Ok(RuntimeStore::over(
        Box::new(move || {
            Ok(Memory {
                disk: Arc::clone(&shared),
            })
        }),
        key,
    )?)
}

#[test]
fn one_report_examines_its_own_session_among_ten_thousand() -> TestResult {
    let disk = Arc::new(Mutex::new(Disk::default()));
    let mut store = opened(&disk)?;
    for n in 0..10_000 {
        store.report(report(operation(n), session(n), Reported::Starting))?;
    }
    assert_eq!(store.sessions().len(), 10_000);

    let before = store.examined();
    let tracked = store.report(report(operation(10_000), session(4_000), Reported::Running))?;
    assert_eq!(tracked.session, session(4_000));
    assert_eq!(tracked.reports.len(), 2);
    assert_eq!(
        store.examined() - before,
        1,
        "a report on a kept session examines that session alone"
    );

    let before = store.examined();
    let again = store.report(report(operation(10_000), session(4_000), Reported::Running))?;
    assert_eq!(again.reports.len(), 2, "the same report is kept once");
    assert_eq!(
        store.examined() - before,
        1,
        "a report sent again examines the one session it was kept on"
    );
    Ok(())
}

#[test]
fn the_index_is_never_sealed() -> TestResult {
    let mut held = Held::default();
    for n in 0..3 {
        held.hold(report(operation(n), session(n), Reported::Starting))?;
    }
    held.hold(report(operation(3), session(1), Reported::Running))?;
    let sealed = held.encode()?;
    let expected = serde_json::to_vec(&serde_json::json!({
        "format": "lys-runtime-state/v1",
        "held": { "sessions": held.sessions },
    }))?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&sealed)?,
        serde_json::from_slice::<serde_json::Value>(&expected)?,
        "the snapshot carries the sessions and nothing beside them"
    );
    let read = Held::decode(&sealed)?;
    assert_eq!(read, held);
    assert_eq!(read.encode()?, sealed, "read back, it seals the same");
    assert_eq!(
        read.session(&session(1)).map(|tracked| tracked.reports.len()),
        Some(2),
        "the index is rebuilt when a sealed state is read"
    );
    assert_eq!(
        read.operation(&operation(3)).map(|kept| kept.state),
        Some(Reported::Running)
    );
    Ok(())
}
