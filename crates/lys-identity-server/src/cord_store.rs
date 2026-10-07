//! The master off switch as it is kept: a leaf store of its own beside the
//! directory log, one leaf for each record, written before anything it
//! records is done or answered. A leaf is stored whole or not at all.
//!
//! A pull is kept twice: as pulled, before any session is asked to end, so
//! every start is refused from that moment; and finished, with everything it
//! did, once every runner has answered or been named unreached. So a pull
//! sent again under its operation id answers what it did and never pulls
//! twice, and a pull cut off before it finished is finished by sending it
//! again. A release names the pull it lets go, and from then starts go
//! through.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written. What the leaves fold to is sealed in the log's signed
//! snapshot every [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a
//! start reads the snapshot and only the leaves after it.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_cord::CordError;

/// The origin the cord's leaf store is created with.
pub const ORIGIN: &str = "lys/identity/cord";

/// The snapshot domain the cord's folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/cord-state/v1";

const FORMAT: &str = "lys-cord-state/v1";

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// Who stopped everything, when and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CordPull)]
pub struct Pull {
    /// The operation id it was sent under, which names it.
    pub operation: String,
    /// Who pulled the cord.
    pub by: String,
    /// The display name or console claim; null when no name was held.
    pub by_name: Option<String>,
    /// Why, in their words.
    pub reason: String,
    /// Whether anything that did not stop on a hang-up was killed.
    pub kill: bool,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

impl Pull {
    /// Whether `other` is the same pull in the same words, whenever sent.
    fn same_words(&self, other: &Self) -> bool {
        self.operation == other.operation
            && self.by == other.by
            && self.reason == other.reason
            && self.kill == other.kill
    }
}

/// One session the pull names, with its agent's and its computer's names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CordSession)]
pub struct Named {
    /// The session.
    pub session: String,
    /// Its agent; null for a session found running under no agent.
    pub agent: Option<String>,
    /// Its agent's name; null when the directory names none.
    pub agent_name: Option<String>,
    /// The computer it runs on.
    pub machine: String,
    /// The computer's name; null when the machines are not kept here.
    pub machine_name: Option<String>,
}

/// A computer whose runner could not be asked or did not answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CordUnreached)]
pub struct Unreached {
    /// The computer.
    pub machine: String,
    /// Its name; null when the machines are not kept here.
    pub machine_name: Option<String>,
    /// The refusal, by name.
    pub refusal: String,
    /// Why, in words.
    pub reason: String,
}

/// An agent whose credential handles the broker did not confirm ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CordHandlesRefused)]
pub struct HandlesRefused {
    /// The agent.
    pub agent: String,
    /// The broker's refusal, in its words.
    pub refusal: String,
}

/// What one pull did, kept whole once it finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CordPullResult)]
pub struct PullResult {
    /// The operation id the pull was sent under.
    pub operation: String,
    /// Who pulled it, when and why.
    pub pulled: Pull,
    /// The sessions their runners confirmed ended by this pull.
    pub stopped: Vec<Named>,
    /// The sessions not confirmed ended when the pull finished.
    pub still_running: Vec<Named>,
    /// The computers whose runners could not be asked or did not answer.
    pub unreached: Vec<Unreached>,
    /// The credential handles ended at the broker, by id.
    pub handles_ended: Vec<String>,
    /// The agents whose handles the broker did not confirm ended.
    pub handles_refused: Vec<HandlesRefused>,
}

/// A release: who let agents start again, and which pull it let go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CordRelease)]
pub struct Release {
    /// The operation id it was sent under, which names it.
    pub operation: String,
    /// The operation id of the pull it let go.
    pub pull: String,
    /// The administrator who released it.
    pub by: String,
    /// Their name when it was released; null when the directory names none.
    pub by_name: Option<String>,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// One leaf.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case")]
enum Record {
    Pulled(Pull),
    Finished(PullResult),
    Released(Release),
}

/// A pull and, once it finished, what it did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// The pull.
    pub pull: Pull,
    /// What it did; null while it has not finished.
    pub result: Option<PullResult>,
}

/// The cord as its log folds it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
struct Held {
    pulls: Vec<Kept>,
    releases: Vec<Release>,
    /// The pull in force, by its place in `pulls`; null once released.
    standing: Option<usize>,
    #[serde(skip)]
    operations: HashMap<String, Spent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spent {
    Pull(usize, bool),
    Release(usize),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    pulls: Vec<Kept>,
    releases: Vec<Release>,
    standing: Option<usize>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let cleared_operations: HashSet<&str> = records
            .releases
            .iter()
            .map(|release| release.pull.as_str())
            .collect();
        let pulls = records.pulls.iter().enumerate();
        let releases = records.releases.iter().enumerate();
        let operations = pulls
            .map(|(at, kept)| {
                (
                    kept.pull.operation.clone(),
                    Spent::Pull(
                        at,
                        cleared_operations.contains(kept.pull.operation.as_str()),
                    ),
                )
            })
            .chain(releases.map(|(at, release)| (release.operation.clone(), Spent::Release(at))))
            .collect();
        Self {
            pulls: records.pulls,
            releases: records.releases,
            standing: records.standing,
            operations,
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    fn hold(&mut self, record: Record) -> Result<(), String> {
        match record {
            Record::Pulled(pull) => {
                if self.operations.contains_key(&pull.operation) {
                    return Err(format!("operation `{}` is already spent", pull.operation));
                }
                let at = self.pulls.len();
                self.operations
                    .insert(pull.operation.clone(), Spent::Pull(at, false));
                self.pulls.push(Kept { pull, result: None });
                self.standing = Some(at);
            }
            Record::Finished(result) => {
                let Some(Spent::Pull(at, _)) = self.operations.get(&result.operation).copied()
                else {
                    return Err(format!("no pull `{}` to finish", result.operation));
                };
                let kept = &mut self.pulls[at];
                if kept.result.is_some() {
                    return Err(format!("pull `{}` is already finished", result.operation));
                }
                kept.result = Some(result);
            }
            Record::Released(release) => {
                let standing = self.standing.map(|at| &self.pulls[at].pull.operation);
                if standing != Some(&release.pull)
                    || self.operations.contains_key(&release.operation)
                {
                    return Err(format!(
                        "release `{}` lets go no pull in force",
                        release.operation
                    ));
                }
                let at = self.releases.len();
                let Some(Spent::Pull(_, released)) = self.operations.get_mut(&release.pull) else {
                    return Err(format!(
                        "release `{}` names no kept pull",
                        release.operation
                    ));
                };
                *released = true;
                self.operations
                    .insert(release.operation.clone(), Spent::Release(at));
                self.releases.push(release);
                self.standing = None;
            }
        }
        Ok(())
    }

    fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let record = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a cord record: {error}"))?;
            self.hold(record)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("cord state: {error}"))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("cord state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "cord state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}

/// What a pull sent finds: its result when it finished before, or the pull
/// to carry out, newly kept or kept before and cut off.
pub enum Pulling {
    /// The pull finished before; this is what it did.
    Answered(Box<PullResult>),
    /// The pull to carry out.
    Go(Pull),
}

/// The cord, read from its leaf store and appended to it.
pub struct CordStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    CordError::Unavailable {
        reason: what.to_string(),
    }
    .into()
}

impl CordStore<FileLeafStore> {
    /// The cord kept in the directory `dir`, created when it does not exist,
    /// its snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        let reopen: Reopen<FileLeafStore> = Box::new(move || FileLeafStore::open(&dir));
        let (log, held, start) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
            start: start.clone(),
            since_snapshot: 0,
            snapshot_failure: None,
            uncertain: false,
        };
        store.after_start(&start);
        Ok(store)
    }
}

impl<S: LeafStore> CordStore<S> {
    /// How the log was started: from its snapshot, or from every leaf.
    pub fn start(&self) -> &Start {
        &self.start
    }

    fn after_start(&mut self, start: &Start) {
        match start {
            Start::Resumed { replayed, .. } => self.since_snapshot = *replayed,
            Start::Rebuilt { .. } => self.write_snapshot(),
        }
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot();
        }
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
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

    fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.held = held;
            self.start = start.clone();
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    fn append(&mut self, record: Record) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&record).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(record) {
                self.uncertain = true;
                return Err(unavailable(reason));
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

    /// The pull in force; none when nothing is stopped.
    pub fn standing(&mut self) -> Result<Option<Pull>, ServerError> {
        self.settle()?;
        Ok(self
            .held
            .standing
            .map(|at| self.held.pulls[at].pull.clone()))
    }

    /// The latest pull and what it did, and the latest release.
    pub fn latest(&mut self) -> Result<(Option<Kept>, Option<Release>), ServerError> {
        self.settle()?;
        Ok((
            self.held.pulls.last().cloned(),
            self.held.releases.last().cloned(),
        ))
    }

    /// Keep `pull` as pulled, answering what it did when it finished before,
    /// and the pull to carry out otherwise. The same operation in other
    /// words, or naming a release, is refused `cord_reused`.
    pub fn pull(&mut self, pull: Pull) -> Result<Pulling, ServerError> {
        self.pulling(pull, false)
    }

    /// A console replay binds the recorded claim and cannot restore a released pull.
    pub(crate) fn console_pull(&mut self, pull: Pull) -> Result<Pulling, ServerError> {
        self.pulling(pull, true)
    }

    fn pulling(&mut self, pull: Pull, console: bool) -> Result<Pulling, ServerError> {
        self.settle()?;
        match self.held.operations.get(&pull.operation).copied() {
            Some(Spent::Pull(at, released)) => {
                let kept = &self.held.pulls[at];
                if !kept.pull.same_words(&pull)
                    || (console && (released || kept.pull.by_name != pull.by_name))
                {
                    return Err(reused(pull.operation));
                }
                Ok(match &kept.result {
                    Some(result) => Pulling::Answered(Box::new(result.clone())),
                    None => Pulling::Go(kept.pull.clone()),
                })
            }
            Some(Spent::Release(_)) => Err(reused(pull.operation)),
            None => {
                self.append(Record::Pulled(pull.clone()))?;
                Ok(Pulling::Go(pull))
            }
        }
    }

    /// Keep what the pull did, once: a pull already finished answers what it
    /// kept then.
    pub fn finish(&mut self, result: PullResult) -> Result<PullResult, ServerError> {
        self.settle()?;
        let Some(Spent::Pull(at, _)) = self.held.operations.get(&result.operation).copied() else {
            return Err(unavailable(format!(
                "no pull `{}` is kept to finish",
                result.operation
            )));
        };
        if let Some(kept) = &self.held.pulls[at].result {
            return Ok(kept.clone());
        }
        self.append(Record::Finished(result.clone()))?;
        Ok(result)
    }

    /// Let the pull in force go. The same release sent again answers what
    /// was kept; a release while nothing is stopped is refused
    /// `cord_not_pulled`; the operation of a pull, or of a release in other
    /// words, is refused `cord_reused`.
    pub fn release(
        &mut self,
        (operation, by, by_name, at): (String, String, Option<String>, u64),
    ) -> Result<Release, ServerError> {
        self.settle()?;
        match self.held.operations.get(&operation).copied() {
            Some(Spent::Release(kept)) if self.held.releases[kept].by == by => {
                return Ok(self.held.releases[kept].clone());
            }
            Some(_) => return Err(reused(operation)),
            None => {}
        }
        let standing = self.held.standing.ok_or(CordError::NotPulled)?;
        let release = Release {
            operation,
            pull: self.held.pulls[standing].pull.operation.clone(),
            by,
            by_name,
            at,
        };
        self.append(Record::Released(release.clone()))?;
        Ok(release)
    }
}

fn reused(operation: String) -> ServerError {
    CordError::Reused { operation }.into()
}

/// Open the log from its snapshot, or from every leaf when the snapshot's
/// state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<(FrontierLog<S>, Held, Start), ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => {
            let (log, tail) =
                FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
            let mut held = Held::default();
            held.fold(&tail).map_err(unavailable)?;
            let replayed = log.len();
            let refusal = SnapshotRefusal::StateUnreadable { reason };
            return Ok((log, held, Start::Rebuilt { refusal, replayed }));
        }
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}
