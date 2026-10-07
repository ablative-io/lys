//! Ordinary termination preserves the harness's final output.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Read};

use lys_runner::peer::{Leader, start_identity};
use lys_runner::pty::{self, Spawn, Spawned};

type TestResult = Result<(), Box<dyn Error>>;

struct Harness(Spawned);

impl Harness {
    fn start() -> Result<Self, Box<dyn Error>> {
        Self::running(
            "import signal, sys\ndef ended(signum, frame):\n print('final-usage:7', flush=True)\n sys.exit(0)\nsignal.signal(signal.SIGHUP, ended)\nprint('ready', flush=True)\nsignal.pause()",
        )
    }

    /// A leader that ignores the terminate signal, as an interactive shell does.
    fn deaf() -> Result<Self, Box<dyn Error>> {
        Self::running(
            "import signal\nsignal.signal(signal.SIGTERM, signal.SIG_IGN)\nprint('ready', flush=True)\nwhile True:\n signal.pause()",
        )
    }

    fn running(script: &str) -> Result<Self, Box<dyn Error>> {
        let arguments = vec!["-c".to_owned(), script.to_owned()];
        let mut harness = Self(pty::spawn(&Spawn {
            program: "/usr/bin/python3",
            arguments: &arguments,
            directory: "/",
            environment: &BTreeMap::new(),
            columns: 80,
            rows: 24,
        })?);
        let mut ready = String::new();
        BufReader::new(&mut harness.0.reader).read_line(&mut ready)?;
        if ready.trim() != "ready" {
            return Err("harness ended before becoming ready".into());
        }
        Ok(harness)
    }

    fn leader(&self) -> Result<Leader, Box<dyn Error>> {
        Ok(Leader {
            pid: self.0.pid,
            start: start_identity(self.0.pid)?,
        })
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        match self.0.child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => eprintln!("harness cleanup status: {error}"),
        }
        if let Err(error) = self.0.child.kill() {
            eprintln!("harness cleanup signal: {error}");
        }
        if let Err(error) = self.0.child.wait() {
            eprintln!("harness cleanup wait: {error}");
        }
    }
}

#[test]
fn ordinary_end_preserves_the_final_usage_line() -> TestResult {
    let mut harness = Harness::start()?;
    let leader = harness.leader()?;
    pty::end(&leader)?;
    let mut output = String::new();
    harness.0.reader.read_to_string(&mut output)?;
    let status = harness.0.child.wait()?;
    assert!(status.success(), "ordinary stop must let the trap finish");
    assert_eq!(
        output
            .lines()
            .filter(|line| *line == "final-usage:7")
            .count(),
        1
    );
    assert_eq!(pty::end_left_group(&leader)?, pty::Left::Gone);
    Ok(())
}

#[test]
fn ordinary_end_refuses_a_reused_leader_identity() -> TestResult {
    let harness = Harness::start()?;
    let mut leader = harness.leader()?;
    leader.start.0.push_str(":different");
    let answer = pty::end(&leader);
    assert_eq!(
        answer.err().ok_or("reused leader accepted")?.name(),
        "process_start_mismatch"
    );
    assert_eq!(start_identity(harness.0.pid)?, harness.leader()?.start);
    Ok(())
}

#[test]
fn emergency_end_does_not_wait_for_final_usage() -> TestResult {
    let mut harness = Harness::start()?;
    pty::end_group(harness.0.pid)?;
    assert!(!harness.0.child.wait()?.success());
    let mut output = String::new();
    harness.0.reader.read_to_string(&mut output)?;
    assert!(!output.contains("final-usage"));
    Ok(())
}

#[test]
fn ordinary_end_ends_a_leader_that_ignores_the_terminate_signal() -> TestResult {
    let mut harness = Harness::deaf()?;
    let leader = harness.leader()?;
    pty::end(&leader)?;
    let status = harness.0.child.wait()?;
    assert!(!status.success(), "the hang-up ends a leader deaf to TERM");
    assert_eq!(pty::end_left_group(&leader)?, pty::Left::Gone);
    Ok(())
}

#[test]
fn cancellation_refuses_an_unconfirmed_exit_without_a_clock() -> TestResult {
    use std::os::fd::AsFd;
    let mut harness = Harness::start()?;
    let leader = harness.leader()?;
    let (cancel, sender) = std::os::unix::net::UnixStream::pair()?;
    let ending = pty::prepare_left_group(&leader)?;
    drop(sender);
    let pty::Left::Unended { reason } = ending.wait(Some(cancel.as_fd())) else {
        return Err("cancelled cleanup accepted an unconfirmed exit".into());
    };
    assert!(reason.contains("cancelled"), "{reason}");
    assert!(reason.contains(&leader.pid.to_string()), "{reason}");
    assert!(!harness.0.child.wait()?.success());
    Ok(())
}
