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

#[test]
fn a_migrated_population_keeps_all_spent_ids_across_large_proportional_updates()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    old_install(dir.path(), 1200)?;
    let mut operations = Operations::open_at(dir.path(), super::RETAIN_MS + 2)?;
    let mut written_checkpoints = operations.checkpoint_bytes;
    let mut checkpoints = 0;
    for number in 0..16 {
        let mut large = outcome(format!("large-{number:06}"));
        large.at = crate::session::now_ms();
        large.words = "generated-private-evidence".repeat(4096);
        let before = operations.checkpoint_offset;
        operations.record(large)?;
        if before != operations.checkpoint_offset {
            checkpoints += 1;
            written_checkpoints += operations.checkpoint_bytes;
            let kept: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&operations.checkpoint)?)?;
            assert_eq!(
                kept["seen"]
                    .as_array()
                    .ok_or("guard is not an array")?
                    .len(),
                1200 + number + 1
            );
        }
        for id in 0..1200 {
            assert!(operations.seen.contains(&format!("notice-{id:06}")));
        }
        assert!(
            operations.journal_offset - operations.checkpoint_offset < operations.checkpoint_bytes
        );
        let retained = operations.journal_offset + operations.checkpoint_bytes;
        assert!(
            operations.journal_offset + written_checkpoints
                <= 2 * retained + operations.checkpoint_bytes
        );
    }
    assert!(checkpoints >= 3);
    drop(operations);
    let operations = Operations::open_at(dir.path(), crate::session::now_ms())?;
    assert_eq!(operations.seen.len(), 1216);
    for id in 0..1200 {
        assert!(operations.seen.contains(&format!("notice-{id:06}")));
    }
    Ok(())
}

fn control_owner(
    state: OperationState,
) -> Result<(tempfile::TempDir, std::sync::Arc<crate::session::Sessions>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    {
        let mut table = sessions.lock()?;
        table.operations.writer = None;
        let mut kept = outcome("control".to_owned());
        kept.state = state;
        kept.at = crate::session::now_ms();
        table.operations.record(kept)?;
    }
    Ok((dir, sessions))
}

fn control_preparation() -> Result<super::Prepared, Box<dyn Error>> {
    Ok(serde_json::from_value(serde_json::json!({
        "binding":{"session":"session","generation":1,"leader":{"pid":42,"start":"proved-start"},
            "conversation":"conversation","entry":{"path":"entry","sha256":"entry-digest"},
            "harness":{"path":"harness","sha256":"harness-digest"},"harness_version":"9.8.7","adapter":"fixture-adapter/1"},
        "uuid":"request","frame":{"type":"user","uuid":"request","message":{"content":"private words"}},"reference":null
    }))?)
}

fn personal_decision() -> super::Reconciled {
    super::Reconciled {
        operation: "decision".to_owned(),
        by: "person".to_owned(),
        at: crate::session::now_ms(),
        decision: super::Reconciliation::NotSeen,
    }
}

#[test]
fn a_refused_control_write_keeps_preparation_arm_and_admission_unchanged()
-> Result<(), Box<dyn Error>> {
    for phase in 0..3 {
        let (dir, sessions) = control_owner(OperationState::Accepted)?;
        let prepared = control_preparation()?;
        let mut table = sessions.lock()?;
        let operations = &mut table.operations;
        if phase > 0 {
            operations.prepare(
                "control",
                prepared.clone(),
                super::TextDigest::of("private words"),
            )?;
        }
        if phase > 1 {
            operations.arm("control")?;
        }
        let before_control = operations.controls.get("control").cloned();
        let before_outcome = operations.get("control").cloned();
        let before = std::fs::read(&operations.path)?;
        let archive = dir.path().join("unavailable.journal");
        std::fs::rename(&operations.path, &archive)?;
        let result = match phase {
            0 => operations.prepare(
                "control",
                prepared.clone(),
                super::TextDigest::of("private words"),
            ),
            1 => operations.arm("control"),
            _ => operations.observed("control", &prepared.binding, &prepared.uuid, None),
        };
        let error = result
            .err()
            .ok_or("an unavailable control journal accepted a write")?;
        assert!(error.to_string().contains("operations record"), "{error}");
        assert_eq!(
            operations.controls.get("control"),
            before_control.as_ref(),
            "phase {phase}"
        );
        assert_eq!(
            operations.get("control"),
            before_outcome.as_ref(),
            "phase {phase}"
        );
        assert_eq!(std::fs::read(&archive)?, before);
    }
    Ok(())
}

#[test]
fn a_refused_control_enqueue_keeps_memory_and_journal_unchanged() -> Result<(), Box<dyn Error>> {
    let (dir, sessions) = control_owner(OperationState::Accepted)?;
    let writer = crate::durable::Writer::new()?;
    writer.append(dir.path(), vec![1])?;
    assert!(writer.barrier().is_err());
    let mut table = sessions.lock()?;
    let operations = &mut table.operations;
    let before = std::fs::read(&operations.path)?;
    let outcome = operations.get("control").cloned();
    operations.writer(writer);
    let error = operations
        .prepare(
            "control",
            control_preparation()?,
            super::TextDigest::of("private words"),
        )
        .err()
        .ok_or("a failed writer accepted metadata")?;
    assert_eq!(error.name(), "durable_writer_unavailable");
    assert!(!operations.controls.contains_key("control"));
    assert_eq!(operations.get("control"), outcome.as_ref());
    assert_eq!(std::fs::read(&operations.path)?, before);
    Ok(())
}

#[test]
fn a_refused_reconciliation_write_keeps_memory_and_journal_unchanged() -> Result<(), Box<dyn Error>>
{
    let (dir, sessions) = control_owner(OperationState::Uncertain)?;
    let path = dir.path().join("operations.v2.journal");
    let archive = dir.path().join("unavailable.journal");
    let before = std::fs::read(&path)?;
    std::fs::rename(path, &archive)?;
    assert!(
        sessions
            .reconcile_control("control", personal_decision())
            .is_err()
    );
    assert!(!sessions.lock()?.operations.controls.contains_key("control"));
    assert_eq!(std::fs::read(archive)?, before);
    Ok(())
}

#[test]
fn reconciliation_of_a_non_uncertain_outcome_refuses_without_mutation() -> Result<(), Box<dyn Error>>
{
    let (dir, sessions) = control_owner(OperationState::Confirmed)?;
    let path = dir.path().join("operations.v2.journal");
    let before = std::fs::read(&path)?;
    let error = sessions
        .reconcile_control("control", personal_decision())
        .err()
        .ok_or("confirmed evidence was reconciled")?;
    assert_eq!(error.name(), "control_not_uncertain");
    assert!(!sessions.lock()?.operations.controls.contains_key("control"));
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}

#[test]
fn an_accepted_control_record_survives_a_failed_checkpoint_replace() -> Result<(), Box<dyn Error>> {
    let (dir, sessions) = control_owner(OperationState::Uncertain)?;
    let checkpoint = dir.path().join("operations.v2.snapshot");
    let previous = dir.path().join("previous.snapshot");
    std::fs::rename(&checkpoint, &previous)?;
    std::fs::create_dir(&checkpoint)?;
    sessions.lock()?.operations.checkpoint_bytes = 1;
    let decision = personal_decision();
    let before = sessions.lock()?.operations.journal_offset;
    assert!(
        sessions
            .reconcile_control("control", decision.clone())
            .is_err()
    );
    {
        let table = sessions.lock()?;
        assert!(table.operations.journal_offset > before);
        assert_eq!(
            table
                .operations
                .controls
                .get("control")
                .and_then(|control| control.decision.as_ref()),
            Some(&decision)
        );
    }
    std::fs::remove_dir(&checkpoint)?;
    std::fs::rename(previous, checkpoint)?;
    drop(sessions);
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    assert_eq!(
        sessions.control_receipt("control")?.reconciled,
        Some(decision)
    );
    assert_eq!(
        sessions.outcome("control")?.state,
        OperationState::Uncertain
    );
    Ok(())
}

#[test]
fn a_torn_decision_batch_reopens_with_all_members_or_none() -> Result<(), Box<dyn Error>> {
    for truncated in [None, Some(2), Some(1)] {
        let dir = tempfile::tempdir()?;
        let writer = crate::durable::Writer::new()?;
        let mut operations = Operations::open_at(dir.path(), 1)?;
        operations.writer(writer.clone());
        operations.checkpoint_bytes = u64::MAX;
        operations.begin_journal_batch()?;
        for number in 0..4 {
            operations.record(outcome(format!("member-{number}")))?;
        }
        operations.finish_journal_batch()?;
        writer.barrier()?;
        let path = operations.path.clone();
        let bytes = std::fs::read(&path)?;
        drop(operations);
        if let Some(divisor) = truncated {
            let length = if divisor == 1 {
                bytes.len() - 1
            } else {
                bytes.len() / divisor
            };
            let file = std::fs::OpenOptions::new().write(true).open(&path)?;
            file.set_len(u64::try_from(length)?)?;
            file.sync_data()?;
        }
        let operations = Operations::open_at(dir.path(), 1)?;
        for number in 0..4 {
            assert_eq!(
                operations.get(&format!("member-{number}")).is_some(),
                truncated.is_none()
            );
        }
        if truncated.is_some() {
            assert_eq!(std::fs::metadata(path)?.len(), 0);
        }
    }
    Ok(())
}

#[test]
fn two_queued_decisions_remain_two_whole_journal_batches() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let writer = crate::durable::Writer::new()?;
    let mut operations = Operations::open_at(dir.path(), 1)?;
    operations.writer(writer.clone());
    operations.checkpoint_bytes = u64::MAX;
    for (decision, count) in [("first", 2), ("second", 3)] {
        operations.begin_journal_batch()?;
        for number in 0..count {
            operations.record(outcome(format!("{decision}-{number}")))?;
        }
        operations.finish_journal_batch()?;
    }
    writer.barrier()?;
    let bytes = std::fs::read_to_string(&operations.path)?;
    let batches = bytes
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(batches.len(), 2);
    assert_eq!(
        batches[0]["batch"]
            .as_array()
            .ok_or("first batch absent")?
            .len(),
        2
    );
    assert_eq!(
        batches[1]["batch"]
            .as_array()
            .ok_or("second batch absent")?
            .len(),
        3
    );
    drop(operations);
    let operations = Operations::open_at(dir.path(), 1)?;
    for (decision, count) in [("first", 2), ("second", 3)] {
        for number in 0..count {
            let id = format!("{decision}-{number}");
            assert_eq!(operations.get(&id), Some(&outcome(id.clone())));
        }
    }
    Ok(())
}

#[test]
fn a_dropped_nonempty_decision_batch_faults_the_writer_without_a_prefix()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let writer = crate::durable::Writer::new()?;
    let mut operations = Operations::open_at(dir.path(), 1)?;
    operations.writer(writer.clone());
    operations.begin_journal_batch()?;
    operations.record(outcome("decision".to_owned()))?;
    operations.cancel_journal_batch();
    let error = writer
        .barrier()
        .err()
        .ok_or("dropped decision did not fault the writer")?;
    assert!(error.to_string().contains("journal_batch_dropped"));
    let path = operations.path.clone();
    drop(operations);
    assert_eq!(std::fs::metadata(path)?.len(), 0);
    assert!(
        Operations::open_at(dir.path(), 1)?
            .get("decision")
            .is_none()
    );
    Ok(())
}
