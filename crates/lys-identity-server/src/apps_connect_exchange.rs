//! The exchange: the app's server collects its new values with the one-time
//! code (DIRECTORY-102 R1). It issues the app's new workspace credential as
//! the administrator who approved, checkpoints its value, records the values
//! handed over, and only then revokes every client credential that was live
//! at the approval, with the reason "the app was connected again". Until
//! this runs, the running app's credentials are untouched.
//!
//! A revocation that fails after the hand-over never withholds the values
//! already recorded as handed over: it is logged, and asked again by the
//! next exchange of the same code (refused `connect_code_used`) and by the
//! app's next connect, whose prior set holds every credential still live.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::checkpoint::{Checkpoint, Opened, Record};
use super::{ExchangeBody, malformed, operation, unavailable, with_apps};
use crate::apps_state::{ClientCredentialRevoked, Line};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

/// Why the exchange revokes a credential its new values replace.
const REASON: &str = "the app was connected again";

/// The refusal `name` in plain words for the person reading the app's page.
fn refusal_words(name: &str) -> &'static str {
    match name {
        "connect_code_used" => "this connection code was already used; start the connection again",
        "connect_code_expired" => {
            "this connection code is more than ten minutes old; start the connection again"
        }
        "connect_code_replaced" => {
            "the app was connected again after this code was made; use the newest connection, or start the connection again"
        }
        _ => "Lys does not know this connection code; start the connection again",
    }
}

pub(super) async fn exchange(
    State(state): State<Arc<AppState>>,
    body: Result<Json<ExchangeBody>, JsonRejection>,
) -> Result<Response, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let checkpoint = Checkpoint::open(&state)?;
    let gate = checkpoint.gate().await?;
    let answer = match checkpoint.open_code(&body.code, now())? {
        Opened::Ready(mut record) => {
            workspace_credential(&state, &mut record, &checkpoint).await?;
            let answer = checkpoint.hand_over(&mut record)?;
            revoke_prior(&state, &mut record, &checkpoint).await;
            Ok(answer)
        }
        Opened::Refused(refusal, Some(mut record)) => {
            revoke_prior(&state, &mut record, &checkpoint).await;
            Err(refusal)
        }
        Opened::Refused(refusal, None) => Err(refusal),
    };
    drop(gate);
    Ok(match answer {
        Ok(answer) => Json(answer).into_response(),
        Err(refusal) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"refusal": refusal, "reason": refusal_words(refusal)})),
        )
            .into_response(),
    })
}

/// Issue the app's new workspace credential as the approving administrator,
/// its one-time value checkpointed before anything is handed over.
async fn workspace_credential(
    state: &AppState,
    record: &mut Record,
    checkpoint: &Checkpoint,
) -> Result<(), ServerError> {
    if record.bearer.is_some() {
        return Ok(());
    }
    let (by, person) = record.acting.clone().ok_or_else(|| {
        unavailable("the connection names no administrator to issue the app's workspace credential as; the running app keeps its credentials; start a new connect operation")
    })?;
    let given = crate::apps_client_credentials::bearer_as(
        state,
        &person,
        &record.app,
        operation(&record.operation, "bearer"),
        |_apps, _projection| Ok(by.clone()),
    )
    .await?;
    let value = given.credential.ok_or_else(|| unavailable(
        "the workspace credential's issue is recorded but its one-time value was not checkpointed; the running app keeps its credentials; start a new connect operation"))?;
    record.bearer = Some((given.reference, Some(value)));
    checkpoint.save(record)
}

/// Revoke, once the values are handed over, every prior credential still
/// live, then ask the broker to end them; a failure is logged and asked again.
async fn revoke_prior(state: &AppState, record: &mut Record, checkpoint: &Checkpoint) {
    if !record.handed || record.prior_revoked {
        return;
    }
    if let Err(error) = revoked(state, record) {
        tracing::warn!(app = %record.app, %error, "the credentials an app's new ones replace were not revoked; the next exchange of this code or the app's next connect revokes them");
        return;
    }
    record.prior_revoked = true;
    if let Err(error) = checkpoint.save(record) {
        tracing::warn!(app = %record.app, %error, "the revocation of the credentials an app's new ones replace is kept, but its connection checkpoint does not say so yet");
    }
    if let Some((_by, person)) = &record.acting {
        crate::apps_client_credentials::ended_as(state, person, &record.app).await;
    }
}

fn revoked(state: &AppState, record: &Record) -> Result<(), ServerError> {
    let (by, _person) = record
        .acting
        .as_ref()
        .ok_or_else(|| unavailable("the connection names no administrator to revoke as"))?;
    with_apps(state, |apps, _projection| {
        for credential in &record.prior {
            let operation = operation(&record.operation, &format!("revoke/{credential}"));
            let live = apps
                .app(&record.app)
                .is_some_and(|app| app.live_client_credentials().contains(credential));
            if !live || apps.held().operation(&operation).is_some() {
                continue;
            }
            apps.keep(Line::ClientCredentialRevoked(ClientCredentialRevoked {
                operation,
                app: record.app.clone(),
                credential_id: credential.clone(),
                reason: REASON.to_owned(),
                by: by.clone(),
                at: now(),
            }))?;
        }
        Ok(())
    })
}
