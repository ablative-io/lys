//! What a runner keeps about the seats it started owners for, and how an
//! owner runner answers for itself (AGENTS-004 R1).
//!
//! The index of owned seats is a map by session: a registry read is one
//! keyed lookup, no file is opened. Starting an owner writes the owner's
//! directory, plan and public key, spawns the owner and blocks on its ready
//! line; the runner then holds the endpoint and launches no harness of its
//! own for that session.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use super::process::{self, OwnerState};
use super::protocol::{OwnerAnswer, OwnerBinding, OwnerCommand, OwnerEndpoint, owner_dir};
use super::spawn::{self, OwnerPlan, OwnerSpawn};
use crate::error::RunnerError;
use crate::harness_control::ManagedLaunch;
use crate::peer::Leader;
use crate::protocol::Ended;
use crate::session::{Sessions, now_ms};

/// The public key file an owner verifies acts with, in its directory.
pub const SERVER_KEY: &str = "server.pub";

/// A seat this runner started an owner for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedSeat {
    /// The seat, session, conversation and generation.
    pub binding: OwnerBinding,
    /// Where the owner is reached.
    pub endpoint: OwnerEndpoint,
    /// When the owner announced itself ready, in milliseconds since the
    /// Unix epoch.
    pub established_at: u64,
}

fn refused(code: &str, words: impl Into<String>) -> RunnerError {
    RunnerError::refused(code, words)
}

impl Sessions {
    /// The server's public key, said once by the runner that serves.
    pub fn set_server_key(&self, key: [u8; 32]) {
        if self.server_key.set(key).is_err() {
            crate::error::said(
                "seat_owner_server_key_repeated: the server key was said twice; the first stands",
            );
        }
    }

    /// The program owners are started as, said once; an install says its
    /// `lys`, a test its fixture. Unsaid, owners run this runner's own
    /// executable.
    pub fn set_owner_program(&self, program: std::path::PathBuf) {
        if self.owner_program.set(program).is_err() {
            crate::error::said(
                "seat_owner_program_repeated: the owner program was said twice; the first stands",
            );
        }
    }

    /// Makes this runner the owner `state` describes; said once.
    ///
    /// # Errors
    ///
    /// `seat_owner_held` when this runner already is an owner.
    pub fn adopt_owner(&self, state: Arc<OwnerState>) -> Result<(), RunnerError> {
        self.owner_state
            .set(state)
            .map_err(|_| refused("seat_owner_held", "this runner already is a seat owner"))
    }

    /// This runner's owner state, when it is a seat owner.
    #[must_use]
    pub fn owner_state(&self) -> Option<&Arc<OwnerState>> {
        self.owner_state.get()
    }

    /// The process-start identity of `id`'s leader, as recorded at spawn.
    ///
    /// # Errors
    ///
    /// `seat_owner_unknown` when the session has no recorded leader.
    pub fn leader_of(&self, id: &str) -> Result<Leader, RunnerError> {
        let table = self.lock()?;
        table
            .sessions
            .get(id)
            .and_then(|session| session.leader_start.clone())
            .ok_or_else(|| {
                refused(
                    "seat_owner_unknown",
                    format!("session {id} has no recorded leader"),
                )
            })
    }

    /// Ends the one session an owner holds, deliberately, and answers how
    /// it ended once it has.
    ///
    /// # Errors
    ///
    /// The session's end refusals.
    pub fn stop_owned(&self, id: &str) -> Result<Ended, RunnerError> {
        self.end(id, &AtomicBool::new(false))
    }

    /// The seats this runner started owners for: one keyed index, read
    /// whole for a registry listing.
    ///
    /// # Errors
    ///
    /// `seat_owner_store_unavailable` when the index's lock is poisoned.
    pub fn owned_seats(&self) -> Result<Vec<OwnedSeat>, RunnerError> {
        Ok(self.owned_index()?.values().cloned().collect())
    }

    /// The owned seat for `session`, by key.
    ///
    /// # Errors
    ///
    /// `seat_owner_store_unavailable` when the index's lock is poisoned.
    pub fn owned_seat(&self, session: &str) -> Result<Option<OwnedSeat>, RunnerError> {
        Ok(self.owned_index()?.get(session).cloned())
    }

    fn owned_index(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, OwnedSeat>>, RunnerError> {
        self.owned.lock().map_err(|_| {
            refused(
                "seat_owner_store_unavailable",
                "the owned-seat index's lock is poisoned",
            )
        })
    }

    /// Starts an independent owner for the supervised seat `binding`
    /// launches as `managed`, and holds its endpoint. Answers the owner's
    /// pid and when it announced itself, as a start answers.
    ///
    /// # Errors
    ///
    /// `seat_owner_held` when the session already has an owner or a
    /// session of this runner; `seat_owner_start_failed` when this runner
    /// serves no socket yet, or the owner cannot be started; the owner's
    /// own refusals, read from its ready pipe.
    pub fn start_owned(
        self: &Arc<Self>,
        mut managed: ManagedLaunch,
        binding: OwnerBinding,
        responsible: Option<String>,
    ) -> Result<(u32, u64), RunnerError> {
        managed.owner = None;
        binding.validate()?;
        let server_key = *self.server_key.get().ok_or_else(|| {
            refused(
                "seat_owner_start_failed",
                "this runner serves no socket yet, so it cannot start an owner",
            )
        })?;
        if self.lock()?.sessions.contains_key(&binding.session) {
            return Err(refused(
                "seat_owner_held",
                format!(
                    "session {} is a session of this runner, not a seat",
                    binding.session
                ),
            ));
        }
        if self.owned_index()?.contains_key(&binding.session) {
            return Err(refused(
                "seat_owner_held",
                format!("session {} already has an owner", binding.session),
            ));
        }
        let dir = owner_dir(&self.state_dir, &binding.session);
        fs::create_dir_all(&dir)
            .and_then(|()| fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)))
            .map_err(|error| {
                refused(
                    "seat_owner_start_failed",
                    format!("making {}: {error}", dir.display()),
                )
            })?;
        let key_path = dir.join(SERVER_KEY);
        let mut key_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&key_path)
            .map_err(|error| {
                refused(
                    "seat_owner_start_failed",
                    format!("writing {}: {error}", key_path.display()),
                )
            })?;
        key_file
            .write_all(crate::protocol::hex(&server_key).as_bytes())
            .and_then(|()| key_file.sync_all())
            .map_err(|error| {
                refused(
                    "seat_owner_start_failed",
                    format!("writing {}: {error}", key_path.display()),
                )
            })?;
        let lys = match self.owner_program.get() {
            Some(program) => program.clone(),
            None => std::env::current_exe().map_err(|error| {
                refused(
                    "seat_owner_start_failed",
                    format!("this runner's own executable is unknown: {error}"),
                )
            })?,
        };
        let plan = OwnerPlan::new(binding.clone(), managed, responsible);
        let endpoint = spawn::start(
            &OwnerSpawn {
                lys: &lys,
                state: &self.state_dir,
                server_key: &key_path,
                scrollback: self.scrollback,
            },
            &plan,
        )?;
        let seat = OwnedSeat {
            binding,
            endpoint,
            established_at: now_ms(),
        };
        let pid = seat.endpoint.owner.pid;
        let at = seat.established_at;
        let mut owned = self.owned_index()?;
        if let Some(held) = owned.get(&seat.binding.session) {
            return Err(refused(
                "seat_owner_held",
                format!(
                    "session {} got an owner at pid {} meanwhile",
                    seat.binding.session, held.endpoint.owner.pid
                ),
            ));
        }
        owned.insert(seat.binding.session.clone(), seat);
        Ok((pid, at))
    }
}

/// Answers an owner command on this runner, which must be an owner.
///
/// # Errors
///
/// `seat_owner_not_an_owner` when this runner owns no seat; the owner's
/// refusals otherwise.
pub fn answer_owner(
    sessions: &Arc<Sessions>,
    command: OwnerCommand,
    peer: Option<(u32, u32)>,
) -> Result<OwnerAnswer, RunnerError> {
    let state = sessions
        .owner_state()
        .ok_or_else(|| refused("seat_owner_not_an_owner", "this runner owns no seat"))?;
    process::answer(state, sessions, command, peer)
}
