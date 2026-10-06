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
    assert!(dir.path().join("operations.v2.journal").is_file());
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
    let path = dir.path().join("operations.v2.journal");
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
    let log = std::fs::read(dir.path().join("operations.v2.journal"))?;
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

#[test]
fn a_v1_journal_with_a_torn_tail_migrates_once_and_preserves_interrupted_ids()
-> Result<(), Box<dyn Error>> {
    use std::io::Write;
    let dir = tempfile::tempdir()?;
    let original = dir.path().join("operations.jsonl");
    let mut file = std::fs::File::create(&original)?;
    let mut accepted = outcome("waiting".to_owned());
    accepted.state = OperationState::Accepted;
    let mut delivering = outcome("writing".to_owned());
    delivering.state = OperationState::Delivering;
    for kept in [outcome("done".to_owned()), accepted, delivering] {
        serde_json::to_writer(&mut file, &kept)?;
        file.write_all(b"\n")?;
    }
    file.write_all(b"{\"operation\":\"torn")?;
    file.sync_all()?;
    drop(file);
    let archived = std::fs::read(&original)?;
    let operations = Operations::open_at(dir.path(), 1)?;
    assert_eq!(
        operations
            .get("done")
            .ok_or("missing completed outcome")?
            .state,
        OperationState::Delivered
    );
    assert_eq!(
        operations
            .get("waiting")
            .ok_or("missing accepted outcome")?
            .state,
        OperationState::Uncertain
    );
    assert_eq!(
        operations
            .get("writing")
            .ok_or("missing interrupted outcome")?
            .state,
        OperationState::Uncertain
    );
    assert!(!original.exists());
    assert_eq!(
        std::fs::read(dir.path().join("operations.v1.jsonl"))?,
        archived
    );
    let before = std::fs::read(&operations.path)?;
    drop(operations);
    let operations = Operations::open_at(dir.path(), 1)?;
    assert_eq!(
        std::fs::read(&operations.path)?,
        before,
        "second open repeated migration or recovery"
    );
    assert!(!original.exists());
    assert!(operations.seen.contains("writing"));
    Ok(())
}

#[test]
fn proportional_checkpoints_bound_rewrite_bytes_and_keep_every_spent_id()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut operations = Operations::open_at(dir.path(), 1)?;
    let writer = crate::durable::Writer::new()?;
    operations.writer(writer.clone());
    let mut written_checkpoints = operations.checkpoint_bytes;
    let mut checkpoints = 0;
    for number in 0..1200 {
        let before = operations.checkpoint_offset;
        operations.record(outcome(format!("used-{number:08}")))?;
        if operations.checkpoint_offset != before {
            checkpoints += 1;
            written_checkpoints += operations.checkpoint_bytes;
            writer.barrier()?;
            let kept: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&operations.checkpoint)?)?;
            assert_eq!(
                kept["seen"]
                    .as_array()
                    .ok_or("guard is not an array")?
                    .len(),
                number + 1
            );
        }
        if number % 15 == 14 {
            writer.barrier()?;
        }
        assert!(
            operations.journal_offset - operations.checkpoint_offset < operations.checkpoint_bytes
        );
    }
    assert!(checkpoints > 1);
    let retained = operations.journal_offset + operations.checkpoint_bytes;
    let written = operations.journal_offset + written_checkpoints;
    assert!(
        written <= 2 * retained + operations.checkpoint_bytes,
        "written {written}, retained {retained}"
    );
    writer.barrier()?;
    drop(operations);
    let reopened = Operations::open_at(dir.path(), super::RETAIN_MS + 2)?;
    assert_eq!(reopened.seen.len(), 1200);
    assert!(reopened.held.is_empty());
    Ok(())
}

#[test]
fn an_old_checkpoint_and_a_flushed_tail_recover_without_replaying_the_prefix()
-> Result<(), Box<dyn Error>> {
    use std::io::{Seek, Write};
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), 512)?;
    let operations = Operations::open_at(dir.path(), 1)?;
    let offset = operations.checkpoint_offset;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&operations.path)?;
    file.write_all(b"checkpointed prefix is not read")?;
    file.seek(std::io::SeekFrom::Start(offset))?;
    let tail = serde_json::json!({"outcome":outcome("tail-after-flush".to_owned()),"control":null});
    serde_json::to_writer(&mut file, &tail)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    drop(operations);
    let operations = Operations::open_at(dir.path(), 1)?;
    assert_eq!(operations.seen.len(), 513);
    assert!(operations.get("tail-after-flush").is_some());
    assert_eq!(operations.checkpoint_offset, offset);
    assert!(operations.journal_offset - offset < operations.checkpoint_bytes);
    Ok(())
}
