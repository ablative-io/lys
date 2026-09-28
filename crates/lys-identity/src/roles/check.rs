//! The one seam every role-version check is answered through.
//!
//! A check asks which capacity an actor holds for a holding or a role:
//! [`Capacity::ResponsiblePerson`] when the actor is the person the agent's
//! registration record names as responsible for it, the value every grant
//! the agent holds carries in its responsible field;
//! [`Capacity::ProjectOwner`] when the actor holds the owner relation on the
//! role's project; and neither otherwise. The owner test is the one
//! ownership test for assign, move, the policy change and the role editor,
//! and it reads only the owner relation: never a name, a label, a rank or
//! any other grant.
//!
//! [`RecordCheck`] is the seam's one implementation until the `SpiceDB`
//! evaluator is on the main branch: it answers from the registration record
//! and, for the owner relation, from the grants' current permission
//! relationships. This file is the only file of the roles module that names
//! that module; when the evaluator lands, the identity server supplies an
//! implementation of [`RoleCheck`] that asks it, and no other file of the
//! roles module changes. A seam that cannot answer refuses by the check's
//! name and permits nothing.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use super::error::RoleError;
use super::types::Capacity;
use crate::grants::permission::{ObjectRef, Relationship, RelationshipStore, confirm};
use crate::grants::types::OWNER_RELATION;
use crate::grants::{GrantId, Resource};
use crate::id::{AgentId, IdentityId, PersonId};
use crate::projection::{Projection, Record};

/// The checks a role act asks the seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Check {
    /// Give the owner relation on a project.
    GiveOwner,
    /// Assign a role to an agent.
    Assign,
    /// Make a role.
    MakeRole,
    /// Change a role's grant templates.
    EditTemplates,
    /// Change a role's title.
    ChangeTitle,
    /// Change a role's default move policy.
    ChangeDefault,
    /// Move a holding to a newer version.
    Move,
    /// Renew a holding.
    Renew,
    /// Change one holding's move policy.
    ChangePolicy,
}

impl Check {
    /// The check's name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GiveOwner => "give_owner",
            Self::Assign => "assign",
            Self::MakeRole => "make_role",
            Self::EditTemplates => "edit_templates",
            Self::ChangeTitle => "change_title",
            Self::ChangeDefault => "change_default",
            Self::Move => "move",
            Self::Renew => "renew",
            Self::ChangePolicy => "change_policy",
        }
    }
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What the seam may read: the directory and the grants' permission
/// relationships as they stand.
pub struct Facts<'a> {
    /// The directory's projection, holding each agent's registration record.
    pub directory: &'a Projection,
    /// The grants' permission relationships.
    pub relationships: &'a dyn RelationshipStore,
}

/// One question put to the seam.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The check asked.
    pub check: Check,
    /// The role's project.
    pub project: &'a Resource,
    /// The agent whose holding is acted on, or none for a check about the role itself.
    pub holder: Option<AgentId>,
    /// When the check is made.
    pub at: u64,
}

/// The seam: the capacity an actor holds for a holding or a role.
pub trait RoleCheck {
    /// The capacity `asked.actor` holds for the check, or none.
    fn capacity(
        &self,
        facts: &Facts<'_>,
        asked: &Asked<'_>,
    ) -> Result<Option<Capacity>, RoleError>;

    /// Every person holding the owner relation on `project` at `at`.
    fn owners(
        &self,
        facts: &Facts<'_>,
        check: Check,
        project: &Resource,
        at: u64,
    ) -> Result<Vec<PersonId>, RoleError>;
}

/// The capacity the seam answers, refused `not_permitted` when it answers neither.
pub fn admit(
    check: &dyn RoleCheck,
    facts: &Facts<'_>,
    asked: &Asked<'_>,
) -> Result<Capacity, RoleError> {
    check
        .capacity(facts, asked)?
        .ok_or_else(|| RoleError::NotPermitted {
            actor: asked.actor.to_string(),
            check: asked.check,
        })
}

/// The person the agent's registration record names as responsible for it.
pub fn responsible_for(
    directory: &Projection,
    agent: AgentId,
    check: Check,
) -> Result<PersonId, RoleError> {
    directory
        .record(IdentityId::Agent(agent))
        .and_then(Record::responsible)
        .ok_or_else(|| RoleError::CheckUnavailable {
            check,
            reason: format!("no registration record names the person responsible for {agent}"),
        })
}

/// The seam's implementation from the registration record and the grants'
/// current permission relationships.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecordCheck;

/// The person holding `grant` unexpired at `at` as a root rooted in that
/// person, as the relationships give it.
fn rooted_holder(held: &BTreeSet<Relationship>, grant: &ObjectRef, at: u64) -> Option<PersonId> {
    let id = GrantId::from_str(&grant.id).ok()?;
    confirm(held, &[id], at).ok()?;
    let holder = held.iter().find(|relationship| {
        &relationship.resource == grant
            && relationship.relation == "holder"
            && relationship.subject.kind == "person"
    })?;
    let rooted = held.iter().any(|relationship| {
        &relationship.resource == grant
            && relationship.relation == "source"
            && relationship.subject == holder.subject
    });
    if rooted {
        PersonId::from_str(&holder.subject.id).ok()
    } else {
        None
    }
}

/// Every person holding the owner relation on `project`, in the order the relationships hold them.
fn owners_in(
    facts: &Facts<'_>,
    check: Check,
    project: &Resource,
    at: u64,
) -> Result<Vec<PersonId>, RoleError> {
    let held = facts
        .relationships
        .read()
        .map_err(|error| RoleError::CheckUnavailable {
            check,
            reason: error.to_string(),
        })?;
    let target = ObjectRef::resource(project);
    let mut owners = Vec::new();
    for relationship in &held {
        if relationship.resource == target
            && relationship.relation == OWNER_RELATION
            && relationship.subject.kind == "grant"
            && relationship.subject_relation.as_deref() == Some("holder")
            && let Some(person) = rooted_holder(&held, &relationship.subject, at)
            && !owners.contains(&person)
        {
            owners.push(person);
        }
    }
    Ok(owners)
}

impl RoleCheck for RecordCheck {
    fn capacity(
        &self,
        facts: &Facts<'_>,
        asked: &Asked<'_>,
    ) -> Result<Option<Capacity>, RoleError> {
        let IdentityId::Person(actor) = asked.actor else {
            return Ok(None);
        };
        if let Some(agent) = asked.holder
            && responsible_for(facts.directory, agent, asked.check)? == actor
        {
            return Ok(Some(Capacity::ResponsiblePerson));
        }
        let owners = owners_in(facts, asked.check, asked.project, asked.at)?;
        Ok(owners
            .contains(&actor)
            .then_some(Capacity::ProjectOwner))
    }

    fn owners(
        &self,
        facts: &Facts<'_>,
        check: Check,
        project: &Resource,
        at: u64,
    ) -> Result<Vec<PersonId>, RoleError> {
        owners_in(facts, check, project, at)
    }
}
