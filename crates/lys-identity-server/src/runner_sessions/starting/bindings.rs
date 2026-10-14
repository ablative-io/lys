//! Restore the frozen signed-record bindings before admitting a replay.
use super::{AppState, Launch, ServerError, Value, record_unproved};

pub(super) fn restore_record_bindings(
    state: &AppState,
    (agent, machine): (&str, &str),
    launch: &mut Launch,
) -> Result<(), ServerError> {
    if state.runtime.is_none() && !launch.environment.contains_key("LYS_LAUNCH_RECORD") {
        return Ok(());
    }
    let view = crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .session(&launch.session)
            .and_then(|tracked| tracked.first())
            .and_then(|first| first.launch.clone()))
    })?;
    let Some(view) = view else {
        if launch.environment.contains_key("LYS_LAUNCH_RECORD") {
            return Err(record_unproved(
                "the directory launch has no retained binding metadata",
            ));
        }
        return Ok(());
    };
    let Some(record) = view.get("launch_record") else {
        return Ok(());
    };
    let id = record
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| lys_identity::start::launch_record::is_launch_record_id(id))
        .ok_or_else(|| record_unproved("the retained launch record id is invalid"))?;
    if super::launch_session(id) != launch.session
        || view["agent"].as_str() != Some(agent)
        || view["machine"].as_str() != Some(machine)
    {
        return Err(record_unproved(
            "the retained record belongs to another launch",
        ));
    }
    let ids: Vec<String> =
        serde_json::from_value(record.get("credential_ids").cloned().ok_or_else(|| {
            record_unproved("the retained record names no frozen credential ids")
        })?)
        .map_err(|error| {
            record_unproved(&format!(
                "the retained credential ids are not a list of strings; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ))
        })?;
    launch
        .environment
        .insert("LYS_LAUNCH_RECORD".to_owned(), id.to_owned());
    launch
        .environment
        .insert("LYS_HANDLES".to_owned(), ids.join(","));
    Ok(())
}
