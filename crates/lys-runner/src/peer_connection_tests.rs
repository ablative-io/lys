#![cfg(test)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::{Connection, Leader, Processes, Proof, StartIdentity};
use crate::{Launch, RunnerError, Sessions};

struct Tree {
    leader: Leader,
    peer_start: StartIdentity,
    parents: Cell<usize>,
    starts: Cell<usize>,
}

impl Processes for Tree {
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        self.parents.set(self.parents.get() + 1);
        if pid == u32::MAX {
            Ok(self.leader.pid)
        } else {
            Err(RunnerError::refused("peer_unproved", "no parent"))
        }
    }

    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        self.starts.set(self.starts.get() + 1);
        if pid == u32::MAX {
            Ok(self.peer_start.clone())
        } else if pid == self.leader.pid {
            Ok(self.leader.start.clone())
        } else {
            Err(RunnerError::refused("peer_unproved", "no process start"))
        }
    }
}

struct Held {
    dir: tempfile::TempDir,
    sessions: Arc<Sessions>,
}

impl Held {
    fn open() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 4096)?;
        sessions.start(Launch {
            session: "proved".to_owned(),
            program: "/bin/cat".to_owned(),
            arguments: Vec::new(),
            directory: "/".to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        })?;
        Ok(Self { dir, sessions })
    }

    fn tree(&self) -> Result<Tree, Box<dyn Error>> {
        let leader = self
            .sessions
            .guard("proved")?
            .and_then(|guard| guard.leader)
            .ok_or("session has no leader")?;
        Ok(Tree {
            leader,
            peer_start: StartIdentity("peer-start".to_owned()),
            parents: Cell::new(0),
            starts: Cell::new(0),
        })
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = self.sessions.stop_all() {
            crate::error::said(&format!("connection fixture could not stop: {error}"));
        }
    }
}

#[test]
fn a_connection_walks_once_and_rechecks_only_its_process_starts() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    let tree = held.tree()?;
    let mut connection = Connection::default();
    let first = connection.proved_with(&held.sessions, &tree, (u32::MAX, 5, 5))?;
    assert_eq!(first.session, "proved");
    assert_eq!(tree.parents.get(), 1);
    assert_eq!(tree.starts.get(), 4);
    for _ in 0..32 {
        assert_eq!(
            connection
                .proved_with(&held.sessions, &tree, (u32::MAX, 5, 5))?
                .session,
            "proved"
        );
    }
    assert_eq!(tree.parents.get(), 1);
    assert_eq!(tree.starts.get(), 68);
    Ok(())
}

#[test]
fn a_connection_refuses_a_reused_peer_pid_without_another_walk() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    let mut tree = held.tree()?;
    let mut connection = Connection::default();
    connection.proved_with(&held.sessions, &tree, (u32::MAX, 5, 5))?;
    tree.peer_start = StartIdentity("replacement-peer".to_owned());
    let error = connection
        .proved_with(&held.sessions, &tree, (u32::MAX, 5, 5))
        .expect_err("a reused process id kept its connection proof");
    assert_eq!(error.name(), "peer_unproved");
    assert_eq!(tree.parents.get(), 1);
    Ok(())
}

#[test]
fn a_connection_refuses_an_ended_or_restarted_launch() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    let tree = held.tree()?;
    let mut connection = Connection::default();
    connection.proved_with(&held.sessions, &tree, (u32::MAX, 5, 5))?;
    held.sessions
        .restart_proved("proved", "replace-proved", &tree.leader)?;
    let operations = std::fs::read_to_string(held.dir.path().join("operations.jsonl"))?;
    assert!(operations.contains("replace-proved"));
    let after = held.tree()?;
    let error = connection
        .proved_with(&held.sessions, &after, (u32::MAX, 5, 5))
        .expect_err("a restarted launch kept the old connection proof");
    assert_eq!(error.name(), "peer_unproved");
    assert_eq!(after.parents.get(), 0);
    let mut fresh = Connection::default();
    fresh.proved_with(&held.sessions, &after, (u32::MAX, 5, 5))?;
    held.sessions.end("proved", &AtomicBool::new(false))?;
    let error = fresh
        .proved_with(&held.sessions, &after, (u32::MAX, 5, 5))
        .expect_err("an ended launch kept its connection proof");
    assert_eq!(error.name(), "peer_unproved");
    assert_eq!(after.parents.get(), 1);
    Ok(())
}

#[test]
fn a_cached_connection_names_a_poisoned_session_table() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let tree = Tree {
        leader: Leader {
            pid: 42,
            start: StartIdentity("leader-start".to_owned()),
        },
        peer_start: StartIdentity("peer-start".to_owned()),
        parents: Cell::new(0),
        starts: Cell::new(0),
    };
    let mut connection = Connection {
        proof: Some(Proof {
            session: "proved".to_owned(),
            generation: 1,
            leader: tree.leader.clone(),
            pid: u32::MAX,
            uid: 5,
            start: tree.peer_start.clone(),
        }),
    };
    let held = Arc::clone(&sessions);
    let poison = std::thread::spawn(move || {
        let table = held.read_lock();
        assert!(table.is_ok());
        panic!("abandon the session table during a mutation");
    });
    assert!(poison.join().is_err());
    let error = connection
        .proved_with(&sessions, &tree, (u32::MAX, 5, 5))
        .expect_err("the cached proof read poisoned session state");
    assert_eq!(error.name(), "session_table_poisoned");
    assert_eq!(tree.parents.get(), 0);
    Ok(())
}
