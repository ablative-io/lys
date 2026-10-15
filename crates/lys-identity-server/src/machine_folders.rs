//! `POST /network/machines/{id}/folders` `{"under"?}`: the folders directly
//! inside one folder of a computer, as its runner names them, so a person
//! chooses where an agent works from what that computer holds and never
//! types a path. With no `under`, the runner looks in its own home folder
//! and the answer says which folder that is.
//!
//! `POST /network/machines/{id}/choose-folder` `{"start"?}`: the computer's
//! own folder dialog, opened on its screen by its runner, answered with the
//! folder the person picks there. The answer waits for the person; a cancel
//! is the runner's `folder_choice_cancelled`.
//!
//! The computer's owner, an administrator, or an agent exercising the
//! machine's read grant asks either (`not_your_computer` for other people). The
//! machine must be kept (`MachineUnknown`) and name a runner
//! (`runner_absent`); a folder the runner cannot read, or a dialog it cannot
//! show, is the runner's own refusal, by its name.

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

/// Where the computer's folder dialog opens.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChooseFolderBody {
    /// An absolute folder on the machine the dialog opens in; its runner's home folder when absent.
    #[serde(default)]
    start: Option<String>,
}

/// The folder picked in a machine's own dialog.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct FolderChosenView {
    /// The machine asked.
    machine: String,
    /// The folder picked, absolute.
    folder: String,
}

/// The folders and folder-dialog routes.
pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/network/machines/{id}/folders", post(folders))
        .route("/network/machines/{id}/choose-folder", post(choose_folder))
}

/// The machine's runner, when the machine is kept and names one.
fn runner_of(
    state: &Arc<AppState>,
    id: &str,
) -> Result<crate::runner_client::RunnerRecord, ServerError> {
    with_network(state, |store| {
        store.machine(id).ok_or(ServerError::MachineUnknown)?;
        Ok(store.runner(id).cloned())
    })?
    .ok_or_else(|| ServerError::RunnerAbsent {
        machine: id.to_owned(),
    })
}

fn folder_reader(
    state: &Arc<AppState>,
    actor: &lys_identity::Actor,
    headers: &HeaderMap,
    id: &str,
) -> Result<(), ServerError> {
    if headers.contains_key(crate::agent_pass::HEADER)
        || crate::agent_signature::relayed_agent().is_some()
    {
        // Ingress already exercised this machine's read grant for the verified agent.
        crate::routes::with_directory(state, |directory| {
            match crate::caller_admission::active_caller(directory.projection()?, actor)? {
                lys_identity::IdentityId::Agent(_) => Ok(()),
                _ => Err(ServerError::NoPerson),
            }
        })?;
        return with_network(state, |store| {
            let machine = store.machine(id).ok_or(ServerError::MachineUnknown)?;
            if machine.retired.is_some() {
                return Err(ServerError::MachineRetired);
            }
            Ok(())
        });
    }
    crate::network_api::computer_owner(state, actor, id).map(|_| ())
}

async fn choose_folder(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<ChooseFolderBody>, JsonRejection>,
) -> Result<Json<FolderChosenView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    folder_reader(&state, &actor, &headers, &id)?;
    let Json(body) = given.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let start = body
        .start
        .map(|given| crate::provisioning_api::folder("start", given))
        .transpose()?;
    let runner = runner_of(&state, &id)?;
    match crate::runner_client::ask(&state, &id, runner, Act::ChooseFolder { start }).await? {
        Answer::FolderChosen { folder } => Ok(Json(FolderChosenView {
            machine: id,
            folder,
        })),
        other => Err(ServerError::Runner {
            refusal: "runner_answer_unexpected".to_owned(),
            words: format!(
                "the folder dialog answered {}",
                crate::runner_sessions::kind(&other)
            ),
        }),
    }
}

async fn folders(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<FoldersBody>, JsonRejection>,
) -> Result<Json<FoldersView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    folder_reader(&state, &actor, &headers, &id)?;
    let Json(body) = given.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let under = body
        .under
        .map(|given| crate::provisioning_api::folder("under", given))
        .transpose()?;
    let runner = runner_of(&state, &id)?;
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

#[cfg(test)]
#[path = "machine_folders_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "lysfix_srv_folder_tests.rs"]
mod lysfix_srv_folder_tests;
