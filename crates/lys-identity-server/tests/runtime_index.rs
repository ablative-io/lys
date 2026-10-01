#![cfg(test)]
//! A held budget view cannot make report appends copy the estate's session index.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::Ed25519Identity;
use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::{ORIGIN, RuntimeStore, SessionActivity};
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

#[derive(Clone)]
struct MemoryStore(Arc<Mutex<Kept>>);

impl MemoryStore {
    fn new() -> Self {
        let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
        Self(Arc::new(Mutex::new(Kept {
            leaves: Vec::new(),
            pin: PinnedRoot { root, tree_size },
            snapshot: None,
        })))
    }

    fn lock(&self) -> StoreResult<MutexGuard<'_, Kept>> {
        self.0.lock().map_err(|error| StoreError::Io {
            context: "memory_store_poisoned".to_owned(),
            source: std::io::Error::other(error.to_string()),
        })
    }
}

impl LeafStore for MemoryStore {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        u64::try_from(self.lock().expect("memory store lock").leaves.len())
            .expect("memory store extent fits u64")
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        let held = self.lock()?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| held.leaves.get(index).cloned()))
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        let mut held = self.lock()?;
        let next = u64::try_from(held.leaves.len()).expect("memory store extent fits u64");
        if index < next {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > next {
            return Err(StoreError::LeafWouldLeaveGap { index, next });
        }
        held.leaves.push(bytes.to_vec());
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.lock().expect("memory store lock").pin
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

fn allocation(
    view: impl Deref<Target = BTreeMap<String, SessionActivity>>,
) -> *const BTreeMap<String, SessionActivity> {
    std::ptr::from_ref(&*view)
}

#[test]
fn a_budget_view_keeps_the_map_allocation_across_reports_and_reads_only_selected_agents()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("runtime.key"),
    )?);
    let memory = MemoryStore::new();
    let mut store = RuntimeStore::over(Box::new(move || Ok(memory.clone())), key)?;
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
    let initial = allocation(store.agents_with_sessions()?);
    let reports: Vec<_> = (0..REPORTS / 2)
        .flat_map(|index| {
            let session = format!("reported-{index}");
            [
                report("agent-0", &session, Reported::Starting),
                report("agent-0", &session, Reported::Stopped),
            ]
        })
        .collect();
    let started = Instant::now();
    let mut mismatches = 0;
    for report in reports {
        store.report(report)?;
        if !std::ptr::eq(initial, allocation(store.agents_with_sessions()?)) {
            mismatches += 1;
        }
    }
    let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
    let seen: BTreeSet<_> = view.keys().cloned().collect();
    println!(
        "agents={AGENTS} initial_sessions={} reports={REPORTS} total_ms={elapsed:.3} per_report_ms={:.6} map_mismatches={mismatches} selected_agents={} read_agents={}",
        AGENTS * SESSIONS_PER_AGENT,
        elapsed / f64::from(REPORTS),
        selected.len(),
        seen.len()
    );
    assert_eq!(mismatches, 0, "a held budget view moved the session map");
    assert_eq!(seen, selected, "the budget read copied unrelated agents");
    assert_eq!(
        store.sessions().len(),
        usize::try_from(AGENTS * SESSIONS_PER_AGENT + REPORTS / 2)?
    );
    Ok(())
}
