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

impl Sessions {
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
            if session.ended.is_some() {
                continue;
            }
            let launch = session.launch.clone().ok_or_else(|| {
                RunnerError::refused(
                    "session_launch_missing",
                    format!("session {id} has no launch to retain; no runner was stopped"),
                )
            })?;
            let conversation = table
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
                crate::pty_conversation::prepare_restart(&mut checked, conversation.as_deref())?;
            }
            saved.push(UpgradeSession {
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
            });
        }
        self.upgrading.store(true, Ordering::Release);
        Ok(saved)
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

#[cfg(test)]
mod tests {
    use super::*;
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
