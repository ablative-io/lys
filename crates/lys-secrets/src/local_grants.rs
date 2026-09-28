//! Permission held in process: which identity may use or read which secret,
//! and the person who granted it. It answers the same trait a `SpiceDB`
//! check does.

use std::collections::BTreeMap;
use std::sync::{PoisonError, RwLock};

use crate::permission::{Denied, PermissionCheck, Permitted, Relation};

/// One relation: `identity` may use `secret`, granted by `granted_by`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRelation {
    /// The identity that may use the secret.
    pub identity: String,
    /// The secret's name.
    pub secret: String,
    /// The person who granted it, or `None` when no person stands behind it.
    pub granted_by: Option<String>,
}

type Key = (String, String, Relation);

/// In-process relations.
#[derive(Debug, Default)]
pub struct LocalGrants {
    relations: RwLock<BTreeMap<Key, Option<String>>>,
}

impl LocalGrants {
    /// An empty set of relations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the `use` relation, replacing any earlier grant of the pair.
    pub fn grant(&self, relation: SecretRelation) {
        self.grant_as(Relation::Use, relation);
    }

    /// Records `kind` for the relation's pair, replacing any earlier grant.
    pub fn grant_as(&self, kind: Relation, relation: SecretRelation) {
        self.relations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                (relation.identity, relation.secret, kind),
                relation.granted_by,
            );
    }

    /// Removes the `use` relation; the next check refuses. Answers whether
    /// one was held.
    pub fn revoke(&self, identity: &str, secret: &str) -> bool {
        self.revoke_as(Relation::Use, identity, secret)
    }

    /// Removes `kind` for the pair. Answers whether one was held.
    pub fn revoke_as(&self, kind: Relation, identity: &str, secret: &str) -> bool {
        self.relations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&(identity.to_owned(), secret.to_owned(), kind))
            .is_some()
    }

    fn check(&self, kind: Relation, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        let relations = self
            .relations
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        match relations.get(&(identity.to_owned(), secret.to_owned(), kind)) {
            Some(Some(person)) => Ok(Permitted {
                person: person.clone(),
            }),
            Some(None) => Err(Denied {
                reason: "the grant traces to no person".to_owned(),
                no_person_root: true,
            }),
            None => Err(Denied {
                reason: format!("no {} relation", kind.label()),
                no_person_root: false,
            }),
        }
    }
}

impl PermissionCheck for LocalGrants {
    fn may_use(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Use, identity, secret)
    }

    fn may_read(&self, identity: &str, record: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Read, identity, record)
    }

    fn may_lend(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Lend, identity, secret)
    }

    fn member_of(&self, identity: &str, target: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Member, identity, target)
    }
}
