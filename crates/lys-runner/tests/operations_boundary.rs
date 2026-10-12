//! Operations hold tracked input until a signal or the named fallback.
use super::*;
use crate::tracking::{Harness, Tracking};
use crate::{Launch, Sessions};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::atomic::AtomicBool;
type TestResult = Result<(), Box<dyn Error>>;

struct Cleanup {
    dir: tempfile::TempDir,
    sessions: std::sync::Arc<Sessions>,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Err(error) = self.sessions.stop_all() {
            crate::error::said(&format!(
                "test state {}: cleanup failed: {error}",
                self.dir.path().display()
            ));
        }
    }
}
fn tracked(harness: Harness) -> Result<(Cleanup, std::sync::Arc<Sessions>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(&dir.path().join("state"), 4096)?;
    sessions.start(Launch {
        session: "s1".to_owned(),
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
    let mut table = sessions.lock()?;
    let guard = &mut table.sessions.get_mut("s1").ok_or("no session")?.guard;
    guard.tracking = Some(Tracking {
        harness,
        adapter: harness.adapter().to_owned(),
        version: "test".to_owned(),
        config_home: dir.path().display().to_string(),
        context_window: 200_000,
        profile_version: 1,
        account: None,
        requires_pre_tool: false,
    });
    guard.idle = false;
    drop(table);
    Ok((
        Cleanup {
            dir,
            sessions: std::sync::Arc::clone(&sessions),
        },
        sessions,
    ))
}
fn asked(id: &str, request: OperationRequest) -> Operation {
    Operation {
        operation: id.to_owned(),
        session: "s1".to_owned(),
        request,
    }
}
fn compact() -> OperationRequest {
    OperationRequest::Compact {
        text: "/compact".to_owned(),
    }
}

#[test]
fn operations_busy_tracking_holds_delivery_until_the_boundary() -> TestResult {
    let (_dir, sessions) = tracked(Harness::ClaudeCode)?;
    let mut table = sessions.lock()?;
    accept(
        &mut table,
        asked(
            "held",
            OperationRequest::Notice {
                text: "warning".to_owned(),
            },
        ),
    )?;
    deliver(&mut table, "s1");
    assert_eq!(
        table.operations.get("held").ok_or("no operation")?.state,
        OperationState::Accepted
    );
    table.sessions.get_mut("s1").ok_or("no session")?.guard.idle = true;
    deliver(&mut table, "s1");
    drop(table);
    sessions.until_any(&AtomicBool::new(false), |table| {
        table
            .operations
            .get("held")
            .filter(|held| held.state == OperationState::Delivered)
            .cloned()
    })?;
    sessions.stop_all()?;
    Ok(())
}
#[test]
fn operations_boundary_fallback_names_the_wait_on_the_receipt() -> TestResult {
    let (_dir, sessions) = tracked(Harness::ClaudeCode)?;
    let mut table = sessions.lock()?;
    accept(
        &mut table,
        asked(
            "held",
            OperationRequest::Notice {
                text: "warning".to_owned(),
            },
        ),
    )?;
    let at = table.operations.get("held").ok_or("no operation")?.at;
    delivery::deliver_at(&mut table, "s1", at + 20 * 60 * 1000 - 1);
    assert_eq!(
        table.operations.get("held").ok_or("no operation")?.state,
        OperationState::Accepted
    );
    delivery::deliver_at(&mut table, "s1", at + 20 * 60 * 1000);
    drop(table);
    let delivered = sessions.until_any(&AtomicBool::new(false), |table| {
        table
            .operations
            .get("held")
            .filter(|held| held.state == OperationState::Delivered)
            .cloned()
    })?;
    assert!(
        delivered.words.contains("boundary_unsignalled"),
        "{delivered:?}"
    );
    assert!(
        delivered.words.contains("wait_seconds=1200"),
        "{delivered:?}"
    );
    sessions.stop_all()?;
    Ok(())
}
#[test]
fn operations_codex_turn_end_confirms_a_delivered_compact() -> TestResult {
    let (_dir, sessions) = tracked(Harness::Codex)?;
    let mut table = sessions.lock()?;
    accept(&mut table, asked("compact", compact()))?;
    table
        .operations
        .set("compact", OperationState::Delivered, "typed".to_owned())?;
    table.operations.accepted.remove("s1");
    confirm_on_turn_end(&mut table, "s1");
    assert_eq!(
        table.operations.get("compact").ok_or("no operation")?.state,
        OperationState::Confirmed
    );
    drop(table);
    sessions.stop_all()?;
    Ok(())
}

#[test]
fn operations_turn_end_is_a_no_op_for_other_requests_states_and_harnesses() -> TestResult {
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        for state in [
            OperationState::Accepted,
            OperationState::Delivering,
            OperationState::Delivered,
            OperationState::Confirmed,
            OperationState::Refused,
        ] {
            for request in [
                compact(),
                OperationRequest::Notice {
                    text: "warning".to_owned(),
                },
            ] {
                if harness == Harness::Codex
                    && state == OperationState::Delivered
                    && request.name() == "compact"
                {
                    continue;
                }
                let (_dir, sessions) = tracked(harness)?;
                let mut table = sessions.lock()?;
                accept(&mut table, asked("held", request))?;
                let before = table.operations.set("held", state, "fixture".to_owned())?;
                confirm_on_turn_end(&mut table, "s1");
                confirm_on_turn_end(&mut table, "unknown");
                assert_eq!(table.operations.get("held"), Some(&before));
                drop(table);
                sessions.stop_all()?;
            }
        }
    }
    Ok(())
}

#[test]
fn operations_wake_waits_for_confirmation_and_is_next_once() -> TestResult {
    let (_dir, sessions) = tracked(Harness::ClaudeCode)?;
    sessions.operate(asked("compact", compact()))?;
    sessions.operate(asked(
        "wake-up-compact",
        OperationRequest::WakeUp {
            text: "resume".to_owned(),
            compaction: "compact".to_owned(),
        },
    ))?;
    let mut table = sessions.lock()?;
    table.sessions.get_mut("s1").ok_or("no session")?.guard.idle = true;
    deliver(&mut table, "s1");
    drop(table);
    sessions.until_any(&AtomicBool::new(false), |table| {
        table
            .operations
            .get("compact")
            .filter(|held| held.state == OperationState::Delivered)
            .cloned()
    })?;
    let mut table = sessions.lock()?;
    assert_eq!(
        table
            .operations
            .get("wake-up-compact")
            .ok_or("no wake")?
            .state,
        OperationState::Accepted
    );
    compacting(&mut table, "s1");
    compacting(&mut table, "s1");
    accept(
        &mut table,
        asked(
            "later",
            OperationRequest::Notice {
                text: "later".to_owned(),
            },
        ),
    )?;
    table.sessions.get_mut("s1").ok_or("no session")?.guard.idle = true;
    deliver(&mut table, "s1");
    drop(table);
    let wake = sessions.until_any(&AtomicBool::new(false), |table| {
        table
            .operations
            .get("wake-up-compact")
            .filter(|held| held.state == OperationState::Delivered)
            .cloned()
    })?;
    assert!(wake.words.contains("compaction=compact"), "{wake:?}");
    assert_eq!(sessions.outcome("later")?.state, OperationState::Accepted);
    assert_eq!(
        sessions.operate(asked(
            "wake-up-compact",
            OperationRequest::WakeUp {
                text: "resume".to_owned(),
                compaction: "compact".to_owned()
            }
        ))?,
        wake
    );
    sessions.stop_all()?;
    Ok(())
}
