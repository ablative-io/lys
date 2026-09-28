//! The one writer of `SpiceDB` relationships: the projection of committed
//! signed grant events.
//!
//! The projector applies each committed event once, in log order, as the
//! grant book's pure mapping (`lys_identity::grants::relationships`) answers
//! it, and records the log position and the `SpiceDB` revision token it
//! reached. A log position counts the events up to and including it, so the
//! first event stands at position 1 and a projector that has applied nothing
//! stands at 0.
//!
//! Invariants: nothing is written for an event that is not committed, since
//! only the book's folded events are planned. Every write is idempotent. A
//! revoke stays one event: the withdrawn grant's standing relationship is
//! deleted alone, as the one update of one write, so nothing through it or
//! through any grant derived from it is permitted from then on; the rest
//! follows in writes of at most the configured `max_updates_per_write`, each
//! as full as that cap allows; and the position does not move past the
//! revoke until a read shows none of those relationships remains, so a
//! projector stopped between two writes resumes the revoke on replay. An
//! event the projection refuses is reported by name with its position,
//! nothing is written for it, and no later event is applied.

use std::path::PathBuf;
use std::sync::Arc;

use lys_identity::PersonId;
use lys_identity::grants::relationships::{Step, Tuple, Update, Window, plan};
use lys_identity::grants::{GrantBook, GrantError, ObjectRef};
use prost_types::value::Kind;
use serde::{Deserialize, Serialize};

use super::client::SpiceDbApi;
use super::error::SpiceDbError;
use super::wire::authzed::api::v1::consistency::Requirement;
use super::wire::authzed::api::v1::relationship_update::Operation;
use super::wire::authzed::api::v1::{
    Consistency, ContextualizedCaveat, ObjectReference, ReadRelationshipsRequest, Relationship,
    RelationshipFilter, RelationshipUpdate, SubjectFilter, SubjectReference,
    WriteRelationshipsRequest, WriteRelationshipsResponse,
};

/// The relationship write the client sends.
pub type RelationshipsWrite = WriteRelationshipsRequest;
/// The answer to a relationship write.
pub type RelationshipsWritten = WriteRelationshipsResponse;

/// Where the projector stands: the log position of the last event it
/// applied, and the revision token of its last write.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reached {
    /// The log position of the last event applied; 0 before any.
    pub position: u64,
    /// The revision token `SpiceDB` answered the last write with.
    pub token: Option<String>,
}

/// Where the projector keeps what it reached.
pub trait PositionStore: Send {
    /// What was reached, or the start when nothing is kept.
    fn load(&self) -> Result<Reached, GrantError>;

    /// Keep `reached`, durably before answering.
    fn save(&mut self, reached: &Reached) -> Result<(), GrantError>;
}

/// What was reached, kept as JSON in one file, replaced whole on each save.
#[derive(Debug, Clone)]
pub struct FilePosition {
    path: PathBuf,
}

impl FilePosition {
    /// The position kept at `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

fn kept(reason: String) -> GrantError {
    GrantError::LogUnavailable { reason }
}

impl PositionStore for FilePosition {
    fn load(&self) -> Result<Reached, GrantError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!(
                    path = %self.path.display(),
                    "the projector's position is not kept; the projection is replayed from the first event"
                );
                return Ok(Reached::default());
            }
            Err(error) => {
                return Err(kept(format!(
                    "the projector's position {} could not be read: {error}",
                    self.path.display()
                )));
            }
        };
        match serde_json::from_str(&text) {
            Ok(reached) => Ok(reached),
            Err(error) => {
                tracing::warn!(
                    path = %self.path.display(),
                    %error,
                    "the projector's position is refused; the projection is replayed from the first event"
                );
                Ok(Reached::default())
            }
        }
    }

    fn save(&mut self, reached: &Reached) -> Result<(), GrantError> {
        let text = serde_json::to_string(reached)
            .map_err(|error| kept(format!("the projector's position: {error}")))?;
        let fresh = self.path.with_extension("fresh");
        let write = || -> std::io::Result<()> {
            let mut file = std::fs::File::create(&fresh)?;
            std::io::Write::write_all(&mut file, text.as_bytes())?;
            file.sync_all()?;
            std::fs::rename(&fresh, &self.path)?;
            if let Some(dir) = self.path.parent() {
                std::fs::File::open(dir)?.sync_all()?;
            }
            Ok(())
        };
        write().map_err(|error| {
            kept(format!(
                "the projector's position {} could not be kept: {error}",
                self.path.display()
            ))
        })
    }
}

/// A caveat parameter's value: a whole number of seconds, written as the
/// protocol's number, exact below 2^53.
pub fn number(value: u64) -> prost_types::Value {
    let high = u32::try_from(value >> 32).unwrap_or(u32::MAX);
    let low = u32::try_from(value & 0xffff_ffff).unwrap_or(u32::MAX);
    prost_types::Value {
        kind: Some(Kind::NumberValue(
            f64::from(high) * 4_294_967_296.0 + f64::from(low),
        )),
    }
}

/// The wire form of an object.
pub fn object_reference(object: &ObjectRef) -> ObjectReference {
    ObjectReference {
        object_type: object.kind.clone(),
        object_id: object.id.clone(),
    }
}

fn caveat(window: Window) -> ContextualizedCaveat {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("starts_at".to_owned(), number(window.starts_at));
    if let Some(ends_at) = window.ends_at {
        fields.insert("ends_at".to_owned(), number(ends_at));
    }
    ContextualizedCaveat {
        caveat_name: window.caveat().to_owned(),
        context: Some(prost_types::Struct { fields }),
    }
}

/// The wire form of a relationship.
pub fn relationship(tuple: &Tuple) -> Relationship {
    Relationship {
        resource: Some(object_reference(&tuple.resource)),
        relation: tuple.relation.clone(),
        subject: Some(SubjectReference {
            object: Some(object_reference(&tuple.subject)),
            optional_relation: String::new(),
        }),
        optional_caveat: tuple.window.map(caveat),
        optional_expires_at: None,
    }
}

fn update(change: &Update) -> RelationshipUpdate {
    let (operation, tuple) = match change {
        Update::Touch(tuple) => (Operation::Touch, tuple),
        Update::Delete(tuple) => (Operation::Delete, tuple),
    };
    RelationshipUpdate {
        operation: operation.into(),
        relationship: Some(relationship(tuple)),
    }
}

/// The filter naming exactly `tuple`.
fn exactly(tuple: &Tuple) -> RelationshipFilter {
    RelationshipFilter {
        resource_type: tuple.resource.kind.clone(),
        optional_resource_id: tuple.resource.id.clone(),
        optional_resource_id_prefix: String::new(),
        optional_relation: tuple.relation.clone(),
        optional_subject_filter: Some(SubjectFilter {
            subject_type: tuple.subject.kind.clone(),
            optional_subject_id: tuple.subject.id.clone(),
            optional_relation: None,
        }),
    }
}

/// The projection of committed grant events into `SpiceDB`.
pub struct Projector {
    client: Arc<dyn SpiceDbApi>,
    store: Box<dyn PositionStore>,
    reached: Reached,
    max_updates: usize,
    root_authority: PersonId,
}

impl std::fmt::Debug for Projector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Projector")
            .field("reached", &self.reached)
            .field("max_updates", &self.max_updates)
            .finish_non_exhaustive()
    }
}

impl Projector {
    /// The projector writing through `client`, keeping what it reached in
    /// `store`, sending no write of more than `max_updates_per_write`
    /// updates, for the directory whose root authority is `root_authority`.
    pub fn open(
        client: Arc<dyn SpiceDbApi>,
        store: Box<dyn PositionStore>,
        max_updates_per_write: u32,
        root_authority: PersonId,
    ) -> Result<Self, GrantError> {
        let max_updates = usize::try_from(max_updates_per_write)
            .ok()
            .filter(|cap| *cap > 0)
            .ok_or_else(|| {
                GrantError::from(SpiceDbError::ConfigInvalid {
                    reason: format!(
                        "max_updates_per_write is {max_updates_per_write}, and a write carries at least one update"
                    ),
                })
            })?;
        let reached = store.load()?;
        Ok(Self {
            client,
            store,
            reached,
            max_updates,
            root_authority,
        })
    }

    /// Where the projector stands.
    pub fn reached(&self) -> &Reached {
        &self.reached
    }

    /// Apply every committed event after the position reached, up to log
    /// position `committed`, as `book` records them, keeping the position
    /// after each. An event refused or not fully applied stops the projector
    /// before it.
    pub fn catch_up(&mut self, book: &GrantBook, committed: u64) -> Result<&Reached, GrantError> {
        if self.reached.position >= committed {
            return Ok(&self.reached);
        }
        for planned in plan(
            book,
            self.reached.position + 1,
            committed,
            self.root_authority,
        ) {
            let step = planned
                .step
                .map_err(|refusal| GrantError::ProjectionRefused {
                    position: planned.position,
                    refusal,
                })?;
            let token = self.apply(&step)?;
            let mut next = Reached {
                position: planned.position,
                token: self.reached.token.clone(),
            };
            if token.is_some() {
                next.token = token;
            }
            self.store.save(&next)?;
            self.reached = next;
        }
        Ok(&self.reached)
    }

    /// Apply one event's step, answering the token of its last write.
    fn apply(&self, step: &Step) -> Result<Option<String>, GrantError> {
        match step {
            Step::Unchanged => Ok(None),
            Step::Issue(updates) => self.write(updates),
            Step::Revoke {
                grant,
                standing,
                rest,
            } => {
                self.write(std::slice::from_ref(standing))?;
                let token = self.write(rest)?;
                let withdrawn: Vec<&Update> = std::iter::once(standing).chain(rest).collect();
                for change in withdrawn {
                    let (Update::Delete(tuple) | Update::Touch(tuple)) = change;
                    if self.holds(tuple)? {
                        return Err(GrantError::EngineUnanswered {
                            reason: format!(
                                "a relationship of {grant} or of a grant derived from it remains after its revoke was written"
                            ),
                        });
                    }
                }
                Ok(token)
            }
        }
    }

    /// Send `updates` in writes of at most the cap, each as full as it allows.
    fn write(&self, updates: &[Update]) -> Result<Option<String>, GrantError> {
        let mut token = None;
        for chunk in updates.chunks(self.max_updates) {
            let written = self.client.write_relationships(WriteRelationshipsRequest {
                updates: chunk.iter().map(update).collect(),
                optional_preconditions: Vec::new(),
                optional_transaction_metadata: None,
            })?;
            let written_at = written.written_at.ok_or_else(|| SpiceDbError::Malformed {
                operation: "WriteRelationships",
                address: self.client.address().to_owned(),
                missing: "the revision it was written at",
            })?;
            token = Some(written_at.token);
        }
        Ok(token)
    }

    /// Whether the store holds `tuple`, read fully consistently.
    fn holds(&self, tuple: &Tuple) -> Result<bool, GrantError> {
        let held = self.client.read_relationships(ReadRelationshipsRequest {
            consistency: Some(Consistency {
                requirement: Some(Requirement::FullyConsistent(true)),
            }),
            relationship_filter: Some(exactly(tuple)),
            optional_limit: 0,
            optional_cursor: None,
        })?;
        Ok(held.iter().any(|answer| answer.relationship.is_some()))
    }
}
