//! A held budget view cannot make report appends copy the estate's session index.

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::Ed25519Identity;
use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::{ORIGIN, RuntimeStore};
use lys_log_store::{LeafStore, PinnedRoot, StoreError, StoreResult};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const AGENTS: u32 = 200;
const SESSIONS_PER_AGENT: u32 = 5;
const REPORTS: u32 = 1_000;

struct Kept {
    leaves: Vec<Vec<u8>>,
    pin: PinnedRoot,
    snapshot: Option<Vec<u8>>,
}

struct MemoryStore {
    kept: Arc<Mutex<Kept>>,
    extent: u64,
    pin: PinnedRoot,
}

impl MemoryStore {
    fn empty() -> Arc<Mutex<Kept>> {
        let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
        Arc::new(Mutex::new(Kept {
            leaves: Vec::new(),
            pin: PinnedRoot { root, tree_size },
            snapshot: None,
        }))
    }

    fn open(kept: Arc<Mutex<Kept>>) -> StoreResult<Self> {
        let held = kept.lock().map_err(poisoned)?;
        let extent = u64::try_from(held.leaves.len()).map_err(|error| StoreError::Io {
            context: "memory_store_extent_overflow".to_owned(),
            source: std::io::Error::other(error),
        })?;
        let pin = held.pin;
        drop(held);
        Ok(Self { kept, extent, pin })
    }

    fn lock(&self) -> StoreResult<MutexGuard<'_, Kept>> {
        self.kept.lock().map_err(poisoned)
    }
}

fn poisoned(error: impl std::fmt::Display) -> StoreError {
    StoreError::Io {
        context: "memory_store_poisoned".to_owned(),
        source: std::io::Error::other(error.to_string()),
    }
}

impl LeafStore for MemoryStore {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        self.extent
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        let held = self.lock()?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| held.leaves.get(index).cloned()))
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        let mut held = self.lock()?;
        let next = u64::try_from(held.leaves.len()).map_err(|error| StoreError::Io {
            context: "memory_store_extent_overflow".to_owned(),
            source: std::io::Error::other(error),
        })?;
        if index < next {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > next {
            return Err(StoreError::LeafWouldLeaveGap { index, next });
        }
        let following = next.checked_add(1).ok_or_else(|| StoreError::Io {
            context: "memory_store_extent_overflow".to_owned(),
            source: std::io::Error::other("one more leaf exceeds the store extent"),
        })?;
        held.leaves.push(bytes.to_vec());
        drop(held);
        self.extent = following;
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.pin
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        let mut held = self.lock()?;
        if pin.tree_size < held.pin.tree_size {
            return Err(StoreError::PinWentBackwards {
                pinned: held.pin.tree_size,
                requested: pin.tree_size,
            });
        }
        if pin.tree_size == held.pin.tree_size && pin.root != held.pin.root {
            return Err(StoreError::PinRootChanged {
                tree_size: pin.tree_size,
                held: STANDARD.encode(held.pin.root),
                offered: STANDARD.encode(pin.root),
            });
        }
        held.pin = pin;
        drop(held);
        self.pin = pin;
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.lock()?.snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.lock()?.snapshot = Some(bytes.to_vec());
        Ok(())
    }
}

fn report(agent: &str, session: &str, state: Reported) -> Report {
    Report {
        operation: format!("{session}-{}", state.name()),
        session: session.to_owned(),
        agent: Some(agent.to_owned()),
        machine: "machine".to_owned(),
        state,
        what: String::new(),
        confirmation: if state == Reported::Stopped {
            "process exited".to_owned()
        } else {
            String::new()
        },
        reported_by: agent.to_owned(),
        at: 1,
        launch: None,
    }
}

#[test]
fn a_budget_view_retains_a_selected_snapshot_without_whole_map_copies() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("runtime.key"),
    )?);
    let memory = MemoryStore::empty();
    let mut store = RuntimeStore::over(
        Box::new(move || MemoryStore::open(Arc::clone(&memory))),
        key,
    )?;
    for agent in 0..AGENTS {
        for session in 0..SESSIONS_PER_AGENT {
            store.report(report(
                &format!("agent-{agent}"),
                &format!("session-{agent}-{session}"),
                Reported::Starting,
            ))?;
        }
    }
    let selected = BTreeSet::from(["agent-0".to_owned()]);
    let view = store.agents_with_sessions()?;
    let initial: BTreeSet<_> = (0..SESSIONS_PER_AGENT)
        .map(|index| format!("session-0-{index}"))
        .collect();
    let reports: Vec<_> = (0..REPORTS / 2)
        .flat_map(|index| {
            let session = format!("reported-{index}");
            [
                report("agent-0", &session, Reported::Starting),
                report(
                    &format!("agent-{}", AGENTS / 2 + index / SESSIONS_PER_AGENT),
                    &format!(
                        "session-{}-{}",
                        AGENTS / 2 + index / SESSIONS_PER_AGENT,
                        index % SESSIONS_PER_AGENT
                    ),
                    Reported::Stopped,
                ),
            ]
        })
        .collect();
    let started = Instant::now();
    let mut copies = 0;
    for report in reports {
        store.report_observing(report, |count| copies += count)?;
    }
    let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
    let seen: BTreeSet<_> = view.keys().cloned().collect();
    println!(
        "agents={AGENTS} initial_sessions={} reports={REPORTS} total_ms={elapsed:.3} per_report_ms={:.6} whole_map_copies={copies} selected_agents={} read_agents={}",
        AGENTS * SESSIONS_PER_AGENT,
        elapsed / f64::from(REPORTS),
        selected.len(),
        seen.len()
    );
    assert_eq!(
        view.get("agent-0")
            .ok_or("agent missing from held view")?
            .live_sessions(),
        &initial,
        "the held view changed after reports"
    );
    let fresh = store.agents_with_sessions()?;
    let mut expected = initial;
    expected.extend((0..REPORTS / 2).map(|index| format!("reported-{index}")));
    assert_eq!(
        fresh
            .get("agent-0")
            .ok_or("agent missing from fresh view")?
            .live_sessions(),
        &expected,
        "a fresh read must include all 500 new sessions"
    );
    assert_eq!(copies, 0, "a held budget view copied the whole session map");
    assert_eq!(seen, selected, "the budget read copied unrelated agents");
    assert_eq!(
        store.sessions().len(),
        usize::try_from(AGENTS * SESSIONS_PER_AGENT + REPORTS / 2)?
    );
    Ok(())
}
