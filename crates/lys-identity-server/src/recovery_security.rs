//! Recovery events are visible only to the administrator, or an agent given
//! reading on the configuration, without bearer codes.

use crate::error::ServerError;
use crate::routes::{AppState, is_administrator, signed_in};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::{Json, Router, routing::get};
use lys_identity::recovery_events::{self, Kind};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

/// One local recovery event shown in Lys.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecoveryEvent {
    /// Unique event identity, never a bearer code.
    pub id: String,
    /// The named recovery step.
    pub kind: String,
    /// Unix seconds at the step.
    pub at: u64,
    /// The security notification the administrator reads.
    pub detail: String,
}

/// Events visible to the signed-in person.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecoveryEvents {
    /// Whether this person administers the install.
    pub administrator: bool,
    /// All retained events, newest first. Other people see none.
    pub events: Vec<RecoveryEvent>,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/security-events", get(list))
}

fn directory(state: &AppState) -> PathBuf {
    state.setup.as_ref().map_or_else(
        || state.estate_plan_file.with_file_name("recovery-events"),
        |settings| settings.code_file.with_file_name("recovery-events"),
    )
}

fn unavailable(error: impl std::fmt::Display) -> ServerError {
    ServerError::SetupUnavailable {
        reason: format!("RecoveryEventsUnavailable: {error}"),
    }
}

/// Persist a step before signalling the existing change stream.
pub(crate) fn record(state: &AppState, kind: Kind, code: &str) -> Result<(), ServerError> {
    recovery_events::record(
        &directory(state),
        kind,
        &crate::setup::digest(code),
        crate::session::now(),
    )
    .map_err(unavailable)?;
    state.changes.signal()?;
    Ok(())
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<RecoveryEvents>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let administrator = is_administrator(&state, &actor)?;
    // An agent reads them by its own grant of reading the configuration,
    // judged at the door (AGENTS-019 R4).
    let given = crate::routes::door_agent(&state, &actor)?.is_some();
    let events = if administrator || given {
        recovery_events::read(&directory(&state))
            .map_err(unavailable)?
            .into_iter()
            .map(view)
            .collect()
    } else {
        Vec::new()
    };
    Ok(Json(RecoveryEvents {
        administrator,
        events,
    }))
}

fn view(event: recovery_events::Event) -> RecoveryEvent {
    let (kind, detail) = match event.kind {
        Kind::FirstRunCodeIssued => (
            "first_run_code_issued",
            "A first-run setup code was requested on this Mac after system authentication.",
        ),
        Kind::PasswordCodeIssued => (
            "password_code_issued",
            "An administrator password recovery code was requested on this Mac after system authentication.",
        ),
        Kind::PasswordResetStarted => (
            "password_reset_started",
            "An administrator password reset started. A separate event records completion.",
        ),
        Kind::PasswordResetCompleted => (
            "password_reset_completed",
            "The administrator password was reset. Other sign-in factors were kept.",
        ),
    };
    RecoveryEvent {
        id: event.id,
        kind: kind.to_owned(),
        at: event.at,
        detail: detail.to_owned(),
    }
}
