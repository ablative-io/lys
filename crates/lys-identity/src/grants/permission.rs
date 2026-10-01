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

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use super::error::GrantError;
use super::types::{Grant, GrantId, Resource, Source};
use crate::id::IdentityId;

/// The permission model the relationships follow, in the `SpiceDB` schema
/// language. Each resource kind is defined beside these with one relation per
/// model relation, each of subject type `grant#holder`.
pub const SCHEMA: &str = "\
caveat unexpired(now uint, ends_at uint) {
  now < ends_at
}

definition person {}

definition agent {}

definition service_account {}

definition grant {
  relation holder: person | person with unexpired | agent | agent with unexpired | service_account | service_account with unexpired
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
            IdentityId::ServiceAccount(_) => "service_account",
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

/// The relation a resource of an app kind is placed in its parent by: the
/// schema's `parent_` and the parent kind's name within its app, so each
/// parent kind of a kind has a relation of its own.
pub fn parent_relation(parent_kind: &str) -> String {
    let local = parent_kind
        .split_once('.')
        .map_or(parent_kind, |(_, local)| local);
    format!("{}{local}", super::schema::PARENT_RELATION)
}

/// The relationship placing `child` in `parent`, through which the relations
/// held on the parent flow down to the child. It carries no grant and no end:
/// it says where a resource is, never who may act on it.
pub fn placement(child: &Resource, parent: &Resource) -> Relationship {
    Relationship {
        resource: ObjectRef::resource(child),
        relation: parent_relation(parent.kind()),
        subject: ObjectRef::resource(parent),
        subject_relation: None,
        ends_at: None,
    }
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
        Ok(self
            .state
            .lock()
            .map_err(|error| GrantError::PermissionEngineUnavailable {
                reason: format!("permission relationships lock poisoned: {error}"),
            })?
            .0)
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[Relationship],
        delete: &[Relationship],
    ) -> Result<(), GrantError> {
        let mut state =
            self.state
                .lock()
                .map_err(|error| GrantError::PermissionEngineUnavailable {
                    reason: format!("permission relationships lock poisoned: {error}"),
                })?;
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
            .map_err(|error| GrantError::PermissionEngineUnavailable {
                reason: format!("permission relationships lock poisoned: {error}"),
            })?
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

#[cfg(test)]
mod poison_tests {
    use super::{GrantError, MemoryRelationships, RelationshipStore};

    #[test]
    fn poisoned_permission_projection_refuses_reads_and_writes_until_rebuilt() {
        let mut engine = MemoryRelationships::default();
        assert!(engine.write(1, &[], &[]).is_ok());
        let poisoned = engine.clone();
        let panic = std::panic::catch_unwind(move || {
            let mut state = match poisoned.state.lock() {
                Ok(state) => state,
                Err(error) => panic!("fixture_lock_poisoned: {error}"),
            };
            state.0 = 2;
            panic!("injected partial permission update");
        });
        assert!(panic.is_err());
        let unavailable = |error| {
            assert!(
                matches!(error, GrantError::PermissionEngineUnavailable { reason }
                if reason.contains("lock poisoned"))
            );
        };
        unavailable(match engine.revision() {
            Err(error) => error,
            Ok(revision) => panic!("poisoned revision was exposed: {revision}"),
        });
        unavailable(match engine.read() {
            Err(error) => error,
            Ok(held) => panic!("poisoned relationships were exposed: {held:?}"),
        });
        unavailable(match engine.write(3, &[], &[]) {
            Err(error) => error,
            Ok(()) => panic!("poisoned relationships accepted another write"),
        });
        let mut rebuilt = MemoryRelationships::default();
        assert!(rebuilt.write(1, &[], &[]).is_ok());
        assert_eq!(rebuilt.revision(), Ok(1));
    }
}
