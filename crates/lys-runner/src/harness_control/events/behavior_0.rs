use super::*;

fn boundary_syncs(refused: usize) -> Result {
    let (release, gate) = mpsc::channel();
    gated_fixture(
        Transport::Claude,
        false,
        false,
        Some(gate),
        move |sessions, source, frames, terminal| {
            let release = Release(release);
            for number in 0..=refused {
                sessions.operate(operation(
                    &format!("delivery-{number}"),
                    OperationRequest::GoalReminder {
                        text: "saved words".to_owned(),
                        reference: crate::harness_control::ReminderReference {
                            goal: format!("goal-{number}"),
                            occurrence: format!("occurrence-{number}"),
                            version: "version".to_owned(),
                            prior: None,
                        },
                    },
                ))?;
            }
            sessions.writer.barrier()?;
            let (status, path) = {
                let mut table = sessions.lock()?;
                (
                    crate::harness_control::events::runtime(
                        &mut table,
                        "fixture",
                        source.generation,
                    )?
                    .controller
                    .control_status(),
                    table.operations.test_journal_path().to_owned(),
                )
            };
            let reminders = status
                .queued
                .iter()
                .enumerate()
                .map(|(number, queued)| {
                    if number < refused {
                        crate::harness_control::ReminderDecision::Refuse {
                            operation: queued.operation.clone(),
                            reference: queued.reference.clone(),
                            reason: "current goal withdrawn".to_owned(),
                        }
                    } else {
                        crate::harness_control::ReminderDecision::Deliver {
                            operation: queued.operation.clone(),
                            reference: queued.reference.clone(),
                            text: "current words".to_owned(),
                        }
                    }
                })
                .collect();
            sessions.writer.serial_appends();
            let before = sessions.writer.sync_count(&path)?;
            let answer = sessions.apply_boundary_reply(operation(
                "decision",
                OperationRequest::BoundaryReply {
                    reply: crate::harness_control::BoundaryReply {
                        generation: source.generation,
                        boundary: status.boundary,
                        context: crate::harness_control::ContextDecision::Released,
                        reminders,
                    },
                },
            ))?;
            let syncs = sessions.writer.sync_count(&path)? - before;
            let frame = frames.recv()?;
            drop(release);
            assert_eq!(answer.state, OperationState::Confirmed);
            assert!(!frame.is_empty());
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            println!("boundary_batch refused={refused} dispatched=1 journal_syncs={syncs}");
            assert_eq!(syncs, 1);
            Ok(())
        },
    )
}
#[test]
fn one_boundary_dispatch_is_one_explicit_journal_sync() -> Result {
    boundary_syncs(0)
}
#[test]
fn five_refusals_and_a_boundary_dispatch_are_one_explicit_journal_sync() -> Result {
    boundary_syncs(5)
}

#[test]
fn a_boundary_journal_refusal_keeps_authority_and_queued_words_unchanged() -> Result {
    controlled_fixture(
        Transport::Claude,
        false,
        false,
        |sessions, source, frames, terminal| {
            let reference = crate::harness_control::ReminderReference {
                goal: "goal".to_owned(),
                occurrence: "occurrence".to_owned(),
                version: "old".to_owned(),
                prior: None,
            };
            sessions.operate(operation(
                "delivery",
                OperationRequest::GoalReminder {
                    text: "old words".to_owned(),
                    reference: reference.clone(),
                },
            ))?;
            let (before, offset) = {
                let mut table = sessions.lock()?;
                let status = crate::harness_control::events::runtime(
                    &mut table,
                    "fixture",
                    source.generation,
                )?
                .controller
                .control_status();
                let offset = table.operations.test_journal_offset(u64::MAX);
                (status, offset)
            };
            let mut reference = reference;
            reference.version = "current".to_owned();
            let reply = operation(
                "decision",
                OperationRequest::BoundaryReply {
                    reply: crate::harness_control::BoundaryReply {
                        generation: source.generation,
                        boundary: before.boundary.clone(),
                        context: crate::harness_control::ContextDecision::Released,
                        reminders: vec![crate::harness_control::ReminderDecision::Deliver {
                            operation: "delivery".to_owned(),
                            reference,
                            text: "current words".to_owned(),
                        }],
                    },
                },
            );
            let refused = sessions.apply_boundary_reply(reply.clone());
            let after = {
                let mut table = sessions.lock()?;
                table.operations.test_journal_offset(offset);
                assert!(table.operations.get("decision").is_none());
                crate::harness_control::events::runtime(&mut table, "fixture", source.generation)?
                    .controller
                    .control_status()
            };
            assert!(
                refused
                    .err()
                    .ok_or("journal refusal was accepted")?
                    .to_string()
                    .contains("journal offset overflows")
            );
            assert_eq!(after, before);
            assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            assert_eq!(
                sessions.apply_boundary_reply(reply)?.state,
                OperationState::Confirmed
            );
            let frame: Value = serde_json::from_slice(&frames.recv()?)?;
            assert_eq!(frame["message"]["content"], "Lys reminder\ncurrent words");
            Ok(())
        },
    )
}

#[test]
fn channel_loss_releases_human_waiters_refuses_more_input_and_ends_its_child() -> Result {
    fixture(
        Transport::Codex,
        true,
        |sessions, source, frames, terminal| {
            sessions.operate(operation(
                "current",
                OperationRequest::Compact {
                    text: String::new(),
                },
            ))?;
            let frame: Value = serde_json::from_slice(&frames.recv()?)?;
            assert_eq!(frame["method"], "thread/compact/start");
            let human = Arc::clone(sessions);
            let waiter = std::thread::spawn(move || human.input("fixture", "human words", true));
            sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                match crate::harness_control::events::runtime(table, "fixture", source.generation) {
                    Ok(runtime) => (!runtime.admissions.is_empty()).then_some(Ok(())),
                    Err(error) => Some(Err(error)),
                }
            })??;
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            sessions.managed_lost("fixture", source.generation, "reader lost")?;
            let error = waiter
                .join()
                .map_err(|error| format!("human caller panicked: {error:?}"))?
                .err()
                .ok_or("lost channel admitted human input")?;
            assert!(error.to_string().contains("control_transport_lost"));
            let ending = {
                let table = sessions.lock()?;
                let session = table.sessions.get("fixture").ok_or("fixture missing")?;
                session.ending
            };
            assert!(ending);
            let error = sessions
                .managed_input("fixture", source.generation, b"later")
                .err()
                .ok_or("lost generation accepted more human input")?;
            assert_eq!(error.name(), "session_ending");
            sessions.until(
                "fixture",
                &std::sync::atomic::AtomicBool::new(false),
                |state, _| state.ended().map(Ok),
            )?;
            let outcome = sessions.operate(operation(
                "later",
                OperationRequest::Reminder {
                    text: "later".to_owned(),
                },
            ))?;
            assert_eq!(outcome.state, OperationState::Refused);
            assert!(outcome.words.starts_with("session_ended:"));
            Ok(())
        },
    )
}

#[test]
fn the_terminal_writer_spy_records_zero_automated_writes_during_claude_compaction() -> Result {
    fixture(
        Transport::Claude,
        false,
        |sessions, source, frames, terminal| {
            let outcome = sessions.operate(operation(
                "compact",
                OperationRequest::Compact {
                    text: "untrusted terminal command".to_owned(),
                },
            ))?;
            assert_eq!(outcome.state, OperationState::Accepted);
            let frame: Value = serde_json::from_slice(&frames.recv()?)?;
            assert_eq!(frame["message"]["content"], "/compact");
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            observe(
                sessions,
                source,
                &json!({"type":"user","session_id":"conversation","uuid":frame["uuid"],
            "parent_tool_use_id":null,"message":{"role":"user","content":"/compact"}}),
            )?;
            observe(
                sessions,
                source,
                &json!({"type":"system","subtype":"compact_boundary","session_id":"conversation","compact_metadata":{"trigger":"manual"}}),
            )?;
            observe(
                sessions,
                source,
                &json!({"type":"result","session_id":"conversation","uuid":"compact-result","is_error":false}),
            )?;
            assert_eq!(
                sessions.outcome("compact")?.state,
                OperationState::Confirmed
            );
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        },
    )
}

#[test]
fn a_hook_callback_returns_while_its_turn_remains_active() -> Result {
    fixture(
        Transport::Codex,
        false,
        |sessions, source, frames, terminal| {
            observe(
                sessions,
                source,
                &json!({"method":"turn/started","params":{"threadId":"conversation","turn":{"id":"active"}}}),
            )?;
            let outcome = sessions.operate(operation(
                "reminder",
                OperationRequest::Reminder {
                    text: "saved reminder".to_owned(),
                },
            ))?;
            assert_eq!(outcome.state, OperationState::Accepted);
            sessions.collect(
                "fixture",
                &crate::peer::Collected::Hook {
                    event: "Stop".to_owned(),
                    input: json!({}),
                },
            )?;
            let mut table = sessions.lock()?;
            assert!(
                !crate::harness_control::events::runtime(&mut table, "fixture", source.generation)?
                    .controller
                    .idle()
            );
            drop(table);
            assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        },
    )
}

#[test]
fn a_concurrent_human_message_cannot_overtake_the_boundary_dispatcher() -> Result {
    fixture(
        Transport::Claude,
        true,
        |sessions, source, frames, terminal| {
            sessions.operate(operation(
                "compact",
                OperationRequest::Compact {
                    text: "/compact".to_owned(),
                },
            ))?;
            let first: Value = serde_json::from_slice(&frames.recv()?)?;
            let human = Arc::clone(sessions);
            let sender = std::thread::spawn(move || human.input("fixture", "human words", true));
            sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                match crate::harness_control::events::runtime(table, "fixture", source.generation) {
                    Ok(runtime) => (!runtime.admissions.is_empty()).then_some(Ok(())),
                    Err(error) => Some(Err(error)),
                }
            })??;
            assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
            observe(
                sessions,
                source,
                &json!({"type":"user","session_id":"conversation","uuid":first["uuid"],
            "parent_tool_use_id":null,"message":{"role":"user","content":"/compact"}}),
            )?;
            observe(
                sessions,
                source,
                &json!({"type":"system","subtype":"compact_boundary","session_id":"conversation","compact_metadata":{"trigger":"manual"}}),
            )?;
            observe(
                sessions,
                source,
                &json!({"type":"result","session_id":"conversation","uuid":"compact-result","is_error":false}),
            )?;
            let second: Value = serde_json::from_slice(&frames.recv()?)?;
            assert_eq!(second["message"]["content"], "human words");
            assert_eq!(first["message"]["content"], "/compact");
            observe(
                sessions,
                source,
                &json!({"type":"user","session_id":"conversation","uuid":second["uuid"],
            "parent_tool_use_id":null,"message":{"role":"user","content":"human words"}}),
            )?;
            sender
                .join()
                .map_err(|cause| format!("human input worker panicked: {cause:?}"))??;
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        },
    )
}
