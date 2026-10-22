//! `acted_by_agents` on `GET /dashboard`: what the caller's agents did for
//! them, newest first.
//!
//! An agent registered by an agent is read from the actor of its
//! registration event in the directory's log, and a start an agent gave from
//! its launch record (AGENTS-016 R5). An act an agent took with a grant a
//! person may now give it (AGENTS-019 R1, R2: admitting a person, changing a
//! person's email, issuing a certificate and the rest) is read from the
//! grant book, which notes each such use as it applies it
//! (`GrantBook::acted_by`): the dashboard never walks the grant log. Each
//! such line names the act, the resource it was taken on, the use's index in
//! the grant log and the act in the shipped sentence's words.

use std::collections::BTreeSet;

use axum::http::HeaderMap;
use lys_identity::AgentId;
use lys_identity::grants::ActedUse;
use lys_identity::grants::shipped::ACTION_SENTENCES;
use serde::{Serialize, Serializer};

use crate::error::ServerError;
use crate::read_api::own_person;
use crate::routes::start::StartService;
use crate::routes::{AppState, signed_in, with_directory};

/// One thing an agent did for the caller (AGENTS-016 R5, AGENTS-019 R5).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActedLine {
    /// `registered` for an agent an agent registered; `started` for a start
    /// of the caller's agent an agent gave; otherwise the act an agent took
    /// with a grant, by its action's name, such as `organisation.admit` or
    /// `person.email.set`.
    #[schema(value_type = String)]
    pub kind: ActedKind,
    /// The agent that acted.
    pub acting_agent: String,
    /// The agent registered or started; for an act, the agent that acted,
    /// the one the line's Withdraw ends.
    pub agent: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
    /// The session a start runs as; absent for a registration or an act.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The resource an act was taken on; absent for a registration or a start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<ActedSubject>,
    /// The index of an act's use event in the grant log, its receipt's
    /// leaf; absent for a registration or a start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
    /// An act in plain words, the shipped sentence for its action; absent
    /// for a registration, a start, or an act the sentences do not name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence: Option<String>,
}

/// The resource an agent's act was taken on.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActedSubject {
    /// The resource's kind.
    pub kind: String,
    /// The resource's id within its kind.
    pub id: String,
}

/// What an agent did: answered as its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActedKind {
    /// It registered the agent: `registered`.
    Registered,
    /// It started the agent: `started`.
    Started,
    /// It took this act with a grant, named by the action.
    Act(String),
}

impl ActedKind {
    /// The kind's name as the dashboard answers it.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Registered => "registered",
            Self::Started => "started",
            Self::Act(action) => action.as_str(),
        }
    }
}

impl Serialize for ActedKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

/// `agent`'s noted use `one`, as a line.
fn act_line(agent: AgentId, one: &ActedUse) -> ActedLine {
    let action = one.exercised.action.as_str();
    ActedLine {
        kind: ActedKind::Act(action.to_owned()),
        acting_agent: agent.to_string(),
        agent: agent.to_string(),
        at: one.at,
        session: None,
        subject: Some(ActedSubject {
            kind: one.exercised.resource.kind().to_owned(),
            id: one.exercised.resource.id().to_owned(),
        }),
        index: Some(one.index),
        sentence: ACTION_SENTENCES
            .iter()
            .find(|(named, _)| *named == action)
            .map(|(_, sentence)| (*sentence).to_owned()),
    }
}

/// Every agent of the caller's an agent registered, from the actor of its
/// registration event; every start of one an agent gave, from its launch
/// record; and every act one of the caller's agents took with a grant a
/// person may now give it, from the grant book's notes; newest first.
pub(super) fn acted(
    state: &AppState,
    headers: &HeaderMap,
    starts: &StartService,
) -> Result<Vec<ActedLine>, ServerError> {
    let actor = signed_in(state, headers)?;
    let (mine, mut lines) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let mut mine = Vec::new();
        for entry in projection.agents_of(person) {
            if let (lys_identity::IdentityId::Agent(agent), _) = entry? {
                mine.push(*agent);
            }
        }
        let mut lines = Vec::new();
        for agent in &mine {
            if let Some((by, at)) = crate::agent_acts::registrar(directory, *agent)? {
                lines.push(ActedLine {
                    kind: ActedKind::Registered,
                    acting_agent: by.to_string(),
                    agent: agent.to_string(),
                    at,
                    session: None,
                    subject: None,
                    index: None,
                    sentence: None,
                });
            }
        }
        Ok((mine, lines))
    })?;
    let acts = crate::grants::with_grants(state, |judged| {
        let book = judged.grants.book();
        Ok(mine
            .iter()
            .flat_map(|agent| {
                book.acted_by(*agent)
                    .iter()
                    .map(move |one| act_line(*agent, one))
            })
            .collect::<Vec<_>>())
    })?;
    lines.extend(acts);
    let mine: BTreeSet<String> = mine.iter().map(ToString::to_string).collect();
    let unavailable = |error: lys_identity::start::StartError| ServerError::RuntimeUnavailable {
        reason: format!("the launch records cannot be read: {error}"),
    };
    let mut launches = starts.launches().map_err(unavailable)?;
    launches.settle().map_err(unavailable)?;
    for record in launches.records() {
        if mine.contains(&record.agent)
            && let Ok(by) = record.given_by.parse::<AgentId>()
        {
            lines.push(ActedLine {
                kind: ActedKind::Started,
                acting_agent: by.to_string(),
                agent: record.agent.clone(),
                at: record.given_at,
                session: Some(crate::runner_sessions::launch_session(&record.id)),
                subject: None,
                index: None,
                sentence: None,
            });
        }
    }
    // Newest first; at the same instant a start follows the registration it
    // needs, and acts follow one another in the grant log's order.
    lines.sort_by_key(|line| {
        std::cmp::Reverse((line.at, line.kind == ActedKind::Started, line.index))
    });
    Ok(lines)
}
