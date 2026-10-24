#![cfg(test)]
//! The engine's schema admits every subject the model writes as a grant's
//! source: another grant, the person giving from the install's standing and,
//! since 24 October 2026, an agent giving from its organisation's standing.
//! The first agent-issued grant jammed a live projection while the schema
//! named no agent there: every write after it was committed and never
//! projected.

use std::error::Error;

use crate::grants::{
    Action, Grant, GrantId, GrantParts, PassOn, Relation, Resource, Source, Window,
    permission::{SCHEMA, relationships_of},
};
use crate::{AgentId, IdentityId, OperationId, PersonId};

/// A grant from administrator standing, issued by `issuer`.
fn from_standing(issuer: IdentityId) -> Result<Grant, Box<dyn Error>> {
    Ok(Grant::new(GrantParts {
        id: GrantId::from_bytes([7; 16]),
        issuer,
        holder: IdentityId::Agent(AgentId::from_bytes([8; 16])),
        responsible: PersonId::from_bytes([1; 16]),
        resource: Resource::new("notes.workspace", "team")?,
        relation: Relation::new("member")?,
        actions: [Action::new("read")?].into(),
        pass_on: PassOn::UseOnly,
        source: Source::Administrator,
        window: Window::new(0, None)?,
        model_version: 3,
        operation: OperationId::from_bytes([9; 16]),
    })?)
}

/// The subject kinds the schema allows on `grant#source`.
fn source_subjects() -> Vec<String> {
    SCHEMA
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("relation source:"))
        .map(|kinds| {
            kinds
                .split('|')
                .map(|kind| kind.trim().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_schema_admits_every_source_the_model_writes() -> Result<(), Box<dyn Error>> {
    let subjects = source_subjects();
    assert!(subjects.contains(&"grant".to_owned()), "{subjects:?}");
    for (issuer, kind) in [
        (IdentityId::Person(PersonId::from_bytes([1; 16])), "person"),
        (IdentityId::Agent(AgentId::from_bytes([2; 16])), "agent"),
    ] {
        let grant = from_standing(issuer)?;
        let source = relationships_of(&grant)
            .into_iter()
            .find(|relationship| relationship.relation == "source")
            .ok_or("no source relationship")?;
        assert_eq!(source.subject.kind, kind);
        assert!(
            subjects.contains(&source.subject.kind),
            "{kind} is written as a grant's source and the schema does not admit it: {subjects:?}"
        );
    }
    Ok(())
}
