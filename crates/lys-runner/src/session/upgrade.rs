//! An upgrade retains admitted launch inputs before the runner is stopped.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};

use super::Sessions;
use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::harness_control::{ManagedLaunch, Settings};
use crate::protocol::Launch;
use crate::tracking::Tracking;
use crate::tracking_proxy::ProxyTracking;

/// Private launch inputs retained for a controlled runner replacement.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeSession {
    /// The session's admitted launch, including its private configuration.
    pub launch: Launch,
    /// The agent whose policy this launch carries, when it is an agent.
    pub agent: Option<String>,
    /// The native conversation observed by the harness.
    pub conversation: Option<String>,
    /// The retained tracking contract.
    pub tracking: Option<Tracking>,
    /// The retained model-proxy binding.
    pub proxy: Option<ProxyTracking>,
    /// The managed transport, when the session uses one.
    pub managed: Option<ManagedLaunch>,
    /// The caller responsible for a launch made as a caller.
    pub responsible: Option<String>,
}

impl std::fmt::Debug for UpgradeSession {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("UpgradeSession")
            .field("session", &self.launch.session)
            .field("agent", &self.agent)
            .field("directory", &self.launch.directory)
            .field("conversation", &self.conversation)
            .finish_non_exhaustive()
    }
}

/// The sessions captured, and each that could not be with its reason.
pub type Captured = (Vec<UpgradeSession>, Vec<(String, RunnerError)>);

impl Sessions {
    /// Whether a controlled replacement has taken responsibility for restoration.
    pub(crate) fn upgrade_prepared(&self) -> bool {
        self.upgrading.load(Ordering::Acquire)
    }

    /// Capture every live launch and prevent new starts until replacement or cancellation.
    ///
    /// # Errors
    /// Refuses an unfinished start, missing launch inputs, or an unreadable session table.
    pub fn prepare_upgrade(&self) -> Result<Vec<UpgradeSession>, RunnerError> {
        let table = self.lock()?;
        if !table.starting.is_empty() {
            return Err(RunnerError::refused(
                "runner_start_in_progress",
                "a session is still starting; no runner was stopped",
            ));
        }
        let mut saved = Vec::new();
        for (id, session) in &table.sessions {
            if session.ended.is_none() {
                saved.push(capture(&table, id, session)?);
            }
        }
        self.upgrading.store(true, Ordering::Release);
        Ok(saved)
    }

    /// Capture each live session on its own, for a stop that goes ahead
    /// whatever is captured: a session that cannot be captured is answered
    /// with its reason beside the ones that were, never instead of them.
    ///
    /// # Errors
    /// Refuses only an unreadable session table.
    pub fn capture_each(&self) -> Result<Captured, RunnerError> {
        let table = self.lock()?;
        let (mut saved, mut failed) = (Vec::new(), Vec::new());
        for (id, session) in &table.sessions {
            if session.ended.is_some() {
                continue;
            }
            match capture(&table, id, session) {
                Ok(kept) => saved.push(kept),
                Err(error) => failed.push((id.clone(), error)),
            }
        }
        self.upgrading.store(true, Ordering::Release);
        Ok((saved, failed))
    }

    /// Release a captured runner when an upgrade fails before its stop.
    pub fn cancel_upgrade(&self) {
        self.upgrading.store(false, Ordering::Release);
    }

    /// Resume retained inputs only after the previous session's exit is recorded.
    ///
    /// # Errors
    /// Refuses live-session replacement, missing transcript access, and launch failures.
    pub fn resume_upgrade(
        self: &Arc<Self>,
        mut saved: UpgradeSession,
    ) -> Result<(u32, u64), RunnerError> {
        let id = saved.launch.session.clone();
        let policy = saved
            .launch
            .policy
            .clone()
            .map(|admitted| Admitted::verified(*admitted))
            .transpose()?;
        if saved.managed.is_none() {
            crate::pty_conversation::prepare_restart(
                &mut saved.launch,
                saved.conversation.as_deref(),
            )?;
        }
        let mut table = self.lock()?;
        if table
            .sessions
            .get(&id)
            .is_some_and(|session| session.ended.is_none())
        {
            return Err(RunnerError::refused(
                "session_still_running",
                format!("session {id} is still running; it was not replaced"),
            ));
        }
        if table
            .sessions
            .get(&id)
            .and_then(|session| {
                session.stopped.as_ref().or_else(|| {
                    session
                        .ended
                        .as_ref()
                        .and_then(|ended| ended.stopped.as_ref())
                })
            })
            .is_some_and(|stopped| stopped.by != "the runner")
        {
            return Err(RunnerError::refused(
                "session_stopped_by_console",
                format!(
                    "session {id} was deliberately stopped; an upgrade does not undo that stop"
                ),
            ));
        }
        let previous = table.sessions.remove(&id);
        drop(table);
        let managed = saved.managed.map(|managed| Settings {
            transport: managed.transport,
            conversation: managed.conversation,
            resume: managed.resume,
        });
        let result = self.begin_transport(
            saved.launch,
            policy,
            saved.tracking,
            saved.proxy,
            saved.responsible,
            managed,
        );
        if result.is_err() {
            let mut table = self.lock()?;
            if let Some(previous) = previous {
                table.sessions.entry(id).or_insert(previous);
            }
        }
        result
    }
}

/// The arguments of `launch` without any that name a native conversation.
fn without_conversation(arguments: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut given = arguments.iter();
    while let Some(argument) = given.next() {
        let flag = argument
            .split_once('=')
            .map_or(argument.as_str(), |(flag, _)| flag);
        if matches!(flag, "--session-id" | "--resume" | "-r") {
            if !argument.contains('=') {
                given.next();
            }
            continue;
        }
        kept.push(argument.clone());
    }
    kept
}

/// One live session's admitted launch and the conversation it holds. A
/// Claude Code session nobody has written to yet has no transcript, so there
/// is nothing to resume: it is kept to start afresh, never refused.
fn capture(
    table: &super::Table,
    id: &str,
    session: &super::Session,
) -> Result<UpgradeSession, RunnerError> {
    let mut launch = session.launch.clone().ok_or_else(|| {
        RunnerError::refused(
            "session_launch_missing",
            format!("session {id} has no launch to retain; no runner was stopped"),
        )
    })?;
    let mut conversation = table
        .feed
        .source(id)
        .and_then(|source| source.native_conversation.clone())
        .or_else(|| {
            launch.arguments.windows(2).find_map(|pair| {
                matches!(pair[0].as_str(), "--session-id" | "--resume" | "resume")
                    .then(|| pair[1].clone())
            })
        });
    if session.managed.is_none() {
        let mut checked = launch.clone();
        match crate::pty_conversation::prepare_restart(&mut checked, conversation.as_deref()) {
            Ok(()) => {}
            Err(error) if error.name() == "pty_resume_missing" => {
                launch.arguments = without_conversation(&launch.arguments);
                conversation = None;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(UpgradeSession {
        agent: launch
            .policy
            .as_ref()
            .map(|admitted| admitted.policy.agent.clone()),
        conversation: conversation.clone(),
        tracking: session.guard.tracking.clone(),
        proxy: session.guard.proxy.clone(),
        managed: session.managed.as_ref().map(|settings| ManagedLaunch {
            launch: launch.clone(),
            transport: settings.transport,
            conversation: conversation
                .clone()
                .unwrap_or_else(|| settings.conversation.clone()),
            resume: conversation.as_deref().is_some_and(|id| !id.is_empty())
                || !settings.conversation.is_empty(),
            requires_controls: true,
            owner: None,
        }),
        launch,
        responsible: table.responsible.get(id).cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_with_nothing_said_yet_is_kept_without_its_conversation_arguments() {
        let given: Vec<String> = [
            "--mcp-config",
            "mcp.json",
            "--session-id",
            "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee",
            "--resume=x",
            "-r",
            "y",
            "--model",
            "opus",
        ]
        .map(str::to_owned)
        .to_vec();
        assert_eq!(
            without_conversation(&given),
            ["--mcp-config", "mcp.json", "--model", "opus"]
                .map(str::to_owned)
                .to_vec()
        );
    }

    #[test]
    fn capture_each_answers_every_live_session_and_skips_ended_ones()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let sessions = Sessions::open(root.path(), 4096)?;
        let launch = |id: &str| Launch {
            session: id.into(),
            program: "/bin/cat".into(),
            arguments: Vec::new(),
            directory: root.path().display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        sessions.start(launch("live"))?;
        sessions.start(launch("ended"))?;
        sessions.end("ended", &AtomicBool::new(false))?;
        let (kept, failed) = sessions.capture_each()?;
        assert_eq!(
            kept.iter()
                .map(|saved| saved.launch.session.as_str())
                .collect::<Vec<_>>(),
            ["live"]
        );
        assert!(failed.is_empty());
        sessions.cancel_upgrade();
        sessions.end("live", &AtomicBool::new(false))?;
        Ok(())
    }
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn captured_sessions_resume_after_reopen_and_new_starts_are_fenced()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let sessions = Sessions::open(root.path(), 4096)?;
        let launch = Launch {
            session: "retained".into(),
            program: "/bin/cat".into(),
            arguments: Vec::new(),
            directory: root.path().display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        let (old_pid, _) = sessions.start(launch.clone())?;
        let saved = sessions
            .prepare_upgrade()?
            .pop()
            .ok_or("no live launch retained")?;
        assert_eq!(saved.launch.directory, launch.directory);
        let mut other = launch;
        other.session = "racing-start".into();
        assert!(
            sessions
                .start(other)
                .is_err_and(|error| error.name() == "runner_stopping")
        );
        sessions.end("retained", &AtomicBool::new(false))?;
        drop(sessions);
        let replacement = Sessions::open(root.path(), 4096)?;
        let (new_pid, _) = replacement.resume_upgrade(saved)?;
        assert_ne!(new_pid, old_pid);
        assert!(
            replacement
                .status(Some("retained"))?
                .sessions
                .iter()
                .all(|session| session.ended.is_none())
        );
        replacement.end("retained", &AtomicBool::new(false))?;
        Ok(())
    }
    #[test]
    fn a_console_stop_is_not_undone_by_an_upgrade_resume() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?;
        let sessions = Sessions::open(root.path(), 4096)?;
        let launch = Launch {
            session: "console-stopped".into(),
            program: "/bin/cat".into(),
            arguments: Vec::new(),
            directory: root.path().display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        sessions.start(launch)?;
        let saved = sessions
            .prepare_upgrade()?
            .pop()
            .ok_or("no launch retained")?;
        sessions.stop_everything(
            "a person at the console",
            "deliberate stop",
            super::super::Ask {
                kill: true,
                settling: false,
            },
            &AtomicBool::new(false),
        )?;
        drop(sessions);
        let replacement = Sessions::open(root.path(), 4096)?;
        let result = replacement.resume_upgrade(saved);
        if result.is_ok() {
            replacement.end("console-stopped", &AtomicBool::new(false))?;
        }
        assert!(
            result.is_err_and(|error| error.name() == "session_stopped_by_console"),
            "the console stop was undone by the upgrade"
        );
        Ok(())
    }
}
