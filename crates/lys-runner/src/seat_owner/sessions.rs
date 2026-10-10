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
        self.owner_state.set(state).map_err(|_already_set| {
            refused("seat_owner_held", "this runner already is a seat owner")
        })
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
        self.owned.lock().map_err(|_poisoned| {
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
    /// `seat_owner_held` when the session already has an owner, another
    /// start for it is under way, or it is a session of this runner;
    /// `seat_owner_start_failed` when this runner serves no socket yet, or
    /// the owner cannot be started; the owner's own refusals, read from its
    /// ready pipe.
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
        // Reserved until this start ends, ready or refused: a concurrent
        // start for the same seat is refused here by name and spawns nothing.
        let reservation = self.reserve_owner_start(&binding.session)?;
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
        super::recovery::record_endpoint(&seat)?;
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
        drop(owned);
        drop(reservation);
        Ok((pid, at))
    }

    /// Reserves `session` for one owner start, under the owned index's lock,
    /// so the check and the reservation are one step.
    ///
    /// # Errors
    ///
    /// `seat_owner_held` when the session already has an owner or another
    /// start for it is under way; `seat_owner_store_unavailable` when a lock
    /// is poisoned.
    fn reserve_owner_start(&self, session: &str) -> Result<OwnerStartReservation<'_>, RunnerError> {
        let owned = self.owned_index()?;
        if owned.contains_key(session) {
            return Err(refused(
                "seat_owner_held",
                format!("session {session} already has an owner"),
            ));
        }
        let mut starts = self.owner_starts.lock().map_err(|_poisoned| {
            refused(
                "seat_owner_store_unavailable",
                "the owner-start reservations' lock is poisoned",
            )
        })?;
        if !starts.insert(session.to_owned()) {
            return Err(refused(
                "seat_owner_held",
                format!("an owner for session {session} is already being started"),
            ));
        }
        drop(starts);
        drop(owned);
        Ok(OwnerStartReservation {
            sessions: self,
            session: session.to_owned(),
        })
    }
}

/// One owner start under way for `session`; released when the start ends,
/// whether its owner became ready or was refused.
struct OwnerStartReservation<'a> {
    sessions: &'a Sessions,
    session: String,
}

impl Drop for OwnerStartReservation<'_> {
    fn drop(&mut self) {
        match self.sessions.owner_starts.lock() {
            Ok(mut starts) => {
                starts.remove(&self.session);
            }
            Err(poisoned) => {
                // The reservation is released all the same, and said: a
                // poisoned lock must not hold the seat reserved for ever.
                poisoned.into_inner().remove(&self.session);
                crate::error::said(&format!(
                    "seat_owner_store_unavailable: the owner-start reservations' lock was poisoned while releasing session {}",
                    self.session
                ));
            }
        }
    }
}

impl Sessions {
    /// Finds the owners a previous runner started under this state and
    /// proves each at the kernel: the live ones are indexed, the others
    /// kept and named (AGENTS-004 R2). Nothing is restarted or replayed.
    ///
    /// # Errors
    ///
    /// The recovery's refusals; an endpoint record that does not read is
    /// never skipped.
    pub fn recover_owners(&self) -> Result<super::recovery::RecoveryCounts, RunnerError> {
        let (found, counts) = super::recovery::recover(&self.state_dir)?;
        let mut owned = self.owned_index()?;
        let mut unreachable = self.unreachable_lock()?;
        for owner in found {
            if owner.live() {
                owned.insert(owner.seat.binding.session.clone(), owner.seat);
            } else {
                unreachable.push(owner);
            }
        }
        Ok(counts)
    }

    /// The owners on record the kernel did not confirm at open.
    ///
    /// # Errors
    ///
    /// `seat_owner_store_unavailable` when the list's lock is poisoned.
    pub fn unreachable_owners(&self) -> Result<Vec<super::recovery::Found>, RunnerError> {
        Ok(self.unreachable_lock()?.clone())
    }

    fn unreachable_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Vec<super::recovery::Found>>, RunnerError> {
        self.unreachable.lock().map_err(|_poisoned| {
            refused(
                "seat_owner_store_unavailable",
                "the unreachable-owner list's lock is poisoned",
            )
        })
    }

    /// Moves this owner's durable hook cursor forward by one for `id`, when
    /// this runner is the owner of `id`; a manual session moves nothing.
    ///
    /// # Errors
    ///
    /// The store's refusals.
    pub fn advance_hook_cursor(&self, id: &str) -> Result<(), RunnerError> {
        let Some(state) = self.owner_state() else {
            return Ok(());
        };
        if state.binding().session != id {
            return Ok(());
        }
        let mut cursors = state.view()?.cursors;
        cursors.hook = cursors.hook.saturating_add(1);
        let intent = process::derived_intent(&format!("{id}:hook"), &cursors.hook.to_string());
        state.record(
            &intent,
            &super::record::Intent::Cursors {
                session: id.to_owned(),
                cursors,
            },
        )?;
        Ok(())
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
