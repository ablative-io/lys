//! What each agent did with the acts a person may now give it (AGENTS-019
//! R5), kept as the rest of the book is kept.
//!
//! When the book applies a use whose caller is an agent and which names one
//! of [`ACTED_FOR_THEIR_PERSON`] as the act it exercised, it notes the use
//! here under that agent, in log order. `GET /dashboard` reads an agent's
//! notes from the book directly; nothing walks the grant log to answer it.
//! The snapshot keeps the notes beside the grants (grant state version 7),
//! so a book reopened from a checkpoint reads them and folds only the tail,
//! exactly as it does every grant. A use written before AGENTS-019 R5 names
//! no act, so a state written before the notes existed holds none to miss.
//!
//! Growth: one note for each such act an agent took, a few dozen bytes beside
//! the leaf it names, which the log keeps whole anyway. These are a person's
//! administrative acts (admitting a person, changing an email, issuing a
//! certificate, setting a budget), rare beside the reads and starts an agent
//! makes, which are never noted. Nothing is pruned, because nothing becomes
//! unreachable: R5 lists every such act on the person's dashboard, a
//! withdrawn grant's acts and a retired agent's included, since what was done
//! for the person stays done.

use std::collections::BTreeMap;

use ciborium::Value;

use crate::grants::events::{Exercised, GrantChange, GrantEvent};
use crate::grants::types::{Action, GrantId, Resource};
use crate::id::{AgentId, IdentityId};
use crate::state_value::{
    Unreadable, array, bytes, identity, list, read_fixed, read_identity, read_text, read_uint,
    text, tuple, uint,
};

/// The acts R1 moved from a person's alone to what a person may give an
/// agent (AGENTS-019 R1), `organisation.admit` among them (R2): an agent's
/// use of any of them is listed on its responsible person's dashboard.
pub const ACTED_FOR_THEIR_PERSON: &[&str] = &[
    "agent.certificate.issue",
    "agent.certificate.withdraw",
    "agent.policy.set",
    "budget.set",
    "configuration.zone.set",
    "grant",
    "identity.transition",
    "organisation.administer",
    "organisation.admit",
    "organisation.grant",
    "person.email.set",
    "request.approve",
    "request.decline",
    "role.create",
    "role.holder.assign",
    "role.holder.end",
    "role.holder.move",
    "role.revise",
    "runtime.stop-everything",
    "seat.import",
    "secret.add",
    "secret.drop",
    "secret.recipients",
    "secret.replace",
    "secret.retire",
    "secret.scope",
    "service-account.create",
    "service-account.retire",
    "team.budget.set",
];

/// One act an agent took with a grant, as its use event recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActedUse {
    /// The use event's index in the grant log: its receipt's leaf.
    pub index: u64,
    /// When the use was recorded, in seconds since the Unix epoch.
    pub at: u64,
    /// The grant exercised.
    pub grant: GrantId,
    /// The act, and the resource it was asked on.
    pub exercised: Exercised,
}

/// Every agent's noted uses, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Acted(BTreeMap<AgentId, Vec<ActedUse>>);

impl Acted {
    /// Note `event`, applied at `index`, when it is an agent's use of an act
    /// the person's dashboard lists.
    pub(crate) fn note(&mut self, event: &GrantEvent, index: u64) {
        let (IdentityId::Agent(agent), GrantChange::Use { grant, .. }, Some(exercised)) =
            (event.caller(), event.change(), event.exercised())
        else {
            return;
        };
        if !ACTED_FOR_THEIR_PERSON.contains(&exercised.action.as_str()) {
            return;
        }
        self.0.entry(agent).or_default().push(ActedUse {
            index,
            at: event.recorded_at(),
            grant: *grant,
            exercised: exercised.clone(),
        });
    }

    /// `agent`'s noted uses, oldest first.
    pub(crate) fn of(&self, agent: AgentId) -> &[ActedUse] {
        self.0.get(&agent).map(Vec::as_slice).unwrap_or_default()
    }

    /// Whether nothing is noted, so the state is written as before the notes.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The notes as a state value: each agent with its uses, each use its
    /// index, time, grant, action, resource kind and resource id.
    pub(crate) fn encode(&self) -> Value {
        array(
            self.0
                .iter()
                .map(|(agent, uses)| {
                    array(vec![
                        identity(IdentityId::Agent(*agent)),
                        array(
                            uses.iter()
                                .map(|one| {
                                    array(vec![
                                        uint(one.index),
                                        uint(one.at),
                                        bytes(one.grant.as_bytes()),
                                        text(one.exercised.action.as_str()),
                                        text(one.exercised.resource.kind()),
                                        text(one.exercised.resource.id()),
                                    ])
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        )
    }

    /// The notes a state value holds.
    pub(crate) fn decode(value: Value) -> Result<Self, Unreadable> {
        let mut acted = BTreeMap::new();
        for held in list(value, "the acted uses")? {
            let [agent, uses] = tuple::<2>(held, "an agent's acted uses")?;
            let IdentityId::Agent(agent) = read_identity(agent)? else {
                return Err("acted uses are noted only for an agent".to_owned());
            };
            let mut noted = Vec::new();
            for one in list(uses, "an agent's acted uses")? {
                let [index, at, grant, action, kind, id] = tuple::<6>(one, "an acted use")?;
                let refused = |error: crate::grants::GrantError| error.to_string();
                noted.push(ActedUse {
                    index: read_uint(&index, "an acted use's index")?,
                    at: read_uint(&at, "an acted use's time")?,
                    grant: GrantId::from_bytes(read_fixed(grant, "an acted use's grant")?),
                    exercised: Exercised {
                        action: Action::new(&read_text(action, "an acted use's action")?)
                            .map_err(refused)?,
                        resource: Resource::new(
                            &read_text(kind, "an acted use's resource kind")?,
                            &read_text(id, "an acted use's resource id")?,
                        )
                        .map_err(refused)?,
                    },
                });
            }
            if acted.insert(agent, noted).is_some() {
                return Err(format!("agent {agent}'s acted uses are recorded twice"));
            }
        }
        Ok(Self(acted))
    }
}

#[cfg(test)]
#[path = "projection_acted_tests.rs"]
mod tests;
