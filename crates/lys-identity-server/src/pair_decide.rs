//! The person's side of a pairing (AGENTS-020 R2, R3): a signed-in person
//! looks up the agent waiting with four words, approves it with a name and
//! the roots it is given, or declines it.
//!
//! An approval does what the dashboard's register act does, in the same
//! order and through the same directory, policy and grant calls: register
//! the agent under the signed-in person (`register_reporting_agent`),
//! activate it, give it each root named (a delegation from a grant the
//! person holds, `use_only`), then certify the pairing's key through the
//! path `POST /agents/{id}/certificates` uses (`certificates_issue::issued_for_pairing`:
//! the same claims, signing key and log entry, over a request made under the
//! name the agent proposed before it had an id; entered under the
//! `certificate` operation, which is its serial) and record the decision so the agent's held wait answers
//! with its identity, the certificate and the organisation's approved apps
//! as `/apps` shows them to a person (id, name, address).
//!
//! Lys keeps no handles: the agent's handle in the answer is the name the
//! person gave it, and the person's handle is their display name. The
//! operation ids (`operation` for the registration, `activate`, `certificate`,
//! and one per root) come from the page, as every other act's do, so a retry
//! carries the same ids and nothing is derived from another id.
//!
//! A decline records only the decision; the kept request is dropped with
//! it (`pair_store`), so nothing of the key stays.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use lys_identity::grants::{DelegateRequest, GrantId, PassOn, Relation, Resource, Route, Window};
use lys_identity::{AgentId, IdentityId, OperationId, PersonId, Profile, Transition};
use serde::{Deserialize, Serialize};

use crate::agent_sight::SeenAgent;
use crate::apps_binding::Acting;
use crate::error::ServerError;
use crate::pair::pairings;
use crate::pair_store::{Answer, Decision, Pairing, Reach, refused};
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// The agent waiting with the four words the person entered.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = PairWaiting)]
pub struct Waiting {
    /// The four words, as kept.
    pub code: String,
    /// The name the agent proposed.
    pub name: String,
    /// When it asked, seconds since the Unix epoch.
    pub asked_at: u64,
}

/// The object a root is on.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = PairResourceBody)]
#[serde(deny_unknown_fields)]
pub struct ResourceBody {
    /// The kind.
    pub kind: String,
    /// The id.
    pub id: String,
}

/// When a root may be exercised.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = PairWindowBody)]
#[serde(deny_unknown_fields)]
pub struct WindowBody {
    /// From, seconds since the Unix epoch.
    pub starts_at: u64,
    /// Until, or null for no end.
    #[serde(default)]
    pub ends_at: Option<u64>,
}

/// One root the agent is given: a delegation from a grant the person holds,
/// as the dashboard's register act makes it.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = PairRootBody)]
#[serde(deny_unknown_fields)]
pub struct RootBody {
    /// The delegation's operation id.
    pub operation: String,
    /// The grant the person holds that it is passed on from.
    pub source: String,
    /// The object it is on.
    pub resource: ResourceBody,
    /// The relation given.
    pub relation: String,
    /// When it may be exercised.
    pub window: WindowBody,
}

/// What the person approves with.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = PairApproveBody)]
#[serde(deny_unknown_fields)]
pub struct ApproveBody {
    /// The registration's operation id.
    pub operation: String,
    /// The activation's operation id.
    pub activate: String,
    /// The certificate's operation id, its serial in the certificate log.
    pub certificate: String,
    /// The agent's name, as the person settled it.
    pub display_name: String,
    /// The roots the agent is given.
    #[serde(default)]
    pub grants: Vec<RootBody>,
}

/// What an approval answers the person.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = PairApproved)]
pub struct Approved {
    /// The four words decided.
    pub code: String,
    /// The agent registered.
    pub agent: String,
    /// The certificate's serial in the log.
    pub serial: String,
    /// How many approved apps the agent was told of.
    pub apps: usize,
}

/// What a decline answers the person.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = PairDeclined)]
pub struct Declined {
    /// The four words decided.
    pub code: String,
    /// Always `declined`.
    pub decision: &'static str,
}

/// A root as the directory takes it.
struct Root {
    operation: OperationId,
    source: GrantId,
    resource: Resource,
    relation: Relation,
    window: Window,
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn operation(text: &str) -> Result<OperationId, ServerError> {
    OperationId::from_str(text).map_err(ServerError::from)
}

impl RootBody {
    fn parsed(&self) -> Result<Root, ServerError> {
        Ok(Root {
            operation: operation(&self.operation)?,
            source: GrantId::from_str(&self.source)
                .map_err(|error| malformed(format!("source: {error}")))?,
            resource: Resource::new(&self.resource.kind, &self.resource.id)?,
            relation: Relation::new(&self.relation)?,
            window: Window::new(self.window.starts_at, self.window.ends_at)?,
        })
    }
}

/// The four words as the store keys them: lowercase, single spaces.
fn normalised(code: &str) -> String {
    code.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The undecided pairing `code` names, or `pair_unknown`.
fn undecided(state: &AppState, code: &str) -> Result<Pairing, ServerError> {
    pairings(state)?
        .by_code(&normalised(code))
        .cloned()
        .ok_or_else(|| refused("pair_unknown", "no agent is waiting with these words"))
}

/// `GET /pair/{code}/waiting`: the agent waiting with these words, for the
/// signed-in person who will decide.
pub(crate) async fn waiting(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<Waiting>, ServerError> {
    signed_in(&state, &headers)?;
    let pairing = undecided(&state, &code)?;
    Ok(Json(Waiting {
        code: pairing.code,
        name: pairing.name,
        asked_at: pairing.asked_at,
    }))
}

/// `POST /pair/{code}/approve`.
pub(crate) async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(code): Path<String>,
    body: Result<Json<ApproveBody>, JsonRejection>,
) -> Result<Json<Approved>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|rejected| malformed(rejected.body_text()))?;
    let pairing = undecided(&state, &code)?;
    let request = STANDARD.decode(&pairing.request).map_err(|error| {
        refused(
            "pair_request_invalid",
            format!("the kept request is not standard base64: {error}"),
        )
    })?;
    let (register, activate, certificate) = (
        operation(&body.operation)?,
        operation(&body.activate)?,
        operation(&body.certificate)?,
    );
    let profile = Profile::new(&body.display_name)?;
    let roots = body
        .grants
        .iter()
        .map(RootBody::parsed)
        .collect::<Result<Vec<_>, _>>()?;
    let at = now();
    let (person, person_name, agent) =
        registered(&state, &actor, (register, activate), profile, at)?;
    crate::agent_policy_api::with_policies(&state, |policies| {
        policies.ensure_default(&agent.to_string())
    })?;
    crate::grants::with_grants(&state, |judged| {
        for root in roots {
            let request = DelegateRequest {
                operation: root.operation,
                caller: IdentityId::Person(person),
                route: Route::Browser,
                source: root.source,
                recipient: IdentityId::Agent(agent),
                responsible: person,
                resource: root.resource,
                relation: root.relation,
                pass_on: PassOn::UseOnly,
                window: root.window,
            };
            judged.grants.delegate(judged.directory, &request, at)?;
        }
        Ok(())
    })?;
    let seen = SeenAgent {
        agent,
        responsible: Some(person),
    };
    let issued = crate::certificates_issue::issued_for_pairing(
        &state,
        &seen,
        &request,
        &pairing.name,
        certificate.to_string(),
        at,
    )?;
    let der = issued.der.clone();
    crate::certificates_issue::with_store(&state, |store| store.issue(issued))?;
    let certificate_pem = crate::certificates_api::certificate_pem(&der);
    let acting = Acting::Person(actor);
    let apps = crate::apps_api::with_apps(&state, |apps, _projection| {
        Ok(apps
            .held()
            .apps
            .iter()
            .filter(|app| crate::apps_read::sees(&acting, app))
            .filter_map(|app| {
                crate::apps_read::address(app).map(|address| Reach {
                    id: app.registered.app.clone(),
                    name: app.registered.name.clone(),
                    address,
                })
            })
            .collect::<Vec<_>>())
    })?;
    let count = apps.len();
    let answer = Answer {
        agent: agent.to_string(),
        handle: body.display_name.trim().to_owned(),
        certificate_pem,
        serial: certificate.to_string(),
        lys: format!("{}/api", state.oidc.public_origin()),
        person: person_name,
        apps,
    };
    pairings(&state)?.decide(
        &pairing.code,
        Decision::Approved {
            answer: Box::new(answer),
        },
    )?;
    Ok(Json(Approved {
        code: pairing.code,
        agent: agent.to_string(),
        serial: certificate.to_string(),
        apps: count,
    }))
}

/// Register the agent under the signed-in person and activate it, as the
/// dashboard's register act does: the person, their display name and the
/// agent.
fn registered(
    state: &AppState,
    actor: &lys_identity::Actor,
    (register, activate): (OperationId, OperationId),
    profile: Profile,
    at: u64,
) -> Result<(PersonId, String, AgentId), ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = crate::read_api::own_person(projection, actor)?;
        let person_name = projection
            .record(IdentityId::Person(person))
            .map(|record| record.profile().display_name().to_owned())
            .ok_or(ServerError::NotAdmitted {
                reason: "the signed-in person is not in the directory",
            })?;
        let answer = directory.register_reporting_agent(
            actor.clone(),
            register,
            IdentityId::Person(person),
            profile,
            at,
        )?;
        let agent = answer.agent;
        directory.transition(
            actor.clone(),
            activate,
            IdentityId::Agent(agent),
            Transition::Activate,
            "paired by its person",
            at,
        )?;
        Ok((person, person_name, agent))
    })
}

/// `POST /pair/{code}/decline`.
pub(crate) async fn decline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<Declined>, ServerError> {
    signed_in(&state, &headers)?;
    let code = normalised(&code);
    pairings(&state)?.decide(&code, Decision::Declined)?;
    Ok(Json(Declined {
        code,
        decision: "declined",
    }))
}
