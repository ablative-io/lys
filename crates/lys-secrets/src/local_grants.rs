//! Permission held in process: which identity may use which secret, and the
//! person who granted it. It answers the same trait a `SpiceDB` check does.

use std::collections::BTreeMap;
use std::sync::{PoisonError, RwLock};

use crate::permission::{Denied, PermissionCheck, Permitted};

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

/// In-process relations.
#[derive(Debug, Default)]
pub struct LocalGrants {
    relations: RwLock<BTreeMap<(String, String), Option<String>>>,
}

impl LocalGrants {
    /// An empty set of relations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `relation`, replacing any earlier grant of the same pair.
    pub fn grant(&self, relation: SecretRelation) {
        self.relations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((relation.identity, relation.secret), relation.granted_by);
    }

    /// Removes the relation; the next check refuses. Answers whether one was held.
    pub fn revoke(&self, identity: &str, secret: &str) -> bool {
        self.relations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&(identity.to_owned(), secret.to_owned()))
            .is_some()
    }
}

impl PermissionCheck for LocalGrants {
    fn may_use(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        let relations = self
            .relations
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        match relations.get(&(identity.to_owned(), secret.to_owned())) {
            Some(Some(person)) => Ok(Permitted {
                person: person.clone(),
            }),
            Some(None) => Err(Denied {
                reason: "the grant traces to no person".to_owned(),
                no_person_root: true,
            }),
            None => Err(Denied {
                reason: "no use relation".to_owned(),
                no_person_root: false,
            }),
        }
    }
}
