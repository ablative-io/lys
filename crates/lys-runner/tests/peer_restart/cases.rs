#![cfg(test)]
//! Restart is a peer act; the socket proof chooses its session.

use std::error::Error;

#[test]
fn a_restart_request_carries_an_operation_without_trusting_its_session_claim()
-> Result<(), Box<dyn Error>> {
    let line = r#"{"version":1,"peer":{"act":"restart","operation":"own-restart","session":"someone-else"}}"#;
    let request = serde_json::from_str::<super::PeerRequest>(line)?;
    assert_eq!(request.version, crate::PROTOCOL_VERSION);
    Ok(())
}

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Leader, PeerAct, PeerRequest, Processes, StartIdentity, restart_with};
use crate::operations::OperationState;
use crate::{Launch, RunnerError, Sessions};

struct Tree {
    parents: BTreeMap<u32, u32>,
    starts: BTreeMap<u32, StartIdentity>,
}

impl Processes for Tree {
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        self.parents.get(&pid).copied().ok_or_else(|| {
            RunnerError::refused("peer_unproved", "the process has no recorded parent")
        })
    }

    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        self.starts.get(&pid).cloned().ok_or_else(|| {
            RunnerError::refused("peer_unproved", "the process has no recorded start")
        })
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
        let held = Self { dir, sessions };
        for id in ["own", "other"] {
            held.sessions.start(Launch {
                session: id.to_owned(),
                program: "/bin/cat".to_owned(),
                arguments: Vec::new(),
                directory: "/".to_owned(),
                environment: BTreeMap::from([(
                    "LYS_HANDLES".to_owned(),
                    "credential-id-a,credential-id-b".to_owned(),
                )]),
                config: None,
                columns: 80,
                rows: 24,
                rotation: None,
                policy: None,
            })?;
        }
        Ok(held)
    }

    fn tree(&self, session: &str) -> Result<Tree, Box<dyn Error>> {
        let leaders = self.sessions.leaders();
        let leader = leaders.get(session).ok_or("session has no leader")?;
        Ok(Tree {
            parents: BTreeMap::from([(u32::MAX, leader.pid)]),
            starts: leaders
                .values()
                .map(|leader| (leader.pid, leader.start.clone()))
                .collect(),
        })
    }

    fn leader(&self, session: &str) -> Result<Leader, Box<dyn Error>> {
        self.sessions
            .leaders()
            .get(session)
            .cloned()
            .ok_or_else(|| "session has no leader".into())
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        self.sessions.stop_all();
    }
}

#[test]
fn a_proved_peer_restarts_its_own_launch_with_unchanged_credentials() -> Result<(), Box<dyn Error>>
{
    let held = Held::open()?;
    let before = held.leader("own")?;
    let launch = held
        .sessions
        .lock()
        .sessions
        .get("own")
        .ok_or("session missing")?
        .launch
        .clone();
    let outcome = restart_with(
        &held.sessions,
        &held.tree("own")?,
        (u32::MAX, 5, 5),
        "restart-own",
    )?;
    assert_eq!(outcome.state, OperationState::Confirmed);
    assert!(
        outcome.ended.is_some(),
        "the restart carries the old process's exit"
    );
    assert_ne!(held.leader("own")?, before);
    assert_eq!(
        held.sessions
            .lock()
            .sessions
            .get("own")
            .ok_or("session missing")?
            .launch,
        launch
    );
    Ok(())
}

#[test]
fn a_body_naming_another_session_restarts_only_the_proved_session() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    let own = held.leader("own")?;
    let other = held.leader("other")?;
    let request: PeerRequest = serde_json::from_str(
        r#"{"version":1,"peer":{"act":"restart","operation":"restart-own","session":"other"}}"#,
    )?;
    let PeerAct::Restart { operation, .. } = request.peer else {
        return Err("request is not a restart".into());
    };
    let outcome = restart_with(
        &held.sessions,
        &held.tree("own")?,
        (u32::MAX, 5, 5),
        &operation,
    )?;
    assert_eq!(outcome.session, "own");
    assert_ne!(held.leader("own")?, own);
    assert_eq!(held.leader("other")?, other);
    Ok(())
}

#[test]
fn an_unproved_peer_is_refused_without_restarting_any_session() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    let before = held.sessions.leaders();
    let tree = Tree {
        parents: BTreeMap::from([(u32::MAX, 1)]),
        starts: BTreeMap::new(),
    };
    let error = restart_with(&held.sessions, &tree, (u32::MAX, 5, 5), "restart-own")
        .expect_err("unproved peer restarted a session");
    assert_eq!(error.name(), "not_a_session");
    assert_eq!(held.sessions.leaders(), before);
    Ok(())
}

#[test]
fn a_replayed_operation_answers_the_kept_restart_once() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    let first = restart_with(
        &held.sessions,
        &held.tree("own")?,
        (u32::MAX, 5, 5),
        "restart-own",
    )?;
    let after = held.leader("own")?;
    let again = restart_with(
        &held.sessions,
        &held.tree("own")?,
        (u32::MAX, 5, 5),
        "restart-own",
    )?;
    assert_eq!(again, first);
    assert_eq!(held.leader("own")?, after);
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(held.dir.path().join("operations.json"))?)?;
    let operations = record["operations"].as_array().ok_or("no operations")?;
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0]["operation"], "restart-own");
    assert_eq!(operations[0]["state"], "confirmed");
    Ok(())
}

#[test]
fn a_proved_session_with_no_held_launch_is_refused_by_name() -> Result<(), Box<dyn Error>> {
    let held = Held::open()?;
    held.sessions
        .lock()
        .sessions
        .get_mut("own")
        .ok_or("session missing")?
        .launch = None;
    let before = held.leader("own")?;
    let error = restart_with(
        &held.sessions,
        &held.tree("own")?,
        (u32::MAX, 5, 5),
        "restart-own",
    )
    .expect_err("missing launch restarted");
    assert_eq!(error.name(), "session_launch_missing");
    assert_eq!(held.leader("own")?, before);
    Ok(())
}
