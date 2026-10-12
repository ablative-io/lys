use super::{
    AppState, Arc, HeaderMap, Json, JsonRejection, Path, RunnerBody, RunnerRecord, ServerError,
    State, Value, body, signed_in, with_network,
};
use lys_runner::Act;
use serde_json::json;

/// A machine's runner record, and whether that runner answers a status
/// request now, by its runner's word: never guessed from its last report.
pub(super) async fn runner(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    signed_in(&state, &headers)?;
    let runner = with_network(&state, |store| {
        store.machine(&id).ok_or(ServerError::MachineUnknown)?;
        Ok(store.runner(&id).cloned())
    })?;
    let joining = crate::network_join::joining(&state, &id)?;
    let Some(record) = runner.clone() else {
        return Ok(Json(
            json!({ "machine": id, "runner": runner, "answers": null, "joining": joining }),
        ));
    };
    let asked =
        if matches!(record, RunnerRecord::Dialled { .. }) && !state.runners.hub().bridged(&id)? {
            Err(ServerError::Runner {
                refusal: "runner_not_dialled_in".to_owned(),
                words: "this computer's runner is not connected to Lys right now".to_owned(),
            })
        } else {
            crate::runner_client::ask(&state, &id, record, Act::Status { session: None })
                .await
                .map(drop)
        };
    Ok(Json(match asked {
        Ok(()) => json!({ "machine": id, "runner": runner, "answers": true, "joining": joining }),
        Err(refused) => {
            let reason = match &refused {
                ServerError::Runner { words, .. } => words.clone(),
                other => other.to_string(),
            };
            json!({ "machine": id, "runner": runner, "answers": false, "joining": joining,
                "refusal": refused.name(), "reason": reason })
        }
    }))
}

pub(super) async fn name_runner(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<RunnerBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let runner = body(given)?.runner.map(RunnerRecord::checked).transpose()?;
    with_network(&state, |store| store.name_runner(&id, runner.clone()))?;
    Ok(Json(json!({ "machine": id, "runner": runner })))
}

pub(super) async fn protocol() -> Json<Value> {
    Json(lys_runner::published::section())
}
