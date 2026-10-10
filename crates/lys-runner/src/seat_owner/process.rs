//! The owner process (AGENTS-004 R1): how it serves, and how it answers
//! the owner command. How a runner starts one is [`super::spawn`].
//!
//! An owner is `lys runner seat-owner`, a runner of its own serving one
//! managed session on its own socket, so every ordinary act reaches the
//! harness unchanged; what it adds is the lease it records before it acts,
//! and the kernel-proved binding a client makes before it is answered. The
//! lease is recorded before the harness is spawned, the harness is recorded
//! before the ready line is written, and a deliberate stop is recorded as
//! stopping before the harness is ended and as exited once it has.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::counts::Meter;
use super::protocol::{
    ClientKind, OWNER_PROTOCOL, OwnerAnswer, OwnerBinding, OwnerCommand, OwnerView, RUNNER_STATE,
    SOCKET, ready_line,
};
use super::record::{Cursors, Custody, Holder, Intent, Lease, OwnerRecord};
use super::spawn::OwnerPlan;
use super::store::{Applied, OwnerStore};
use crate::error::RunnerError;
use crate::peer::{self, Leader};
use crate::session::{Sessions, now_ms};

/// What the owner process holds while it serves.
#[derive(Debug)]
pub struct OwnerState {
    store: Mutex<OwnerStore>,
    binding: OwnerBinding,
    own: Leader,
    runner: String,
    socket: PathBuf,
    meter: Arc<Meter>,
}

impl OwnerState {
    /// The state of an owner that is the process `own`, serving `binding`
    /// as runner `runner` on `socket`, over its established `store`.
    #[must_use]
    pub fn new(
        store: OwnerStore,
        binding: OwnerBinding,
        own: Leader,
        runner: String,
        socket: PathBuf,
    ) -> Self {
        Self {
            store: Mutex::new(store),
            binding,
            own,
            runner,
            socket,
            meter: Arc::new(Meter::default()),
        }
    }

    /// The owner's binding.
    #[must_use]
    pub fn binding(&self) -> &OwnerBinding {
        &self.binding
    }

    /// The owner's meter.
    #[must_use]
    pub fn meter(&self) -> &Arc<Meter> {
        &self.meter
    }

    /// The owner's view of itself, from its record.
    ///
    /// # Errors
    ///
    /// `seat_owner_unknown` when the record is gone.
    pub fn view(&self) -> Result<OwnerView, RunnerError> {
        let store = self.store()?;
        let record = store.owner(&self.binding.session).ok_or_else(|| {
            RunnerError::refused(
                "seat_owner_unknown",
                format!("session {} has no owner record", self.binding.session),
            )
        })?;
        Ok(OwnerView {
            protocol: OWNER_PROTOCOL,
            binding: OwnerBinding {
                generation: record.lease.generation,
                ..self.binding.clone()
            },
            owner: self.own.clone(),
            runner: self.runner.clone(),
            socket: self.socket.clone(),
            harness: record.harness.clone(),
            lease: record.lease.clone(),
            custody: record.custody.clone(),
            cursors: record.cursors,
        })
    }

    fn store(&self) -> Result<std::sync::MutexGuard<'_, OwnerStore>, RunnerError> {
        self.store.lock().map_err(|_poisoned| {
            RunnerError::refused(
                "seat_owner_store_unavailable",
                "the owner store's lock is poisoned",
            )
        })
    }

    /// Records one intent, counting it.
    ///
    /// # Errors
    ///
    /// The store's refusals.
    pub fn record(&self, id: &str, intent: &Intent) -> Result<Applied, RunnerError> {
        let mut store = self.store()?;
        let before = store.counts();
        let applied = store.record(id, intent)?;
        let after = store.counts();
        self.meter.call();
        if after.journal_appends > before.journal_appends {
            self.meter.appended();
        }
        self.meter
            .copied(after.bytes_copied.saturating_sub(before.bytes_copied));
        Ok(applied)
    }
}

/// What `lys runner seat-owner` is given.
#[derive(Debug, Clone)]
pub struct OwnerServe {
    /// The owner's directory, holding its plan.
    pub dir: PathBuf,
    /// The server's public key.
    pub server_key: [u8; 32],
    /// Scrollback for the one session.
    pub scrollback: usize,
}

/// Establishes the owner record in `dir` for `plan` as the process `own`,
/// or finds it already established by this same process; a record held by
/// another process is refused.
///
/// # Errors
///
/// `seat_owner_held` when another process holds the lease; the store's
/// refusals.
pub fn establish(
    store: &mut OwnerStore,
    plan: &OwnerPlan,
    own: &Leader,
) -> Result<(), RunnerError> {
    if let Some(record) = store.owner(&plan.binding.session) {
        let holder = match &record.lease.holder {
            Holder::Owner { start } | Holder::Successor { start } => start,
        };
        if holder == own {
            return Ok(());
        }
        return Err(RunnerError::refused(
            "seat_owner_held",
            format!(
                "session {} is held by pid {} at generation {}",
                plan.binding.session, holder.pid, record.lease.generation
            ),
        ));
    }
    let record = OwnerRecord {
        seat: plan.binding.seat.clone(),
        session: plan.binding.session.clone(),
        conversation: plan.binding.conversation.clone(),
        harness: None,
        lease: Lease {
            generation: plan.binding.generation,
            holder: Holder::Owner { start: own.clone() },
            taken_at: now_ms(),
            build: super::protocol::BUILD.to_owned(),
        },
        custody: Custody::Owned,
        cursors: Cursors::default(),
        credential_references: plan.credential_references.clone(),
        established_at: now_ms(),
    };
    store.record(
        &plan.establish,
        &Intent::Establish {
            record: Box::new(record),
        },
    )?;
    Ok(())
}

/// The owner's first act: a session and process group of its own, so no
/// client's exit, hangup or group signal reaches it or its harness
/// (AGENTS-004 R1). Every owner, the installed one and the test fixture
/// alike, takes this step before it reads its plan.
///
/// # Errors
///
/// `seat_owner_start_failed` when the kernel refuses the new session, as it
/// does for a process that already leads a group.
pub fn detach() -> Result<(), RunnerError> {
    rustix::process::setsid().map_err(|error| {
        RunnerError::refused("seat_owner_start_failed", format!("setsid: {error}"))
    })?;
    Ok(())
}

/// Serves as the owner in `serve.dir` until stopped: establishes the lease,
/// opens a runner of its own, starts the one managed session, records its
/// harness, then listens and writes the ready line on standard output.
///
/// # Errors
///
/// The plan's, store's, runner's and launch's refusals; the harness is not
/// started before the lease is recorded, and the ready line is not written
/// before the harness is recorded.
pub fn serve(serve: &OwnerServe) -> Result<(), RunnerError> {
    detach()?;
    let plan = OwnerPlan::read(&serve.dir)?;
    let pid = std::process::id();
    let own = Leader {
        pid,
        start: peer::start_identity(pid)?,
    };
    let mut store = OwnerStore::open(&serve.dir)?;
    establish(&mut store, &plan, &own)?;
    let socket = serve.dir.join(SOCKET);
    let runner = crate::Runner::open(&crate::Options {
        socket: socket.clone(),
        state: serve.dir.join(RUNNER_STATE),
        server_key: serve.server_key,
        scrollback: serve.scrollback,
    })?;
    let sessions = Arc::clone(runner.sessions());
    let runner_id = sessions.runner().to_owned();
    let state = Arc::new(OwnerState::new(
        store,
        plan.binding.clone(),
        own.clone(),
        runner_id.clone(),
        socket.clone(),
    ));
    sessions.adopt_owner(Arc::clone(&state))?;
    sessions.start_managed(plan.managed.clone(), None, plan.responsible.clone())?;
    let harness = sessions.leader_of(&plan.binding.session)?;
    state.record(
        &plan.harness,
        &Intent::Harness {
            session: plan.binding.session.clone(),
            start: harness,
        },
    )?;
    let ready = ready_line(&runner_id, &socket, &own, super::protocol::BUILD);
    runner.serve_until_stopped(move || {
        let mut out = std::io::stdout();
        if let Err(error) = writeln!(out, "{ready}").and_then(|()| out.flush()) {
            crate::error::said(&format!("seat_owner_ready_unwritten: {error}"));
        }
    })
}

/// Answers one owner command from the peer process `peer` (pid, uid), as
/// the kernel reported it on the socket.
///
/// # Errors
///
/// `seat_owner_client_unproved` for a peer the kernel does not confirm as
/// the process it claims; `seat_owner_binding_mismatch` for another seat;
/// `seat_owner_generation_stale` for another generation; the store's
/// refusals for a custody or lease intent it cannot apply.
pub fn answer(
    state: &OwnerState,
    sessions: &Sessions,
    command: OwnerCommand,
    peer: Option<(u32, u32)>,
) -> Result<OwnerAnswer, RunnerError> {
    command.validate()?;
    let session = state.binding.session.clone();
    match command {
        OwnerCommand::Hello {
            binding,
            client,
            client_start,
        } => {
            prove_client(client, &client_start, peer)?;
            if !binding.same_seat(&state.binding) {
                return Err(RunnerError::refused(
                    "seat_owner_binding_mismatch",
                    format!(
                        "this owner serves seat {} session {}; the client named seat {} session {}",
                        state.binding.seat, state.binding.session, binding.seat, binding.session
                    ),
                ));
            }
            let view = state.view()?;
            if binding.generation != view.lease.generation {
                return Err(RunnerError::refused(
                    "seat_owner_generation_stale",
                    format!(
                        "the client named generation {} and the lease is at {}",
                        binding.generation, view.lease.generation
                    ),
                ));
            }
            state.meter.call();
            Ok(OwnerAnswer::Bound { view })
        }
        OwnerCommand::Status => {
            state.meter.call();
            Ok(OwnerAnswer::Status {
                view: state.view()?,
            })
        }
        OwnerCommand::Cursors { intent, cursors } => {
            state.record(&intent, &Intent::Cursors { session, cursors })?;
            Ok(OwnerAnswer::Cursors { cursors })
        }
        OwnerCommand::Stop { intent, by, reason } => {
            state.record(
                &intent,
                &Intent::Custody {
                    session: session.clone(),
                    custody: Custody::Stopping {
                        intent: intent.clone(),
                        since: now_ms(),
                    },
                },
            )?;
            let mut exit = sessions.stop_owned(&session)?;
            exit.stopped = Some(crate::protocol::Stopped {
                by,
                reason,
                at: now_ms(),
            });
            state.record(
                &derived_intent(&intent, "exited"),
                &Intent::Custody {
                    session,
                    custody: Custody::Exited {
                        intent: intent.clone(),
                        exit,
                    },
                },
            )?;
            Ok(OwnerAnswer::Stopping { intent })
        }
        OwnerCommand::Prepare { intent, successor } => {
            state.record(
                &intent,
                &Intent::Custody {
                    session,
                    custody: Custody::HandingOver {
                        intent: intent.clone(),
                        successor,
                        since: now_ms(),
                    },
                },
            )?;
            let generation = state.view()?.lease.generation.saturating_add(1);
            Ok(OwnerAnswer::Prepared { intent, generation })
        }
        OwnerCommand::Transfer { intent, generation } => {
            let view = state.view()?;
            let successor = match &view.custody {
                Custody::HandingOver {
                    intent: prepared,
                    successor,
                    ..
                } if prepared == &intent => successor.clone(),
                Custody::HandingOver { .. } => {
                    return Err(RunnerError::refused(
                        "seat_owner_intent_invalid",
                        "the transfer names another handover than the one prepared",
                    ));
                }
                _ => {
                    return Err(RunnerError::refused(
                        "seat_owner_stop_fenced",
                        "no handover is prepared; the lease does not move",
                    ));
                }
            };
            state.record(
                &derived_intent(&intent, "transfer"),
                &Intent::Lease {
                    session,
                    generation,
                    holder: Holder::Successor { start: successor },
                    taken_at: now_ms(),
                    build: super::protocol::BUILD.to_owned(),
                },
            )?;
            Ok(OwnerAnswer::Transferred { intent, generation })
        }
        OwnerCommand::Release { intent } => {
            let view = state.view()?;
            match &view.custody {
                Custody::HandingOver {
                    intent: prepared, ..
                } if prepared == &intent => {}
                _ => {
                    return Err(RunnerError::refused(
                        "seat_owner_intent_invalid",
                        "no handover under that intent is prepared",
                    ));
                }
            }
            state.record(
                &derived_intent(&intent, "release"),
                &Intent::Custody {
                    session,
                    custody: Custody::Owned,
                },
            )?;
            Ok(OwnerAnswer::Released { intent })
        }
    }
}

/// A second intent id derived from a handover's, stable for the same
/// handover and step, so a lost reply is resolved by the same id.
#[must_use]
pub fn derived_intent(intent: &str, step: &str) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(format!("{intent}:{step}").as_bytes());
    crate::protocol::hex(&digest[..16])
}

fn prove_client(
    client: ClientKind,
    claimed: &Leader,
    peer: Option<(u32, u32)>,
) -> Result<(), RunnerError> {
    let (pid, uid) = peer.ok_or_else(|| {
        RunnerError::refused(
            "seat_owner_client_unproved",
            "the socket reported no peer process",
        )
    })?;
    if uid != peer::own_user() {
        return Err(RunnerError::refused(
            "seat_owner_client_unproved",
            format!(
                "the peer runs as user {uid}, and the owner as user {}",
                peer::own_user()
            ),
        ));
    }
    if pid != claimed.pid {
        return Err(RunnerError::refused(
            "seat_owner_client_unproved",
            format!(
                "the {} claims pid {} and the kernel reports pid {pid}",
                client_name(client),
                claimed.pid
            ),
        ));
    }
    let proved = peer::start_identity(pid)?;
    if proved != claimed.start {
        return Err(RunnerError::refused(
            "seat_owner_client_unproved",
            format!(
                "the {} names a start identity that is not pid {pid}'s",
                client_name(client)
            ),
        ));
    }
    Ok(())
}

fn client_name(client: ClientKind) -> &'static str {
    match client {
        ClientKind::Runner => "runner",
        ClientKind::IdentityServer => "identity server",
        ClientKind::Successor => "successor",
    }
}
