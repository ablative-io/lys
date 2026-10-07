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

fn request_identity_owner()
-> Result<(tempfile::TempDir, std::sync::Arc<crate::session::Sessions>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let kept = crate::state::Kept::new(vec![crate::state::KeptSession {
        session: "session".to_owned(),
        pid: None,
        leader_start: None,
        started_at: 1,
        columns: 80,
        rows: 24,
        ended: None,
    }]);
    std::fs::write(dir.path().join("sessions.json"), serde_json::to_vec(&kept)?)?;
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    Ok((dir, sessions))
}

fn original_request(request: super::OperationRequest) -> super::Operation {
    super::Operation {
        operation: "original-request".to_owned(),
        session: "session".to_owned(),
        request,
    }
}

fn refuse_changed_request(
    sessions: &crate::session::Sessions,
    operation: super::Operation,
) -> Result<(), Box<dyn Error>> {
    let error = sessions
        .operate(operation)
        .err()
        .ok_or("a changed request returned another request's outcome")?;
    assert_eq!(error.name(), "operation_reused");
    assert!(!error.to_string().contains("private initial words"));
    Ok(())
}

#[test]
fn a_goal_request_refuses_every_changed_reference_live_and_after_reopen()
-> Result<(), Box<dyn Error>> {
    let (dir, sessions) = request_identity_owner()?;
    let original = original_request(super::OperationRequest::GoalReminder {
        text: "private initial words".to_owned(),
        reference: crate::harness_control::ReminderReference {
            goal: "goal".to_owned(),
            occurrence: "occurrence".to_owned(),
            version: "version".to_owned(),
            prior: None,
        },
    });
    let held = sessions.operate(original.clone())?;
    for field in 0..4 {
        let mut changed = original.clone();
        if let super::OperationRequest::GoalReminder { reference, .. } = &mut changed.request {
            match field {
                0 => reference.goal = "another-goal".to_owned(),
                1 => reference.occurrence = "another-occurrence".to_owned(),
                2 => reference.version = "another-version".to_owned(),
                _ => reference.prior = Some("possible-prior".to_owned()),
            }
        }
        refuse_changed_request(&sessions, changed)?;
    }
    assert_eq!(sessions.operate(original.clone())?, held);
    drop(sessions);
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    assert_eq!(sessions.operate(original.clone())?, held);
    let mut changed = original;
    if let super::OperationRequest::GoalReminder { reference, .. } = &mut changed.request {
        reference.goal = "another-goal".to_owned();
    }
    refuse_changed_request(&sessions, changed)?;
    Ok(())
}

#[test]
fn a_context_request_refuses_a_changed_crossing_live_and_after_reopen() -> Result<(), Box<dyn Error>>
{
    let (dir, sessions) = request_identity_owner()?;
    let original = original_request(super::OperationRequest::ContextCompact {
        text: "private initial words".to_owned(),
        crossing: "original-request".to_owned(),
    });
    let held = sessions.operate(original.clone())?;
    let changed = original_request(super::OperationRequest::ContextCompact {
        text: "private initial words".to_owned(),
        crossing: "another-crossing".to_owned(),
    });
    refuse_changed_request(&sessions, changed.clone())?;
    assert_eq!(sessions.operate(original.clone())?, held);
    drop(sessions);
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    assert_eq!(sessions.operate(original)?, held);
    refuse_changed_request(&sessions, changed)?;
    Ok(())
}

fn managed_v1_refusal(kind: &str) -> Result<(), Box<dyn Error>> {
    for journal in [false, true] {
        let dir = tempfile::tempdir()?;
        let mut record = outcome("managed-in-v1".to_owned());
        record.request = kind.to_owned();
        if journal {
            let mut bytes = serde_json::to_vec(&record)?;
            bytes.push(b'\n');
            std::fs::write(dir.path().join("operations.jsonl"), bytes)?;
        } else {
            std::fs::write(
                dir.path().join("operations.json"),
                serde_json::to_vec(&Kept {
                    format: FORMAT.to_owned(),
                    operations: vec![record],
                })?,
            )?;
        }
        let error = Operations::open_at(dir.path(), 1)
            .err()
            .ok_or("an installed v1 record accepted a managed request kind")?;
        assert_eq!(error.name(), "operation_legacy_request_invalid");
        assert!(!dir.path().join("operations.v2.journal").exists());
    }
    Ok(())
}

#[test]
fn v1_migration_refuses_goal_reminder() -> Result<(), Box<dyn Error>> {
    managed_v1_refusal("goal_reminder")
}
#[test]
fn v1_migration_refuses_context_compact() -> Result<(), Box<dyn Error>> {
    managed_v1_refusal("context_compact")
}
#[test]
fn v1_migration_refuses_boundary_reply() -> Result<(), Box<dyn Error>> {
    managed_v1_refusal("boundary_reply")
}

#[test]
fn the_original_request_digest_survives_a_changed_preparation_and_reopen()
-> Result<(), Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    let (dir, sessions) = request_identity_owner()?;
    let original = original_request(super::OperationRequest::GoalReminder {
        text: "private initial words".to_owned(),
        reference: crate::harness_control::ReminderReference {
            goal: "goal".to_owned(),
            occurrence: "occurrence".to_owned(),
            version: "version".to_owned(),
            prior: None,
        },
    });
    let canonical = r#"{"request":"goal_reminder","text":"private initial words","reference":{"goal":"goal","occurrence":"occurrence","version":"version","prior":null}}"#;
    assert_eq!(serde_json::to_string(&original.request)?, canonical);
    let mut hash = Sha256::new();
    hash.update(b"lys-operation-request-json/v1\n");
    hash.update(canonical.as_bytes());
    let expected: [u8; 32] = hash.finalize().into();
    assert_eq!(
        crate::protocol::hex(&expected),
        "535271fdca7ad0a677c9d622c318447ece4edf4bc1fa99cfdf18e7e4e5872d1f"
    );
    assert_eq!(original.request.identity()?, expected);
    sessions.operate(original.clone())?;
    {
        let mut table = sessions.lock()?;
        let operations = &mut table.operations;
        operations.set(
            &original.operation,
            OperationState::Accepted,
            "waiting".to_owned(),
        )?;
        let mut prepared = control_preparation()?;
        prepared.reference = Some(crate::harness_control::ReminderReference {
            goal: "goal".to_owned(),
            occurrence: "occurrence".to_owned(),
            version: "current-version".to_owned(),
            prior: None,
        });
        prepared.frame = serde_json::json!({"type":"user","message":"private current words"});
        operations.prepare(
            &original.operation,
            prepared,
            super::TextDigest::of("private current words"),
        )?;
        operations.arm(&original.operation)?;
        operations.set(
            &original.operation,
            OperationState::Uncertain,
            "write was unconfirmed".to_owned(),
        )?;
        assert_eq!(
            operations.original_request(&original.operation),
            Some(&expected)
        );
    }
    let held = sessions.outcome(&original.operation)?;
    assert_eq!(sessions.operate(original.clone())?, held);
    drop(sessions);
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    assert_eq!(original.request.identity()?, expected);
    assert_eq!(
        sessions
            .lock()?
            .operations
            .original_request(&original.operation),
        Some(&expected)
    );
    assert_eq!(sessions.operate(original.clone())?, held);
    let mut changed = original;
    if let super::OperationRequest::GoalReminder { reference, .. } = &mut changed.request {
        reference.version = "current-version".to_owned();
    }
    refuse_changed_request(&sessions, changed)?;
    Ok(())
}

#[test]
fn a_boundary_reply_refuses_a_changed_field_live_and_after_reopen() -> Result<(), Box<dyn Error>> {
    let (dir, sessions) = request_identity_owner()?;
    let reply = crate::harness_control::BoundaryReply {
        generation: 1,
        boundary: Some("boundary".to_owned()),
        context: crate::harness_control::ContextDecision::Released,
        reminders: Vec::new(),
    };
    let original = original_request(super::OperationRequest::BoundaryReply {
        reply: reply.clone(),
    });
    let mut held = outcome(original.operation.clone());
    held.request = "boundary_reply".to_owned();
    held.state = OperationState::Confirmed;
    held.at = crate::session::now_ms();
    held.text = Some(super::TextDigest::of(&serde_json::to_string(&reply)?));
    sessions
        .lock()?
        .operations
        .keep_control(held.clone(), &original.request)?;
    assert_eq!(sessions.apply_boundary_reply(original.clone())?, held);
    let mut changed = original.clone();
    if let super::OperationRequest::BoundaryReply { reply } = &mut changed.request {
        reply.generation = 2;
    }
    let error = sessions
        .apply_boundary_reply(changed.clone())
        .err()
        .ok_or("a changed reply returned the original decision")?;
    assert_eq!(error.name(), "operation_reused");
    drop(sessions);
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    assert_eq!(sessions.apply_boundary_reply(original)?, held);
    let error = sessions
        .apply_boundary_reply(changed)
        .err()
        .ok_or("a changed reply returned the original decision after reopen")?;
    assert_eq!(error.name(), "operation_reused");
    Ok(())
}

fn typed_checkpoint_cost(count: usize) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = crate::session::Sessions::open(dir.path(), 4096)?;
    let at = crate::session::now_ms();
    let retained = count.min(1_000);
    {
        let mut table = sessions.lock()?;
        let operations = &mut table.operations;
        operations.seen = (0..count)
            .map(|index| format!("operation-{index:032x}"))
            .collect();
        for index in 0..retained {
            let id = format!("operation-{index:032x}");
            let reference = crate::harness_control::ReminderReference {
                goal: format!("goal-{index}"),
                occurrence: format!("occurrence-{index}"),
                version: "version".to_owned(),
                prior: None,
            };
            let request = super::OperationRequest::GoalReminder {
                text: "private saved input".to_owned(),
                reference: reference.clone(),
            };
            let mut prepared = control_preparation()?;
            prepared.uuid = crate::harness_control::Pending::for_goal(
                id.clone(),
                "private saved input".to_owned(),
                reference.clone(),
            )
            .uuid;
            prepared.reference = Some(reference);
            prepared.frame = serde_json::json!({"type":"user","uuid":prepared.uuid,"message":{"content":"private saved input"}});
            let admission = super::control::Admission {
                binding: prepared.binding.clone(),
                uuid: prepared.uuid.clone(),
                turn: Some(format!("turn-{index}")),
            };
            operations
                .original_requests
                .insert(id.clone(), request.identity()?);
            operations.controls.insert(
                id.clone(),
                super::control::Control {
                    prepared: Some(prepared),
                    original_text: Some(super::TextDigest::of("private saved input")),
                    certainty: super::Certainty::Observed,
                    admitted: Some(admission),
                    decision: None,
                },
            );
            let mut kept = outcome(id);
            kept.request = "goal_reminder".to_owned();
            kept.state = OperationState::Confirmed;
            kept.at = at;
            kept.text = Some(super::TextDigest::of("private saved input"));
            operations.fold(kept, at);
        }
    }
    sessions.writer.barrier()?;
    super::store::measurement::begin();
    let acquired = std::time::Instant::now();
    let mut table = sessions.lock()?;
    let locked = std::time::Instant::now();
    table.operations.test_checkpoint()?;
    let bytes = table.operations.checkpoint_bytes;
    let lock_hold = locked.elapsed();
    let acquire_and_hold = acquired.elapsed();
    drop(table);
    let replacement = std::time::Instant::now();
    sessions.writer.barrier()?;
    let replace_wait = replacement.elapsed();
    let replace_io = sessions
        .writer
        .replacement_cost(&dir.path().join("operations.v2.snapshot"))?;
    let opening = std::time::Instant::now();
    let restored = Operations::open_at(dir.path(), at)?;
    let startup = opening.elapsed();
    let costs =
        super::store::measurement::finish().ok_or("checkpoint measurement was not active")?;
    assert_eq!(restored.seen.len(), count);
    assert_eq!(restored.held.len(), retained);
    assert_eq!(restored.controls.len(), retained);
    assert_eq!(restored.original_requests.len(), retained);
    assert!(
        restored
            .controls
            .values()
            .all(super::control::Control::matching_admission)
    );
    assert_eq!(restored.journal_offset, restored.checkpoint_offset);
    println!(
        "typed_checkpoint used_ids={count} held={retained} controls={retained} prepared={retained} identities={retained} bytes={bytes} encode_us={} replace_and_sync_us={} replace_barrier_us={} read_us={} decode_us={} table_hold_us={} table_acquire_and_hold_us={} startup_us={} tail_bytes=0 whole_state_clones=0 table_thread_syncs=0",
        costs.encode.as_micros(),
        replace_io.as_micros(),
        replace_wait.as_micros(),
        costs.read.as_micros(),
        costs.decode.as_micros(),
        lock_hold.as_micros(),
        acquire_and_hold.as_micros(),
        startup.as_micros()
    );
    Ok(())
}

#[test]
fn typed_checkpoint_one_thousand_used_ids() -> Result<(), Box<dyn Error>> {
    typed_checkpoint_cost(1_000)
}
#[test]
fn typed_checkpoint_ten_thousand_used_ids() -> Result<(), Box<dyn Error>> {
    typed_checkpoint_cost(10_000)
}
#[test]
fn typed_checkpoint_one_hundred_thousand_used_ids() -> Result<(), Box<dyn Error>> {
    typed_checkpoint_cost(100_000)
}
#[test]
fn typed_checkpoint_one_million_used_ids() -> Result<(), Box<dyn Error>> {
    typed_checkpoint_cost(1_000_000)
}
