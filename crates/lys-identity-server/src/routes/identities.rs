//! Directory identity registration, profiles, transitions and login binding.
use super::{Shared, identity_id, malformed, people_giving, signed_in, with_directory};
use crate::directory_views::{
    AgentRegistered, IdentitiesView, IdentityRecordView, PersonRegistered, ReceiptAnswer,
    receipt_view, record_view,
};
use crate::error::ServerError;
use crate::session::now;
use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{OriginalUri, Path, State};
use axum::http::HeaderMap;
use lys_identity::{IdentityId, LoginBinding, OperationId, PersonId, Profile, Transition};
use serde::Deserialize;
use std::str::FromStr;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamedBody)]
pub(crate) struct Named {
    pub(super) operation: String,
    pub(super) display_name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AgentRegistrationBody)]
pub(crate) struct AgentRegistration {
    pub(super) operation: String,
    pub(super) display_name: String,
    #[serde(default)]
    pub(super) answers_to: Option<String>,
}

fn registration_target(
    own: PersonId,
    requested: Option<&str>,
    may_choose: bool,
) -> Result<IdentityId, ServerError> {
    let target = requested.map_or(Ok(IdentityId::Person(own)), identity_id)?;
    if target != IdentityId::Person(own) && !may_choose {
        return Err(ServerError::NotAdmitted {
            reason: "only an administrator may register an agent under another person",
        });
    }
    Ok(target)
}

fn operation(text: &str) -> Result<OperationId, ServerError> {
    OperationId::from_str(text).map_err(ServerError::from)
}

pub(crate) async fn register_person(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Named>,
) -> Result<Json<PersonRegistered>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let (op, profile) = (
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    // The caller holds organisation.admit on their own organisation
    // (ACCESS-009 R1), exercised under the hold the write takes; the new
    // person belongs to that organisation (DIRECTORY-109 R1), never to one
    // the caller names.
    crate::grants::with_directory_grants(
        &state,
        |mut judged| {
            crate::people_admission::organisation(
                &mut judged,
                &actor,
                None,
                crate::grants::Decision::Exercise,
            )
        },
        |directory, organisation| {
            let (id, receipt) =
                directory.register_person(actor.clone(), op, profile, organisation, now())?;
            Ok(Json(PersonRegistered {
                person: id.to_string(),
                receipt: receipt_view(&receipt),
                own_computer_unnamed: None,
            }))
        },
    )
}

pub(crate) async fn register_agent(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<AgentRegistration>,
) -> Result<Json<AgentRegistered>, ServerError> {
    // An agent's own signed call, admitted at the door (AGENTS-015 R1).
    if let Some(admitted) = crate::agent_acts::admitted() {
        return super::identities_agent::register(&state, admitted.agent, &body).map(Json);
    }
    let app_actor = crate::provider::app_person_actor(&state, &headers)?;
    if app_actor.is_some()
        || !headers.contains_key(axum::http::header::AUTHORIZATION)
        || crate::mcp_oauth::native_actor(&state, &headers)?.is_some()
        || crate::provider::personal_actor(&state, &headers)?.is_some()
    {
        let app_pass = app_actor.is_some();
        let actor = match app_actor {
            Some(actor) => actor,
            None => signed_in(&state, &headers)?,
        };
        return with_directory(&state, |directory| {
            let projection = directory.projection()?;
            let administrator =
                !app_pass && state.admission.is_administrator(projection, &actor)?;
            let own = if administrator {
                match crate::caller_admission::active_caller(projection, &actor)? {
                    IdentityId::Person(person) => person,
                    IdentityId::Agent(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_)
                    | IdentityId::Machine(_)
                    | IdentityId::Organisation(_) => {
                        return Err(ServerError::NotAdmitted {
                            reason: "an agent registration requires a person or an admitted service account",
                        });
                    }
                }
            } else {
                crate::caller_admission::admitted_person(projection, &actor)?
            };
            let (op, profile) = (
                operation(&body.operation)?,
                Profile::new(&body.display_name)?,
            );
            let target = registration_target(own, body.answers_to.as_deref(), administrator)?;
            crate::agent_policy_api::with_policies(&state, |policies| {
                let answer =
                    directory.register_reporting_agent(actor, op, target, profile, now())?;
                policies.ensure_default(&answer.agent.to_string())?;
                crate::reporting_api::registered(&answer).map(Json)
            })
        });
    }
    let (op, profile) = (
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    crate::grants_reads::with_read_directory_grants(
        &state,
        None,
        now(),
        |judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, judged)?;
            Ok(vec![lys_identity::grants::ExerciseRequest {
                caller,
                route: lys_identity::grants::Route::Api,
                resource: lys_identity::grants::Resource::new("directory", "agents")?,
                action: lys_identity::grants::Action::new("edit")?,
            }])
        },
        |mut judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
            crate::service_account_grants::admit(&mut judged, caller, "agents")?;
            let (actor, responsible) = crate::service_account_grants::actor(&judged, caller)?;
            let target = registration_target(responsible, body.answers_to.as_deref(), false)?;
            Ok((actor, target))
        },
        |directory, (actor, target)| {
            crate::agent_policy_api::with_policies(&state, |policies| {
                let answer =
                    directory.register_reporting_agent(actor, op, target, profile, now())?;
                policies.ensure_default(&answer.agent.to_string())?;
                crate::reporting_api::registered(&answer).map(Json)
            })
        },
    )
}

pub(crate) async fn list(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<IdentitiesView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        // Within the administrator's organisation (DIRECTORY-109 R4).
        let scope = crate::caller_admission::administrators_organisation(projection, &actor);
        let identities = projection
            .records()
            .filter(|(id, _)| scope.is_some() && projection.organisation_of(**id) == scope)
            .map(|(id, record)| record_view(*id, record))
            .collect::<Vec<_>>();
        Ok(Json(IdentitiesView { identities }))
    })
}

pub(crate) async fn read(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<IdentityRecordView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let id = identity_id(&id)?;
    with_directory(&state, |directory| {
        let unknown = || lys_identity::IdentityError::IdentityUnknown {
            identity: id.to_string(),
        };
        // An identity of another organisation is answered exactly as one
        // the directory does not hold (DIRECTORY-109 R4).
        let projection = directory.projection()?;
        let scope = crate::caller_admission::administrators_organisation(projection, &actor);
        if scope.is_none() || projection.organisation_of(id) != scope {
            return Err(unknown().into());
        }
        let record = directory.record(id)?.ok_or_else(unknown)?;
        Ok(Json(record_view(id, &record)))
    })
}

pub(crate) async fn change_profile(
    State(state): State<Shared>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(id): Path<String>,
    principal: Option<axum::Extension<crate::agent_signature::TokenPrincipal>>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let bytes = body.map_err(|error| malformed(error.body_text()))?;
    let actor = people_giving::actor(
        &state,
        &headers,
        ("POST", uri.path(), &bytes),
        principal.as_ref().map(|value| &value.0),
    )?;
    if actor.provenance().agent().is_none() {
        crate::routes::administrator(&state, &actor)?;
    }
    let body = people_giving::body(headers, bytes).await?;
    let (id, op, profile) = (
        identity_id(&id)?,
        operation(&body.operation)?,
        Profile::new(&body.display_name)?,
    );
    if actor.provenance().agent().is_some() {
        return people_giving::profile(&state, &actor, id, op, profile).map(Json);
    }
    with_directory(&state, |directory| {
        let receipt = directory.change_profile(actor, op, id, profile, now())?;
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = TransitionBody)]
pub(crate) struct Moved {
    pub(super) operation: String,
    pub(super) transition: String,
    #[serde(default)]
    pub(super) reason: String,
}

pub(crate) async fn transition(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Moved>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let app_actor = crate::provider::app_person_actor(&state, &headers)?;
    let app_pass = app_actor.is_some();
    let actor = match app_actor {
        Some(actor) => actor,
        None => signed_in(&state, &headers)?,
    };
    if app_pass && body.transition != "activate" {
        return Err(ServerError::NotAdmitted {
            reason: "a person's app pass may only activate an own registered agent",
        });
    }
    let moved = match body.transition.as_str() {
        "activate" => Transition::Activate,
        "suspend" => Transition::Suspend,
        "reinstate" => Transition::Reinstate,
        "retire" => Transition::Retire,
        other => return Err(malformed(format!("{other} is not a transition"))),
    };
    let (id, op) = (identity_id(&id)?, operation(&body.operation)?);
    if !app_pass && !crate::routes::is_administrator(&state, &actor)? {
        if let (IdentityId::Agent(agent), Transition::Activate) = (id, moved) {
            crate::routes::responsible_or_administrator(&state, &headers, &agent.to_string())?;
        } else {
            return Err(ServerError::NotAdmitted {
                reason: "only the administrator changes this identity's state",
            });
        }
    }
    with_directory(&state, |directory| {
        if app_pass {
            let projection = directory.projection()?;
            let person = crate::read_api::own_person(projection, &actor)?;
            let record = projection
                .record(id)
                .filter(|record| {
                    matches!(id, IdentityId::Agent(_)) && record.responsible() == Some(person)
                })
                .ok_or(ServerError::NotAdmitted {
                    reason: "a person's app pass may only activate an own registered agent",
                })?;
            if record.state() != lys_identity::LifecycleState::Registered {
                return Err(lys_identity::IdentityError::TransitionRefused {
                    transition: moved,
                    from: record.state(),
                }
                .into());
            }
        }
        let receipt = directory.transition(actor, op, id, moved, &body.reason, now())?;
        if matches!(moved, Transition::Suspend | Transition::Retire) {
            let projection = directory.projection()?;
            state.sessions.end_matching(|actor| {
                projection
                    .person_for(actor.binding())
                    .map(IdentityId::Person)
                    == Some(id)
                    || projection.agent_for(actor.binding()).map(IdentityId::Agent) == Some(id)
                    || actor.provenance().agent().map(IdentityId::Agent) == Some(id)
            })?;
        }
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = BindLoginBody)]
pub(crate) struct Bound {
    pub(super) operation: String,
    pub(super) issuer: String,
    pub(super) subject: String,
}

pub(crate) async fn bind_login(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Bound>,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let person = PersonId::from_str(&id)?;
    let (op, binding) = (
        operation(&body.operation)?,
        LoginBinding::new(&body.issuer, &body.subject)?,
    );
    with_directory(&state, |directory| {
        let receipt = directory.bind_login(actor, op, person, binding, now())?;
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}
