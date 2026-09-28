//! The permission projection: each grant as `SpiceDB` relationships.
//!
//! A grant is written as three relationships, as [`SCHEMA`] gives them: the
//! holder, caveated with the grant's end; the source relation, to the grant
//! it derives from or, for a root, to its responsible person; and the
//! relation on the resource, whose subject is the grant's holder set. Revoking
//! a grant deletes every relationship of that grant and of every grant
//! derived from it. The relationships are derived from committed grant events
//! only, in log order, and the store's revision is the number of events it
//! reflects, so a decision can require the revision a change made.
//!
//! Road step 2 answers the grant question behind this decision with an
//! [`Evaluator`]: [`Grants::decide`] asks it exactly once and admits nothing
//! it does not permit. The book only names what was decided, the grant a
//! permit rests on or the reason for a refusal; it never turns a refusal
//! into a permit. The directory's lifecycle rule stands in front of it: a
//! caller the directory does not record as active is refused
//! `IdentityNotActive` before `SpiceDB` is asked, and a permit is refused the
//! same way when a holder on the grant's path is no longer active or answered
//! for by the person the directory records.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use lys_log_store::LeafStore;

use super::admission::{active, holders_active};
use super::authority::{ExerciseRequest, Grants, Permit};
use super::error::GrantError;
use super::events::GrantChange;
use super::lineage::Lineage;
use super::projection::{GrantBook, GrantRecord};
use super::revocation::unrevoked;
use super::types::{Action, Grant, GrantId, Resource, Source};
use crate::id::IdentityId;
use crate::projection::Projection;

/// The permission model the relationships follow, in the `SpiceDB` schema
/// language. Each resource kind is defined beside these with one relation per
/// model relation, each of subject type `grant#holder`.
pub const SCHEMA: &str = "\
caveat unexpired(now uint, ends_at uint) {
  now < ends_at
}

definition person {}

definition agent {}

definition grant {
  relation holder: person | person with unexpired | agent | agent with unexpired
  relation source: grant | person
}
";

/// An object: a kind and an id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectRef {
    /// The object's kind.
    pub kind: String,
    /// The object's id within its kind.
    pub id: String,
}

impl ObjectRef {
    /// The grant `id`.
    pub fn grant(id: GrantId) -> Self {
        Self {
            kind: "grant".to_owned(),
            id: id.to_string(),
        }
    }

    /// The person or agent `identity`.
    pub fn identity(identity: IdentityId) -> Self {
        let kind = match identity {
            IdentityId::Person(_) => "person",
            IdentityId::Agent(_) => "agent",
        };
        Self {
            kind: kind.to_owned(),
            id: identity.to_string(),
        }
    }

    /// The resource `resource`.
    pub fn resource(resource: &Resource) -> Self {
        Self {
            kind: resource.kind().to_owned(),
            id: resource.id().to_owned(),
        }
    }
}

/// One relationship: `resource#relation@subject[#subject_relation]`, with an
/// optional end from the `unexpired` caveat.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Relationship {
    /// The object the relation is on.
    pub resource: ObjectRef,
    /// The relation.
    pub relation: String,
    /// The subject.
    pub subject: ObjectRef,
    /// The subject's relation, for a subject set.
    pub subject_relation: Option<String>,
    /// The end the `unexpired` caveat carries.
    pub ends_at: Option<u64>,
}

impl Relationship {
    /// Whether the relationship names the grant `id`, as its resource or its subject.
    pub fn names(&self, id: GrantId) -> bool {
        let grant = ObjectRef::grant(id);
        self.resource == grant || self.subject == grant
    }
}

/// The relationships `grant` is written as.
pub fn relationships_of(grant: &Grant) -> Vec<Relationship> {
    let this = ObjectRef::grant(grant.id());
    let source = match grant.source() {
        Source::Root => ObjectRef::identity(IdentityId::Person(grant.responsible())),
        Source::Grant(source) => ObjectRef::grant(source),
    };
    vec![
        Relationship {
            resource: this.clone(),
            relation: "holder".to_owned(),
            subject: ObjectRef::identity(grant.holder()),
            subject_relation: None,
            ends_at: grant.window().ends_at(),
        },
        Relationship {
            resource: this.clone(),
            relation: "source".to_owned(),
            subject: source,
            subject_relation: None,
            ends_at: None,
        },
        Relationship {
            resource: ObjectRef::resource(grant.resource()),
            relation: grant.parts().relation.to_string(),
            subject: this,
            subject_relation: Some("holder".to_owned()),
            ends_at: None,
        },
    ]
}

/// A permission engine holding relationships at a revision.
pub trait RelationshipStore {
    /// The number of grant events the relationships reflect.
    fn revision(&self) -> Result<u64, GrantError>;

    /// Delete `delete`, write `touch` and move to `revision`, all at once.
    /// `revision` is one more than the current revision, or the write is refused.
    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError>;

    /// Every relationship at the current revision.
    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError>;
}

/// An in-process permission engine. Clones share one set of relationships,
/// as clients of one engine do, so a reopened owner finds what was written.
#[derive(Debug, Clone, Default)]
pub struct MemoryRelationships {
    state: Arc<Mutex<(u64, BTreeSet<Relationship>)>>,
}

impl RelationshipStore for MemoryRelationships {
    fn revision(&self) -> Result<u64, GrantError> {
        Ok(self.state.lock().unwrap_or_else(PoisonError::into_inner).0)
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if revision != state.0 + 1 {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: format!("a write at revision {revision} does not follow {}", state.0),
            });
        }
        for relationship in delete {
            state.1.remove(relationship);
        }
        state.1.extend(touch.iter().cloned());
        state.0 = revision;
        Ok(())
    }

    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
            .clone())
    }
}

/// Every relationship in `held` naming one of `grants`.
pub fn naming(held: &BTreeSet<Relationship>, grants: &BTreeSet<GrantId>) -> Vec<Relationship> {
    held.iter()
        .filter(|relationship| grants.iter().any(|grant| relationship.names(*grant)))
        .cloned()
        .collect()
}

/// Refuse unless `held` gives every grant on `path` a holder whose caveat is
/// unexpired at `at`, and a source.
pub fn confirm(held: &BTreeSet<Relationship>, path: &[GrantId], at: u64) -> Result<(), GrantError> {
    for grant in path {
        let this = ObjectRef::grant(*grant);
        let holder = held.iter().find(|relationship| {
            relationship.resource == this && relationship.relation == "holder"
        });
        let sourced = held
            .iter()
            .any(|relationship| relationship.resource == this && relationship.relation == "source");
        let Some(holder) = holder.filter(|_| sourced) else {
            return Err(GrantError::PermissionAbsent {
                grant: grant.to_string(),
            });
        };
        if let Some(ended_at) = holder.ends_at
            && at >= ended_at
        {
            return Err(GrantError::Expired {
                grant: grant.to_string(),
                ended_at,
            });
        }
    }
    Ok(())
}

/// The grant question the permission decision asks its evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question<'a> {
    /// The identity asked about.
    pub subject: IdentityId,
    /// The object acted on.
    pub resource: &'a Resource,
    /// The action.
    pub action: &'a Action,
}

/// The evaluator's answer to one grant question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    /// Whether the evaluator permits.
    pub permitted: bool,
    /// The log position the evaluator's relationships stand at.
    pub revision: u64,
}

/// What answers the grant question behind the permission decision: in road
/// step 2, `SpiceDB` through the identity server's one evaluator.
pub trait Evaluator {
    /// Answer `question` with one evaluation, the committed grant events
    /// being those `book` holds up to log position `committed`. A question
    /// the evaluator cannot answer now is refused by name, never guessed.
    fn evaluate(
        &self,
        book: &GrantBook,
        committed: u64,
        question: &Question<'_>,
    ) -> Result<Answer, GrantError>;
}

/// The refusal when no grant covers the question, `no_grant`.
fn no_grant(question: &Question<'_>) -> GrantError {
    GrantError::NoGrant {
        identity: question.subject.to_string(),
        resource: question.resource.to_string(),
        action: question.action.to_string(),
    }
}

/// The grants `question`'s identity holds carrying its action on its resource.
fn candidates<'a>(book: &'a GrantBook, question: &Question<'a>) -> impl Iterator<Item = &'a Grant> {
    let (resource, action) = (question.resource, question.action);
    book.held_by(question.subject)
        .map(GrantRecord::grant)
        .filter(move |grant| grant.resource() == resource && grant.actions().contains(action))
}

/// The live grant a permitted decision about `question` rests on, with its
/// ancestry: the first of the identity's grants for the question whose
/// ancestry holds no withdrawn grant.
pub fn resting<'a>(book: &'a GrantBook, question: &Question<'a>) -> Option<(&'a Grant, Lineage)> {
    candidates(book, question).find_map(|grant| {
        let lineage = book.lineage(grant.id()).ok()?;
        unrevoked(book, &lineage).ok()?;
        Some((grant, lineage))
    })
}

/// The reason a refused decision about `question` at `at` names, read from
/// the book: `permission_revoked` with the withdrawn grant when a committed
/// revoke withdrew a grant on the path, else the ended or unstarted grant,
/// else `no_grant`.
pub fn refusal(book: &GrantBook, question: &Question<'_>, at: u64) -> GrantError {
    let mut ended = None;
    for grant in candidates(book, question) {
        let Ok(lineage) = book.lineage(grant.id()) else {
            continue;
        };
        let withdrawn = lineage.path.iter().find(|id| {
            book.record(**id)
                .is_some_and(|record| record.revoked().is_some())
        });
        if let Some(withdrawn) = withdrawn {
            return GrantError::PermissionRevoked {
                grant: withdrawn.to_string(),
            };
        }
        if let Some((ended_at, by)) = lineage.ends
            && at >= ended_at
        {
            ended.get_or_insert(GrantError::Expired {
                grant: by.to_string(),
                ended_at,
            });
        }
        let unstarted = lineage.path.iter().find_map(|id| {
            let held = book.grant(*id)?;
            (at < held.window().starts_at()).then(|| GrantError::NotStarted {
                grant: id.to_string(),
                starts_at: held.window().starts_at(),
            })
        });
        if let Some(unstarted) = unstarted {
            ended.get_or_insert(unstarted);
        }
    }
    ended.unwrap_or_else(|| no_grant(question))
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// The permission decision answered by `evaluator`: exactly one
    /// evaluation, and nothing recorded. A permit names the live grant it
    /// rests on; a refusal names the withdrawn grant when a committed revoke
    /// is the reason, else the ended or unstarted grant, else `no_grant`.
    /// A caller `directory` does not record as active is refused before the
    /// evaluator is asked, and a permit whose path holds an identity that is
    /// no longer active is refused by that identity's state.
    pub fn decide(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        evaluator: &dyn Evaluator,
        at: u64,
    ) -> Result<Permit, GrantError> {
        self.settle_log().ok();
        let question = Question {
            subject: request.caller,
            resource: &request.resource,
            action: &request.action,
        };
        self.unresolved_for(&question)?;
        active(directory, request.caller)?;
        let answer = evaluator.evaluate(&self.book, self.folded, &question)?;
        if !answer.permitted {
            return Err(refusal(&self.book, &question, at));
        }
        let Some((grant, lineage)) = resting(&self.book, &question) else {
            return Err(no_grant(&question));
        };
        holders_active(&self.book, directory, &lineage.path)?;
        Ok(Permit {
            grant: grant.id(),
            path: lineage.path,
            root_person: lineage.root_person,
            actions: grant.actions().clone(),
            model_version: grant.parts().model_version,
            revision: answer.revision,
            use_event: None,
        })
    }

    /// As [`Grants::decide`], at the moment the action is taken: a permitted
    /// decision records the use.
    pub fn check_with(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        evaluator: &dyn Evaluator,
        at: u64,
    ) -> Result<Permit, GrantError> {
        let mut permit = self.decide(directory, request, evaluator, at)?;
        permit.use_event = Some(self.record_use(request.caller, permit.grant, request.route, at));
        Ok(permit)
    }

    /// Refuse while an append the question depends on is uncertain.
    fn unresolved_for(&self, question: &Question<'_>) -> Result<(), GrantError> {
        let Some(held) = self.ledger.uncertain() else {
            return Ok(());
        };
        let unresolved = || GrantError::OperationUnresolved {
            operation: held.operation.to_string(),
            grant: held.grant.to_string(),
        };
        match held.event.change() {
            GrantChange::Issue(grant)
                if grant.holder() == question.subject && grant.resource() == question.resource =>
            {
                Err(unresolved())
            }
            GrantChange::Revoke { grant, .. } => {
                let on_path = candidates(&self.book, question).any(|candidate| {
                    self.book
                        .lineage(candidate.id())
                        .is_ok_and(|lineage| lineage.path.contains(grant))
                });
                if on_path { Err(unresolved()) } else { Ok(()) }
            }
            GrantChange::Issue(_) | GrantChange::Use { .. } => Ok(()),
        }
    }
}
