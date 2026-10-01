#![cfg(test)]
//! Migration retains old outcomes; each change appends only its own bytes.

use std::error::Error;

use super::{FORMAT, Kept, OperationOutcome, OperationState, Operations};

fn outcome(id: String) -> OperationOutcome {
    OperationOutcome {
        operation: id,
        session: "session".to_owned(),
        request: "notice".to_owned(),
        state: OperationState::Delivered,
        at: 1,
        words: "typed into the session".to_owned(),
        text: None,
        ended: None,
    }
}

fn old_install(dir: &std::path::Path, count: usize) -> Result<(), Box<dyn Error>> {
    let operations = (0..count)
        .map(|index| outcome(format!("notice-{index:06}")))
        .collect();
    std::fs::write(
        dir.join("operations.json"),
        serde_json::to_vec(&Kept {
            format: FORMAT.to_owned(),
            operations,
        })?,
    )?;
    Ok(())
}

#[test]
fn old_install_outcomes_move_to_the_log_without_losing_identity() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), 2)?;
    let operations = Operations::open_at(dir.path(), 1)?;
    assert_eq!(
        operations.get("notice-000000"),
        Some(&outcome("notice-000000".to_owned()))
    );
    assert!(dir.path().join("operations.jsonl").is_file());
    assert!(!dir.path().join("operations.json").exists());
    drop(operations);
    assert_eq!(
        Operations::open_at(dir.path(), 1)?.get("notice-000001"),
        Some(&outcome("notice-000001".to_owned()))
    );
    Ok(())
}

fn changed_bytes(count: usize) -> Result<usize, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), count)?;
    let mut operations = Operations::open_at(dir.path(), 1)?;
    let path = dir.path().join("operations.jsonl");
    let before = std::fs::read(&path)?;
    operations.set(
        "notice-000000",
        OperationState::Delivered,
        "typed into the session".to_owned(),
    )?;
    let after = std::fs::read(&path)?;
    assert!(
        after.starts_with(&before),
        "a state change rewrote earlier history"
    );
    Ok(after.len() - before.len())
}

#[test]
fn operation_state_change_bytes_do_not_grow_with_history() -> Result<(), Box<dyn Error>> {
    let small = changed_bytes(4)?;
    let large = changed_bytes(512)?;
    assert_eq!(large, small);
    assert!(small > 0 && small < 1024);
    Ok(())
}

#[test]
fn an_expired_terminal_outcome_leaves_the_runner_heap() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), 1)?;
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    assert!(sessions.lock()?.operations.get("notice-000000").is_none());
    let log = std::fs::read(dir.path().join("operations.jsonl"))?;
    assert!(String::from_utf8(log)?.contains("notice-000000"));
    Ok(())
}

#[test]
fn an_expired_operation_id_is_refused_after_a_restart() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), 1)?;
    drop(crate::session::Sessions::open(dir.path(), 4096)?);
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    let result = sessions.operate(super::Operation {
        operation: "notice-000000".to_owned(),
        session: "session".to_owned(),
        request: super::OperationRequest::Notice {
            text: "again".to_owned(),
        },
    });
    let error = result
        .err()
        .ok_or("an expired id was accepted or replayed")?;
    assert!(error.to_string().contains("operation_repeated"), "{error}");
    Ok(())
}

#[test]
fn terminal_retention_ends_at_twenty_four_hours() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), 1)?;
    let mut operations = Operations::open_at(dir.path(), super::RETAIN_MS)?;
    assert!(operations.get("notice-000000").is_some());
    operations.prune(super::RETAIN_MS + 1);
    assert!(operations.get("notice-000000").is_none());
    assert!(operations.repeated("notice-000000").is_err());
    Ok(())
}

#[test]
fn poisoned_session_state_refuses_operations_and_grant_channels() -> Result<(), Box<dyn Error>> {
    use std::sync::{Arc, atomic::AtomicBool};

    use crate::error::RunnerError;
    use crate::refusals::{Channel, GrantAnswer, GrantQuestion};

    fn poisoned<T>(result: Result<T, RunnerError>) -> Result<(), Box<dyn Error>> {
        let error = result.err().ok_or("poisoned state was used")?;
        assert_eq!(error.name(), "session_table_poisoned", "{error}");
        Ok(())
    }

    let dir = tempfile::tempdir()?;
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    let channel = Channel::new(&sessions)?;
    let held = Arc::clone(&sessions);
    let poison = std::thread::spawn(move || {
        let table = held.lock();
        assert!(table.is_ok());
        panic!("abandon the session state during a mutation");
    });
    assert!(poison.join().is_err());

    poisoned(sessions.outcome("notice"))?;
    poisoned(sessions.operate(super::Operation {
        operation: "notice".to_owned(),
        session: "session".to_owned(),
        request: super::OperationRequest::Notice {
            text: "message".to_owned(),
        },
    }))?;
    poisoned(Channel::new(&sessions))?;
    poisoned(channel.ready())?;
    poisoned(channel.next())?;
    let question = GrantQuestion {
        attempt: "attempt".to_owned(),
        session: "session".to_owned(),
        agent: "agent".to_owned(),
        policy_version: 1,
        rule: "rule".to_owned(),
        resource: crate::judge::NamedResource {
            kind: "path".to_owned(),
            id: "records".to_owned(),
        },
        action: "read".to_owned(),
    };
    let answer = GrantAnswer {
        attempt: question.attempt.clone(),
        session: question.session.clone(),
        policy_version: question.policy_version,
        rule: question.rule.clone(),
        permitted: true,
        grantor: None,
        words: String::new(),
    };
    poisoned(channel.answer(&question, &serde_json::to_string(&answer)?))?;
    let error = crate::refusals::ask(&sessions, question, &AtomicBool::new(false))
        .err()
        .ok_or("a grant was asked using poisoned state")?;
    assert_eq!(error.name(), "session_table_poisoned", "{error}");
    Ok(())
}
