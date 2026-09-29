//! The existing directory visibility predicate, apart from transport or apps.

use std::collections::BTreeSet;

use lys_identity::projection::Projection;
use lys_identity::{IdentityId, PersonId};

/// Select records from the caller's already-held projection. The caller must
/// authenticate the person and determine administrator admission first. This
/// helper grants no authority and adds no lifecycle filter or store overlay.
pub(crate) fn visible(
    directory: &Projection,
    person: PersonId,
    administrator: bool,
) -> BTreeSet<String> {
    let caller = IdentityId::Person(person);
    directory
        .records()
        .filter(|(id, record)| {
            administrator || **id == caller || record.responsible() == Some(person)
        })
        .map(|(id, _)| id.to_string())
        .collect()
}

#[cfg(test)]
#[path = "identity_sight_tests.rs"]
mod tests;
