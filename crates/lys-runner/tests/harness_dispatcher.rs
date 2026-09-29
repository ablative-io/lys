#![cfg(test)]
//! Real pipes and typed fixture events, without a model or a production session.
use std::process::{Child, Command, Stdio};

use lys_runner::RunnerError;
use lys_runner::containment_policy::Binding;
use lys_runner::harness_control::codex;
use lys_runner::harness_control::dispatcher::{Dispatcher, Prepared};
use lys_runner::harness_control::events::{Boundary, Event, Kind, Source};
use lys_runner::harness_control::process::{Pipes, WriteAhead};
use lys_runner::peer::{Leader, Processes, System};

type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Held(Child);
impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = self.0.kill() {
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::InvalidInput,
                "the fixture child could not be stopped: {error}"
            );
        }
        self.0
            .wait()
            .expect("the fixture child's exit must be reaped");
    }
}

#[derive(Default)]
struct Journal(Vec<String>);
impl WriteAhead for Journal {
    fn before_write(&mut self, operation: &str, _encoded: &[u8]) -> Result<(), RunnerError> {
        self.0.push(operation.to_owned());
        Ok(())
    }
}

fn setup() -> Result<(Held, Source, Dispatcher), Box<dyn std::error::Error>> {
    let child = Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let leader = Leader {
        pid: child.id(),
        start: System.start(child.id())?,
    };
    let mut child = Held(child);
    let source = Source {
        binding: Binding {
            runner: "runner".into(),
            session: "session".into(),
            incarnation: "lifetime".into(),
            agent: "agent".into(),
            revision: 1,
            digest: "digest".into(),
        },
        leader: leader.clone(),
        generation: 1,
        conversation: "thread".into(),
    };
    let pipes = Pipes::attach(&mut child.0, &leader)?;
    let dispatcher = Dispatcher::new(source.clone(), pipes)?;
    Ok((child, source, dispatcher))
}

fn event(source: &Source, kind: Kind) -> Event {
    Event {
        source: source.clone(),
        source_id: format!("{kind:?}"),
        kind,
    }
}

fn request(id: &str) -> Result<Prepared, RunnerError> {
    codex::Request::reminder(id, "thread", &Boundary::Idle, "saved words").map(Prepared::Codex)
}

#[test]
fn mismatched_native_operation_is_refused_before_journal_or_pipe_write() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut |_| Ok(()))?;
    let mut journal = Journal::default();
    assert_eq!(
        dispatcher
            .dispatch(
                "recorded",
                &request("other")?,
                &mut journal,
                &mut |_| Ok(())
            )
            .expect_err("foreign request")
            .name(),
        "control_operation_mismatch"
    );
    assert!(journal.0.is_empty());
    assert!(dispatcher.flight().is_none());
    let mut reader = dispatcher.take_reader()?;
    drop(dispatcher);
    assert_eq!(
        reader.next_frame()?,
        None,
        "nothing reached the actual pipe"
    );
    Ok(())
}

#[test]
fn a_flight_blocks_other_input_until_both_admission_and_terminal_are_durable() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    let mut journal = Journal::default();
    assert!(
        dispatcher
            .dispatch("one", &request("one")?, &mut journal, &mut |_| Ok(()))
            .is_err()
    );
    assert!(journal.0.is_empty());
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut |_| Ok(()))?;
    dispatcher.dispatch("one", &request("one")?, &mut journal, &mut |_| Ok(()))?;
    assert_eq!(journal.0, ["one"]);
    assert!(
        dispatcher
            .dispatch("two", &request("two")?, &mut journal, &mut |_| Ok(()))
            .is_err()
    );
    dispatcher.observe(
        &event(
            &source,
            Kind::TurnStarted {
                turn: "turn-one".into(),
            },
        ),
        &mut |_| Ok(()),
    )?;
    dispatcher.observe(
        &event(
            &source,
            Kind::TurnCompleted {
                turn: "turn-one".into(),
            },
        ),
        &mut |_| Ok(()),
    )?;
    assert!(
        dispatcher
            .dispatch("two", &request("two")?, &mut journal, &mut |_| Ok(()))
            .is_err(),
        "terminal alone is not admission"
    );
    dispatcher.observe(
        &event(
            &source,
            Kind::Admitted {
                operation: "one".into(),
                turn: Some("turn-one".into()),
            },
        ),
        &mut |_| Ok(()),
    )?;
    assert!(dispatcher.flight().is_none());
    dispatcher.dispatch("two", &request("two")?, &mut journal, &mut |_| Ok(()))?;
    assert!(
        dispatcher
            .observe(
                &event(
                    &source,
                    Kind::TurnCompleted {
                        turn: "turn-one".into()
                    }
                ),
                &mut |_| Ok(())
            )
            .is_err(),
        "an old terminal cannot release the next input"
    );
    assert_eq!(dispatcher.flight().ok_or("lost flight")?.operation, "two");
    assert_eq!(journal.0, ["one", "two"]);
    Ok(())
}

#[test]
fn failed_event_persistence_holds_the_flight_and_unknown_boundary() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    let mut journal = Journal::default();
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut |_| Ok(()))?;
    dispatcher.dispatch("one", &request("one")?, &mut journal, &mut |_| Ok(()))?;
    let admission = event(
        &source,
        Kind::Admitted {
            operation: "one".into(),
            turn: Some("turn-one".into()),
        },
    );
    assert!(
        dispatcher
            .observe(&admission, &mut |_| Err(RunnerError::refused(
                "fixture_store_failed",
                "not durable"
            )))
            .is_err()
    );
    assert_eq!(dispatcher.boundary(), &Boundary::Unknown);
    assert!(!dispatcher.flight().ok_or("lost flight")?.admitted);
    assert!(
        dispatcher
            .dispatch("two", &request("two")?, &mut journal, &mut |_| Ok(()))
            .is_err()
    );
    assert_eq!(journal.0, ["one"]);
    Ok(())
}

#[test]
fn another_operations_compaction_is_never_recorded_as_this_flight() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    let mut journal = Journal::default();
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut |_| Ok(()))?;
    dispatcher.dispatch("one", &request("one")?, &mut journal, &mut |_| Ok(()))?;
    dispatcher.observe(
        &event(
            &source,
            Kind::TurnStarted {
                turn: "turn-one".into(),
            },
        ),
        &mut |_| Ok(()),
    )?;
    let mut kept = 0;
    for (operation, turn) in [("other", "turn-one"), ("one", "other-turn")] {
        let compacted = event(
            &source,
            Kind::Compacted {
                operation: operation.into(),
                turn: turn.into(),
                item: "item".into(),
            },
        );
        assert_eq!(
            dispatcher
                .observe(&compacted, &mut |_| {
                    kept += 1;
                    Ok(())
                })
                .expect_err("foreign compaction")
                .name(),
            "control_operation_mismatch"
        );
    }
    assert_eq!(kept, 0);
    assert_eq!(dispatcher.boundary(), &Boundary::Active("turn-one".into()));
    Ok(())
}

#[test]
fn compact_flight_waits_for_actual_compaction_after_terminal_notification() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut |_| Ok(()))?;
    let mut journal = Journal::default();
    let compact = Prepared::Codex(codex::Request::compact(
        "compact-1",
        "thread",
        &Boundary::Idle,
    )?);
    dispatcher.dispatch("compact-1", &compact, &mut journal, &mut |_| Ok(()))?;
    for kind in [
        Kind::Admitted {
            operation: "compact-1".into(),
            turn: None,
        },
        Kind::TurnStarted {
            turn: "compact-turn".into(),
        },
        Kind::TurnCompleted {
            turn: "compact-turn".into(),
        },
    ] {
        dispatcher.observe(&event(&source, kind), &mut |_| Ok(()))?;
    }
    assert_eq!(dispatcher.boundary(), &Boundary::Idle);
    assert!(
        dispatcher.flight().is_some(),
        "terminal alone must not discard the compaction correlation"
    );
    assert!(
        dispatcher
            .dispatch("next", &request("next")?, &mut journal, &mut |_| Ok(()))
            .is_err()
    );
    dispatcher.observe(
        &event(
            &source,
            Kind::Compacted {
                operation: "compact-1".into(),
                turn: "compact-turn".into(),
                item: "native-item".into(),
            },
        ),
        &mut |_| Ok(()),
    )?;
    assert!(dispatcher.flight().is_none());
    dispatcher.dispatch("next", &request("next")?, &mut journal, &mut |_| Ok(()))?;
    assert_eq!(journal.0, ["compact-1", "next"]);
    Ok(())
}
