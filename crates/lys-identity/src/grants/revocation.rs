//! Revocation: withdrawing a grant withdraws everything derived from it.
//!
//! A revoked grant is revoked for good. Every grant whose ancestry passes
//! through it is refused by the revoked grant's name at the next decision,
//! and the permission projection deletes every relationship of the revoked
//! grant and of every grant derived from it, so none remains. Grants that do
//! not derive from it, to the same person or on the same resource, are untouched.

use std::collections::BTreeSet;

use lys_log_store::LeafStore;

use super::authority::Grants;
use super::error::GrantError;
use super::lineage::Lineage;
use super::permission::{RelationshipStore, naming};
use super::projection::GrantBook;
use super::types::GrantId;
use crate::id::{IdentityId, PersonId};

/// Refuse, by the revoked grant's name, an ancestry with a revoked grant on it.
pub fn unrevoked(book: &GrantBook, lineage: &Lineage) -> Result<(), GrantError> {
    for id in &lineage.path {
        let record = book.record(*id).ok_or_else(|| GrantError::SourceUnknown {
            grant: id.to_string(),
        })?;
        if record.revoked().is_some() {
            return Err(GrantError::Revoked {
                grant: id.to_string(),
            });
        }
    }
    Ok(())
}

/// Refuse a revocation unless `caller` issued the grant, holds a grant it
/// derives from, or is the root authority.
pub fn judge_revoke(
    book: &GrantBook,
    root_authority: PersonId,
    caller: IdentityId,
    grant: GrantId,
) -> Result<(), GrantError> {
    let record = book.record(grant).ok_or_else(|| GrantError::GrantUnknown {
        grant: grant.to_string(),
    })?;
    if record.revoked().is_some() {
        return Err(GrantError::AlreadyRevoked {
            grant: grant.to_string(),
        });
    }
    if caller == IdentityId::Person(root_authority) || record.grant().parts().issuer == caller {
        return Ok(());
    }
    let lineage = book.lineage(grant)?;
    let holds_ancestor = lineage
        .path
        .iter()
        .skip(1)
        .filter_map(|ancestor| book.grant(*ancestor))
        .any(|ancestor| ancestor.holder() == caller);
    if holds_ancestor {
        Ok(())
    } else {
        Err(GrantError::RevokeRefused {
            caller: caller.to_string(),
            grant: grant.to_string(),
        })
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// How many permission relationships name `grant` or any grant derived from it.
    pub fn relationships_derived_from(&self, grant: GrantId) -> Result<usize, GrantError> {
        let mut tree: BTreeSet<GrantId> = self.book.descendants(grant).into_iter().collect();
        tree.insert(grant);
        Ok(naming(&self.relationships.read()?, &tree).len())
    }
}
