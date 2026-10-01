//! The runtime reports as they are kept: a leaf store of their own, one leaf
//! for each report, written before the report is answered. A leaf is stored
//! whole or not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A report is named by the operation id it was sent with, so sending it
//! again in the same words answers the session as it stands, and the same
//! operation in other words is refused. An agent's session begins with
//! `starting`; a found session begins with `running`, since its runtime first
//! sees it already running. A stopped session takes no further report.
//!
//! What the leaves fold to is sealed in the log's signed snapshot every
//! [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a start reads the
//! snapshot and only the leaves after it. A snapshot refused, or a state that
//! does not read back, sends the start to every leaf, by name, never silently.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};

use lys_identity::signer::load_service_key;

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;
use crate::runtime_state::{DOMAIN, Held, Report, Reported, Tracked};

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the reports' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/runtime-reports";

/// The runtime reports, read from their leaf store and appended to it.
pub struct RuntimeStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    agents_with_sessions: Arc<BTreeMap<String, SessionActivity>>,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

/// A log opened and folded: the log, what it folds to, and how it started.
type Opened<S> = (FrontierLog<S>, Held, Start);

/// Derived session activity, bounded to one entry per agent.
#[derive(Clone, Default)]
pub struct SessionActivity {
    live: usize,
    sessions: BTreeSet<String>,
    stopped_at: Option<u64>,
}

impl SessionActivity {
    /// The live session ids, without their report history.
    pub fn live_sessions(&self) -> &BTreeSet<String> {
        &self.sessions
    }

    pub(crate) fn unreported_session(&self, reported: impl Fn(&str) -> bool) -> Option<&str> {
        self.sessions
            .iter()
            .find(|session| !reported(session))
            .map(String::as_str)
    }

    pub(crate) fn active_since(&self, since_ms: Option<i64>) -> bool {
        self.live > 0
            || since_ms.is_none_or(|since| {
                self.stopped_at
                    .is_some_and(|at| i128::from(at) * 1_000 >= i128::from(since))
            })
    }
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::RuntimeUnavailable {
        reason: what.to_string(),
    }
}

impl RuntimeStore<FileLeafStore> {
    /// The reports in the directory `config` names, their snapshots signed
    /// by the service's event key, saying through `say` how the log started;
    /// none when it names no directory.
    pub fn configured(config: &Config, say: &Say) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.runtime_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, Arc::new(load_service_key(&config.event_key_file)?))?;
        say(&format!(
            "runtime reports log {}, holding {} sessions",
            store.start(),
            store.sessions().len()
        ));
        Ok(Some(store))
    }

    /// The reports kept in the directory `dir`, which is created when it
    /// does not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> RuntimeStore<S> {
    /// The reports kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, held, start) = opened(&reopen, &key)?;
        let agents_with_sessions = session_agents(&held)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
            agents_with_sessions,
            start: start.clone(),
            since_snapshot: 0,
            snapshot_failure: None,
            uncertain: false,
        };
        store.after_start(&start);
        Ok(store)
    }

    /// How the log was started: from its snapshot, or from every leaf and
    /// the refusal that sent it there.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// Owe a snapshot for the leaves the start read, and write one at once
    /// when the start rebuilt.
    fn after_start(&mut self, start: &Start) {
        match start {
            Start::Resumed { replayed, .. } => self.since_snapshot = *replayed,
            Start::Rebuilt { .. } => self.write_snapshot(),
        }
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot();
        }
    }

    fn write_snapshot(&mut self) {
        let written = self.held.encode().and_then(|state| {
            self.log
                .write_snapshot(DOMAIN, &state, &self.key)
                .map_err(|error| error.to_string())
        });
        match written {
            Ok(_) => {
                self.since_snapshot = 0;
                self.snapshot_failure = None;
            }
            Err(reason) => self.snapshot_failure = Some(reason),
        }
    }

    /// Resolve an append whose outcome is not known, by opening the leaf
    /// store again and reading what it holds. Until that succeeds nothing is
    /// answered from memory and nothing is appended.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.agents_with_sessions = session_agents(&held)?;
            self.held = held;
            self.start = start.clone();
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    /// Append one report as one leaf. A failed append is settled by reading
    /// back: the report is kept only if the leaf store holds exactly it.
    fn append(&mut self, report: Report, copies: &mut usize) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&report).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            let transition = report
                .agent
                .as_ref()
                .filter(|_| matches!(report.state, Reported::Starting | Reported::Stopped))
                .map(|agent| {
                    (
                        agent.clone(),
                        report.session.clone(),
                        report.state,
                        report.at,
                    )
                });
            if let Err(reason) = self.held.hold(report) {
                self.uncertain = true;
                return Err(unavailable(reason));
            }
            if let Some((agent, session, state, at)) = transition
                && let Err(error) = self.session_transition(agent, session, state, at, copies)
            {
                self.uncertain = true;
                return Err(error);
            }
            self.since_snapshot += 1;
            if self.since_snapshot >= SNAPSHOT_EVERY.get() {
                self.write_snapshot();
            }
            return Ok(());
        };
        self.uncertain = true;
        self.settle()?;
        match self.log.leaf_bytes(index).map_err(unavailable)? {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure)),
        }
    }

    /// Every session, in the order first reported.
    pub fn sessions(&self) -> &[Tracked] {
        &self.held.sessions
    }

    /// Agents with a durably recorded session, shared without copying their history.
    pub fn agents_with_sessions(
        &mut self,
    ) -> Result<Arc<BTreeMap<String, SessionActivity>>, ServerError> {
        self.settle()?;
        Ok(Arc::clone(&self.agents_with_sessions))
    }

    fn session_transition(
        &mut self,
        agent: String,
        session: String,
        state: Reported,
        at: u64,
        copies: &mut usize,
    ) -> Result<(), ServerError> {
        if Arc::strong_count(&self.agents_with_sessions) > 1 {
            *copies = copies
                .checked_add(1)
                .ok_or_else(|| unavailable("session index copy count overflows"))?;
        }
        let activity = Arc::make_mut(&mut self.agents_with_sessions)
            .entry(agent)
            .or_default();
        match state {
            Reported::Starting => {
                if !activity.sessions.insert(session) {
                    return Err(unavailable("started session is already in the live index"));
                }
                activity.live = activity
                    .live
                    .checked_add(1)
                    .ok_or_else(|| unavailable("live session count overflows"))?;
            }
            Reported::Stopped => {
                if !activity.sessions.remove(&session) {
                    return Err(unavailable("stopped session is absent from the live index"));
                }
                activity.live = activity
                    .live
                    .checked_sub(1)
                    .ok_or_else(|| unavailable("stopped session has no live session count"))?;
                activity.stopped_at =
                    Some(activity.stopped_at.map_or(at, |earlier| earlier.max(at)));
            }
            Reported::Running | Reported::StopAsked => {
                return Err(unavailable("a non-transition reached the session index"));
            }
        }
        Ok(())
    }

    /// The session named `session`.
    pub fn session(&self, session: &str) -> Option<&Tracked> {
        self.held.session(session)
    }

    /// Keep `report` and answer the session as it then stands. Sent again in
    /// the same words it is kept once; the same operation in other words is
    /// refused, and so is a report the session as it stands does not take.
    pub fn report(&mut self, report: Report) -> Result<Tracked, ServerError> {
        self.report_observing(report, |_| {})
    }

    /// Keep one report and observe the full index copies caused by it.
    pub fn report_observing(
        &mut self,
        report: Report,
        copied: impl FnOnce(usize),
    ) -> Result<Tracked, ServerError> {
        let mut copies = 0;
        let result = self.report_counting(report, &mut copies);
        copied(copies);
        result
    }

    fn report_counting(
        &mut self,
        report: Report,
        copies: &mut usize,
    ) -> Result<Tracked, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.operation(&report.operation) {
            if !same_words(kept, &report) {
                return Err(ServerError::RuntimeReportReused {
                    operation: report.operation,
                });
            }
            return self.standing(&report.session);
        }
        match self.held.session(&report.session) {
            None => {
                let begins = match report.agent {
                    Some(_) => report.state == Reported::Starting,
                    None => report.state == Reported::Running,
                };
                if !begins {
                    return Err(ServerError::RuntimeSessionUnknown);
                }
            }
            Some(tracked) => {
                if tracked.agent != report.agent {
                    return Err(ServerError::RuntimeSessionUnknown);
                }
                if tracked.stopped() {
                    return Err(ServerError::RuntimeSessionStopped {
                        session: report.session,
                    });
                }
                if report.state == Reported::Starting {
                    return Err(ServerError::RuntimeSessionStarted {
                        session: report.session,
                    });
                }
                if tracked.machine != report.machine {
                    return Err(ServerError::RequestMalformed {
                        reason: format!(
                            "session `{}` runs on machine `{}`, not `{}`",
                            report.session, tracked.machine, report.machine
                        ),
                    });
                }
            }
        }
        let session = report.session.clone();
        self.append(report, copies)?;
        self.standing(&session)
    }

    fn standing(&self, session: &str) -> Result<Tracked, ServerError> {
        self.held
            .session(session)
            .cloned()
            .ok_or(ServerError::RuntimeSessionUnknown)
    }
}

fn session_agents(held: &Held) -> Result<Arc<BTreeMap<String, SessionActivity>>, ServerError> {
    let mut agents: BTreeMap<String, SessionActivity> = BTreeMap::new();
    for tracked in &held.sessions {
        let Some(agent) = &tracked.agent else {
            continue;
        };
        let latest = tracked
            .latest()
            .ok_or_else(|| unavailable("a runtime session has no reports"))?;
        let activity = agents.entry(agent.clone()).or_default();
        if latest.state == Reported::Stopped {
            activity.stopped_at = Some(
                activity
                    .stopped_at
                    .map_or(latest.at, |at| at.max(latest.at)),
            );
        } else {
            if !activity.sessions.insert(tracked.session.clone()) {
                return Err(unavailable(
                    "a live session is repeated in the runtime state",
                ));
            }
            activity.live = activity
                .live
                .checked_add(1)
                .ok_or_else(|| unavailable("live session count overflows"))?;
        }
    }
    Ok(Arc::new(agents))
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`.
fn rebuilt<S: LeafStore>(reopen: &Reopen<S>, reason: String) -> Result<Opened<S>, ServerError> {
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(&tail).map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}

/// Whether two reports are the same report, whenever each was received.
fn same_words(kept: &Report, report: &Report) -> bool {
    let timeless = Report {
        at: kept.at,
        ..report.clone()
    };
    *kept == timeless
}
