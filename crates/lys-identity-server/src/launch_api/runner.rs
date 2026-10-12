//! Admitted launches retain their runner checks and their selected conversation.

use super::{AppState, Arc, Json, ServerError, Value, kept_launch, with_provisioning};
use crate::network_store::NetworkStore;
use crate::runtime_state::Tracked;
use std::sync::Mutex;

pub(super) fn keep_with_runner(
    network: &Mutex<NetworkStore>,
    machine: &str,
    keep: impl FnOnce() -> Result<Tracked, ServerError>,
) -> Result<Tracked, ServerError> {
    let mut network = network
        .lock()
        .map_err(|error| ServerError::NetworkUnavailable {
            reason: format!("the network lock is poisoned: {error}"),
        })?;
    network.settle()?;
    network
        .runner(machine)
        .ok_or(ServerError::MachineWithoutRunner)?;
    // Removal is ordered after the append, including an uncertain append result.
    let kept = keep();
    drop(network);
    kept
}

/// A replay requires the recorded runner even though its launch is already kept.
pub(super) async fn run(
    state: &Arc<AppState>,
    view: Value,
    admitted_by: &str,
) -> Result<Json<Value>, ServerError> {
    let member = |name: &str| {
        view.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ServerError::LaunchUnrenderable {
                reason: format!("the kept start names no {name}"),
            })
    };
    let (agent, machine) = (member("agent")?, member("machine")?);
    crate::runner_sessions::machine_runner(state, &machine)?
        .ok_or(ServerError::MachineWithoutRunner)?;
    let mut launch = with_provisioning(state, |store| kept_launch(store, &view))?;
    launch.policy = crate::agent_policy_api::launch_policy(state, &agent)?;
    let conversation = view.get("conversation").and_then(Value::as_str);
    let ran = crate::runner_sessions::run_resuming(
        state,
        (&agent, &machine, admitted_by),
        launch,
        conversation,
    )
    .await?;
    let mut view = view;
    view["runner"] = ran;
    Ok(Json(view))
}
