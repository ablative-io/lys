//! Team membership limits a granted agent's write without granting authority.

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::FromRequest;
use axum::http::{HeaderMap, Request};
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{Actor, AgentId, IdentityId, Provenance};
use serde::de::DeserializeOwned;

use crate::error::ServerError;
use crate::error_holding::HoldingError;
use crate::error_team::TeamError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::read_api::login;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::teams_api::{TeamChanged, kept, malformed, with_teams};
use crate::teams_nesting::CreatedV1;
use crate::teams_state::{Changed, Created, Line, Team};

pub(crate) fn actor(
    state: &AppState,
    headers: &HeaderMap,
    request: (&str, &str, &[u8]),
    principal: Option<&crate::agent_signature::TokenPrincipal>,
) -> Result<Actor, ServerError> {
    if !headers.contains_key(crate::agent_signature::HEADER) && principal.is_none() {
        return signed_in(state, headers);
    }
    if headers.contains_key(axum::http::header::COOKIE) {
        return Err(ServerError::AgentSignatureRefused {
            reason: "a signed agent request cannot also carry a session cookie",
        });
    }
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let agent = crate::agent_signature::signed_agent_with_token(
            state, projection, headers, request, principal,
        )?
        .ok_or(ServerError::NoPerson)?;
        let person = projection
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .and_then(|person| projection.record(IdentityId::Person(person)))
            .ok_or(ServerError::NoPerson)?;
        let binding = person.bindings().first().ok_or(ServerError::NoPerson)?;
        Ok(Actor::new(
            binding.clone(),
            Provenance::by_agent(agent, now()),
        ))
    })
}

pub(crate) async fn json<T: DeserializeOwned>(
    headers: HeaderMap,
    body: Bytes,
) -> Result<T, ServerError> {
    let mut request = Request::new(Body::from(body));
    *request.headers_mut() = headers;
    Json::<T>::from_request(request, &())
        .await
        .map(|Json(body)| body)
        .map_err(|refused| malformed(refused.body_text()))
}

pub(crate) fn holds(team: &Team, agent: AgentId) -> Result<(), ServerError> {
    let agent = agent.to_string();
    if team.retired.is_none()
        && team.members.iter().any(|member| member == &agent)
        && !team.held.iter().any(|member| member.member == agent)
    {
        return Ok(());
    }
    Err(HoldingError::NotHeld {
        giver: agent,
        reason: format!("no admitted membership in active team {}", team.created.id),
    }
    .into())
}

fn team_request(
    judged: &mut Judged<'_>,
    agent: AgentId,
    team: &str,
) -> Result<ExerciseRequest, ServerError> {
    judged.apps.admit_kind(None, "team")?;
    judged.apps.admit_action("team", "write")?;
    Ok(ExerciseRequest {
        caller: IdentityId::Agent(agent),
        route: Route::Api,
        resource: Resource::new("team", team)?,
        action: Action::new("write")?,
    })
}

fn member_request(
    judged: &mut Judged<'_>,
    agent: AgentId,
    member: &str,
) -> Result<ExerciseRequest, ServerError> {
    use std::str::FromStr;

    if let Err(agent_error) = AgentId::from_str(member) {
        lys_identity::PersonId::from_str(member).map_err(|person_error| {
            malformed(format!(
                "member is neither an agent nor a person: {agent_error}; {person_error}"
            ))
        })?;
        return Err(ServerError::NotAdmitted {
            reason: "a person joins a team only by their own act or the administrator's",
        });
    }
    crate::teams_api::member_holds(judged.directory, member)?;
    judged.apps.admit_kind(None, "agent")?;
    judged
        .apps
        .admit_action("agent", crate::runner_sessions::OPERATE)?;
    Ok(ExerciseRequest {
        caller: IdentityId::Agent(agent),
        route: Route::Api,
        resource: Resource::new("agent", member)?,
        action: Action::new(crate::runner_sessions::OPERATE)?,
    })
}

pub(crate) fn add(
    state: &AppState,
    actor: &Actor,
    changed: Changed,
) -> Result<TeamChanged, ServerError> {
    let agent = actor.provenance().agent().ok_or(ServerError::NoPerson)?;
    crate::teams_migration::advance(state)?;
    with_grants(state, |mut judged| {
        crate::caller_admission::active_caller(judged.directory, actor)?;
        let team_request = team_request(&mut judged, agent, &changed.team)?;
        decide(
            &mut judged,
            &team_request,
            changed.at,
            None,
            Decision::Explain,
        )?;
        with_teams(state, |store| {
            let team = store.team(&changed.team).ok_or(TeamError::Unknown)?;
            holds(team, agent)?;
            let member_request = member_request(&mut judged, agent, &changed.member)?;
            decide(
                &mut judged,
                &member_request,
                changed.at,
                None,
                Decision::Explain,
            )?;
            decide(
                &mut judged,
                &team_request,
                changed.at,
                None,
                Decision::Exercise,
            )?;
            decide(
                &mut judged,
                &member_request,
                changed.at,
                None,
                Decision::Exercise,
            )?;
            kept(store, Line::Added(changed))
        })
    })
}

pub(crate) fn create(
    state: &AppState,
    actor: &Actor,
    id: String,
    name: String,
    description: String,
    parent: Option<String>,
    lead: Option<String>,
) -> Result<TeamChanged, ServerError> {
    let agent = actor.provenance().agent().ok_or(ServerError::NoPerson)?;
    let parent = parent.ok_or_else(|| HoldingError::NotHeld {
        giver: agent.to_string(),
        reason: "an agent cannot create a top-level team".to_owned(),
    })?;
    crate::teams_migration::advance(state)?;
    let at = now();
    with_grants(state, |mut judged| {
        crate::caller_admission::active_caller(judged.directory, actor)?;
        let owner = judged
            .directory
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::NoPerson)?;
        let request = team_request(&mut judged, agent, &parent)?;
        decide(&mut judged, &request, at, None, Decision::Explain)?;
        with_teams(state, |store| {
            let team = store.team(&parent).ok_or(TeamError::Unknown)?;
            holds(team, agent)?;
            if let Some(lead) = lead {
                return Err(TeamError::LeadNotMember { team: id, lead }.into());
            }
            let line = Line::CreatedV1(CreatedV1 {
                created: Created {
                    id,
                    owner: owner.to_string(),
                    name,
                    description,
                    by: login(actor.binding()),
                    at,
                },
                parent: Some(parent),
                lead: None,
            });
            decide(&mut judged, &request, at, None, Decision::Exercise)?;
            kept(store, line)
        })
    })
}
