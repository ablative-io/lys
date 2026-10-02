#![cfg(test)]
//! A recorded group number is not proof that the group still belongs to a session.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::AtomicBool;

use lys_runner::peer::{Leader, start_identity};
use lys_runner::state::{Kept, KeptSession};
use lys_runner::{EndedHow, Launch, Sessions};
use serde_json::json;

#[path = "support/restart_child.rs"]
mod restart_child;
use restart_child::PtyChild;

type TestResult = Result<(), Box<dyn Error>>;

fn process(pid: u32) -> Result<rustix::process::Pid, Box<dyn Error>> {
    rustix::process::Pid::from_raw(i32::try_from(pid)?)
        .ok_or_else(|| "FixtureProcessIdInvalid".into())
}

struct OrphanedChild {
    leader: lys_runner::pty::Spawned,
    child: u32,
    input: Option<File>,
    output: Option<BufReader<File>>,
    closed: bool,
}

impl OrphanedChild {
    fn start(dir: &std::path::Path) -> Result<Self, Box<dyn Error>> {
        let input = dir.join("input");
        let output = dir.join("output");
        let made = Command::new("/usr/bin/mkfifo")
            .args([&input, &output])
            .status()?;
        if !made.success() {
            return Err(format!("FixturePipesFailed: {made}").into());
        }
        let arguments = vec![
            "-c".to_owned(),
            "/bin/sh -c 'trap \"\" HUP; printf \"%s\\n\" \"$$\"; IFS= read -r line; printf \"reply:%s\\n\" \"$line\"; IFS= read -r line' < \"$1\" > \"$2\" & wait".to_owned(),
            "orphan-fixture".to_owned(),
            input.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        ];
        let leader = lys_runner::pty::spawn(&lys_runner::pty::Spawn {
            program: "/bin/sh",
            arguments: &arguments,
            directory: "/",
            environment: &BTreeMap::new(),
            columns: 80,
            rows: 24,
        })?;
        let mut held = Self {
            leader,
            child: 0,
            input: None,
            output: None,
            closed: false,
        };
        held.input = Some(OpenOptions::new().write(true).open(input)?);
        let mut pipe = BufReader::new(File::open(output)?);
        let mut ready = String::new();
        pipe.read_line(&mut ready)?;
        held.child = ready.trim().parse()?;
        process(held.child)?;
        held.output = Some(pipe);
        Ok(held)
    }

    fn end_leader(&mut self) -> TestResult {
        rustix::process::kill_process(process(self.leader.pid)?, rustix::process::Signal::KILL)?;
        self.leader.child.wait()?;
        Ok(())
    }

    fn answers(&mut self) -> Result<bool, Box<dyn Error>> {
        let sent = self
            .input
            .as_mut()
            .ok_or("FixtureInputMissing")?
            .write_all(b"alive\n");
        match sent {
            Ok(()) => {
                let mut answer = String::new();
                self.output
                    .as_mut()
                    .ok_or("FixtureOutputMissing")?
                    .read_line(&mut answer)?;
                Ok(answer == "reply:alive\n")
            }
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
            Err(error) => Err(format!("OrphanProbeFailed: {error}").into()),
        }
    }

    fn close(&mut self) -> TestResult {
        if !self.closed {
            match rustix::process::kill_process_group(
                process(self.leader.pid)?,
                rustix::process::Signal::KILL,
            ) {
                Ok(()) | Err(rustix::io::Errno::SRCH) => {}
                Err(error) => return Err(format!("OrphanCleanupFailed: {error}").into()),
            }
            self.leader.child.wait()?;
            if let Some(output) = &mut self.output {
                std::io::copy(output, &mut std::io::sink())?;
            }
            self.closed = true;
        }
        Ok(())
    }
}

impl Drop for OrphanedChild {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("OrphanCleanupFailed: {error}");
        }
    }
}

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
    assert_eq!(
        ended.reason.as_deref(),
        Some("start identity not recorded; process group not ended")
    );
    drop(restarted);
    let migrated: Kept = serde_json::from_slice(&std::fs::read(dir.path().join("sessions.json"))?)?;
    assert_eq!(migrated.format, "lys-runner-sessions/v2");
    assert_eq!(migrated.sessions[0].leader_start, None);
    let reopened = Sessions::open(dir.path(), 4096)?;
    assert_eq!(
        reopened.status(None)?.sessions[0].ended.as_ref(),
        Some(ended)
    );
    Ok(())
}

fn record_group(dir: &std::path::Path, pid: u32, leader: Leader) -> TestResult {
    let record = Kept::new(vec![KeptSession {
        session: "recorded".to_owned(),
        pid: Some(pid),
        leader_start: Some(leader),
        started_at: 1,
        columns: 80,
        rows: 24,
        ended: None,
    }]);
    std::fs::write(dir.join("sessions.json"), serde_json::to_vec(&record)?)?;
    Ok(())
}

#[test]
fn a_restart_never_signals_a_group_whose_start_identity_changed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut child = BlockedChild::start()?;
    let pid = child.child.id();
    let mut start = start_identity(pid)?;
    start.0.push_str(":different");
    record_group(dir.path(), pid, Leader { pid, start })?;
    let restarted = Sessions::open(dir.path(), 4096)?;
    let status = restarted.status(None)?;
    let ended = status.sessions[0]
        .ended
        .as_ref()
        .ok_or("RestartEndMissing")?;
    let survived = child.answers()?;
    child.close()?;
    assert!(survived, "a different start identity is never signalled");
    assert_eq!(ended.how, EndedHow::EndedByRunnerRestart);
    assert_eq!(ended.status, None);
    assert_eq!(ended.signal, None);
    assert_eq!(
        ended.reason.as_deref(),
        Some("process group reused, not ended")
    );
    Ok(())
}

#[test]
fn a_restart_ends_only_a_group_with_its_recorded_start_identity() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut child = PtyChild::start()?;
    let pid = child.id();
    record_group(
        dir.path(),
        pid,
        Leader {
            pid,
            start: start_identity(pid)?,
        },
    )?;
    let restarted = Sessions::open(dir.path(), 4096)?;
    let status = restarted.status(None)?;
    let ended = status.sessions[0]
        .ended
        .as_ref()
        .ok_or("RestartEndMissing")?;
    let exit = child.wait()?;
    child.close()?;
    assert_eq!(exit.signal(), Some(9), "the owned group really ended");
    assert_eq!(ended.how, EndedHow::EndedByRunnerRestart);
    assert_eq!(ended.status, None);
    assert_eq!(ended.signal.as_deref(), Some("SIGKILL"));
    assert_eq!(ended.reason, None);
    Ok(())
}

#[test]
fn a_restart_ends_an_orphaned_member_and_preserves_an_unrelated_group() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut orphan = OrphanedChild::start(dir.path())?;
    let mut unrelated = BlockedChild::start()?;
    let pid = orphan.leader.pid;
    let child_session = rustix::process::getsid(Some(process(orphan.child)?))?;
    let child_group = rustix::process::getpgid(Some(process(orphan.child)?))?;
    record_group(
        dir.path(),
        pid,
        Leader {
            pid,
            start: start_identity(pid)?,
        },
    )?;
    orphan.end_leader()?;
    let restarted = Sessions::open(dir.path(), 4096)?;
    let status = restarted.status(None)?;
    let ended = status.sessions[0]
        .ended
        .as_ref()
        .ok_or("RestartEndMissing")?;
    let orphan_survived = orphan.answers()?;
    let unrelated_survived = unrelated.answers()?;
    orphan.close()?;
    unrelated.close()?;
    assert_eq!(
        child_session,
        process(pid)?,
        "the child belongs to the recorded session"
    );
    assert_eq!(
        child_group,
        process(pid)?,
        "the child retained its leader's group"
    );
    assert!(
        !orphan_survived,
        "the owned child must end after its leader has exited"
    );
    assert!(unrelated_survived, "another group must survive the restart");
    assert_eq!(ended.how, EndedHow::EndedByRunnerRestart);
    assert_eq!(ended.status, None);
    assert_eq!(ended.signal.as_deref(), Some("SIGKILL"));
    assert_eq!(
        ended.reason.as_deref(),
        Some(format!("leaderless process group {pid}").as_str())
    );
    Ok(())
}

#[test]
fn a_restart_names_and_preserves_a_member_in_another_session() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut child = BlockedChild::start()?;
    let pid = child.child.id();
    record_group(
        dir.path(),
        pid,
        Leader {
            pid,
            start: start_identity(pid)?,
        },
    )?;
    let restarted = Sessions::open(dir.path(), 4096)?;
    let status = restarted.status(None)?;
    let ended = status.sessions[0]
        .ended
        .as_ref()
        .ok_or("RestartEndMissing")?;
    let survived = child.answers()?;
    child.close()?;
    assert!(
        survived,
        "a member outside the recorded session is left running"
    );
    assert_eq!(ended.signal, None);
    let reason = ended.reason.as_ref().ok_or("UnprovedMemberReasonMissing")?;
    assert!(reason.contains(&format!("process {pid} not ended")));
    assert!(reason.contains("process_session_mismatch"));
    Ok(())
}

#[test]
fn a_spawn_records_the_actual_leader_identity_before_it_is_answered() -> TestResult {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let started = sessions.start(Launch {
        session: "spawned".to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    });
    let observed = (|| -> Result<_, Box<dyn Error>> {
        let (pid, _) = started?;
        let actual = start_identity(pid)?;
        let kept: Kept = serde_json::from_slice(&std::fs::read(dir.path().join("sessions.json"))?)?;
        let session = kept
            .sessions
            .into_iter()
            .find(|session| session.session == "spawned")
            .ok_or("SpawnRecordMissing")?;
        Ok((pid, actual, session))
    })();
    let cleanup = sessions.end("spawned", &AtomicBool::new(false));
    let (pid, actual, kept) = match (observed, cleanup) {
        (Ok(observed), Ok(_)) => observed,
        (Err(error), Ok(_)) => return Err(error),
        (Ok(_), Err(error)) => return Err(format!("SpawnCleanupFailed: {error}").into()),
        (Err(error), Err(cleanup)) => {
            return Err(format!("{error}; SpawnCleanupFailed: {cleanup}").into());
        }
    };
    assert_eq!(kept.pid, Some(pid));
    assert_eq!(kept.leader_start, Some(Leader { pid, start: actual }));
    Ok(())
}

#[test]
fn a_legacy_completed_session_migrates_without_changing_its_observed_end() -> TestResult {
    let dir = tempfile::tempdir()?;
    let record = json!({
        "format": "lys-runner-sessions/v1",
        "sessions": [{
            "session": "completed",
            "pid": null,
            "started_at": 1,
            "columns": 80,
            "rows": 24,
            "ended": { "how": "exited", "at": 2, "status": 3, "signal": null }
        }]
    });
    std::fs::write(
        dir.path().join("sessions.json"),
        serde_json::to_vec(&record)?,
    )?;
    let restarted = Sessions::open(dir.path(), 4096)?;
    let status = restarted.status(None)?;
    let ended = status.sessions[0]
        .ended
        .as_ref()
        .ok_or("MigratedEndMissing")?;
    assert_eq!(ended.how, EndedHow::Exited);
    assert_eq!(ended.at, 2);
    assert_eq!(ended.status, Some(3));
    assert_eq!(ended.signal, None);
    assert_eq!(ended.reason, None);
    let kept: Kept = serde_json::from_slice(&std::fs::read(dir.path().join("sessions.json"))?)?;
    assert_eq!(kept.format, "lys-runner-sessions/v2");
    assert_eq!(kept.sessions[0].leader_start, None);
    Ok(())
}
