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
        let arguments = vec![
            "-c".to_owned(),
            "trap 'printf \"final-usage:7\\n\"; exit 0' TERM; printf 'ready\\n'; IFS= read -r line"
                .to_owned(),
        ];
        let mut harness = Self(pty::spawn(&Spawn {
            program: "/bin/sh",
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
    let status = harness.0.child.wait()?;
    let mut output = String::new();
    harness.0.reader.read_to_string(&mut output)?;
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
