#![cfg(test)]
//! Real pipes and typed fixture events, without a model or a production session.
use std::process::{Child, Command, Stdio};

use lys_runner::RunnerError;
use lys_runner::containment_policy::Binding;
use lys_runner::harness_control::codex;
use lys_runner::harness_control::dispatcher::{Dispatcher, EventJournal, Prepared};
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

// Fresh-event fixtures exercise validation without modelling durable replay.
// Recovery and duplicate tests below use the real Feed implementation instead.
struct OnlyKeep<F>(F);
impl<F: FnMut(&Event) -> Result<(), RunnerError>> EventJournal for OnlyKeep<F> {
    fn retained(&mut self, _event: &Event) -> Result<bool, RunnerError> {
        Ok(false)
    }
    fn keep(&mut self, event: &Event) -> Result<(), RunnerError> {
        (self.0)(event)
    }
}

// Historical replay must not run a callback bound to the newer operation.
struct ReplayOnly<'a>(&'a mut lys_runner::tracking_store::Feed);
impl EventJournal for ReplayOnly<'_> {
    fn retained(&mut self, event: &Event) -> Result<bool, RunnerError> {
        self.0.control_retained(event)
    }
    fn keep(&mut self, _event: &Event) -> Result<(), RunnerError> {
        Err(RunnerError::refused(
            "fixture_wrong_receipt",
            "historical replay reached a new receipt writer",
        ))
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
    dispatcher.observe(
        &event(&source, Kind::IdleReconciled),
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
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
    dispatcher.observe(
        &event(&source, Kind::IdleReconciled),
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
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
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
    dispatcher.observe(
        &event(
            &source,
            Kind::TurnCompleted {
                turn: "turn-one".into(),
            },
        ),
        &mut OnlyKeep(|_: &Event| Ok(())),
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
        &mut OnlyKeep(|_: &Event| Ok(())),
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
                &mut OnlyKeep(|_: &Event| Ok(()))
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
    dispatcher.observe(
        &event(&source, Kind::IdleReconciled),
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
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
            .observe(
                &admission,
                &mut OnlyKeep(|_: &Event| Err(RunnerError::refused(
                    "fixture_store_failed",
                    "not durable"
                )))
            )
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
    dispatcher.observe(
        &event(&source, Kind::IdleReconciled),
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
    dispatcher.dispatch("one", &request("one")?, &mut journal, &mut |_| Ok(()))?;
    dispatcher.observe(
        &event(
            &source,
            Kind::TurnStarted {
                turn: "turn-one".into(),
            },
        ),
        &mut OnlyKeep(|_: &Event| Ok(())),
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
                .observe(
                    &compacted,
                    &mut OnlyKeep(|_: &Event| {
                        kept += 1;
                        Ok(())
                    })
                )
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
    dispatcher.observe(
        &event(&source, Kind::IdleReconciled),
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
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
        dispatcher.observe(&event(&source, kind), &mut OnlyKeep(|_: &Event| Ok(())))?;
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
        &mut OnlyKeep(|_: &Event| Ok(())),
    )?;
    assert!(dispatcher.flight().is_none());
    dispatcher.dispatch("next", &request("next")?, &mut journal, &mut |_| Ok(()))?;
    assert_eq!(journal.0, ["compact-1", "next"]);
    Ok(())
}

#[test]
fn retained_terminal_recovers_a_failed_receipt_without_another_pipe_write() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    let dir = tempfile::tempdir()?;
    let mut feed = lys_runner::tracking_store::Feed::open(dir.path())?;
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut feed)?;
    let mut journal = Journal::default();
    dispatcher.dispatch("one", &request("one")?, &mut journal, &mut |_| Ok(()))?;
    for kind in [
        Kind::Admitted {
            operation: "one".into(),
            turn: Some("turn-one".into()),
        },
        Kind::TurnStarted {
            turn: "turn-one".into(),
        },
    ] {
        dispatcher.observe(&event(&source, kind), &mut feed)?;
    }
    let terminal = event(
        &source,
        Kind::TurnCompleted {
            turn: "turn-one".into(),
        },
    );
    assert!(
        dispatcher
            .observe(
                &terminal,
                &mut OnlyKeep(|event: &Event| {
                    feed.append_control(&source, 1, event)?;
                    Err(RunnerError::refused(
                        "fixture_receipt_failed",
                        "feed survived but receipt did not",
                    ))
                })
            )
            .is_err()
    );
    assert_eq!(dispatcher.boundary(), &Boundary::Unknown);
    assert!(!dispatcher.flight().ok_or("flight lost")?.terminal);
    let cursor = feed.end();
    let mut conflict = terminal.clone();
    conflict.kind = Kind::TurnCompleted {
        turn: "other-turn".into(),
    };
    let mut rejected = 0;
    for other in [conflict, event(&source, Kind::IdleReconciled)] {
        assert_eq!(
            dispatcher
                .observe(&other, &mut feed)
                .expect_err("different retry")
                .name(),
            "control_event_unresolved"
        );
        rejected += 1;
    }
    assert_eq!(rejected, 2);
    assert_eq!(dispatcher.boundary(), &Boundary::Unknown);
    assert_eq!(feed.end(), cursor);
    assert!(
        dispatcher
            .dispatch("two", &request("two")?, &mut journal, &mut |_| Ok(()))
            .is_err()
    );
    let retained = feed.control_event(&source, &terminal.source_id)?;
    dispatcher.observe(&retained, &mut feed)?;
    assert!(dispatcher.flight().is_none());
    assert_eq!(dispatcher.boundary(), &Boundary::Idle);
    assert_eq!(journal.0, ["one"]);
    let mut reader = dispatcher.take_reader()?;
    assert_eq!(reader.next_frame()?.ok_or("missing frame")?["id"], "one");
    dispatcher.dispatch("two", &request("two")?, &mut journal, &mut |_| Ok(()))?;
    assert_eq!(
        reader.next_frame()?.ok_or("missing next frame")?["id"],
        "two"
    );
    drop(dispatcher);
    assert!(
        reader.next_frame()?.is_none(),
        "no duplicate first request reached the actual pipe"
    );
    assert_eq!(feed.page(None)?.entries.len(), 4);
    Ok(())
}

#[test]
fn retained_terminal_and_compaction_replays_do_not_release_the_next_flight() -> TestResult {
    let (_child, source, mut dispatcher) = setup()?;
    let dir = tempfile::tempdir()?;
    let mut feed = lys_runner::tracking_store::Feed::open(dir.path())?;
    dispatcher.observe(&event(&source, Kind::IdleReconciled), &mut feed)?;
    let mut journal = Journal::default();
    let compact = Prepared::Codex(codex::Request::compact(
        "compact-1",
        "thread",
        &Boundary::Idle,
    )?);
    dispatcher.dispatch("compact-1", &compact, &mut journal, &mut |_| Ok(()))?;
    let terminal = event(
        &source,
        Kind::TurnCompleted {
            turn: "compact-turn".into(),
        },
    );
    let compacted = event(
        &source,
        Kind::Compacted {
            operation: "compact-1".into(),
            turn: "compact-turn".into(),
            item: "native-item".into(),
        },
    );
    for observed in [
        event(
            &source,
            Kind::Admitted {
                operation: "compact-1".into(),
                turn: None,
            },
        ),
        event(
            &source,
            Kind::TurnStarted {
                turn: "compact-turn".into(),
            },
        ),
        terminal.clone(),
        compacted.clone(),
    ] {
        dispatcher.observe(&observed, &mut feed)?;
    }
    assert!(dispatcher.flight().is_none());
    let cursor = feed.end();
    for replay in [&terminal, &compacted] {
        dispatcher.observe(replay, &mut ReplayOnly(&mut feed))?;
    }
    assert_eq!(feed.end(), cursor);
    dispatcher.dispatch("next", &request("next")?, &mut journal, &mut |_| Ok(()))?;
    for replay in [&terminal, &compacted] {
        dispatcher.observe(replay, &mut ReplayOnly(&mut feed))?;
    }
    let flight = dispatcher.flight().ok_or("new flight was released")?;
    assert_eq!(flight.operation, "next");
    assert!(!flight.admitted && !flight.terminal);
    assert_eq!(feed.end(), cursor);
    assert_eq!(feed.page(None)?.entries.len(), 5);
    assert_eq!(journal.0, ["compact-1", "next"]);
    Ok(())
}
