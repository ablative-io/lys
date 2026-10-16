//! Liveness: what the runner itself knows of each session it holds, so a
//! seat's state is read from the runner and from no other program.
//!
//! For each session: whether its process is held and alive, whether it runs
//! managed or in a pseudo-terminal, the harness session it is bound to, whether
//! a turn is in progress, and when it last signalled (its last output, frame,
//! hook or status line). A process is alive when the runner holds it, has seen
//! no exit, and the process still answers a null signal; the runner is its
//! parent and has not reaped it, so its id names no other process. A process
//! whose existence cannot be read is refused `liveness_unreadable` by name,
//! never reported dead or alive.

use rustix::process::{Pid, test_kill_process};
use serde::{Deserialize, Serialize};

use crate::error::RunnerError;
use crate::harness_control::ControlPhase;
use crate::session::{Session, Sessions, Table, now_ms, unknown};

/// One session's liveness, from the runner's own knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LivenessView {
    /// The session.
    pub session: String,
    /// Its process id, while one is known.
    pub pid: Option<u32>,
    /// Whether its process is held, not seen to exit, and still present.
    pub alive: bool,
    /// The harness channel and whether it began a fresh conversation.
    #[serde(flatten)]
    pub harness: HarnessLiveness,
    /// The harness's own session it is bound to: the managed conversation
    /// once the harness proved it, or the session a `SessionStart` hook bound.
    pub harness_session: Option<String>,
    /// Whether a turn is in progress: a managed turn reserved or active, or
    /// a terminal session whose harness began a turn and has not stopped.
    pub turn_active: bool,
    /// When it last signalled, in milliseconds since the Unix epoch: its
    /// last output, managed frame, hook or status line.
    pub last_signal_at: Option<u64>,
    /// When it started, in milliseconds since the Unix epoch.
    pub started_at: u64,
    /// Whether its end has been recorded.
    pub ended: bool,
}

/// The harness facts reported beside a session's process liveness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HarnessLiveness {
    /// Whether it runs on a managed channel rather than a pseudo-terminal.
    pub managed: bool,
    /// The runner started a fresh conversation because the selected transcript was absent.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub started_fresh: bool,
}

/// A session's view as the table holds it, with whether its process is held.
struct Seen {
    view: LivenessView,
    held: bool,
}

pub(crate) fn recorded_conversation(
    source: &crate::tracking_store::SourceState,
    proxy: Option<&crate::tracking_proxy::ProxyTracking>,
) -> Option<String> {
    source
        .native_conversation
        .clone()
        .or_else(|| {
            (!source.bound.is_empty()
                && proxy.is_none()
                && !lys_home::proxy::usage::is_run_key(&source.bound))
            .then(|| source.bound.clone())
        })
        .filter(|conversation| !conversation.is_empty())
}

fn seen(id: &str, session: &Session, table: &Table) -> Result<Seen, RunnerError> {
    let control = session.live.as_ref().and_then(|live| live.control.as_ref());
    let turn_active = match control {
        Some(control) => {
            let status = control.controller.control_status();
            status.active.is_some()
                || matches!(status.phase, ControlPhase::Reserved | ControlPhase::Active)
        }
        None => session.managed.is_none() && session.ended.is_none() && !session.guard.idle,
    };
    let harness_session = match control {
        Some(control) => control
            .controller
            .ready()
            .then(|| control.controller.binding.conversation.clone())
            .filter(|conversation| !conversation.is_empty()),
        None => table
            .feed
            .source(id)
            .and_then(|source| recorded_conversation(source, session.guard.proxy.as_ref())),
    };
    let harness_session = match harness_session {
        Some(id) => Some(id),
        None => session
            .launch
            .as_ref()
            .map(crate::pty_conversation::fresh_native)
            .transpose()?
            .flatten(),
    };
    let started_fresh = session
        .launch
        .as_ref()
        .is_some_and(crate::pty_conversation::started_fresh);
    let output = session.output.lock()?.last_at();
    let held = session.ended.is_none() && session.live.is_some();
    Ok(Seen {
        view: LivenessView {
            session: id.to_owned(),
            pid: session.pid,
            alive: false,
            harness_session,
            turn_active,
            last_signal_at: session.last_signal.max(output),
            started_at: session.started_at,
            ended: session.ended.is_some(),
            harness: crate::liveness::HarnessLiveness {
                managed: session.managed.is_some()
                    || session.launch.as_ref().is_some_and(|launch| {
                        launch
                            .environment
                            .contains_key(crate::harness_control::seat_program::SPEC_VARIABLE)
                    }),
                started_fresh,
            },
        },
        held,
    })
}

/// Whether process `pid` is still present, refused `liveness_unreadable`
/// when that cannot be read.
fn present(session: &str, pid: u32) -> Result<bool, RunnerError> {
    let id = i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .filter(|id| id.as_raw_nonzero().get() > 1)
        .ok_or_else(|| {
            RunnerError::refused(
                "liveness_unreadable",
                format!("session {session}: {pid} is not a process id to inspect"),
            )
        })?;
    match test_kill_process(id) {
        Ok(()) => Ok(true),
        Err(rustix::io::Errno::SRCH) => Ok(false),
        Err(error) => Err(RunnerError::refused(
            "liveness_unreadable",
            format!("session {session}: process {pid}: {error}"),
        )),
    }
}

impl Sessions {
    /// The liveness of every session the runner holds, or of the one named,
    /// refused `session_unknown` when it holds none by that name.
    ///
    /// # Errors
    /// Returns `session_unknown` for an unknown session, or `liveness_unreadable` when its process cannot be inspected.
    pub fn liveness(&self, only: Option<&str>) -> Result<Vec<LivenessView>, RunnerError> {
        let table = self.lock()?;
        let held = match only {
            Some(id) => vec![seen(
                id,
                table.sessions.get(id).ok_or_else(|| unknown(id))?,
                &table,
            )?],
            None => table
                .sessions
                .iter()
                .map(|(id, session)| seen(id, session, &table))
                .collect::<Result<Vec<_>, _>>()?,
        };
        drop(table);
        held.into_iter()
            .map(|Seen { mut view, held }| {
                view.alive = match (held, view.pid) {
                    (true, Some(pid)) => present(&view.session, pid)?,
                    (true, None) => {
                        return Err(RunnerError::refused(
                            "liveness_unreadable",
                            format!(
                                "session {} is held with no process id recorded",
                                view.session
                            ),
                        ));
                    }
                    (false, _) => false,
                };
                Ok(view)
            })
            .collect()
    }

    /// Record that session `id`'s harness signalled now, through a hook, its
    /// status line or a notice. A session the runner does not hold is left
    /// to the collector, which refuses it by name.
    pub(crate) fn signalled(&self, id: &str) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        if let Some(session) = table.sessions.get_mut(id) {
            session.last_signal = Some(now_ms());
        }
        Ok(())
    }
}

#[cfg(test)]
mod wire_tests {
    use super::LivenessView;
    use serde_json::json;

    #[test]
    fn harness_facts_keep_the_existing_flat_liveness_wire() -> Result<(), Box<dyn std::error::Error>>
    {
        for fresh in [false, true] {
            let mut wire = json!({"session":"fixture", "pid":42, "alive":true, "managed":true,
                "harness_session":"conversation", "turn_active":false, "last_signal_at":1,
                "started_at":1, "ended":false});
            if fresh {
                wire["started_fresh"] = json!(true);
            }
            let view: LivenessView = serde_json::from_value(wire.clone())?;
            assert!(view.harness.managed);
            assert_eq!(view.harness.started_fresh, fresh);
            assert_eq!(serde_json::to_value(view)?, wire);
            wire["unrecognized"] = json!(true);
            assert!(serde_json::from_value::<LivenessView>(wire).is_err());
        }
        Ok(())
    }
}
