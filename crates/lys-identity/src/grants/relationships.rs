//! The grant representation in `SpiceDB` (ADR-078, C5), as a pure mapping
//! from the grant book to relationship updates. Nothing here reads or writes
//! the engine: the identity server's projector writes what this answers.
//!
//! Every grant, root or delegated, is its own grant object keyed by its
//! grant identifier, and is written as its resource relationship (the
//! resource's relationship to the grant object, one for each action the
//! grant carries), its holder relationship (the grant to the identity
//! holding it, carrying the window caveat whose end is the grant's own
//! recorded end) and its standing relationship: a delegated grant's source
//! relationship to its source grant, or a root grant's root relationship to
//! the directory's root authority, the one `root_authority` object. A
//! permission holds through a grant only while its standing holds, so
//! deleting a grant's standing relationship refuses its holder and every
//! grant derived from it at once.
//!
//! A permission is a set of subjects, and a root grant's relationships reach
//! only its own holder and the root authority; so the root authority object
//! carries two wildcard relationships, one to every person and one to every
//! agent, and a grant stands while its walk ends there. They are written by
//! each root issue, which only the root authority makes, and never deleted.
//!
//! Invariants: an update is only ever answered for a committed event the
//! book holds, at its log position, and the same position always answers
//! the same updates. An expiry is written as recorded, never clamped: a
//! committed delegation ending later than its source is refused by name, as
//! is a grant that would stand on nothing. A revoke deletes the withdrawn
//! grant's standing relationship alone first, then its remaining
//! relationships and those of every grant derived from it that stood when
//! the revoke was committed, and nothing of any other grant.

use std::collections::{BTreeMap, BTreeSet};

use super::error::{GrantError, ProjectionRefusal};
use super::permission::ObjectRef;
use super::projection::{Changed, GrantBook};
use super::types::{Action, Grant, GrantId, Resource, Source};
use crate::id::{IdentityId, PersonId};

/// The object type of a resource under one action.
pub const RESOURCE: &str = "resource";
/// The object type of a grant.
pub const GRANT: &str = "grant";
/// The object type of the directory's root authority.
pub const ROOT_AUTHORITY: &str = "root_authority";
/// The one root authority object of a directory.
pub const DIRECTORY: &str = "directory";
/// The permission every check and lookup asks for on a resource.
pub const EXERCISE: &str = "exercise";
/// The resource's relation to a grant.
pub const GRANTED: &str = "granted";
/// The grant's relation to its holder.
pub const HOLDER: &str = "holder";
/// A delegated grant's relation to its source grant.
pub const SOURCE: &str = "source";
/// A root grant's relation to the root authority.
pub const ROOT: &str = "root";
/// The root authority's relation to every person and every agent.
pub const ANYONE: &str = "anyone";
/// The caveat of a holder relationship whose grant ends.
pub const WITHIN_WINDOW: &str = "within_window";
/// The caveat of a holder relationship whose grant has no end.
pub const STARTED: &str = "started";

/// The window a holder relationship's caveat carries, in seconds since the
/// Unix epoch: from `starts_at`, until before `ends_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Window {
    /// The latest start on the grant's ancestry.
    pub starts_at: u64,
    /// The grant's own recorded end, when it has one.
    pub ends_at: Option<u64>,
}

impl Window {
    /// The name of the caveat the window is written with.
    pub fn caveat(&self) -> &'static str {
        if self.ends_at.is_some() {
            WITHIN_WINDOW
        } else {
            STARTED
        }
    }
}

/// One relationship: `resource#relation@subject`, with the holder
/// relationship's window. A subject id of `*` is every subject of its type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tuple {
    /// The object the relation is on.
    pub resource: ObjectRef,
    /// The relation.
    pub relation: String,
    /// The subject.
    pub subject: ObjectRef,
    /// The window caveat, on a holder relationship only.
    pub window: Option<Window>,
}

/// One change to a relationship.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Update {
    /// Write the relationship, or leave it as it is written.
    Touch(Tuple),
    /// Delete the relationship, or leave it absent.
    Delete(Tuple),
}

/// What one committed event does to the relationships.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The event changes no relationship: a use, or a refused use or revoke.
    Unchanged,
    /// A grant issued: its relationships, written together.
    Issue(Vec<Update>),
    /// A grant withdrawn: its standing relationship alone, then the rest.
    Revoke {
        /// The grant withdrawn.
        grant: GrantId,
        /// The deletion of its standing relationship.
        standing: Update,
        /// The deletion of its remaining relationships and of every
        /// relationship of each grant derived from it.
        rest: Vec<Update>,
    },
}

/// One committed event's position and what it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The event's log position: its index plus one, so the first event
    /// stands at position 1.
    pub position: u64,
    /// What it does, or why the projection refuses it.
    pub step: Result<Step, ProjectionRefusal>,
}

fn object(kind: &str, id: String) -> ObjectRef {
    ObjectRef {
        kind: kind.to_owned(),
        id,
    }
}

/// A token as the engine takes it in an object id: `.` is not an id
/// character there, and `|` is not a token character here.
fn engine_token(text: &str) -> String {
    text.replace('.', "|")
}

/// The object a grant of `action` on `resource` is related to.
pub fn resource_object(resource: &Resource, action: &Action) -> ObjectRef {
    object(
        RESOURCE,
        format!(
            "{}/{}/{}",
            engine_token(resource.kind()),
            engine_token(resource.id()),
            engine_token(action.as_str())
        ),
    )
}

/// The directory's root authority object.
pub fn root_authority() -> ObjectRef {
    object(ROOT_AUTHORITY, DIRECTORY.to_owned())
}

/// The root authority's two wildcard relationships.
pub fn anchor() -> [Tuple; 2] {
    ["person", "agent"].map(|kind| Tuple {
        resource: root_authority(),
        relation: ANYONE.to_owned(),
        subject: object(kind, "*".to_owned()),
        window: None,
    })
}

/// The grant's standing relationship: its source relationship, or for a
/// root grant its root relationship.
pub fn standing_tuple(grant: &Grant) -> Tuple {
    let (relation, subject) = match grant.source() {
        Source::Root => (ROOT, root_authority()),
        Source::Grant(source) => (SOURCE, ObjectRef::grant(source)),
    };
    Tuple {
        resource: ObjectRef::grant(grant.id()),
        relation: relation.to_owned(),
        subject,
        window: None,
    }
}

/// The grant's holder relationship, whose caveat starts at `starts_at`.
pub fn holder_tuple(grant: &Grant, starts_at: u64) -> Tuple {
    Tuple {
        resource: ObjectRef::grant(grant.id()),
        relation: HOLDER.to_owned(),
        subject: ObjectRef::identity(grant.holder()),
        window: Some(Window {
            starts_at,
            ends_at: grant.window().ends_at(),
        }),
    }
}

/// The grant's resource relationships, one for each action it carries.
pub fn resource_tuples(grant: &Grant) -> Vec<Tuple> {
    grant
        .actions()
        .iter()
        .map(|action| Tuple {
            resource: resource_object(grant.resource(), action),
            relation: GRANTED.to_owned(),
            subject: ObjectRef::grant(grant.id()),
            window: None,
        })
        .collect()
}

/// The latest start on the grant's ancestry, from the book.
fn ancestry_start(
    book: &GrantBook,
    grant: &Grant,
    position: u64,
) -> Result<u64, ProjectionRefusal> {
    let lineage = book
        .lineage(grant.id())
        .map_err(|refusal| ProjectionRefusal::Refused {
            position,
            reason: refusal.to_string(),
        })?;
    Ok(lineage
        .path
        .iter()
        .filter_map(|id| book.grant(*id))
        .map(|held| held.window().starts_at())
        .fold(grant.window().starts_at(), u64::max))
}

/// Every relationship of `grant`: its resource relationships, its holder
/// relationship and its standing relationship.
pub fn tuples_of(book: &GrantBook, grant: &Grant) -> Vec<Tuple> {
    let starts_at = ancestry_start(book, grant, 0).unwrap_or(grant.window().starts_at());
    let mut tuples = resource_tuples(grant);
    tuples.push(holder_tuple(grant, starts_at));
    tuples.push(standing_tuple(grant));
    tuples
}

fn issue(
    book: &GrantBook,
    grant: &Grant,
    root_authority: PersonId,
    position: u64,
) -> Result<Step, ProjectionRefusal> {
    let mut updates = Vec::new();
    match grant.source() {
        Source::Root => {
            if grant.parts().issuer != IdentityId::Person(root_authority) {
                return Err(ProjectionRefusal::StandingMissing {
                    position,
                    grant: grant.id().to_string(),
                });
            }
            updates.extend(anchor().into_iter().map(Update::Touch));
        }
        Source::Grant(source_id) => {
            let source =
                book.grant(source_id)
                    .ok_or_else(|| ProjectionRefusal::StandingMissing {
                        position,
                        grant: grant.id().to_string(),
                    })?;
            if let Some(source_ends) = source.window().ends_at() {
                let requested = grant.window().ends_at();
                if requested.is_none_or(|ends| ends > source_ends) {
                    return Err(ProjectionRefusal::ExpiryPastSource {
                        position,
                        source_grant: source_id.to_string(),
                        source_ends,
                        requested: requested
                            .map_or_else(|| "no end".to_owned(), |ends| ends.to_string()),
                    });
                }
            }
        }
    }
    let starts_at = ancestry_start(book, grant, position)?;
    let mut tuples = resource_tuples(grant);
    tuples.push(holder_tuple(grant, starts_at));
    tuples.push(standing_tuple(grant));
    updates.extend(tuples.into_iter().map(Update::Touch));
    Ok(Step::Issue(updates))
}

/// The grants derived from `withdrawn` that stood when the revoke at `index`
/// was committed: issued before it, and not withdrawn before it with their
/// own subtree.
fn standing_descendants(book: &GrantBook, withdrawn: GrantId, index: u64) -> Vec<GrantId> {
    let mut children: BTreeMap<GrantId, BTreeSet<GrantId>> = BTreeMap::new();
    for record in book.records() {
        if let Source::Grant(source) = record.grant().source() {
            children
                .entry(source)
                .or_default()
                .insert(record.grant().id());
        }
    }
    let mut found = Vec::new();
    let mut next = vec![withdrawn];
    while let Some(current) = next.pop() {
        for child in children.get(&current).into_iter().flatten() {
            let Some(record) = book.record(*child) else {
                continue;
            };
            let issued_before = record.index() < index;
            let withdrawn_before = record
                .revoked()
                .is_some_and(|revocation| revocation.index < index);
            if issued_before && !withdrawn_before {
                found.push(*child);
                next.push(*child);
            }
        }
    }
    found
}

fn revoke(
    book: &GrantBook,
    withdrawn: GrantId,
    index: u64,
    position: u64,
) -> Result<Step, ProjectionRefusal> {
    let grant = book
        .grant(withdrawn)
        .ok_or_else(|| ProjectionRefusal::Refused {
            position,
            reason: format!("no grant {withdrawn} is held"),
        })?;
    let standing = Update::Delete(standing_tuple(grant));
    let starts_at = ancestry_start(book, grant, position)?;
    let mut rest: Vec<Update> = resource_tuples(grant)
        .into_iter()
        .chain([holder_tuple(grant, starts_at)])
        .map(Update::Delete)
        .collect();
    for derived in standing_descendants(book, withdrawn, index) {
        if let Some(held) = book.grant(derived) {
            rest.extend(tuples_of(book, held).into_iter().map(Update::Delete));
        }
    }
    Ok(Step::Revoke {
        grant: withdrawn,
        standing,
        rest,
    })
}

/// What a committed event the book refused does: a refused use or revoke
/// changes nothing, and any other refusal is the projection's too.
fn refused(position: u64, refusal: &GrantError) -> Result<Step, ProjectionRefusal> {
    match refusal {
        GrantError::EventMismatch { .. }
        | GrantError::GrantUnknown { .. }
        | GrantError::AlreadyRevoked { .. } => Ok(Step::Unchanged),
        GrantError::ExpiryBeyondSource {
            requested,
            source_grant,
            source_ends,
        } => Err(ProjectionRefusal::ExpiryPastSource {
            position,
            source_grant: source_grant.clone(),
            source_ends: *source_ends,
            requested: requested.clone(),
        }),
        other => Err(ProjectionRefusal::Refused {
            position,
            reason: other.to_string(),
        }),
    }
}

/// What each committed event from log position `from` to `to`, both
/// included, does to the relationships, in log order, as the book records
/// it. `root_authority` is the directory's root authority.
pub fn plan(book: &GrantBook, from: u64, to: u64, root_authority: PersonId) -> Vec<Planned> {
    let first = from.max(1);
    let changes = book.changes_from(first - 1);
    (first..=to)
        .map(|position| {
            let index = position - 1;
            let step = if let Some((_, refusal)) = book.refused().get(&index) {
                refused(position, refusal)
            } else {
                match changes.get(&index) {
                    Some(Changed::Issued(id)) => match book.grant(*id) {
                        Some(grant) => issue(book, grant, root_authority, position),
                        None => Err(ProjectionRefusal::Refused {
                            position,
                            reason: format!("no grant {id} is held"),
                        }),
                    },
                    Some(Changed::Revoked(id)) => revoke(book, *id, index, position),
                    None => Ok(Step::Unchanged),
                }
            };
            Planned { position, step }
        })
        .collect()
}
