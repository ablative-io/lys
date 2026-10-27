//! Launch preparation binds runner inputs before a process can spawn.

use std::sync::Arc;

use super::{Guard, Session, Sessions, Starting, lifecycle, now_ms, output};
use crate::error::RunnerError;
use crate::judge::Policy;
use crate::protocol::Launch;
use crate::rotation::RotationState;
use crate::tracking::Tracking;

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

struct LaunchRecords {
    responsible: Option<String>,
    seat_proof: Option<serde_json::Value>,
    launched: Option<(String, String)>,
    trusted: Option<crate::trust::Trust>,
}

impl Sessions {
    fn finish_launch(
        self: &Arc<Self>,
        session: Session,
        pending: lifecycle::Pending,
        reservation: Starting,
        records: LaunchRecords,
    ) -> Result<(u32, u64), RunnerError> {
        let LaunchRecords {
            responsible,
            seat_proof,
            launched,
            trusted,
        } = records;
        let id = session
            .launch
            .as_ref()
            .ok_or_else(|| {
                RunnerError::refused("spawn_install_failed", "installed session has no launch")
            })?
            .session
            .clone();
        let pid = session.pid.ok_or_else(|| {
            RunnerError::refused(
                "spawn_install_failed",
                "the installed child has no process id",
            )
        })?;
        let started_at = session.started_at;
        let mut table = match self.lock() {
            Ok(table) => table,
            Err(error) => {
                if let Err(cleanup) = pending.cancel() {
                    crate::error::said(&format!("cancelled_spawn_cleanup_failed: {cleanup}"));
                }
                return Err(error);
            }
        };
        if table.stopping || self.upgrading.load(std::sync::atomic::Ordering::Acquire) {
            drop(table);
            pending.cancel()?;
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner stopped during the launch",
            ));
        }
        table.sessions.insert(id.clone(), session);
        if let Some(person) = responsible {
            table.responsible.insert(id.clone(), person);
        }
        let answer = table
            .sessions
            .get(&id)
            .and_then(|session| session.live.as_ref())
            .and_then(|live| live.writer.seat.as_ref())
            .and_then(|pipe| pipe.answer.get())
            .cloned();
        let mut recorded = self.persist(&table);
        if let Some(proof) = seat_proof {
            recorded = recorded.and_then(|()| {
                crate::harness_control::seat_program::record(
                    &mut table,
                    &id,
                    "seat_program_started",
                    &proof,
                )
            });
        }
        if let Some(answer) = answer {
            recorded = recorded.and_then(|()| {
                crate::harness_control::seat_program::record(
                    &mut table,
                    &id,
                    "seat_program_answered",
                    &answer,
                )
            });
        }
        if let Some((executable, version)) = launched {
            recorded = recorded
                .and_then(|()| lifecycle::tracking_started(&mut table, &id, &executable, &version));
        }
        if let Some(trust) = &trusted {
            recorded = recorded.and_then(|()| lifecycle::trust_recorded(&mut table, &id, trust));
        }
        // A session tracked through the proxy is bound to its usage file. A
        // run is not refused for a file that cannot be followed: what stands
        // in the way is said in its coverage.
        self.follow_proxy(&mut table, &id);
        drop(table);
        self.activate(&id, pending)?;
        drop(reservation);
        recorded?;
        // The row went into a file the harness rewrites whole at its own
        // start; another harness starting in between can lose it. The run's
        // first screen is watched for the dialog, which is answered for the
        // named folder only and said in the feed.
        if let Some(trust) = trusted
            && self
                .lock()?
                .sessions
                .get(&id)
                .is_some_and(|session| session.managed.is_none())
        {
            self.watch_trust_dialog(&id, trust.directory)?;
        }
        self.writer.barrier()?;
        self.wake();
        Ok((pid, started_at))
    }

    fn reserve_launch(
        self: &Arc<Self>,
        launch: &Launch,
        tracking: Option<&Tracking>,
    ) -> Result<(Starting, Option<RotationState>), RunnerError> {
        if let Some(tracking) = tracking {
            tracking.checked()?;
        }
        if !valid_id(&launch.session) {
            return Err(RunnerError::refused(
                "session_invalid",
                "a session id is one or more letters, digits, '-' or '_'",
            ));
        }
        let rotation = launch
            .rotation
            .clone()
            .map(RotationState::new)
            .transpose()?;
        let mut table = self.lock()?;
        if table.stopping || self.upgrading.load(std::sync::atomic::Ordering::Acquire) {
            return Err(RunnerError::refused(
                "runner_stopping",
                "the runner is stopping and starts nothing",
            ));
        }
        if table.sessions.contains_key(&launch.session) || table.starting.contains(&launch.session)
        {
            return Err(RunnerError::refused(
                "session_exists",
                format!("session {} is already held", launch.session),
            ));
        }
        table.starting.insert(launch.session.clone());
        let reservation = Starting {
            sessions: Arc::clone(self),
            id: launch.session.clone(),
        };
        drop(table);
        Ok((reservation, rotation))
    }

    fn initial_session(
        &self,
        launch: Launch,
        guard: Guard,
        managed: Option<crate::harness_control::Settings>,
        rotation: Option<RotationState>,
    ) -> Session {
        Session {
            started_at: now_ms(),
            pid: None,
            leader_start: None,
            columns: launch.columns,
            rows: launch.rows,
            output: Arc::new(output::OutputHandle::new(self.scrollback, None)),
            ended: None,
            live: None,
            generation: 0,
            launch: Some(launch),
            managed,
            rotation,
            ending: false,
            guard,
            follower: None,
            pending_status: None,
            plan_windows: None,
            stopped: None,
            attach: crate::attach::Ring::new(self.scrollback),
            last_signal: None,
            workers: Vec::new(),
        }
    }

    fn installed_seat_proof(
        session: &mut Session,
        is_seat: bool,
    ) -> Result<Option<serde_json::Value>, RunnerError> {
        if is_seat {
            let proof = session
                .live
                .as_ref()
                .and_then(|live| live.writer.seat.as_ref())
                .ok_or_else(|| {
                    RunnerError::refused(
                        "seat_program_pipe_missing",
                        "the installed seat has no adapter pipe",
                    )
                })?
                .proof
                .clone();
            let launch = session.launch.as_mut().ok_or_else(|| {
                RunnerError::refused(
                    "seat_program_record_missing",
                    "the installed seat has no launch",
                )
            })?;
            launch.environment.insert(
                crate::harness_control::seat_program::PROOF_VARIABLE.to_owned(),
                serde_json::to_string(&proof).map_err(|error| {
                    RunnerError::refused("seat_program_record_invalid", error.to_string())
                })?,
            );
            return Ok(Some(proof));
        }
        Ok(None)
    }

    pub(crate) fn begin_transport(
        self: &Arc<Self>,
        mut launch: Launch,
        policy: Option<Policy>,
        tracking: Option<Tracking>,
        proxy: Option<crate::tracking_proxy::ProxyTracking>,
        responsible: Option<String>,
        managed: Option<crate::harness_control::Settings>,
    ) -> Result<(u32, u64), RunnerError> {
        let (reservation, rotation) = self.reserve_launch(&launch, tracking.as_ref())?;
        if !launch.program.starts_with('/') && !launch.environment.contains_key("PATH") {
            let path = crate::program_path::login_path()
                .map_err(|reason| RunnerError::refused("spawn_failed", reason))?;
            let path = path.into_string().map_err(|path| {
                RunnerError::refused(
                    "spawn_failed",
                    format!("the login PATH is not text: {path:?}"),
                )
            })?;
            launch.environment.insert("PATH".to_owned(), path);
        }
        let launched = tracking
            .as_ref()
            .map(|tracking| lifecycle::launched(&launch, tracking))
            .transpose()?;
        let harness = launch.config.as_ref().and_then(|config| config.harness);
        crate::launch_config::prepare(&self.state_dir, &mut launch)?;
        if managed.is_none() {
            crate::pty_conversation::prepare(&mut launch)?;
        }
        self.status_line_named(&mut launch);
        super::notify_named(&mut launch)?;
        let mut seat_proof = crate::harness_control::seat_program::prepare(&mut launch)?;
        let cwd = lifecycle::bound_directory(&launch.directory);
        // Naming the folder is the trust answer: a Claude Code run's folder
        // is recorded as trusted in the configuration the harness reads for
        // this run, before anything is spawned, so no run sits at a prompt
        // nobody sees. The environment is read after prepare, which binds
        // the variables the profile points at its config files.
        let trusted = match harness {
            Some(crate::tracking::Harness::ClaudeCode) => Some(crate::trust::record(
                &crate::trust::config_home(&launch.environment)?,
                &cwd,
            )?),
            Some(crate::tracking::Harness::Codex) | None => None,
        };
        let guard = Guard {
            policy,
            tracking,
            proxy,
            leader: None,
            cwd,
            idle: true,
            boundary_observed: false,
        };
        let mut session = self.initial_session(launch, guard, managed, rotation);
        let prepared = self.run(&lifecycle::plan(&session, false)?)?;
        let pending = Self::install(&mut session, prepared)?;
        seat_proof = Self::installed_seat_proof(&mut session, seat_proof.is_some())?;
        if let Some(note) = session
            .launch
            .as_ref()
            .and_then(|launch| launch.environment.get("LYS_PTY_STATUS_NOTE"))
            && let Err(error) = session.output.push(
                session.generation,
                format!("\r\nLys: {note}\r\n").as_bytes(),
            )
        {
            pending.cancel()?;
            return Err(error);
        }
        self.finish_launch(
            session,
            pending,
            reservation,
            LaunchRecords {
                responsible,
                seat_proof,
                launched,
                trusted,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::error::Error;

    use super::{Launch, Sessions, valid_id};

    #[test]
    fn session_id_check_accepts_every_allowed_ascii_character() {
        for id in [
            "a",
            "Z",
            "0",
            "-",
            "_",
            "Az09-_",
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_",
        ] {
            assert!(valid_id(id), "refused {id:?}");
        }
    }

    #[test]
    fn session_id_check_rejects_empty_non_ascii_and_punctuation() {
        for id in ["", " ", "a b", "a/b", "a.b", "a\n", "a\0", "é", "aé", "中"] {
            assert!(!valid_id(id), "accepted {id:?}");
        }
        for character in '\0'..='\x7f' {
            if !character.is_ascii_alphanumeric() && character != '-' && character != '_' {
                assert!(!valid_id(&character.to_string()), "accepted {character:?}");
            }
        }
    }

    #[test]
    fn a_fresh_status_note_is_retained_without_refusing_the_installed_child()
    -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 4096)?;
        let note = "no earlier conversation to resume; started a new one";
        let (pid, _) = sessions.start(Launch {
            session: "fresh-status-note".to_owned(),
            program: "/usr/bin/true".to_owned(),
            arguments: Vec::new(),
            directory: dir.path().display().to_string(),
            environment: BTreeMap::from([("LYS_PTY_STATUS_NOTE".to_owned(), note.to_owned())]),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        })?;
        assert!(pid > 0);
        let output = {
            let table = sessions.read_lock()?;
            let session = table
                .sessions
                .get("fresh-status-note")
                .ok_or("started session not installed")?;
            assert!(session.generation > 0);
            session.output.lock()?.scrollback().from(0)?
        };
        assert!(String::from_utf8(output)?.contains(note));
        sessions.end(
            "fresh-status-note",
            &std::sync::atomic::AtomicBool::new(false),
        )?;
        Ok(())
    }

    #[test]
    fn launch_refuses_invalid_session_ids_before_spawning() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let sessions = Sessions::open(dir.path(), 4096)?;
        for id in ["", "../session", "session name", "session-é"] {
            let error = sessions
                .start(Launch {
                    session: id.to_owned(),
                    program: "/no-such-session-program".to_owned(),
                    arguments: Vec::new(),
                    directory: dir.path().display().to_string(),
                    environment: BTreeMap::new(),
                    config: None,
                    columns: 80,
                    rows: 24,
                    rotation: None,
                    policy: None,
                })
                .err()
                .ok_or("invalid session id was accepted")?;
            assert_eq!(error.name(), "session_invalid", "{id:?}: {error}");
        }
        assert!(sessions.read_lock()?.sessions.is_empty());
        Ok(())
    }
}

#[cfg(test)]
#[path = "seat_program_launch_tests.rs"]
pub(crate) mod seat_program_launch_tests;

#[cfg(test)]
#[path = "seat_program_stdin_tests.rs"]
mod seat_program_stdin_tests;
