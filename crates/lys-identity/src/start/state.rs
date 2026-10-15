//! The launch-record log's folded state, and each launch record's state
//! derived from the records it reads.
//!
//! # The log
//!
//! [`LaunchRecords`] keeps launch records and withdrawals as signed events
//! on their own log. A start never replays the whole history: the log opens
//! from its signed snapshot, whose state is every leaf up to it and is
//! checked to fold to the log's own tree, and reads only the leaves after
//! it. A snapshot is written by entry count, and at once after a rebuild. A
//! snapshot refused, or a state that does not read back, is named in
//! [`LaunchRecords::start`], logged, and rebuilt from the whole log; never a
//! silent fallback. An append that fails leaves its outcome unknown, and is
//! settled by opening the store again before anything else is read or
//! written.
//!
//! # A launch record's state
//!
//! Each state is derived from the records it reads, and no copy is kept:
//! running when the record the sessions brief d5055cc1 keeps holds a
//! verified `lys/session-start/v1` report, signed with the agent's own key,
//! whose agent id and launch record id equal the launch record's;
//! unconfirmed while no such report exists and no withdrawal names the
//! record; withdrawn when a withdrawal names it and no such report exists.
//! A report arriving after a withdrawal reads running with the withdrawal
//! beside it. Copying a command changes no state, and nothing here writes a
//! session record or an agent record. A record stands, and a second start
//! for its agent is refused, only while it reads unconfirmed.

use std::num::NonZeroU64;
use std::path::Path;

use ciborium::Value;
use lys_core::Ed25519Identity;
use lys_log_store::{
    FileLeafStore, Frontier, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult,
    open_with_snapshot,
};

use crate::encoding::{MAJOR_ARRAY, bytes, head};
use crate::restart::SNAPSHOT_EVERY;
use crate::start::error::StartError;
use crate::start::launch_record::{
    LaunchEvent, LaunchRecord, as_byte_string, cbor, verify_launch_event,
};
use crate::start::withdrawal::Withdrawal;

/// The origin the launch-record log is created with.
pub const ORIGIN: &str = "lys/identity/launch-records";

/// The snapshot domain the launch-record log's state is sealed under.
pub const DOMAIN: &str = "lys/identity/launch-records-state/v1";

fn unavailable(what: impl std::fmt::Display) -> StartError {
    StartError::Unavailable {
        reason: what.to_string(),
    }
}

/// What the log folds to: every leaf, and the records and withdrawals they name.
#[derive(Debug, Default)]
struct Held {
    leaves: Vec<Vec<u8>>,
    records: Vec<LaunchRecord>,
    withdrawals: Vec<Withdrawal>,
}

impl Held {
    /// Fold one leaf, refusing an event this service did not sign, a second
    /// record under one id, and a withdrawal of no record or of one already
    /// withdrawn.
    fn hold(&mut self, leaf: &[u8], key: &[u8; 32]) -> Result<(), String> {
        match verify_launch_event(leaf, key)? {
            LaunchEvent::Launch(record) => {
                if self.records.iter().any(|held| held.id == record.id) {
                    return Err(format!("launch record {} is kept twice", record.id));
                }
                self.records.push(record);
            }
            LaunchEvent::Withdrawal(withdrawal) => {
                if !self
                    .records
                    .iter()
                    .any(|held| held.id == withdrawal.launch_record)
                {
                    return Err(format!(
                        "a withdrawal names launch record {}, which is not kept",
                        withdrawal.launch_record
                    ));
                }
                if self
                    .withdrawals
                    .iter()
                    .any(|held| held.launch_record == withdrawal.launch_record)
                {
                    return Err(format!(
                        "launch record {} is withdrawn twice",
                        withdrawal.launch_record
                    ));
                }
                self.withdrawals.push(withdrawal);
            }
        }
        self.leaves.push(leaf.to_vec());
        Ok(())
    }

    fn fold(&mut self, from: u64, leaves: &[Vec<u8>], key: &[u8; 32]) -> Result<(), String> {
        for (index, leaf) in (from..).zip(leaves) {
            self.hold(leaf, key)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals: every leaf, as a CBOR array of byte strings.
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        head(&mut out, MAJOR_ARRAY, self.leaves.len() as u64);
        for leaf in &self.leaves {
            bytes(&mut out, leaf);
        }
        out
    }

    /// The state a snapshot sealed, refused by reason unless every leaf in it
    /// verifies, folds, and re-encodes to the very state that was read.
    fn decode(state: &[u8], key: &[u8; 32]) -> Result<Self, String> {
        let Value::Array(items) = cbor(state)? else {
            return Err("the launch-record state is not an array".to_owned());
        };
        let mut held = Self::default();
        for (index, item) in (0_u64..).zip(items) {
            let leaf = as_byte_string(item)?;
            held.hold(&leaf, key)
                .map_err(|reason| format!("state leaf {index}: {reason}"))?;
        }
        if held.encode() != state {
            return Err("the launch-record state is not canonical".to_owned());
        }
        Ok(held)
    }
}

/// How the launch-record store is opened again after an append whose
/// outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The launch records and withdrawals, read from their log and appended to it.
pub struct LaunchRecords<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Ed25519Identity,
    every: NonZeroU64,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

impl LaunchRecords<FileLeafStore> {
    /// The launch records kept in `dir`, created when it does not exist, their
    /// events and snapshots signed by `key`.
    ///
    /// # Errors
    /// - `StartError::Unavailable` if opening, reading or folding the log fails, a signed leaf is invalid, or records and withdrawals conflict.
    pub fn open(dir: &Path, key: Ed25519Identity) -> Result<Self, StartError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(
            Box::new(move || FileLeafStore::open(&dir)),
            key,
            SNAPSHOT_EVERY,
        )
    }
}

type Opened<S> = (FrontierLog<S>, Held, Start);

impl<S: LeafStore> LaunchRecords<S> {
    /// The launch records kept in the store `reopen` opens, their events and
    /// snapshots signed by `key`, with a snapshot owed every `every` entries.
    ///
    /// # Errors
    /// - `StartError::Unavailable` if opening, reading or folding the log fails, a signed leaf is invalid, or records and withdrawals conflict.
    pub fn over(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<Self, StartError> {
        let (log, held, start) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            every,
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

    /// How the log was started: from its snapshot, or from every leaf and the
    /// refusal that sent it there.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// The launch record with `id`.
    pub fn record(&self, id: &str) -> Option<&LaunchRecord> {
        self.held.records.iter().find(|record| record.id == id)
    }

    /// Every launch record, in the order kept.
    pub fn records(&self) -> &[LaunchRecord] {
        &self.held.records
    }

    /// The number of launch records kept.
    pub fn record_count(&self) -> usize {
        self.held.records.len()
    }

    /// The withdrawal of the launch record `id`, when it was withdrawn.
    pub fn withdrawal(&self, id: &str) -> Option<&Withdrawal> {
        self.held
            .withdrawals
            .iter()
            .find(|withdrawal| withdrawal.launch_record == id)
    }

    /// The number of withdrawals kept.
    pub fn withdrawal_count(&self) -> usize {
        self.held.withdrawals.len()
    }

    /// Every withdrawal, borrowed in the order kept.
    pub fn withdrawals(&self) -> &[Withdrawal] {
        &self.held.withdrawals
    }

    /// Every signed event, exactly the bytes appended, in log order.
    pub fn signed_events(&self) -> &[Vec<u8>] {
        &self.held.leaves
    }

    fn after_start(&mut self, start: &Start) {
        match start {
            Start::Resumed { replayed, .. } => self.since_snapshot = *replayed,
            Start::Rebuilt { .. } => self.write_snapshot(),
        }
        if self.since_snapshot >= self.every.get() {
            self.write_snapshot();
        }
    }

    fn write_snapshot(&mut self) {
        let state = self.held.encode();
        match self.log.write_snapshot(DOMAIN, &state, &self.key) {
            Ok(_) => {
                self.since_snapshot = 0;
                self.snapshot_failure = None;
            }
            Err(error) => {
                tracing::error!(domain = DOMAIN, "SnapshotNotWritten: {error}");
                self.snapshot_failure = Some(error.to_string());
            }
        }
    }

    /// Resolve an append whose outcome is not known, by opening the store
    /// again and reading what it holds. Until that succeeds nothing is
    /// answered from memory and nothing is appended.
    ///
    /// # Errors
    /// - `StartError::Unavailable` if opening, reading or folding the log fails, a signed leaf is invalid, or records and withdrawals conflict.
    pub fn settle(&mut self) -> Result<(), StartError> {
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

    /// Append one signed event, holding it once the leaf is durable. A failed
    /// append is settled by reading back: the event is kept only when the
    /// store holds exactly it.
    fn append(&mut self, message: &[u8]) -> Result<(), StartError> {
        self.settle()?;
        let public = self.key.public_key_bytes();
        let index = self.log.len();
        let Err(failure) = self.log.append(message) else {
            if let Err(reason) = self.held.hold(message, &public) {
                self.uncertain = true;
                return Err(unavailable(reason));
            }
            self.since_snapshot += 1;
            if self.since_snapshot >= self.every.get() {
                self.write_snapshot();
            }
            return Ok(());
        };
        self.uncertain = true;
        self.settle()?;
        match self.log.leaf_bytes(index).map_err(unavailable)? {
            Some(held) if held.as_slice() == message => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure)),
        }
    }

    /// Sign and keep `record`, refusing an id already kept.
    pub(crate) fn keep(&mut self, record: &LaunchRecord) -> Result<(), StartError> {
        self.settle()?;
        if self.record(&record.id).is_some() {
            return Err(unavailable(format!(
                "launch record {} is already kept",
                record.id
            )));
        }
        let message = LaunchEvent::Launch(record.clone()).seal(&self.key);
        self.append(&message)
    }

    /// Sign and keep `withdrawal` of a kept record not yet withdrawn.
    pub(crate) fn keep_withdrawal(&mut self, withdrawal: &Withdrawal) -> Result<(), StartError> {
        self.settle()?;
        if self.record(&withdrawal.launch_record).is_none() {
            return Err(StartError::LaunchRecordUnknown {
                launch_record: withdrawal.launch_record.clone(),
            });
        }
        let message = LaunchEvent::Withdrawal(withdrawal.clone()).seal(&self.key);
        self.append(&message)
    }
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, StartError> {
    let public = key.public_key_bytes();
    let store = reopen().map_err(unavailable)?;
    let started = open_with_snapshot(store, DOMAIN, &public).map_err(unavailable)?;
    tracing::info!(domain = DOMAIN, "launch-record log {}", started.start);
    let mut held = match started.state.as_deref() {
        None => Held::default(),
        Some(state) => match Held::decode(state, &public) {
            Ok(held) => held,
            Err(reason) => return rebuilt(reopen, &public, reason),
        },
    };
    held.fold(started.tail.from, &started.tail.leaves, &public)
        .map_err(unavailable)?;
    let folded = Frontier::from_leaves(&held.leaves);
    if folded.size() != started.log.len() || folded.root() != started.log.frontier().root() {
        return rebuilt(
            reopen,
            &public,
            "the snapshot's leaves are not the log's leaves".to_owned(),
        );
    }
    Ok((started.log, held, started.start))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`, and say so.
fn rebuilt<S: LeafStore>(
    reopen: &Reopen<S>,
    public: &[u8; 32],
    reason: String,
) -> Result<Opened<S>, StartError> {
    tracing::warn!(domain = DOMAIN, "launch-record state refused: {reason}");
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(tail.from, &tail.leaves, public)
        .map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}

#[path = "reading.rs"]
mod reading;

pub use reading::{
    LaunchState, Reading, SessionReport, SessionReports, derive, standing, state_of,
};
