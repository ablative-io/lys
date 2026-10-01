//! Signed profile writes require a live person grant and admitted shared membership.

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::FromRequest;
use axum::http::{HeaderMap, Request, header};
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{Actor, AgentId, IdentityId, OperationId, PersonId, Profile, Provenance};

use crate::directory_views::{ReceiptAnswer, receipt_view};
use crate::error::ServerError;
use crate::error_holding::HoldingError;
use crate::error_team::TeamError;
use crate::grants::{Decision, decide, with_directory_grants};
use crate::routes::{AppState, Named, malformed, signed_in, with_directory};
use crate::session::now;
use crate::teams_state::Team;

pub(crate) fn actor(
    state: &AppState,
    headers: &HeaderMap,
    request: (&str, &str, &[u8]),
) -> Result<Actor, ServerError> {
    if !headers.contains_key(crate::agent_signature::HEADER) {
        return signed_in(state, headers);
    }
    if headers.contains_key(header::COOKIE) {
        return Err(ServerError::AgentSignatureRefused {
            reason: "a signed agent request cannot also carry a session cookie",
        });
    }
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let agent = crate::agent_signature::signed_agent(state, projection, headers, request)?
            .ok_or(ServerError::NoPerson)?;
        let binding = projection
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .and_then(|person| projection.record(IdentityId::Person(person)))
            .and_then(|record| record.bindings().first())
            .ok_or(ServerError::NoPerson)?;
        Ok(Actor::new(
            binding.clone(),
            Provenance::by_agent(agent, now()),
        ))
    })
}

pub(super) async fn body(headers: HeaderMap, bytes: Bytes) -> Result<Named, ServerError> {
    let mut request = Request::new(Body::from(bytes));
    *request.headers_mut() = headers;
    Json::<Named>::from_request(request, &())
        .await
        .map(|Json(body)| body)
        .map_err(|error| malformed(error.body_text()))
}

fn admitted(team: &Team, member: &str) -> bool {
    team.retired.is_none()
        && team.members.iter().any(|id| id == member)
        && !team.held.iter().any(|held| held.member == member)
}

pub(crate) fn holds(teams: &[Team], agent: AgentId, person: PersonId) -> Result<(), ServerError> {
    let agent = agent.to_string();
    let person = person.to_string();
    if teams
        .iter()
        .any(|team| admitted(team, &agent) && admitted(team, &person))
    {
        return Ok(());
    }
    Err(HoldingError::NotHeld {
        giver: agent,
        reason: format!("person {person} is not admitted in an active team the agent holds"),
    }
    .into())
}

pub(super) fn profile(
    state: &AppState,
    actor: &Actor,
    id: IdentityId,
    operation: OperationId,
    profile: Profile,
) -> Result<ReceiptAnswer, ServerError> {
    let IdentityId::Person(person) = id else {
        return Err(ServerError::NotAdmitted {
            reason: "a signed agent profile write may name only a person",
        });
    };
    let agent = actor.provenance().agent().ok_or(ServerError::NoPerson)?;
    let at = now();
    with_directory_grants(
        state,
        |mut judged| {
            crate::caller_admission::active_caller(judged.directory, actor)?;
            judged.apps.admit_kind(None, "person")?;
            judged.apps.admit_action("person", "write")?;
            let request = ExerciseRequest {
                caller: IdentityId::Agent(agent),
                route: Route::Api,
                resource: Resource::new("person", &person.to_string())?,
                action: Action::new("write")?,
            };
            decide(&mut judged, &request, at, None, Decision::Explain)?;
            if judged.directory.record(id).is_none() {
                return Err(lys_identity::IdentityError::IdentityUnknown {
                    identity: id.to_string(),
                }
                .into());
            }
            let teams = state.teams.as_ref().ok_or_else(|| TeamError::Unavailable {
                reason: "the configuration names no teams_dir".to_owned(),
            })?;
            let mut teams = teams.lock().map_err(|error| TeamError::Unavailable {
                reason: format!("the teams lock is poisoned: {error}"),
            })?;
            teams.settle()?;
            holds(teams.teams(), agent, person)?;
            decide(&mut judged, &request, at, None, Decision::Exercise)?;
            Ok(teams)
        },
        |directory, teams| {
            let receipt = directory.change_profile(actor.clone(), operation, id, profile, at)?;
            drop(teams);
            Ok(ReceiptAnswer {
                receipt: receipt_view(&receipt),
            })
        },
    )
}
