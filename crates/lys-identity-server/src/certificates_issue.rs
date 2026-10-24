//! Issuing and withdrawing an agent's capability certificate.
//!
//! An agent's certificate is issued over a key the agent proved it holds:
//! the caller presents a PKCS#10 request whose common name is the agent's
//! id, and nothing else of the request reaches the certificate. The claims
//! are gathered here from what the service keeps at that moment, so the
//! party the certificate speaks of sets none of them. The issuance is
//! entered in the certificate log under the operation id, which is the
//! certificate's serial in that log.
//!
//! The certificate is signed with the service's own key, the key that
//! seals the certificate log.
//!
//! A withdrawal is entered beside the issuance and changes nothing already
//! entered.

use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::ca::{
    CertificateAuthority, LYS_OID_ARC, certificate_subject_public_key, encode_extension,
    verify_certificate_request,
};
use lys_identity::grants::admission::effective;
use lys_identity::signer::load_service_key;
use lys_identity::{IdentityId, OperationId};
use serde::Deserialize;
use serde_json::{Value, json};
use std::str::FromStr;

use crate::agent_sight::{SeenAgent, seen_agent};
use crate::certificates_api::{CertificatesView, answer};
use crate::certificates_store::{CertificateStore, Issued, Withdrawn};
use crate::error::ServerError;
use crate::grants::with_grants;
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// How long a certificate is valid for from its issuance.
pub(crate) const VALID_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// The sub-component of the lys arc the capability claims are carried under.
const CLAIMS_COMPONENT: u64 = 1;

/// A certificate to issue. Every member is required.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct IssueBody {
    operation: String,
    request: String,
}

/// A certificate to withdraw.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WithdrawBody {
    reason: String,
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::CertificatesUnavailable {
        reason: what.to_string(),
    }
}

pub(crate) fn with_store<T>(
    state: &AppState,
    act: impl FnOnce(&mut CertificateStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .certificates
        .as_ref()
        .ok_or_else(|| unavailable("the configuration names no certificates_dir"))?;
    let mut store = store
        .lock()
        .map_err(|error| ServerError::CertificatesUnavailable {
            reason: format!("the certificates lock is poisoned: {error}"),
        })?;
    act(&mut store)
}

/// The grants the agent holds that stand at `at`, with every grant above
/// each of them.
fn grants(state: &AppState, seen: &SeenAgent, at: u64) -> Result<Vec<Value>, ServerError> {
    with_grants(state, |judged| {
        let book = judged.grants.book();
        Ok(book
            .held_by(IdentityId::Agent(seen.agent))
            .filter(|record| effective(book, judged.directory, record.grant().id(), at).is_ok())
            .map(|record| {
                let grant = record.grant();
                json!({
                    "id": grant.id().to_string(),
                    "resource": grant.resource().to_string(),
                    "actions": grant.actions().iter().map(ToString::to_string).collect::<Vec<_>>(),
                })
            })
            .collect())
    })
}

/// The roles the agent holds at `at`, null when no roles are kept.
fn roles(state: &AppState, agent: &str, at: u64) -> Result<Value, ServerError> {
    let Some(store) = state.roles.as_ref() else {
        return Ok(Value::Null);
    };
    let mut store = store
        .lock()
        .map_err(|error| ServerError::RolesUnavailable {
            reason: format!("the roles lock is poisoned: {error}"),
        })?;
    store.settle()?;
    Ok(store
        .roles()
        .iter()
        .filter_map(|role| {
            let holding = role.holding(agent)?;
            (holding.state(at) == "holding")
                .then(|| json!({ "role": role.id, "version": holding.version }))
        })
        .collect())
}

/// The version of the agent's profile, null when none was set.
fn profile_version(state: &AppState, agent: &str) -> Result<Value, ServerError> {
    let Some(store) = state.provisioning.as_ref() else {
        return Ok(Value::Null);
    };
    let mut store = store
        .lock()
        .map_err(|error| ServerError::ProvisioningUnavailable {
            reason: format!("the provisioning lock is poisoned: {error}"),
        })?;
    store.settle()?;
    Ok(store
        .profile(agent)
        .map_or(Value::Null, |profile| json!(profile.latest())))
}

/// What holds for the agent at `at`, as the service keeps it.
fn claims(state: &AppState, seen: &SeenAgent, at: u64) -> Result<Value, ServerError> {
    let agent = seen.agent.to_string();
    Ok(json!({
        "agent": agent,
        "person": seen.responsible.map(|person| person.to_string()),
        "roles": roles(state, &agent, at)?,
        "profile_version": profile_version(state, &agent)?,
        "grants": grants(state, seen, at)?,
        "held_at": at,
    }))
}

/// The certificate over `request` for `agent`, carrying `claims`. A pairing's
/// request (AGENTS-020) was made under the name the agent proposed, `asked_as`,
/// before it had an id; every other request names the agent itself.
fn certificate(
    state: &AppState,
    request: &[u8],
    (agent, asked_as): (&str, Option<&str>),
    claims: &Value,
) -> Result<Vec<u8>, ServerError> {
    let key = load_service_key(&state.grant_setup.key_file)?;
    let mut oid = LYS_OID_ARC.to_vec();
    oid.push(CLAIMS_COMPONENT);
    let carried = serde_json::to_vec(claims).map_err(unavailable)?;
    let extensions = vec![encode_extension(&oid, carried)];
    let authority = CertificateAuthority::new(key);
    match asked_as {
        Some(proposed) => {
            authority.issue_certificate_for_pairing(request, proposed, agent, VALID_FOR, extensions)
        }
        None => authority.issue_certificate_for_request(request, agent, VALID_FOR, extensions),
    }
    .map(|certified| certified.der_bytes)
    .map_err(|error| malformed(format!("the request was refused: {error}")))
}

/// The RFC 7468 boundaries of a PKCS#10 request, as `lys ca request` writes it.
const REQUEST_BEGIN: &str = "-----BEGIN CERTIFICATE REQUEST-----";
const REQUEST_END: &str = "-----END CERTIFICATE REQUEST-----";

/// The DER of `text`: a PKCS#10 request as the PEM the CLI writes, or as the
/// standard base64 DER taken before AGENTS-017 R1. Both decode to the same
/// bytes `verify_certificate_request` reads; anything else is refused naming
/// both forms.
pub(crate) fn request_der(text: &str) -> Result<Vec<u8>, ServerError> {
    let neither =
        |why: String| malformed(format!("request is neither PEM nor standard base64: {why}"));
    let trimmed = text.trim();
    if !trimmed.starts_with("-----") {
        return STANDARD
            .decode(trimmed)
            .map_err(|error| neither(error.to_string()));
    }
    let lines: Vec<&str> = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let (Some(&first), Some(&last)) = (lines.first(), lines.last()) else {
        return Err(neither("the PEM is empty".to_owned()));
    };
    if first != REQUEST_BEGIN || last != REQUEST_END || lines.len() < 3 {
        return Err(neither(format!(
            "a PEM request is framed by {REQUEST_BEGIN} and {REQUEST_END}"
        )));
    }
    STANDARD
        .decode(lines[1..lines.len() - 1].concat())
        .map_err(|error| neither(format!("the PEM body is not base64: {error}")))
}

/// Issue a certificate for the agent over the key its request proves.
pub(crate) async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<IssueBody>, JsonRejection>,
) -> Result<Json<CertificatesView>, ServerError> {
    if let Some(admitted) = crate::agent_acts::admitted() {
        return by_agent(&state, admitted.agent, &id, body);
    }
    let seen = if let Some(actor) = crate::provider::app_person_actor(&state, &headers)? {
        with_directory(&state, |directory| {
            let projection = directory.projection()?;
            let person = crate::read_api::own_person(projection, &actor)?;
            let agent = id.parse().map_err(|error| {
                drop(error);
                ServerError::AgentNotVisible
            })?;
            let record = projection
                .record(IdentityId::Agent(agent))
                .filter(|record| record.responsible() == Some(person))
                .ok_or(ServerError::AgentNotVisible)?;
            Ok(SeenAgent {
                agent,
                responsible: record.responsible(),
            })
        })?
    } else {
        crate::routes::responsible_or_administrator(&state, &headers, &id)?;
        seen_agent(&state, &headers, &id)?
    };
    let asked = asked(&state, &seen, body)?;
    if let Some(answered) = asked.answered {
        return Ok(answered);
    }
    let issued = issued(&state, &seen, &asked.request, asked.serial.clone(), now())?;
    let agent = seen.agent.to_string();
    with_store(&state, |store| {
        store.issue(issued)?;
        answer(store, &agent, Some(asked.serial))
    })
}

/// A certificate asked for: its serial and request, or the answer already
/// recorded under that serial.
struct Asked {
    serial: String,
    request: Vec<u8>,
    answered: Option<Json<CertificatesView>>,
}

/// Read `body` for `seen`'s agent: answered from the store when its serial
/// already names the same agent and key, refused when it names another.
fn asked(
    state: &AppState,
    seen: &SeenAgent,
    body: Result<Json<IssueBody>, JsonRejection>,
) -> Result<Asked, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let serial = OperationId::from_str(&body.operation)?.to_string();
    let request = request_der(&body.request)?;
    let proved = verify_certificate_request(&request)
        .map_err(|error| malformed(format!("the request was refused: {error}")))?;
    let agent = seen.agent.to_string();
    seen.responsible.ok_or(ServerError::AgentNotVisible)?;
    let kept = with_store(state, |store| Ok(store.certificate(&serial).cloned()))?;
    let answered = match kept {
        Some(kept) => {
            let der = STANDARD.decode(&kept.issued.der).map_err(unavailable)?;
            let key = certificate_subject_public_key(&der).map_err(unavailable)?;
            if kept.issued.agent != agent || &key != proved.subject_public_key() {
                return Err(ServerError::CertificateReused { serial });
            }
            Some(with_store(state, |store| {
                answer(store, &agent, Some(serial.clone()))
            })?)
        }
        None => None,
    };
    Ok(Asked {
        serial,
        request,
        answered,
    })
}

/// A certificate of agent `id` asked by `caller`, another agent. Through a
/// live grant of `agent.certificate.issue` on the agent, or on the person
/// it answers to, any certificate, the use recorded against that grant and
/// the certificate naming `caller` as its issuer; holding none, only the
/// first certificate of an agent `caller` itself registered ([`first`]).
/// This is how an agent an administrator trusts with
/// `agent.certificate.issue` re-keys the agents of its person without a
/// person signing in for each (24 October 2026).
fn by_agent(
    state: &AppState,
    caller: lys_identity::AgentId,
    id: &str,
    body: Result<Json<IssueBody>, JsonRejection>,
) -> Result<Json<CertificatesView>, ServerError> {
    let agent: lys_identity::AgentId = id.parse().map_err(|error| {
        drop(error);
        ServerError::AgentNotVisible
    })?;
    let resource = lys_identity::grants::Resource::new("agent", id)?;
    let action = lys_identity::grants::Action::new("agent.certificate.issue")?;
    match crate::agent_acts::admitting(state, caller, &resource, &action) {
        Ok(_) => {}
        Err(ServerError::NotPermitted { .. }) => return first(state, caller, id, body),
        Err(error) => return Err(error),
    }
    let person = with_directory(state, |directory| {
        directory
            .projection()?
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::AgentNotVisible)
    })?;
    let seen = SeenAgent {
        agent,
        responsible: Some(person),
    };
    let asked = asked(state, &seen, body)?;
    if let Some(answered) = asked.answered {
        return Ok(answered);
    }
    let issued = issued_by(
        state,
        &seen,
        &asked.request,
        asked.serial.clone(),
        (now(), Some(caller)),
    )?;
    // The use is recorded once the request is sound and before the
    // certificate is entered, so a refused body records no exercise.
    crate::agent_acts::require(state, caller, &resource, &action)?;
    let named = agent.to_string();
    with_store(state, |store| {
        store.issue(issued)?;
        answer(store, &named, Some(asked.serial))
    })
}

/// The first certificate of agent `id`, asked by `caller`, the agent that
/// registered it (AGENTS-016 R4). Admitted only when the registration names
/// `caller`, `caller` holds a live grant of `agent.create` from the agent's
/// person now, and the agent holds no certificate, issued or withdrawn; the
/// admission is read from the registration record, never from a grant. The
/// certificate records `caller` as the one who issued it and the person as
/// responsible.
fn first(
    state: &AppState,
    caller: lys_identity::AgentId,
    id: &str,
    body: Result<Json<IssueBody>, JsonRejection>,
) -> Result<Json<CertificatesView>, ServerError> {
    use crate::error_agent_acts::AgentActsError;
    let agent: lys_identity::AgentId = id.parse().map_err(|error| {
        drop(error);
        ServerError::AgentNotVisible
    })?;
    let (person, registrar) = with_directory(state, |directory| {
        let person = directory
            .projection()?
            .record(IdentityId::Agent(agent))
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::AgentNotVisible)?;
        Ok((person, crate::agent_acts::registrar(directory, agent)?))
    })?;
    let words = |error: AgentActsError| ServerError::from(error);
    if registrar.map(|(by, _)| by) != Some(caller) {
        return Err(words(AgentActsError::AgentNotYours {
            agent: agent.to_string(),
            caller: caller.to_string(),
        }));
    }
    let create = crate::agent_acts::require(
        state,
        caller,
        &lys_identity::grants::Resource::new("person", &person.to_string())?,
        &lys_identity::grants::Action::new(lys_identity::grants::lineage::CREATE)?,
    );
    match create {
        Ok(_) => {}
        Err(ServerError::NotPermitted { .. }) => {
            return Err(words(AgentActsError::AgentCreateRevoked {
                agent: agent.to_string(),
                caller: caller.to_string(),
                person: person.to_string(),
            }));
        }
        Err(error) => return Err(error),
    }
    let seen = SeenAgent {
        agent,
        responsible: Some(person),
    };
    let asked = asked(state, &seen, body)?;
    if let Some(answered) = asked.answered {
        return Ok(answered);
    }
    let issued = issued_by(
        state,
        &seen,
        &asked.request,
        asked.serial.clone(),
        (now(), Some(caller)),
    )?;
    let named = agent.to_string();
    with_store(state, |store| {
        if store
            .certificates()
            .any(|entered| entered.issued.agent == named)
        {
            return Err(words(AgentActsError::CertificateNotFirst { agent: named }));
        }
        store.issue(issued)?;
        answer(store, &named, Some(asked.serial))
    })
}

/// The certificate `serial` for `seen`'s agent over the key `request`
/// proves, carrying what holds for the agent at `at`; not yet entered.
pub(crate) fn issued(
    state: &AppState,
    seen: &SeenAgent,
    request: &[u8],
    serial: String,
    at: u64,
) -> Result<Issued, ServerError> {
    issued_by(state, seen, request, serial, (at, None))
}

/// [`issued`] for a pairing (AGENTS-020 R2): the request was made under the
/// name the agent proposed, `proposed`, before it had an id.
pub(crate) fn issued_for_pairing(
    state: &AppState,
    seen: &SeenAgent,
    request: &[u8],
    proposed: &str,
    serial: String,
    at: u64,
) -> Result<Issued, ServerError> {
    issued_as(state, seen, (request, Some(proposed)), serial, (at, None))
}

/// [`issued`], naming in its claims the agent that issued it, when an agent
/// did, beside the person responsible (AGENTS-016 R4).
fn issued_by(
    state: &AppState,
    seen: &SeenAgent,
    request: &[u8],
    serial: String,
    (at, by): (u64, Option<lys_identity::AgentId>),
) -> Result<Issued, ServerError> {
    issued_as(state, seen, (request, None), serial, (at, by))
}

/// [`issued_by`], over a request made under `asked_as` when it was not made
/// under the agent's own id.
fn issued_as(
    state: &AppState,
    seen: &SeenAgent,
    (request, asked_as): (&[u8], Option<&str>),
    serial: String,
    (at, by): (u64, Option<lys_identity::AgentId>),
) -> Result<Issued, ServerError> {
    let agent = seen.agent.to_string();
    let person = seen
        .responsible
        .ok_or(ServerError::AgentNotVisible)?
        .to_string();
    let mut claims = claims(state, seen, at)?;
    if let Some(by) = by {
        claims["issued_by"] = json!({ "actor": by.to_string(), "responsible": person });
    }
    let der = certificate(state, request, (&agent, asked_as), &claims)?;
    Ok(Issued {
        serial,
        agent,
        person,
        claims,
        der: STANDARD.encode(der),
        issued_at: at,
    })
}

/// Withdraw a certificate of the agent, as the person signed in.
pub(crate) async fn withdraw(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, serial)): Path<(String, String)>,
    body: Result<Json<WithdrawBody>, JsonRejection>,
) -> Result<Json<CertificatesView>, ServerError> {
    let seen = seen_agent(&state, &headers, &id)?;
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let reason = body.reason.trim().to_owned();
    if reason.is_empty() {
        return Err(malformed("reason is empty: say why it is withdrawn"));
    }
    let by = with_directory(&state, |directory| {
        Ok(own_person(directory.projection()?, &actor)?.to_string())
    })?;
    let agent = seen.agent.to_string();
    let view = with_store(&state, |store| {
        let of_agent = store
            .certificate(&serial)
            .is_some_and(|kept| kept.issued.agent == agent);
        if !of_agent {
            return Err(ServerError::CertificateUnknown { serial });
        }
        store.withdraw(Withdrawn {
            serial: serial.clone(),
            by,
            reason,
            withdrawn_at: now(),
        })?;
        answer(store, &agent, Some(serial))
    })?;
    // The certificate is the agent's identity: once it is withdrawn, no run
    // started on it may act again, so every run pass the agent holds ends.
    crate::agent_pass::end_agent(&state, seen.agent).map_err(|error| {
        ServerError::CertificatesUnavailable {
            reason: format!(
                "the certificate was withdrawn, but the agent's runs could not be ended: {error}"
            ),
        }
    })?;
    Ok(view)
}
