//! `GET /harnesses/plugins?machine=<id>&config_dir=<folder>`: the Claude Code
//! plugins installed in the config folder a run on that machine uses, as that
//! machine's runner reads them by the same rule its start's
//! `plugin_not_installed` check uses (the run's `CLAUDE_CONFIG_DIR`, else the
//! login's `~/.claude`). The agent page asks it for the agent's own computer
//! and folder, so what it shows is what a start there would find.
//!
//! An administrator asks it, the same person who saves an agent's settings.
//! The machine must be kept (`MachineUnknown`) and name a runner
//! (`runner_absent`); a folder that is not plain and absolute is
//! `RequestMalformed`; a folder the runner cannot read is the runner's own
//! refusal, `plugins_unreadable`, by name.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use lys_runner::{Act, Answer};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in};

/// Which machine to ask, and which of its config folders.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginsQuery {
    /// The machine the run starts on.
    pub machine: String,
    /// The run's `CLAUDE_CONFIG_DIR`, absolute; the login's `~/.claude` when absent.
    #[serde(default)]
    pub config_dir: Option<String>,
}

/// The plugins installed in one config folder of one machine.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct InstalledPlugins {
    /// The machine asked.
    pub machine: String,
    /// The folder its runner read, absolute.
    pub config_dir: String,
    /// Each installed plugin as name@marketplace, in order.
    pub installed: Vec<String>,
}

/// `GET /harnesses/plugins`.
///
/// # Errors
/// `NotSignedIn`/`NotAdmitted`, `RequestMalformed`, `MachineUnknown`,
/// `runner_absent`, and the runner's own refusal by name.
pub(crate) async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Query<PluginsQuery>, QueryRejection>,
) -> Result<Json<InstalledPlugins>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let Query(query) = given.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let config_dir = query
        .config_dir
        .map(|given| crate::provisioning_api::folder("config_dir", given))
        .transpose()?;
    let machine = query.machine;
    let runner = with_network(&state, |store| {
        store.machine(&machine).ok_or(ServerError::MachineUnknown)?;
        Ok(store.runner(&machine).cloned())
    })?
    .ok_or_else(|| ServerError::RunnerAbsent {
        machine: machine.clone(),
    })?;
    match crate::runner_client::ask(&state, &machine, runner, Act::Plugins { config_dir }).await? {
        Answer::Plugins {
            config_dir,
            installed,
        } => Ok(Json(InstalledPlugins {
            machine,
            config_dir,
            installed,
        })),
        other => Err(ServerError::Runner {
            refusal: "runner_answer_unexpected".to_owned(),
            words: format!(
                "the runner answered a plugins read with {}",
                crate::runner_sessions::kind(&other)
            ),
        }),
    }
}
