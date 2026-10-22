//! Pairing (AGENTS-020): an agent with no identity asks for a code over a
//! fresh key, says the four words to its person, and holds its wait open
//! until the person decides on the dashboard's Pair page.
//!
//! `POST /pair` takes a PKCS#10 request over the agent's key and the name it
//! proposes, keeps them durably (`pair_store`) and answers the four words,
//! the wait id and the address to tell the person. `GET /pair/{wait}` is
//! held by the store's own event until the pairing is decided, then answers
//! the approved [`Answer`] once, or `pair_declined`; an unknown or already
//! answered wait is refused at once as `pair_unknown`. No poll interval and
//! no deadline anywhere: a pairing waits until a person decides it.
//!
//! Both routes are public: a code alone registers, certifies and reaches
//! nothing; only a signed-in person's decision (R2) does.

use std::sync::{Arc, MutexGuard};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::pair_store::{Answer, Decision, PairStore, refused};
use crate::routes::AppState;

/// The agent's ask.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = PairAskBody)]
#[serde(deny_unknown_fields)]
pub struct AskBody {
    /// A PKCS#10 request over the agent's fresh Ed25519 key, as the PEM
    /// `lys ca request` writes or as standard base64 DER.
    pub request: String,
    /// The name the agent proposes for itself.
    pub name: String,
}

/// What the ask answers.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = PairAsked)]
pub struct Asked {
    /// The four words to tell the person.
    pub code: String,
    /// The id to hold `GET /pair/{wait}` on.
    pub wait: String,
    /// Where the person enters the words.
    pub address: String,
}

/// The pairing routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/pair", post(ask))
        .route("/pair/{wait}", get(wait))
}

/// The pairings, one caller at a time.
///
/// # Errors
/// Names a poisoned lock.
pub(crate) fn pairings(state: &AppState) -> Result<MutexGuard<'_, PairStore>, ServerError> {
    state
        .pairings
        .lock()
        .map_err(|poisoned| ServerError::DirectoryUnavailable {
            reason: format!("the pairings cannot be used: their lock is poisoned: {poisoned}"),
        })
}

/// The longest name an agent may propose, in characters.
const NAME_LIMIT: usize = 80;

async fn ask(
    State(state): State<Arc<AppState>>,
    body: Result<Json<AskBody>, JsonRejection>,
) -> Result<Json<Asked>, ServerError> {
    let Json(body) =
        body.map_err(|rejected| refused("pair_request_invalid", rejected.body_text()))?;
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > NAME_LIMIT {
        return Err(refused(
            "pair_request_invalid",
            format!("the proposed name is empty or over {NAME_LIMIT} characters"),
        ));
    }
    let der = crate::certificates_issue::request_der(&body.request)
        .map_err(|error| refused("pair_request_invalid", error.to_string()))?;
    lys_core::ca::verify_certificate_request(&der).map_err(|error| {
        refused(
            "pair_request_invalid",
            format!("the request is not a PKCS#10 request over an Ed25519 key: {error}"),
        )
    })?;
    let (code, wait) = pairings(&state)?.ask(name.to_owned(), STANDARD.encode(&der))?;
    Ok(Json(Asked {
        code,
        wait,
        address: format!("{}/#/pair", state.oidc.public_origin()),
    }))
}

async fn wait(
    State(state): State<Arc<AppState>>,
    Path(wait): Path<String>,
) -> Result<Json<Answer>, ServerError> {
    loop {
        // Taken and subscribed under one hold of the lock, so no decision
        // can fall between the look and the wait.
        let (taken, changed) = {
            let mut store = pairings(&state)?;
            match store.take(&wait)? {
                Some(decision) => (Some(decision), None),
                None => (None, Some(store.subscribe())),
            }
        };
        match taken {
            Some(Decision::Approved { answer }) => return Ok(Json(*answer)),
            Some(Decision::Declined) => {
                return Err(refused(
                    "pair_declined",
                    "the person declined this pairing; nothing of the key was kept",
                ));
            }
            None => {}
        }
        if let Some(mut changed) = changed {
            changed
                .changed()
                .await
                .map_err(|closed| ServerError::DirectoryUnavailable {
                    reason: format!("the pairings closed while a wait was held: {closed}"),
                })?;
        }
    }
}
