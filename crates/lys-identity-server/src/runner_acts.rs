//! The acts on sessions through a runner, as they are kept: a leaf store of
//! their own, one leaf for each act, appended once the act is answered and
//! before its receipt is. A receipt is the leaf and its place in the log;
//! it is never itself appended.
//!
//! A leaf names the caller, the act, the session, its agent and machine, and
//! what the runner answered. Typed text, a message and a pattern are kept as
//! their length and SHA-256 digest only, never as text, for every profile.
//!
//! The log is sealed in its signed snapshot every [`SNAPSHOT_EVERY`] leaves
//! and at once after a rebuild, so a start reads the snapshot and only the
//! leaves after it; a snapshot refused sends the start to every leaf, by
//! name.

use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, open_with_snapshot,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::hex;

/// The origin the acts' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/runner-acts";

/// The snapshot domain the acts' count is sealed under.
pub const DOMAIN: &str = "lys/identity/runner-acts-state/v1";

const FORMAT: &str = "lys-runner-acts-state/v1";

/// Text an act carried, as it is kept: its length and digest, never itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Digested {
    /// Its length in bytes.
    pub length: usize,
    /// The lowercase hex SHA-256 of its bytes.
    pub sha256: String,
}

impl Digested {
    /// `text`'s length and digest.
    pub fn of(text: &str) -> Self {
        Self::bytes(text.as_bytes())
    }

    /// Exact terminal input's length and digest, without lossy decoding or payload logging.
    pub fn bytes(data: &[u8]) -> Self {
        Self {
            length: data.len(),
            sha256: hex(&Sha256::digest(data)),
        }
    }
}

/// One act, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RunnerAct {
    /// The act: `start`, `input`, `keys`, `read`, `wait`, `resize`,
    /// `compact`, `wake` or `end`.
    pub act: String,
    /// The identity that asked for it.
    pub caller: String,
    /// The session.
    pub session: String,
    /// The agent the session is held under.
    pub agent: String,
    /// The machine it runs on.
    pub machine: String,
    /// When it was answered, in seconds since the Unix epoch.
    pub at: u64,
    /// Text typed, a message or a pattern, as length and digest only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Digested>,
    /// Keys sent, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
    /// What the runner answered: its answer's kind, or its refusal's name.
    pub outcome: String,
}

/// A kept act and where the log holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ActReceipt {
    /// The leaf's index in the acts' log.
    pub index: u64,
    /// The lowercase hex RFC 6962 hash of the leaf.
    pub leaf_hash: String,
    /// The act.
    pub act: RunnerAct,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    acts: u64,
}

/// How the leaf store is opened again.
pub type Reopen<S> = Box<dyn Fn() -> lys_log_store::StoreResult<S> + Send>;

/// The acts, appended to their leaf store and read from it.
pub struct ActStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::RuntimeUnavailable {
        reason: format!("runner acts: {what}"),
    }
}

impl ActStore<FileLeafStore> {
    /// The acts kept in `dir`, created when it does not exist, their
    /// snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> ActStore<S> {
    /// The acts kept in the leaf store `reopen` opens.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, started) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            start: started.clone(),
            since_snapshot: 0,
            snapshot_failure: None,
        };
        match started {
            Start::Resumed { replayed, .. } => store.since_snapshot = replayed,
            Start::Rebuilt { .. } => store.write_snapshot(),
        }
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

    /// How many acts are kept.
    pub fn len(&self) -> u64 {
        self.log.len()
    }

    /// Whether no act is kept.
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    fn write_snapshot(&mut self) {
        let sealed = serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            acts: self.log.len(),
        });
        let written = sealed.map_err(|error| error.to_string()).and_then(|state| {
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

    /// Keep `act` as one leaf, answering its receipt. A failed append is
    /// settled by opening the store again: the act is kept only if the
    /// store holds exactly it.
    pub fn keep(&mut self, act: RunnerAct) -> Result<ActReceipt, ServerError> {
        let bytes = serde_json::to_vec(&act).map_err(unavailable)?;
        let index = self.log.len();
        match self.log.append(&bytes) {
            Ok((index, leaf_hash)) => {
                self.since_snapshot += 1;
                if self.since_snapshot >= SNAPSHOT_EVERY.get() {
                    self.write_snapshot();
                }
                Ok(ActReceipt {
                    index,
                    leaf_hash: hex(&leaf_hash),
                    act,
                })
            }
            Err(failure) => {
                let (log, started) = opened(&self.reopen, &self.key)?;
                self.log = log;
                self.start = started;
                match self.log.leaf_bytes(index).map_err(unavailable)? {
                    Some(held) if held == bytes => {
                        self.receipt(index)?.ok_or_else(|| unavailable(failure))
                    }
                    _ => Err(unavailable(failure)),
                }
            }
        }
    }

    /// The receipt of the act at `index`, none when the log holds none there.
    pub fn receipt(&self, index: u64) -> Result<Option<ActReceipt>, ServerError> {
        let Some(bytes) = self.log.leaf_bytes(index).map_err(unavailable)? else {
            return Ok(None);
        };
        let act = serde_json::from_slice(&bytes)
            .map_err(|error| unavailable(format!("leaf {index} is not an act: {error}")))?;
        Ok(Some(ActReceipt {
            index,
            leaf_hash: hex(&leaf_hash(&bytes)),
            act,
        }))
    }
}

/// The RFC 6962 hash of the leaf `bytes`: SHA-256 of a zero byte and them.
fn leaf_hash(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0_u8]);
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Open the log from its snapshot, or from every leaf when the snapshot is
/// refused, naming why.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<(FrontierLog<S>, Start), ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let readable = started.state.as_deref().map(|state| {
        serde_json::from_slice::<Sealed>(state)
            .map_err(|error| error.to_string())
            .and_then(|sealed| {
                if sealed.format == FORMAT {
                    Ok(sealed)
                } else {
                    Err(format!(
                        "state is in format {}, not {FORMAT}",
                        sealed.format
                    ))
                }
            })
    });
    if let Some(Err(reason)) = readable {
        let (log, _tail) =
            FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
        let replayed = log.len();
        let start = Start::Rebuilt {
            refusal: SnapshotRefusal::StateUnreadable { reason },
            replayed,
        };
        return Ok((log, start));
    }
    Ok((started.log, started.start))
}
