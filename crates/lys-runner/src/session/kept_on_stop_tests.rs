//! A runner's stop keeps its live sessions, and its next start resumes them.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use super::super::Sessions;
use super::KEPT;
use crate::protocol::Launch;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn launch(root: &std::path::Path, session: &str) -> Launch {
    Launch {
        session: session.into(),
        program: "/bin/cat".into(),
        arguments: Vec::new(),
        directory: root.display().to_string(),
        environment: BTreeMap::new(),
        config: None,
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

fn running(sessions: &Sessions, id: &str) -> Result<bool> {
    Ok(sessions
        .status(Some(id))?
        .sessions
        .iter()
        .any(|session| session.ended.is_none()))
}

#[test]
fn a_runner_stop_keeps_its_sessions_and_the_next_start_resumes_each_then_forgets_them() -> Result {
    let root = tempfile::tempdir()?;
    let sessions = Sessions::open(root.path(), 4096)?;
    let (first, _) = sessions.start(launch(root.path(), "first"))?;
    sessions.start(launch(root.path(), "second"))?;
    assert_eq!(sessions.keep_for_restart(root.path())?, 2);
    sessions.stopping_because("the runner was asked to stop by SIGTERM")?;
    sessions.end("first", &AtomicBool::new(false))?;
    sessions.end("second", &AtomicBool::new(false))?;
    drop(sessions);
    let replacement = Sessions::open(root.path(), 4096)?;
    replacement.resume_kept(root.path())?;
    assert!(running(&replacement, "first")?);
    assert!(running(&replacement, "second")?);
    let resumed = replacement
        .status(Some("first"))?
        .sessions
        .iter()
        .find_map(|session| session.pid);
    assert_ne!(resumed, Some(first));
    assert!(
        !root.path().join(KEPT).exists(),
        "the kept file outlived a full resume"
    );
    replacement.end("first", &AtomicBool::new(false))?;
    replacement.end("second", &AtomicBool::new(false))?;
    Ok(())
}

#[test]
fn a_session_whose_process_died_with_the_runner_still_resumes() -> Result {
    let root = tempfile::tempdir()?;
    let sessions = Sessions::open(root.path(), 4096)?;
    let (pid, _) = sessions.start(launch(root.path(), "killed"))?;
    assert_eq!(sessions.keep_for_restart(root.path())?, 1);
    // As systemd ends the runner's whole group, or a reboot does: the session's process dies with the runner.
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()?;
    // The runner's own stop path: marked stopped by the runner, every exit seen and recorded.
    sessions.stopping_because("the runner was asked to stop by SIGTERM")?;
    sessions.stop_all()?;
    drop(sessions);
    let replacement = Sessions::open(root.path(), 4096)?;
    replacement.resume_kept(root.path())?;
    assert!(
        running(&replacement, "killed")?,
        "a session killed with its runner was not resumed"
    );
    replacement.end("killed", &AtomicBool::new(false))?;
    Ok(())
}

#[test]
fn a_session_a_person_stopped_is_never_resumed_and_nothing_is_kept_for_it() -> Result {
    let root = tempfile::tempdir()?;
    let sessions = Sessions::open(root.path(), 4096)?;
    sessions.start(launch(root.path(), "stopped"))?;
    sessions.end("stopped", &AtomicBool::new(false))?;
    assert_eq!(sessions.keep_for_restart(root.path())?, 0);
    assert!(!root.path().join(KEPT).exists());
    Ok(())
}
