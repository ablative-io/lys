//! Expiry: every grant ends no later than every grant it derives from, and
//! the end is enforced when a grant is exercised or passed on, not only shown.
//!
//! The earliest end on a grant's ancestry is the one that binds it, and a
//! decision at or after it is refused by the name of the grant that ends
//! there. A new model version, or a relation carrying more actions, changes
//! nothing already held: a grant keeps the actions and the end it was issued
//! with, and a longer or wider holding is only ever a new grant, judged again.

use lys_log_store::LeafStore;

use super::authority::Grants;
use super::error::GrantError;
use super::lineage::Lineage;
use super::model::Model;
use super::permission::RelationshipStore;
use super::projection::GrantBook;

/// Refuse an ancestry whose earliest end is at or before `at`, or with a grant
/// on it that has not started by `at`.
pub fn within_window(book: &GrantBook, lineage: &Lineage, at: u64) -> Result<(), GrantError> {
    if let Some((ended_at, by)) = lineage.ends
        && at >= ended_at
    {
        return Err(GrantError::Expired {
            grant: by.to_string(),
            ended_at,
        });
    }
    for id in &lineage.path {
        let held = book.grant(*id).ok_or_else(|| GrantError::SourceUnknown {
            grant: id.to_string(),
        })?;
        if at < held.window().starts_at() {
            return Err(GrantError::NotStarted {
                grant: id.to_string(),
                starts_at: held.window().starts_at(),
            });
        }
    }
    Ok(())
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Judge new requests against `model` from now on. Its version must be
    /// newer than the current one. Grants already held keep their actions and
    /// their ends.
    pub fn set_model(&mut self, model: Model) -> Result<(), GrantError> {
        if model.version() <= self.model.version() {
            return Err(GrantError::ModelInvalid {
                reason: "a new model version is newer than the one it replaces",
            });
        }
        self.model = model;
        Ok(())
    }
}
