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
const VALID_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// The sub-component of the lys arc the capability claims are carried under.
const CLAIMS_COMPONENT: u64 = 1;
/// The most characters a reason carries.
const REASON_MAX: usize = 500;

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

fn with_store<T>(
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

/// The certificate over `request` for `agent`, carrying `claims`.
fn certificate(
    state: &AppState,
    request: &[u8],
    agent: &str,
    claims: &Value,
) -> Result<Vec<u8>, ServerError> {
    let key = load_service_key(&state.grant_setup.key_file)?;
    let mut oid = LYS_OID_ARC.to_vec();
    oid.push(CLAIMS_COMPONENT);
    let carried = serde_json::to_vec(claims).map_err(unavailable)?;
    CertificateAuthority::new(key)
        .issue_certificate_for_request(
            request,
            agent,
            VALID_FOR,
            vec![encode_extension(&oid, carried)],
        )
        .map(|certified| certified.der_bytes)
        .map_err(|error| malformed(format!("the request was refused: {error}")))
}

/// Issue a certificate for the agent over the key its request proves.
pub(crate) async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<IssueBody>, JsonRejection>,
) -> Result<Json<CertificatesView>, ServerError> {
    let seen = seen_agent(&state, &headers, &id)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let serial = OperationId::from_str(&body.operation)?.to_string();
    let request = STANDARD
        .decode(body.request.trim())
        .map_err(|error| malformed(format!("request is not standard base64: {error}")))?;
    let proved = verify_certificate_request(&request)
        .map_err(|error| malformed(format!("the request was refused: {error}")))?;
    let agent = seen.agent.to_string();
    let person = seen
        .responsible
        .ok_or(ServerError::AgentNotVisible)?
        .to_string();
    let kept = with_store(&state, |store| Ok(store.certificate(&serial).cloned()))?;
    if let Some(kept) = kept {
        let der = STANDARD.decode(&kept.issued.der).map_err(unavailable)?;
        let key = certificate_subject_public_key(&der).map_err(unavailable)?;
        return if kept.issued.agent == agent && &key == proved.subject_public_key() {
            with_store(&state, |store| answer(store, &agent, Some(serial)))
        } else {
            Err(ServerError::CertificateReused { serial })
        };
    }
    let at = now();
    let claims = claims(&state, &seen, at)?;
    let der = certificate(&state, &request, &agent, &claims)?;
    with_store(&state, |store| {
        store.issue(Issued {
            serial: serial.clone(),
            agent: agent.clone(),
            person,
            claims,
            der: STANDARD.encode(der),
            issued_at: at,
        })?;
        answer(store, &agent, Some(serial))
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
    if reason.chars().count() > REASON_MAX {
        return Err(malformed(format!(
            "reason is longer than {REASON_MAX} characters"
        )));
    }
    let by = with_directory(&state, |directory| {
        Ok(own_person(directory.projection()?, &actor)?.to_string())
    })?;
    let agent = seen.agent.to_string();
    with_store(&state, |store| {
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
    })
}
