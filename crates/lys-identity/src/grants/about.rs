//! A subject's grants, their source ancestry and a resource's placements.

use std::collections::{BTreeMap, BTreeSet};

use super::{GrantError, ObjectRef, Relationship, Resource};
use crate::IdentityId;

#[derive(Debug, Default)]
pub(super) struct MemoryState {
    pub(super) revision: u64,
    pub(super) held: BTreeSet<Relationship>,
    objects: BTreeMap<ObjectRef, BTreeSet<Relationship>>,
    subjects: BTreeMap<ObjectRef, BTreeSet<Relationship>>,
}

impl MemoryState {
    pub(super) fn remove(&mut self, relationship: &Relationship) {
        self.held.remove(relationship);
        if let Some(rows) = self.objects.get_mut(&relationship.resource) {
            rows.remove(relationship);
        }
        if let Some(rows) = self.subjects.get_mut(&relationship.subject) {
            rows.remove(relationship);
        }
    }

    pub(super) fn insert(&mut self, relationship: Relationship) {
        self.objects
            .entry(relationship.resource.clone())
            .or_default()
            .insert(relationship.clone());
        self.subjects
            .entry(relationship.subject.clone())
            .or_default()
            .insert(relationship.clone());
        self.held.insert(relationship);
    }

    pub(super) fn about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<Relationship>, GrantError> {
        select(
            subject,
            resource,
            |object| Ok(self.subjects.get(object).cloned().unwrap_or_default()),
            |object| Ok(self.objects.get(object).cloned().unwrap_or_default()),
        )
    }
}

/// Read only the holder's grants and their source lineage, and placements above the resource.
pub fn select(
    subject: IdentityId,
    resource: &Resource,
    mut subjects: impl FnMut(&ObjectRef) -> Result<BTreeSet<Relationship>, GrantError>,
    mut objects: impl FnMut(&ObjectRef) -> Result<BTreeSet<Relationship>, GrantError>,
) -> Result<BTreeSet<Relationship>, GrantError> {
    let mut held = BTreeSet::new();
    let holders = subjects(&ObjectRef::identity(subject))?;
    let mut grants: Vec<ObjectRef> = holders
        .iter()
        .filter(|row| row.resource.kind == "grant" && row.relation == "holder")
        .map(|row| row.resource.clone())
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(grant) = grants.pop() {
        if !seen.insert(grant.clone()) {
            continue;
        }
        let rows = objects(&grant)?;
        grants.extend(
            rows.iter()
                .filter(|row| row.relation == "source" && row.subject.kind == "grant")
                .map(|row| row.subject.clone()),
        );
        held.extend(rows);
        held.extend(
            subjects(&grant)?
                .into_iter()
                .filter(|row| row.subject_relation.as_deref() == Some("holder")),
        );
    }
    let mut parents = vec![ObjectRef::resource(resource)];
    let mut seen = BTreeSet::new();
    while let Some(resource) = parents.pop() {
        if !seen.insert(resource.clone()) {
            continue;
        }
        let placements: Vec<Relationship> = objects(&resource)?
            .into_iter()
            .filter(|row| row.relation.starts_with("parent_"))
            .collect();
        parents.extend(placements.iter().map(|row| row.subject.clone()));
        held.extend(placements);
    }
    Ok(held)
}
