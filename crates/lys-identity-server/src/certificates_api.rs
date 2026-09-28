//! The certificate route: the capability certificates entered for an agent,
//! each with what verifies its entry in the certificate log offline.
//!
//! The administrator, the person responsible for the agent and the agent
//! itself read it; to anyone else the agent's certificates are not
//! visible. A certificate's claims are what held when it was issued. They
//! are never the grants as they stand now, and the answer says so in
//! `claims_are_live`.
//!
//! This route reads what the log holds. It issues nothing and withdraws
//! nothing.

use std::sync::{Arc, PoisonError};

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;

use crate::agent_sight::seen_agent;
use crate::certificates_store::{CertificateStore, Proven, Withdrawn};
use crate::error::ServerError;
use crate::routes::{AppState, hex};

/// What verifies a certificate's entry offline.
#[derive(Debug, Clone, Serialize)]
pub struct EntryView {
    /// The index of the leaf the issuance was entered as.
    pub leaf: u64,
    /// The bytes of that leaf, as lowercase hex.
    pub leaf_bytes: String,
    /// The size of the log the proof is over.
    pub tree_size: u64,
    /// The root of the log at that size, as lowercase hex.
    pub root: String,
    /// The inclusion proof, as lowercase hex.
    pub proof: String,
}

/// One certificate as it stands.
#[derive(Debug, Clone, Serialize)]
pub struct CertificateView {
    /// Its serial.
    pub serial: String,
    /// The person it was issued for.
    pub person: String,
    /// What held when it was issued.
    pub claims: Value,
    /// The certificate, DER, standard base64.
    pub der: String,
    /// When it was issued, in seconds since the Unix epoch.
    pub issued_at: u64,
    /// Its withdrawal, null while it stands.
    pub withdrawn: Option<Withdrawn>,
    /// What verifies its entry offline.
    pub entry: EntryView,
}

/// The answer of the certificate route.
#[derive(Debug, Clone, Serialize)]
pub struct CertificatesView {
    /// The agent.
    pub agent: String,
    /// Every certificate entered for the agent, by serial.
    pub certificates: Vec<CertificateView>,
    /// Whether the claims are the grants as they stand now. They never
    /// are: the current answer comes from enforcement.
    pub claims_are_live: bool,
}

/// The certificate route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/certificates", get(read))
}

fn with_certificates<T>(
    state: &AppState,
    act: impl FnOnce(&CertificateStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store =
        state
            .certificates
            .as_ref()
            .ok_or_else(|| ServerError::CertificatesUnavailable {
                reason: "the configuration names no certificates_dir".to_owned(),
            })?;
    let store = store.lock().unwrap_or_else(PoisonError::into_inner);
    act(&store)
}

fn view(proven: Proven) -> CertificateView {
    let (root, _leaves) = proven.root.to_parts();
    let issued = proven.entered.issued;
    CertificateView {
        serial: issued.serial,
        person: issued.person,
        claims: issued.claims,
        der: issued.der,
        issued_at: issued.issued_at,
        withdrawn: proven.entered.withdrawn,
        entry: EntryView {
            leaf: proven.entered.leaf,
            leaf_bytes: hex(&proven.leaf_bytes),
            tree_size: proven.tree_size,
            root: hex(&root),
            proof: hex(proven.proof.as_bytes()),
        },
    }
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<CertificatesView>, ServerError> {
    let agent = seen_agent(&state, &headers, &id)?.agent.to_string();
    with_certificates(&state, |store| {
        let certificates = store
            .certificates()
            .filter(|entered| entered.issued.agent == agent)
            .map(|entered| store.prove(&entered.issued.serial).map(view))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Json(CertificatesView {
            agent: agent.clone(),
            certificates,
            claims_are_live: false,
        }))
    })
}
