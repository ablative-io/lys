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

use std::collections::BTreeMap;
use std::path::Path;

use lys_log_store::{FileLeafStore, LeafStore, StoreResult};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;

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
enum Line {
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
    leaves: S,
    folded: u64,
    uncertain: bool,
    asked: Vec<Asked>,
    intended: BTreeMap<String, Intended>,
    decided: BTreeMap<String, Decided>,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::RequestsUnavailable {
        reason: what.to_string(),
    }
}

impl RequestStore<FileLeafStore> {
    /// The requests kept in the directory `dir`, which is created when it does not exist.
    pub fn open(dir: &Path) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)))
    }
}

impl<S: LeafStore> RequestStore<S> {
    /// The requests kept in the leaf store `reopen` opens.
    pub fn over(reopen: Reopen<S>) -> Result<Self, ServerError> {
        let leaves = reopen().map_err(unavailable)?;
        let mut store = Self {
            reopen,
            leaves,
            folded: 0,
            uncertain: false,
            asked: Vec::new(),
            intended: BTreeMap::new(),
            decided: BTreeMap::new(),
        };
        store.fold()?;
        Ok(store)
    }

    /// Read every leaf not yet read.
    fn fold(&mut self) -> Result<(), ServerError> {
        while self.folded < self.leaves.extent() {
            let index = self.folded;
            let bytes = self
                .leaves
                .leaf(index)
                .map_err(unavailable)?
                .ok_or_else(|| {
                    unavailable(format!("leaf {index} is within the extent and absent"))
                })?;
            let line = serde_json::from_slice(&bytes).map_err(|error| {
                unavailable(format!("leaf {index} is not a request line: {error}"))
            })?;
            self.hold(line);
            self.folded += 1;
        }
        Ok(())
    }

    /// Resolve an append whose outcome is not known, by opening the leaf
    /// store again and reading what it holds. Until that succeeds nothing is
    /// answered from memory and nothing is appended.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            self.leaves = (self.reopen)().map_err(unavailable)?;
            self.fold()?;
            self.uncertain = false;
        }
        Ok(())
    }

    fn hold(&mut self, line: Line) {
        match line {
            Line::Asked(asked) => self.asked.push(asked),
            Line::Intended(intended) => {
                self.intended.insert(intended.id.clone(), intended);
            }
            Line::Withdrawn(withdrawn) => {
                if self
                    .intended
                    .get(&withdrawn.id)
                    .is_some_and(|kept| kept.operation == withdrawn.operation)
                {
                    self.intended.remove(&withdrawn.id);
                }
            }
            Line::Decided(decided) => {
                self.intended.remove(&decided.id);
                self.decided.insert(decided.id.clone(), decided);
            }
        }
    }

    /// Append one line as one leaf. A failed append is settled by reading
    /// back: the line is kept only if the leaf store holds exactly it.
    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        self.settle()?;
        let bytes = serde_json::to_vec(&line).map_err(unavailable)?;
        let index = self.folded;
        let Err(failure) = self.leaves.put_leaf(index, &bytes) else {
            self.hold(line);
            self.folded += 1;
            return Ok(());
        };
        self.uncertain = true;
        self.settle()?;
        match self.leaves.leaf(index).map_err(unavailable)? {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure)),
        }
    }

    /// Every request, in the order asked.
    pub fn requests(&self) -> impl Iterator<Item = Kept<'_>> {
        self.asked
            .iter()
            .map(|asked| (asked, self.decided.get(&asked.id)))
    }

    /// The request named `id`.
    pub fn request(&self, id: &str) -> Option<Kept<'_>> {
        self.requests().find(|(asked, _)| asked.id == id)
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
        self.intended.get(id)
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
            Some((_, None)) => match self.intended.get(&intended.id) {
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
        match self.intended.get(id) {
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
        if let Some(kept) = self.intended.get(&decided.id)
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
