//! A start request, and the enduring agent record it resolves to.
//!
//! A start request names exactly three members: the agent, the profile
//! version and the machine. A member beside them, the executable and the
//! working directory included, is refused as `start_field_not_allowed`; a
//! missing one as `start_member_missing`. The executable and the working
//! directory come only from the reviewed profile version.
//!
//! The agent is resolved, before anything else runs, to the registered
//! agent record the provision brief DIRECTORY-011 keeps: its enduring
//! identity. With no record the start is refused as `agent_unknown` and
//! nothing is written, because a start never creates an agent. The record
//! is only read: it is never changed and never copied.

use crate::start::error::Refusal;

/// The three members a start request names, in order.
pub const MEMBERS: [&str; 3] = ["agent", "profile_version", "machine"];

/// A start request: the agent, the profile version and the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequest {
    agent: String,
    profile_version: String,
    machine: String,
}

impl StartRequest {
    /// The request naming these three.
    pub fn new(agent: &str, profile_version: &str, machine: &str) -> Self {
        Self {
            agent: agent.to_owned(),
            profile_version: profile_version.to_owned(),
            machine: machine.to_owned(),
        }
    }

    /// Parse a request from its members, each a name and a value. Any member
    /// beside the three is refused first, naming it; then any of the three
    /// that is absent or empty.
    pub fn from_members<I>(members: I) -> Result<Self, Refusal>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut values: [Option<String>; 3] = [None, None, None];
        for (name, value) in members {
            let Some(slot) = MEMBERS.iter().position(|member| *member == name) else {
                return Err(Refusal::StartFieldNotAllowed { member: name });
            };
            if let Some(held) = values.get_mut(slot) {
                *held = Some(value).filter(|text| !text.is_empty());
            }
        }
        let [agent, profile_version, machine] = values;
        let present = |value: Option<String>, member: &'static str| {
            value.ok_or(Refusal::StartMemberMissing { member })
        };
        Ok(Self {
            agent: present(agent, MEMBERS[0])?,
            profile_version: present(profile_version, MEMBERS[1])?,
            machine: present(machine, MEMBERS[2])?,
        })
    }

    /// The agent named.
    pub fn agent(&self) -> &str {
        &self.agent
    }

    /// The profile version named.
    pub fn profile_version(&self) -> &str {
        &self.profile_version
    }

    /// The machine named.
    pub fn machine(&self) -> &str {
        &self.machine
    }
}

/// An agent's registered record, as the start reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRecord {
    /// The enduring agent id every record a start keeps points at.
    pub id: String,
    /// The person responsible for the agent, when the record names one.
    pub responsible: Option<String>,
}

/// The agent records DIRECTORY-011 keeps: read, never written.
pub trait AgentRecords {
    /// The registered record of `agent`, or `None` when none exists.
    fn agent(&self, agent: &str) -> Option<AgentRecord>;
}

/// Resolve `agent` to its enduring record, or refuse it as `agent_unknown`.
pub fn resolve(records: &dyn AgentRecords, agent: &str) -> Result<AgentRecord, Refusal> {
    records.agent(agent).ok_or_else(|| Refusal::AgentUnknown {
        agent: agent.to_owned(),
    })
}
