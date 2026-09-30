//! The certificate log: every capability certificate issued, and every
//! withdrawal, appended as a leaf of a transparency log and never edited.
//!
//! A certificate is entered whole (its DER bytes) with the claims that held
//! when it was issued. A withdrawal is a later leaf naming the serial; the
//! certificate's own leaf is never changed. Anyone holding a certificate, the
//! log's checkpoint and an inclusion proof verifies its entry offline.
//!
//! The leaves are pinned, and what they fold to is sealed in the log's signed
//! snapshot every [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a
//! start reads the snapshot and only the leaves after it. A snapshot refused,
//! or a state that does not read back, sends the start to every leaf by name.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_core::merkle::{InclusionProof, RootHash};
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, Tail,
    open_with_snapshot,
};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// The origin the certificate log is created with.
pub const ORIGIN: &str = "lys/identity/certificates";

/// The snapshot domain the certificate log's folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/certificates-state/v1";

const FORMAT: &str = "lys-certificates-state/v1";

/// A certificate as it is entered in the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issued {
    /// The certificate's serial, which names it.
    pub serial: String,
    /// The agent it was issued to.
    pub agent: String,
    /// The person it was issued for.
    pub person: String,
    /// The claims that held at issuance, as issued; never the live answer.
    pub claims: serde_json::Value,
    /// The certificate, DER, standard base64.
    pub der: String,
    /// When it was issued, in seconds since the Unix epoch.
    pub issued_at: u64,
}

/// A certificate's withdrawal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Withdrawn {
    /// The certificate withdrawn.
    pub serial: String,
    /// The person who withdrew it.
    pub by: String,
    /// Why, in their words.
    pub reason: String,
    /// When, in seconds since the Unix epoch.
    pub withdrawn_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
enum Line {
    Issued(Issued),
    Withdrawn(Withdrawn),
}

/// A certificate as it stands: where it was entered, and its withdrawal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entered {
    /// The certificate as issued.
    pub issued: Issued,
    /// The index of its leaf in the log.
    pub leaf: u64,
    /// Its withdrawal, null while it stands.
    pub withdrawn: Option<Withdrawn>,
}

/// A certificate's entry with what verifies it offline: its leaf bytes, the
/// log's size and root, and the inclusion proof from one to the other.
#[derive(Debug, Clone)]
pub struct Proven {
    /// The certificate as it stands.
    pub entered: Entered,
    /// The leaf's bytes, exactly as entered.
    pub leaf_bytes: Vec<u8>,
    /// The log's size the proof is against.
    pub tree_size: u64,
    /// The log's root at that size.
    pub root: RootHash,
    /// The inclusion proof.
    pub proof: InclusionProof,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Held {
    certificates: BTreeMap<String, Entered>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

type Opened<S> = (FrontierLog<S>, Held, Start);

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::CertificatesUnavailable {
        reason: what.to_string(),
    }
}

impl Held {
    fn hold(&mut self, index: u64, line: Line) {
        match line {
            Line::Issued(issued) => {
                self.certificates.insert(
                    issued.serial.clone(),
                    Entered {
                        issued,
                        leaf: index,
                        withdrawn: None,
                    },
                );
            }
            Line::Withdrawn(withdrawn) => {
                if let Some(entered) = self.certificates.get_mut(&withdrawn.serial)
                    && entered.withdrawn.is_none()
                {
                    entered.withdrawn = Some(withdrawn);
                }
            }
        }
    }

    fn fold(&mut self, tail: &Tail) -> Result<(), ServerError> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes).map_err(|error| {
                unavailable(format!("leaf {index} is not a certificate line: {error}"))
            })?;
            self.hold(index, line);
        }
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
        })
        .map_err(|error| format!("certificates state: {error}"))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed = serde_json::from_slice(bytes)
            .map_err(|error| format!("certificates state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "certificates state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}

/// The certificate log, read from its leaf store and appended to it.
pub struct CertificateStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

impl CertificateStore<FileLeafStore> {
    /// The certificate log in the directory `dir`, created when it does not
    /// exist, its snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }

    /// As `open`, saying through `say` how the log was started and how many
    /// certificates it holds.
    pub fn opened(
        dir: &Path,
        key: Arc<Ed25519Identity>,
        say: &(dyn Fn(&str) + Send + Sync),
    ) -> Result<Self, ServerError> {
        let store = Self::open(dir, key)?;
        say(&format!(
            "certificate log {}, holding {} certificates",
            store.start(),
            store.certificates().count()
        ));
        Ok(store)
    }
}

impl<S: LeafStore> CertificateStore<S> {
    /// The certificate log in the leaf store `reopen` opens, its snapshots
    /// signed by `key`.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
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

    /// How the log was started.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// Every certificate as it stands, by serial.
    pub fn certificates(&self) -> impl Iterator<Item = &Entered> {
        self.held.certificates.values()
    }

    /// The certificate named `serial`, as it stands.
    pub fn certificate(&self, serial: &str) -> Option<&Entered> {
        self.held.certificates.get(serial)
    }

    /// Enter `issued`. Entered again with the same bytes and claims it is
    /// kept once; the same serial with other bytes is refused.
    pub fn issue(&mut self, issued: Issued) -> Result<(), ServerError> {
        self.settle()?;
        match self.held.certificates.get(&issued.serial) {
            Some(kept) if same_certificate(&kept.issued, &issued) => Ok(()),
            Some(_) => Err(ServerError::CertificateReused {
                serial: issued.serial,
            }),
            None => self.append(Line::Issued(issued)),
        }
    }

    /// Withdraw the certificate named in `withdrawn`. Withdrawn again by the
    /// same person for the same reason it is kept once; one already
    /// withdrawn otherwise is refused naming who withdrew it.
    pub fn withdraw(&mut self, withdrawn: Withdrawn) -> Result<(), ServerError> {
        self.settle()?;
        let Some(kept) = self.held.certificates.get(&withdrawn.serial) else {
            return Err(ServerError::CertificateUnknown {
                serial: withdrawn.serial,
            });
        };
        match &kept.withdrawn {
            None => self.append(Line::Withdrawn(withdrawn)),
            Some(earlier) if earlier.by == withdrawn.by && earlier.reason == withdrawn.reason => {
                Ok(())
            }
            Some(earlier) => Err(ServerError::CertificateWithdrawn {
                serial: withdrawn.serial,
                by: earlier.by.clone(),
            }),
        }
    }

    /// The certificate named `serial` with what verifies its entry offline
    /// against the log as it stands now.
    pub fn prove(&self, serial: &str) -> Result<Proven, ServerError> {
        let entered = self.held.certificates.get(serial).cloned().ok_or_else(|| {
            ServerError::CertificateUnknown {
                serial: serial.to_owned(),
            }
        })?;
        let leaf_bytes = self
            .log
            .leaf_bytes(entered.leaf)
            .map_err(unavailable)?
            .ok_or_else(|| unavailable(format!("leaf {} is not in the log", entered.leaf)))?;
        let proof = self
            .log
            .proof_tree()
            .map_err(unavailable)?
            .prove_inclusion(entered.leaf)
            .map_err(unavailable)?;
        Ok(Proven {
            entered,
            leaf_bytes,
            tree_size: self.log.len(),
            root: self.log.root(),
            proof,
        })
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

    /// Resolve an append whose outcome is not known by opening the log again.
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

    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&line).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            self.held.hold(index, line);
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
}

fn same_certificate(kept: &Issued, issued: &Issued) -> bool {
    kept.der == issued.der
        && kept.agent == issued.agent
        && kept.person == issued.person
        && kept.claims == issued.claims
}

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
    held.fold(&started.tail)?;
    Ok((started.log, held, started.start))
}

fn rebuilt<S: LeafStore>(reopen: &Reopen<S>, reason: String) -> Result<Opened<S>, ServerError> {
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(&tail)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}
