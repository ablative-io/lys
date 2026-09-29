#![cfg(test)]
//! Actual pipes, with a non-model echo child: one reader and a durable-write
//! ordering seam. This proves no installed harness capability or protection.
use std::collections::BTreeSet;
use std::process::{Child, Command, Stdio};

use lys_runner::RunnerError;
use lys_runner::harness_control::process::{Pipes, WriteAhead};
use lys_runner::peer::{Leader, Processes, System};
use serde_json::json;

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
struct Journal(BTreeSet<String>);
impl WriteAhead for Journal {
    fn before_write(&mut self, operation: &str, _encoded: &[u8]) -> Result<(), RunnerError> {
        if !self.0.insert(operation.to_owned()) {
            return Err(RunnerError::refused(
                "control_delivery_uncertain",
                "already possibly sent",
            ));
        }
        Ok(())
    }
}

fn echo() -> Result<(Held, Leader), Box<dyn std::error::Error>> {
    let child = Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let leader = Leader {
        pid: child.id(),
        start: System.start(child.id())?,
    };
    Ok((Held(child), leader))
}

#[test]
fn one_reader_sees_json_data_after_the_recorded_write() -> TestResult {
    let (mut child, leader) = echo()?;
    let mut pipes = Pipes::attach(&mut child.0, &leader)?;
    let mut reader = pipes.take_reader()?;
    assert!(pipes.take_reader().is_err());
    let frame = json!({"message":"Lys reminder\n/compact\nquoted \"words\""});
    let mut journal = Journal::default();
    pipes.send("stable-occurrence", &frame, &mut journal)?;
    assert!(journal.0.contains("stable-occurrence"));
    assert_eq!(reader.next_frame()?, Some(frame.clone()));
    assert!(
        pipes
            .send("stable-occurrence", &frame, &mut journal)
            .is_err()
    );
    drop(pipes);
    assert_eq!(reader.next_frame()?, None);
    Ok(())
}

#[test]
fn a_foreign_process_identity_cannot_take_the_pipes() -> TestResult {
    let (mut child, mut leader) = echo()?;
    leader.start.0.push_str("-other");
    assert!(Pipes::attach(&mut child.0, &leader).is_err());
    assert!(child.0.stdin.is_some() && child.0.stdout.is_some());
    Ok(())
}
