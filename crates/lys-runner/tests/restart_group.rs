#![cfg(test)]
//! A recorded group number is not proof that the group still belongs to a session.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdout, Command, Stdio};

use lys_runner::{EndedHow, Sessions};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

struct BlockedChild {
    child: Child,
    output: BufReader<ChildStdout>,
}

impl BlockedChild {
    fn start() -> Result<Self, Box<dyn Error>> {
        let mut child = Command::new("/bin/sh")
            .args([
                "-c",
                "printf 'ready\\n'; IFS= read -r line; printf '%s\\n' \"$line\"; IFS= read -r line",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .process_group(0)
            .spawn()?;
        let output = child.stdout.take().ok_or("ChildOutputMissing")?;
        let mut held = Self {
            child,
            output: BufReader::new(output),
        };
        let mut ready = String::new();
        held.output.read_line(&mut ready)?;
        if ready != "ready\n" {
            return Err("ChildReadyMissing: child ended before its pipe wait".into());
        }
        Ok(held)
    }

    fn answers(&mut self) -> Result<bool, Box<dyn Error>> {
        let sent = self
            .child
            .stdin
            .as_mut()
            .ok_or("ChildInputMissing")?
            .write_all(b"alive\n");
        match sent {
            Ok(()) => {
                let mut answer = String::new();
                self.output.read_line(&mut answer)?;
                Ok(answer == "alive\n")
            }
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
            Err(error) => Err(format!("ChildProbeFailed: {error}").into()),
        }
    }

    fn close(&mut self) -> TestResult {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        self.child.wait()?;
        Ok(())
    }
}

impl Drop for BlockedChild {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("ChildCleanupFailed: {error}");
        }
    }
}

#[test]
fn a_legacy_restart_never_signals_an_unproved_live_process_group() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut child = BlockedChild::start()?;
    let record = json!({
        "format": "lys-runner-sessions/v1",
        "sessions": [{
            "session": "unproved",
            "pid": child.child.id(),
            "started_at": 1,
            "columns": 80,
            "rows": 24,
            "ended": null
        }]
    });
    std::fs::write(
        dir.path().join("sessions.json"),
        serde_json::to_vec(&record)?,
    )?;
    let restarted = Sessions::open(dir.path(), 4096)?;
    let status = restarted.status(None)?;
    let ended = status
        .sessions
        .iter()
        .find(|session| session.session == "unproved")
        .and_then(|session| session.ended.as_ref())
        .ok_or("RestartEndMissing")?;
    let survived = child.answers()?;
    child.close()?;
    assert!(
        survived,
        "a legacy record cannot prove that a live process group is the one the runner started"
    );
    assert_eq!(ended.how, EndedHow::EndedByRunnerRestart);
    assert_eq!(ended.signal, None, "an unproved group is never signalled");
    Ok(())
}
