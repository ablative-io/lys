//! One immutable relationship set, with explicit coverage rather than inferred absence.

use lys_identity::IdentityId;
use lys_identity::grants::{GrantError, ObjectRef, Relationship, RelationshipStore, Resource};
use std::collections::{BTreeMap, BTreeSet};

use super::context::Question;
use super::unavailable;

pub(crate) struct Snapshot {
    revision: u64,
    covered: BTreeSet<Question>,
    objects: BTreeMap<ObjectRef, BTreeSet<Relationship>>,
    subjects: BTreeMap<ObjectRef, BTreeSet<Relationship>>,
}

impl Snapshot {
    pub(crate) fn new(
        revision: u64,
        covered: BTreeSet<Question>,
        rows: BTreeSet<Relationship>,
    ) -> Self {
        let mut objects: BTreeMap<ObjectRef, BTreeSet<Relationship>> = BTreeMap::new();
        let mut subjects: BTreeMap<ObjectRef, BTreeSet<Relationship>> = BTreeMap::new();
        for row in rows {
            objects
                .entry(row.resource.clone())
                .or_default()
                .insert(row.clone());
            subjects.entry(row.subject.clone()).or_default().insert(row);
        }
        Self {
            revision,
            covered,
            objects,
            subjects,
        }
    }

    pub(crate) fn covers(&self, subject: IdentityId, resource: &Resource) -> bool {
        self.covered.contains(&(subject, resource.clone()))
    }
}

impl RelationshipStore for Snapshot {
    fn admit_resource(&self, _: &Resource) -> Result<(), GrantError> {
        Ok(())
    }
    fn revision(&self) -> Result<u64, GrantError> {
        Ok(self.revision)
    }
    fn write(&mut self, _: u64, _: &[Relationship], _: &[Relationship]) -> Result<(), GrantError> {
        Err(unavailable("a relationship snapshot is immutable"))
    }
    fn read_about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        if !self.covers(subject, resource) {
            return Err(unavailable(
                "the resource is outside the relationship batch",
            ));
        }
        lys_identity::grants::about::select(
            subject,
            resource,
            |object| Ok(self.subjects.get(object).cloned().unwrap_or_default()),
            |object| Ok(self.objects.get(object).cloned().unwrap_or_default()),
        )
    }
    fn read_about_at(
        &self,
        subject: IdentityId,
        resource: &Resource,
        revision: u64,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        if revision != self.revision {
            return Err(GrantError::StaleDecision {
                required: revision,
                projected: self.revision,
            });
        }
        self.read_about(subject, resource)
    }
    fn read(&self) -> Result<BTreeSet<Relationship>, GrantError> {
        Ok(self
            .objects
            .values()
            .flat_map(|rows| rows.iter().cloned())
            .collect())
    }
}
