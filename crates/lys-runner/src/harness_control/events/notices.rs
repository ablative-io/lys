use super::*;
use crate::operations::TextDigest;
use std::sync::atomic::AtomicBool;

fn notice(id: &str) -> Operation {
    operation(
        id,
        OperationRequest::Notice {
            text: "notice words".to_owned(),
        },
    )
}

fn delivered(sessions: &Sessions, id: &str) -> Result {
    let outcome = sessions.until_any(&AtomicBool::new(false), |table| {
        table
            .operations
            .get(id)
            .filter(|held| {
                held.state == OperationState::Delivered
                    || held
                        .words
                        .contains("managed request written; harness evidence pending")
            })
            .cloned()
    })?;
    assert_eq!(
        outcome.state,
        OperationState::Delivered,
        "successful managed write remains accepted"
    );
    assert_eq!(outcome.text, Some(TextDigest::of("notice words")));
    sessions.writer.barrier()?;
    let table = sessions.lock()?;
    let states: Vec<_> = table
        .feed
        .page(None)?
        .entries
        .into_iter()
        .filter_map(|entry| match entry.body {
            Body::Operation(outcome) if outcome.operation == id => Some(outcome.state),
            _ => None,
        })
        .collect();
    assert_eq!(
        states,
        [
            OperationState::Accepted,
            OperationState::Delivering,
            OperationState::Delivered
        ]
    );
    let journal = std::fs::read_to_string(table.operations.journal_path())?;
    let mut recorded = Vec::new();
    for line in journal.lines() {
        let row: Value = serde_json::from_str(line)?;
        if row["outcome"]["operation"] == id {
            recorded.push(
                row["outcome"]["state"]
                    .as_str()
                    .ok_or("journal state missing")?
                    .to_owned(),
            );
            assert_eq!(
                row["outcome"]["text"],
                serde_json::to_value(TextDigest::of("notice words"))?
            );
        }
    }
    recorded.dedup();
    assert_eq!(recorded, ["accepted", "delivering", "delivered"]);
    Ok(())
}

fn frame(frames: &mpsc::Receiver<Vec<u8>>, transport: Transport) -> Result<Value> {
    let frame: Value = serde_json::from_slice(&frames.recv()?)?;
    match transport {
        Transport::Claude => assert_eq!(frame["message"]["content"], "notice words"),
        Transport::Codex => assert_eq!(frame["params"]["input"][0]["text"], "notice words"),
        Transport::Pty => return Err("managed transport required".into()),
    }
    Ok(frame)
}

fn replay(sessions: &Sessions, source: &Binding, frame: &Value, transport: Transport) -> Result {
    let event = match transport {
        Transport::Claude => frame.clone(),
        Transport::Codex => {
            json!({"id":frame["id"],"result":{"turn":{"id":format!("turn-{}", frame["id"].as_str().ok_or("frame id missing")?)}}})
        }
        Transport::Pty => return Err("managed transport required".into()),
    };
    observe(sessions, source, &event)
}

fn complete(sessions: &Sessions, source: &Binding, frame: &Value, transport: Transport) -> Result {
    let event = match transport {
        Transport::Claude => {
            json!({"type":"result","session_id":"conversation","uuid":format!("result-{}", frame["uuid"].as_str().ok_or("frame uuid missing")?),"is_error":false})
        }
        Transport::Codex => {
            json!({"method":"turn/completed","params":{"threadId":"conversation","turn":{"id":format!("turn-{}", frame["id"].as_str().ok_or("frame id missing")?),"status":"completed"}}})
        }
        Transport::Pty => return Err("managed transport required".into()),
    };
    observe(sessions, source, &event)
}

#[test]
fn managed_notice_idle_writes_in_the_accepting_tick_and_records_delivery() -> Result {
    for transport in [Transport::Claude, Transport::Codex] {
        controlled_fixture(transport, false, false, |sessions, _, frames, terminal| {
            let outcome = sessions.operate(notice("idle-notice"))?;
            assert_ne!(
                outcome.state,
                OperationState::Accepted,
                "idle notice still awaits a boundary"
            );
            frame(frames, transport)?;
            delivered(sessions, "idle-notice")?;
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn managed_notice_busy_waits_for_the_matching_turn_end() -> Result {
    for transport in [Transport::Claude, Transport::Codex] {
        fixture(transport, false, |sessions, source, frames, terminal| {
            sessions.operate(notice("first-notice"))?;
            let first = frame(frames, transport)?;
            delivered(sessions, "first-notice")?;
            replay(sessions, source, &first, transport)?;
            let outcome = sessions.operate(notice("busy-notice"))?;
            assert_eq!(outcome.state, OperationState::Accepted);
            assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
            complete(sessions, source, &first, transport)?;
            frame(frames, transport)?;
            delivered(sessions, "busy-notice")?;
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn a_managed_notice_waits_for_its_matching_turn_end() -> Result {
    for transport in [Transport::Claude, Transport::Codex] {
        fixture(transport, false, |sessions, source, frames, terminal| {
            sessions.operate(notice("active-notice"))?;
            let active = frame(frames, transport)?;
            delivered(sessions, "active-notice")?;
            replay(sessions, source, &active, transport)?;
            sessions.operate(notice("old-notice"))?;
            let mut table = sessions.lock()?;
            crate::operations::deliver(&mut table, "fixture");
            assert_eq!(
                table
                    .operations
                    .get("old-notice")
                    .ok_or("notice missing")?
                    .state,
                OperationState::Accepted
            );
            drop(table);
            assert!(matches!(frames.try_recv(), Err(mpsc::TryRecvError::Empty)));
            complete(sessions, source, &active, transport)?;
            let next = frame(frames, transport)?;
            delivered(sessions, "old-notice")?;
            assert!(
                !sessions
                    .outcome("old-notice")?
                    .words
                    .contains("boundary_unsignalled")
            );
            replay(sessions, source, &next, transport)?;
            complete(sessions, source, &next, transport)?;
            assert_eq!(terminal.load(Ordering::SeqCst), 0);
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn managed_notice_early_admission_records_delivery_before_confirmation() -> Result {
    for transport in [Transport::Claude, Transport::Codex] {
        let (release, gate) = mpsc::channel();
        if transport == Transport::Codex {
            release.send(())?;
            release.send(())?;
        }
        gated_fixture(
            transport,
            false,
            false,
            Some(gate),
            |sessions, source, frames, terminal| {
                let release = Release(release);
                let mut table = sessions.lock()?;
                let outcome = crate::operations::accept(&mut table, notice("early-admission"))?;
                drop(table);
                sessions.writer.barrier()?;
                assert_eq!(outcome.state, OperationState::Delivering);
                let frame = frame(frames, transport)?;
                replay(sessions, source, &frame, transport)?;
                let states: Vec<_> = sessions
                    .lock()?
                    .feed
                    .page(None)?
                    .entries
                    .into_iter()
                    .filter_map(|entry| match entry.body {
                        Body::Operation(outcome) if outcome.operation == "early-admission" => {
                            Some(outcome.state)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    states,
                    [
                        OperationState::Accepted,
                        OperationState::Delivering,
                        OperationState::Delivered,
                        OperationState::Confirmed
                    ]
                );
                assert_eq!(terminal.load(Ordering::SeqCst), 0);
                drop(release);
                Ok(())
            },
        )?;
    }
    Ok(())
}
