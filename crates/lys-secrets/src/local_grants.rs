//! Permission held in process: which identity may use or read which secret,
//! and the person who granted it. It answers the same trait a `SpiceDB`
//! check does.

use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::SecretsError;
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

/// One relation as held: who granted it, and when its window ends.
#[derive(Debug, Clone)]
struct Held {
    granted_by: Option<String>,
    ends_at_ms: Option<i64>,
}

/// In-process relations.
#[derive(Debug, Default)]
pub struct LocalGrants {
    relations: RwLock<BTreeMap<Key, Held>>,
}

impl LocalGrants {
    /// An empty set of relations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the `use` relation, replacing any earlier grant of the pair.
    ///
    /// # Errors
    /// Returns `StatePoisoned` when a panic interrupted the grant state.
    pub fn grant(&self, relation: SecretRelation) -> Result<(), SecretsError> {
        self.grant_as(Relation::Use, relation)
    }

    /// Records `kind` for the relation's pair, replacing any earlier grant.
    ///
    /// # Errors
    /// Returns `StatePoisoned` when a panic interrupted the grant state.
    pub fn grant_as(&self, kind: Relation, relation: SecretRelation) -> Result<(), SecretsError> {
        self.grant_until(kind, relation, None)
    }

    /// Records `kind` for the relation's pair as a grant whose window ends
    /// at `ends_at_ms`, replacing any earlier grant. A lease cut under it
    /// may end no later than that.
    ///
    /// # Errors
    /// Returns `StatePoisoned` when a panic interrupted the grant state.
    pub fn grant_until(
        &self,
        kind: Relation,
        relation: SecretRelation,
        ends_at_ms: Option<i64>,
    ) -> Result<(), SecretsError> {
        self.relations
            .write()
            .map_err(|error| SecretsError::StatePoisoned {
                reason: error.to_string(),
            })?
            .insert(
                (relation.identity, relation.secret, kind),
                Held {
                    granted_by: relation.granted_by,
                    ends_at_ms,
                },
            );
        Ok(())
    }

    /// Removes the `use` relation; the next check refuses. Answers whether
    /// one was held.
    ///
    /// # Errors
    /// Returns `StatePoisoned` when a panic interrupted the grant state.
    pub fn revoke(&self, identity: &str, secret: &str) -> Result<bool, SecretsError> {
        self.revoke_as(Relation::Use, identity, secret)
    }

    /// Removes `kind` for the pair. Answers whether one was held.
    ///
    /// # Errors
    /// Returns `StatePoisoned` when a panic interrupted the grant state.
    pub fn revoke_as(
        &self,
        kind: Relation,
        identity: &str,
        secret: &str,
    ) -> Result<bool, SecretsError> {
        Ok(self
            .relations
            .write()
            .map_err(|error| SecretsError::StatePoisoned {
                reason: error.to_string(),
            })?
            .remove(&(identity.to_owned(), secret.to_owned(), kind))
            .is_some())
    }

    fn check(&self, kind: Relation, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        let relations = self.relations.read().map_err(|error| Denied {
            reason: SecretsError::StatePoisoned {
                reason: error.to_string(),
            }
            .to_string(),
            no_person_root: false,
        })?;
        match relations.get(&(identity.to_owned(), secret.to_owned(), kind)) {
            Some(Held {
                granted_by: Some(person),
                ends_at_ms,
            }) => Ok(Permitted {
                person: person.clone(),
                ends_at_ms: *ends_at_ms,
            }),
            Some(Held {
                granted_by: None, ..
            }) => Err(Denied {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_local_grants_refuse_reads_and_changes() -> Result<(), SecretsError> {
        let grants = LocalGrants::new();
        let relation = SecretRelation {
            identity: "agent".to_owned(),
            secret: "secret".to_owned(),
            granted_by: Some("person".to_owned()),
        };
        grants.grant(relation.clone())?;
        let interrupted = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let mut relations = grants
                        .relations
                        .write()
                        .expect("healthy initial grant lock");
                    relations.clear();
                    panic!("interrupted grant mutation");
                })
                .join()
        });
        assert!(interrupted.is_err());
        assert!(matches!(
            grants.grant(relation),
            Err(SecretsError::StatePoisoned { .. })
        ));
        assert!(matches!(
            grants.revoke("agent", "secret"),
            Err(SecretsError::StatePoisoned { .. })
        ));
        let denied = grants
            .may_use("agent", "secret")
            .expect_err("poisoned grant cannot authorise");
        assert!(denied.reason.starts_with("StatePoisoned:"));
        assert!(!denied.no_person_root);
        Ok(())
    }
}
