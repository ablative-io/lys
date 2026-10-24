//! An agent's own signed call on the routes that set up, review, start and
//! stop agents and seats, connect a computer's runner, allow agents on a
//! computer and pass grants on (AGENTS-015 R1). Each is admitted beside a
//! person's session and the operator, held to the agent's own live grants.
//!
//! A call reaches one of these routes signed in one of two ways: by the
//! agent's own request signature (`lys-agent-signature`), verified here over
//! the exact request, or relayed from an MCP message the agent signed, whose
//! signature the MCP door verified. Either way it is served as a relayed
//! call for that agent, so `signed_in` names the agent and never a person.
//!
//! A route whose act is named by its path (the table's scope) is judged
//! here: the agent's grants must reach the action on the resource, or on the
//! person the resource's agent answers to, and the use is recorded; anything
//! else is refused `not_permitted`, naming the action and the resource and
//! never a grant the agent could not see. A route whose act depends on its
//! body or on a record (registering an agent, adding a seat, passing a grant
//! on) is judged by its handler through [`require`], and a seat's own acts
//! by the seat's own judgement. Only this door marks a call [`admitted`]: a
//! handler reads that mark to skip the person checks it makes of a session.

use axum::body::{Body, Bytes};
use axum::extract::Request;
use axum::http::{HeaderMap, Method, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use lys_identity::grants::{Action, ExerciseRequest, GrantError, GrantId, Resource, Route};
use lys_identity::{Actor, AgentId, IdentityId, PersonId, Provenance};

use crate::agent_signature::{HEADER, Relay};
use crate::error::ServerError;
use crate::grants::{Decision, decide};
use crate::routes::{AppState, with_directory};
use crate::session::now;

/// Who judges an agent's call on a listed route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Judged {
    /// The door, by the act the route's path names.
    Door,
    /// The handler, by the act its body or its records name.
    Handler,
}

/// The routes an agent's own signed call may reach, as the table writes
/// them, and who judges each.
const ACTS: &[(&str, Judged)] = &[
    ("/agents", Judged::Handler),
    ("/agents/{id}/provisioning", Judged::Door),
    ("/agents/{id}/provisioning/{version}/review", Judged::Door),
    (
        "/agents/{id}/provisioning/{version}/seat-program-review",
        Judged::Door,
    ),
    ("/agents/{id}/start", Judged::Door),
    ("/agents/{id}/start-command", Judged::Door),
    ("/agents/{id}/restart", Judged::Door),
    ("/agents/{id}/stop", Judged::Door),
    ("/agents/{id}/certificates", Judged::Handler),
    (
        "/agents/{id}/certificates/{serial}/withdrawal",
        Judged::Door,
    ),
    ("/seats", Judged::Handler),
    ("/seats/{name}/version", Judged::Handler),
    ("/seats/{name}/start", Judged::Handler),
    ("/seats/{name}/stop", Judged::Handler),
    ("/seats/{name}/restart", Judged::Handler),
    ("/network/machines/{id}/runner", Judged::Door),
    ("/network/machines/{id}/join-code", Judged::Door),
    ("/network/machines/{id}/installer/{kind}", Judged::Door),
    ("/network/machines/{id}/agents", Judged::Door),
    ("/grants", Judged::Handler),
    ("/grants/{id}/revoke", Judged::Handler),
];

/// A grant an agent exercised: the grant and the person at its chain's root.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Exercised {
    /// The grant exercised.
    pub(crate) grant: GrantId,
    /// The person holding the root of its chain.
    pub(crate) root: PersonId,
}

/// An agent's call admitted at the door of a listed route.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Admitted {
    /// The agent calling.
    pub(crate) agent: AgentId,
    /// Who judges the call's act.
    pub(crate) judged: Judged,
    /// The person at the root of the grant the door exercised, when the
    /// door judged the act.
    pub(crate) root: Option<PersonId>,
}

tokio::task_local! {
    static ADMITTED: Admitted;
}

/// The agent whose call this door admitted on the route being served; none
/// for every other caller and every other route.
pub(crate) fn admitted() -> Option<Admitted> {
    ADMITTED.try_with(|admitted| *admitted).ok()
}

/// Who judges an agent's call on `method path`, the route's pattern as the
/// table writes it; none when the route takes no agent's own call here.
pub(crate) fn listed(method: &Method, path: &str) -> Option<Judged> {
    if method != Method::POST {
        return None;
    }
    let path = path.strip_prefix("/api").unwrap_or(path);
    ACTS.iter()
        .find(|(listed, _)| *listed == path)
        .map(|(_, judged)| *judged)
}

/// Serve `request` on a listed route for the agent that signed it or whose
/// MCP message relayed it, judged as `judged` says; refused by name when
/// its signature or its grants do not admit it.
pub(crate) async fn serve(
    state: &AppState,
    judged: Judged,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().as_str().to_owned();
    let path = unprefixed(request.uri().path()).to_owned();
    let (request, agent, relay) = match crate::agent_signature::relayed_agent() {
        Some(agent) => (request, agent, None),
        None => match signed(state, request).await {
            Ok((request, relay)) => {
                let agent = relay.agent;
                (request, agent, Some(relay))
            }
            Err(refusal) => return *refusal,
        },
    };
    let root = match judged {
        Judged::Door => match door(state, agent, &method, &path) {
            Ok(exercised) => Some(exercised.root),
            Err(refusal) => {
                return crate::signed_first::refuse(request, refusal.into_response()).await;
            }
        },
        Judged::Handler => None,
    };
    let work = ADMITTED.scope(
        Admitted {
            agent,
            judged,
            root,
        },
        next.run(request),
    );
    match relay {
        Some(relay) => crate::agent_signature::relayed(relay, work).await,
        None => work.await,
    }
}

fn unprefixed(path: &str) -> &str {
    path.strip_prefix("/api").unwrap_or(path)
}

/// The request with its body read and its signature verified over it, the
/// signature header taken off so nothing judges it twice, and the relay it
/// is served under.
async fn signed(state: &AppState, request: Request) -> Result<(Request, Relay), Box<Response>> {
    if request.headers().contains_key(header::COOKIE)
        || request.headers().contains_key(header::AUTHORIZATION)
    {
        let refusal = ServerError::AgentSignatureRefused {
            reason: "an agent signature cannot carry another credential",
        };
        return Err(Box::new(
            crate::signed_first::refuse(request, refusal.into_response()).await,
        ));
    }
    let (mut parts, body) = request.into_parts();
    let bytes: Bytes = crate::signed_first::bounded(body)
        .await
        .map_err(|refusal| Box::new(refusal.into_response()))?;
    let target = parts
        .uri
        .path_and_query()
        .map_or(parts.uri.path(), |target| target.as_str());
    let target = unprefixed(target).to_owned();
    let agent = verified(
        state,
        &parts.headers,
        (parts.method.as_str(), &target, &bytes),
    )
    .map_err(|refusal| Box::new(refusal.into_response()))?;
    parts.headers.remove(HEADER);
    Ok((
        Request::from_parts(parts, Body::from(bytes)),
        Relay {
            agent,
            signed: None,
        },
    ))
}

fn verified(
    state: &AppState,
    headers: &HeaderMap,
    request: (&str, &str, &[u8]),
) -> Result<AgentId, ServerError> {
    with_directory(state, |directory| {
        crate::agent_signature::signed_agent(state, directory.projection()?, headers, request)?
            .ok_or(ServerError::AgentSignatureRefused {
                reason: "the request carries no signature",
            })
    })
}

/// Judge `agent` for the act `method path` names, answering the grant
/// exercised.
fn door(
    state: &AppState,
    agent: AgentId,
    method: &str,
    path: &str,
) -> Result<Exercised, ServerError> {
    let (resource, action) =
        crate::openapi_table::token_scope(method, path).map_err(|_undeclared| {
            ServerError::NotPermitted {
                reason: format!("{method} {path} names no act an agent may be granted"),
            }
        })?;
    require(state, agent, &resource, &action)
}

/// Exercise `agent`'s own live grant of `action` on `resource`, or, for an
/// agent resource, on the person that agent answers to; the grant exercised. Refused `not_permitted` naming the action
/// and the resource when no grant of the agent's reaches it.
///
/// # Errors
/// `not_permitted` when nothing admits it; the grants' own refusal when they
/// cannot be read or the use cannot be recorded.
pub(crate) fn require(
    state: &AppState,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
) -> Result<Exercised, ServerError> {
    reach(state, agent, resource, action, Decision::Exercise)
}

/// [`require`] with nothing recorded: the grant that admits the act, whose
/// use the act records itself in its own append (an agent registering an
/// agent records the use of `agent.create` with the rights it gives, after
/// the agent exists, AGENTS-016 R3).
///
/// # Errors
/// `not_permitted` when nothing admits it; the grants' own refusal when they
/// cannot be read.
pub(crate) fn admitting(
    state: &AppState,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
) -> Result<Exercised, ServerError> {
    reach(state, agent, resource, action, Decision::Explain)
}

fn reach(
    state: &AppState,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
    decision: Decision,
) -> Result<Exercised, ServerError> {
    if let Some(found) = exercised(state, agent, resource, action, decision)? {
        return Ok(found);
    }
    if resource.kind() == "agent"
        && let Some(person) = answering(state, resource.id())?
        && let Some(found) = exercised(
            state,
            agent,
            &Resource::new("person", &person.to_string())?,
            action,
            decision,
        )?
    {
        return Ok(found);
    }
    // Last, the agent's standing in its own organisation (24 October 2026).
    if let Some(standing) = with_directory(state, |directory| {
        crate::organisation_standing::behind(directory.projection()?, agent, resource, action)
    })? && let Some(found) =
        exercised(state, agent, &standing.resource, &standing.action, decision)?
    {
        return Ok(found);
    }
    Err(ServerError::NotPermitted {
        reason: format!("agent {agent} holds no live grant of {action} on {resource}"),
    })
}

/// The person the agent `id` answers to, when it is a known agent.
fn answering(state: &AppState, id: &str) -> Result<Option<PersonId>, ServerError> {
    let Ok(agent) = id.parse::<AgentId>() else {
        return Ok(None);
    };
    with_directory(state, |directory| {
        Ok(directory
            .projection()?
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible))
    })
}

/// `agent`'s grant of `action` on `resource`, its use recorded when
/// `decision` is an exercise; none when no grant of the agent's admits it.
fn exercised(
    state: &AppState,
    agent: AgentId,
    resource: &Resource,
    action: &Action,
    decision: Decision,
) -> Result<Option<Exercised>, ServerError> {
    let request = ExerciseRequest {
        caller: IdentityId::Agent(agent),
        route: Route::Api,
        resource: resource.clone(),
        action: action.clone(),
    };
    crate::grants_reads::with_read_grants(
        state,
        None,
        now(),
        |judged| {
            let mut asked = crate::grants_reads::resource_questions(judged, resource, action);
            asked.push(request.clone());
            Ok(asked)
        },
        |mut judged| {
            crate::caller_admission::active_caller(
                judged.directory,
                &actor_of(judged.directory, agent)?,
            )?;
            judged.apps.admit_kind(None, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            match decide(&mut judged, &request, now(), None, decision) {
                Ok((permit, _)) => match (permit.use_event, decision) {
                    (Some(Ok(_)), Decision::Exercise) | (None, Decision::Explain) => {
                        Ok(Some(Exercised {
                            grant: permit.grant,
                            root: permit.root_person,
                        }))
                    }
                    (Some(Err(error)), _) => Err(error.into()),
                    (None, Decision::Exercise) => Err(GrantError::LogUnavailable {
                        reason: format!("the exercise of {action} has no recorded use event"),
                    }
                    .into()),
                    (Some(Ok(_)), Decision::Explain) => Err(GrantError::LogUnavailable {
                        reason: format!("a question about {action} recorded a use event"),
                    }
                    .into()),
                },
                Err(error)
                    if crate::error_status::grant_status(&error)
                        == axum::http::StatusCode::FORBIDDEN =>
                {
                    Ok(None)
                }
                Err(error) => Err(error.into()),
            }
        },
    )
}

/// `agent` as the actor of what it writes: the agent by its signature,
/// bound to the login of the person it answers to, who stays responsible.
///
/// # Errors
/// `NoPerson` when the agent is unknown or answers to no person with a login.
pub(crate) fn actor_of(
    directory: &lys_identity::projection::Projection,
    agent: AgentId,
) -> Result<Actor, ServerError> {
    let binding = directory
        .record(IdentityId::Agent(agent))
        .and_then(lys_identity::projection::Record::responsible)
        .and_then(|person| directory.record(IdentityId::Person(person)))
        .and_then(|person| person.bindings().first())
        .ok_or(ServerError::NoPerson)?;
    Ok(Actor::new(
        binding.clone(),
        Provenance::by_agent(agent, now()),
    ))
}

/// The actor the call being served records: the admitted agent with its
/// person responsible, or `actor` for every other caller.
///
/// # Errors
/// `NoPerson` when the admitted agent answers to no person with a login.
pub(crate) fn recorded(state: &AppState, actor: lys_identity::Actor) -> Result<Actor, ServerError> {
    match admitted() {
        Some(admitted) => with_directory(state, |directory| {
            actor_of(directory.projection()?, admitted.agent)
        }),
        None => Ok(actor),
    }
}

/// The person the admitted agent answers to.
///
/// # Errors
/// `NoPerson` when it answers to none.
pub(crate) fn person_of(state: &AppState, agent: AgentId) -> Result<PersonId, ServerError> {
    answering(state, &agent.to_string())?.ok_or(ServerError::NoPerson)
}

/// The agent that registered `agent` and when, read from the actor of its
/// registration event in the directory's log; none when a person registered
/// it or the directory holds no such agent (AGENTS-016 R4, R5).
///
/// # Errors
/// The directory's own refusal when its log cannot be read.
pub(crate) fn registrar(
    directory: &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
    agent: AgentId,
) -> Result<Option<(AgentId, u64)>, ServerError> {
    let Some(first) = directory
        .record(IdentityId::Agent(agent))?
        .and_then(|record| record.events().first().copied())
    else {
        return Ok(None);
    };
    let (signed, _) =
        directory
            .log()?
            .entry(first)?
            .ok_or_else(|| ServerError::DirectoryUnavailable {
                reason: format!("agent {agent}'s registration event {first} is not in the log"),
            })?;
    let event = signed.event()?;
    Ok(match event.change() {
        lys_identity::Change::RegisterAgent { .. }
        | lys_identity::Change::ReportingRegistration { .. } => event
            .actor()
            .provenance()
            .agent()
            .map(|by| (by, event.recorded_at())),
        _ => None,
    })
}
