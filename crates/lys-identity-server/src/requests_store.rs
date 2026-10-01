//! The access requests as they are kept: a leaf store of their own, one leaf
//! for each request asked and each decision made, written before the request
//! is answered. A leaf is stored whole or not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A request is a question and gives no access. The access an approval gives
//! is a grant, signed and receipted by the grants like every other. So these
//! leaves are not signed, and carry no receipt.
//!
//! An approval is two writes: the grant, in the grants' log, and the decision
//! here. So the approval is kept here as an intent before the grant is
//! issued. While an intent stands only that approval, by that person, with
//! that operation, source and note, is taken: nobody else approves and nobody
//! declines. The intent is withdrawn only when the grants hold nothing for
//! its operation.
//!
//! A request is named by the operation id it was asked with, so asking again
//! with the same operation and the same words answers the request already
//! kept, and the same operation with other words is refused.
//!
//! The leaves are pinned, and what they fold to is sealed in the log's signed
//! snapshot every [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a
//! start reads the snapshot and only the leaves after it. A snapshot refused,
//! or a state that does not read back, sends the start to every leaf, by
//! name, never silently.

use std::ops::Bound;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::requests_state::{DOMAIN, Held, pin_unpinned};

/// A request as it was asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asked {
    /// The operation id it was asked with, which names it.
    pub id: String,
    /// The identity that asks, and would hold the access.
    pub asked_by: String,
    /// The person who answers for that identity.
    pub responsible: String,
    /// The kind of the resource asked for.
    pub resource_kind: String,
    /// The id of the resource asked for.
    pub resource_id: String,
    /// The relation asked for.
    pub relation: String,
    /// When the access would end, in seconds since the Unix epoch, or null for no end of its own.
    pub ends_at: Option<u64>,
    /// Why it is asked, in the asker's words.
    pub why: String,
    /// When it was asked, in seconds since the Unix epoch.
    pub asked_at: u64,
}

/// The decision on a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decided {
    /// The request decided.
    pub id: String,
    /// The person who decided.
    pub by: String,
    /// Whether it was approved.
    pub approved: bool,
    /// The decider's words.
    pub note: String,
    /// The grant the approval issued, null for a request declined.
    pub grant: Option<String>,
    /// When it was decided, in seconds since the Unix epoch.
    pub decided_at: u64,
}

/// An approval about to be made: kept before the grant is issued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intended {
    /// The request approved.
    pub id: String,
    /// The person approving.
    pub by: String,
    /// The operation id the grant is issued under.
    pub operation: String,
    /// The grant the access is lent from, null for the root authority's own issue.
    pub source: Option<String>,
    /// The approver's words.
    pub note: String,
    /// When it was intended, in seconds since the Unix epoch.
    pub intended_at: u64,
    /// How long the access is given for; absent in an intent kept before
    /// answers were offered, which gave the window the request asked for.
    #[serde(default, skip_serializing_if = "Answer::as_asked")]
    pub answer: Answer,
}

/// How long an approval gives the access for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum Answer {
    /// The window the request asked for.
    #[default]
    AsAsked,
    /// Once: the grant is spent by its first exercise.
    Once,
    /// For a while: until the given time, in seconds since the Unix epoch.
    Until {
        /// When the access ends.
        ends_at: u64,
    },
    /// Ongoing: until it is revoked.
    Ongoing,
}

impl Answer {
    /// Whether this is the window the request asked for.
    #[must_use]
    pub fn as_asked(&self) -> bool {
        *self == Self::AsAsked
    }

    /// When the access given ends, for a request asked at `asked_at` until `asked_end`.
    #[must_use]
    pub fn ends_at(self, asked_end: Option<u64>) -> Option<u64> {
        match self {
            Self::AsAsked | Self::Once => asked_end,
            Self::Until { ends_at } => Some(ends_at),
            Self::Ongoing => None,
        }
    }
}

/// An intent given up, because the grants hold nothing for its operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Withdrawn {
    /// The request.
    pub id: String,
    /// The operation id of the intent withdrawn.
    pub operation: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub(crate) enum Line {
    Asked(Asked),
    Intended(Intended),
    Withdrawn(Withdrawn),
    Decided(Decided),
}

/// A request with its decision, once it has one.
pub type Kept<'a> = (&'a Asked, Option<&'a Decided>);

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the requests' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/access-requests";

/// The requests, read from their leaf store and appended to it.
pub struct RequestStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    adopted: u64,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

/// A log opened and folded: the log, what it folds to, how it started, and
/// how many unpinned leaves it adopted.
type Opened<S> = (FrontierLog<S>, Held, Start, u64);

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::RequestsUnavailable {
        reason: what.to_string(),
    }
}

impl RequestStore<FileLeafStore> {
    /// The access requests in the directory the configuration names, if it
    /// names one, saying through `say` how the log was started and how many
    /// requests it holds.
    pub fn opened(
        config: &crate::config::Config,
        key: Arc<Ed25519Identity>,
        say: &(dyn Fn(&str) + Send + Sync),
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.requests_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, key)?;
        if store.adopted() > 0 {
            say(&format!(
                "requests log pinned {} leaves written before leaves were pinned",
                store.adopted()
            ));
        }
        say(&format!(
            "requests log {}, holding {} requests",
            store.start(),
            store.requests().count()
        ));
        Ok(Some(store))
    }

    /// The requests kept in the directory `dir`, which is created when it
    /// does not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> RequestStore<S> {
    /// The requests kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, held, start, adopted) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
            start: start.clone(),
            adopted,
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

    /// How many leaves written before leaves were pinned were pinned at open.
    pub fn adopted(&self) -> u64 {
        self.adopted
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
            let (log, held, start, adopted) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.held = held;
            self.start = start.clone();
            self.adopted += adopted;
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    /// Append one line as one leaf. A failed append is settled by reading
    /// back: the line is kept only if the leaf store holds exactly it.
    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        self.settle()?;
        let bytes = serde_json::to_vec(&line).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            self.held.hold(line);
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

    /// Every request, in the order asked.
    pub fn requests(&self) -> impl Iterator<Item = Kept<'_>> {
        self.held
            .asked
            .iter()
            .map(|asked| (asked, self.held.decided.get(&asked.id)))
    }

    /// Requests in identifier order, starting after the supplied identifier.
    pub fn requests_ordered(&self, after: Bound<&str>) -> impl Iterator<Item = &Asked> {
        self.held
            .ordered
            .range::<str, _>((after, Bound::Unbounded))
            .flat_map(|(_, positions)| positions)
            .map(|position| &self.held.asked[*position])
    }

    /// The maintained number of requests.
    pub fn request_count(&self) -> usize {
        self.held.asked.len()
    }

    /// The request named `id`.
    pub fn request(&self, id: &str) -> Option<Kept<'_>> {
        let position = *self.held.ordered.get(id)?.first()?;
        let asked = self.held.asked.get(position)?;
        Some((asked, self.held.decided.get(id)))
    }

    /// Keep `asked`. Asked again in the same words it is kept once; the same
    /// operation in other words is refused.
    pub fn ask(&mut self, asked: Asked) -> Result<(), ServerError> {
        self.settle()?;
        match self.request(&asked.id) {
            Some((kept, _)) if same_words(kept, &asked) => Ok(()),
            Some(_) => Err(ServerError::RequestReused { request: asked.id }),
            None => self.append(Line::Asked(asked)),
        }
    }

    /// The approval of the request `id` being settled, if one is.
    pub fn intent(&self, id: &str) -> Option<&Intended> {
        self.held.intended.get(id)
    }

    /// Keep the intent to approve a request that waits. Intended again in the
    /// same words it is kept once; while one stands any other is refused.
    pub fn intend(&mut self, intended: Intended) -> Result<(), ServerError> {
        self.settle()?;
        match self.request(&intended.id) {
            None => Err(ServerError::RequestUnknown),
            Some((_, Some(_))) => Err(ServerError::RequestDecided {
                request: intended.id,
            }),
            Some((_, None)) => match self.held.intended.get(&intended.id) {
                None => self.append(Line::Intended(intended)),
                Some(kept) if same_intent(kept, &intended) => Ok(()),
                Some(kept) => Err(held(kept)),
            },
        }
    }

    /// Give up the intent on the request `id` made under `operation`, when
    /// one stands.
    pub fn withdraw(&mut self, id: &str, operation: &str) -> Result<(), ServerError> {
        self.settle()?;
        match self.held.intended.get(id) {
            Some(kept) if kept.operation == operation => self.append(Line::Withdrawn(Withdrawn {
                id: id.to_owned(),
                operation: operation.to_owned(),
            })),
            _ => Ok(()),
        }
    }

    /// Keep the decision on a request that waits. Decided again the same way
    /// it is kept once; a request decided another way is refused, and so is
    /// any decision but the one an intent that stands was made for.
    pub fn decide(&mut self, decided: Decided) -> Result<(), ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.intended.get(&decided.id)
            && !(decided.approved && decided.by == kept.by)
        {
            return Err(held(kept));
        }
        match self.request(&decided.id) {
            None => Err(ServerError::RequestUnknown),
            Some((_, None)) => self.append(Line::Decided(decided)),
            Some((_, Some(kept)))
                if kept.approved == decided.approved
                    && kept.by == decided.by
                    && kept.grant == decided.grant =>
            {
                Ok(())
            }
            Some(_) => Err(ServerError::RequestDecided {
                request: decided.id,
            }),
        }
    }
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let mut store = reopen().map_err(unavailable)?;
    let adopted = pin_unpinned(&mut store).map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason, adopted),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start, adopted))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`.
fn rebuilt<S: LeafStore>(
    reopen: &Reopen<S>,
    reason: String,
    adopted: u64,
) -> Result<Opened<S>, ServerError> {
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(&tail).map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start, adopted))
}

fn held(kept: &Intended) -> ServerError {
    ServerError::RequestHeld {
        request: kept.id.clone(),
        by: kept.by.clone(),
    }
}

/// Whether two intents are the same approval, whenever each was made.
fn same_intent(kept: &Intended, intended: &Intended) -> bool {
    let timeless = Intended {
        intended_at: kept.intended_at,
        ..intended.clone()
    };
    *kept == timeless
}

/// Whether two askings are the same request, whenever each was asked.
fn same_words(kept: &Asked, asked: &Asked) -> bool {
    let timeless = Asked {
        asked_at: kept.asked_at,
        ..asked.clone()
    };
    *kept == timeless
}
