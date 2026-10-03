//! `POST /network/machines/{id}/folders` `{"under"?}`: the folders directly
//! inside one folder of a computer, as its runner names them, so a person
//! chooses where an agent works from what that computer holds and never
//! types a path. With no `under`, the runner looks in its own home folder
//! and the answer says which folder that is.
//!
//! An administrator asks it, the same person who saves an agent's settings.
//! The machine must be kept (`MachineUnknown`) and name a runner
//! (`runner_absent`); a folder the runner cannot read is the runner's own
//! refusal, by its name.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_runner::{Act, Answer};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in};

/// Which folder to look in.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FoldersBody {
    /// An absolute folder on the machine; its runner's home folder when absent.
    #[serde(default)]
    under: Option<String>,
}

/// The folders inside one folder of a machine.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct FoldersView {
    /// The machine asked.
    machine: String,
    /// The folder looked in, absolute.
    under: String,
    /// The name of each folder directly inside it, in order.
    folders: Vec<String>,
}

/// The folders route.
pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/network/machines/{id}/folders", post(folders))
}

async fn folders(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<FoldersBody>, JsonRejection>,
) -> Result<Json<FoldersView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let Json(body) = given.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let under = body
        .under
        .map(|given| crate::provisioning_api::folder("under", given))
        .transpose()?;
    let runner = with_network(&state, |store| {
        store.machine(&id).ok_or(ServerError::MachineUnknown)?;
        Ok(store.runner(&id).cloned())
    })?
    .ok_or_else(|| ServerError::RunnerAbsent {
        machine: id.clone(),
    })?;
    match crate::runner_client::ask(&state, &id, runner, Act::Folders { under }).await? {
        Answer::Folders { under, folders } => Ok(Json(FoldersView {
            machine: id,
            under,
            folders,
        })),
        other => Err(ServerError::Runner {
            refusal: "runner_answer_unexpected".to_owned(),
            words: format!(
                "the runner answered a folders read with {}",
                crate::runner_sessions::kind(&other)
            ),
        }),
    }
}
