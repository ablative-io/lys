//! Managed runs keep their reviewed named seat and its structured output channel.

use serde::Deserialize;
use serde_json::Value;

use crate::commands::error::CliResult;
use crate::commands::launch_catalog::{Agent, Seat};
use crate::commands::launch_context::surface;
use crate::commands::seat_client::refused;

pub(super) fn select<'a>(agent: &'a Agent, machine: &str, base: &str) -> CliResult<&'a Seat> {
    let version = agent.profile_version.ok_or_else(|| {
        refused(
            "LaunchRecordMissing",
            "the agent has no provisioning profile",
        )
    })?;
    let mut seats = agent
        .seats
        .iter()
        .filter(|seat| seat.profile_version == version);
    let selected = seats.next().ok_or_else(|| {
        let identity = lys_home::record::blocks::Hash::of(format!("{}:{machine}:{version}", agent.id).as_bytes());
        let name = format!("launch-{}", &identity.as_str()[..16]);
        let folder = agent.working_folder.as_deref().map_or_else(|| "<absolute-working-folder>".to_owned(), super::shell_word);
        refused("launch_managed_seat_missing", format!("the managed agent has no seat on this computer pinned to profile {version}; run `lys seat --server {} add {name} --agent {} --profile-version {version} --machine {machine} --working-folder {folder}`", super::shell_word(base), agent.id))
    })?;
    if seats.next().is_some() {
        return Err(refused(
            "launch_managed_seat_ambiguous",
            format!(
                "the agent has multiple seats on this computer pinned to profile {version}; choose the intended seat in {}",
                surface(base, "canvas?view=tree")
            ),
        ));
    }
    if selected.state.starts_with("online-") {
        return Err(refused(
            "seat_already_running",
            format!(
                "seat {} already runs; attach with `lys attach {} --type --server {}`",
                selected.name,
                selected.name,
                super::shell_word(base)
            ),
        ));
    }
    if selected.state == "unknown" {
        let reason = selected.reason.as_deref().ok_or_else(|| {
            refused(
                "launch_catalog_invalid",
                "the unknown seat state has no server reason",
            )
        })?;
        return Err(refused(
            "seat_state_unknown",
            format!(
                "seat {}: {reason}; inspect {} before starting",
                selected.name,
                surface(base, "canvas?view=tree")
            ),
        ));
    }
    Ok(selected)
}

#[derive(Deserialize)]
struct Started {
    seat: View,
    session: String,
    start: Value,
}

#[derive(Deserialize)]
struct View {
    name: String,
    agent: String,
    machine: String,
    profile_version: u32,
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    session: Option<String>,
}

pub(super) fn start(
    answer: Value,
    agent: &Agent,
    seat: &Seat,
    machine: &str,
    session: &str,
) -> CliResult<Value> {
    let answer: Started = serde_json::from_value(answer).map_err(|error| {
        refused(
            "launch_answer_unreadable",
            format!(
                "Lys did not answer in its recorded managed seat start shape; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })?;
    if answer.session != session
        || answer.seat.name != seat.name
        || answer.seat.agent != agent.id
        || answer.seat.machine != machine
        || answer.seat.profile_version != seat.profile_version
        || answer.seat.session.as_deref() != Some(session)
    {
        return Err(refused(
            "launch_answer_mismatch",
            "Lys's managed start does not name the requested seat, agent, machine, profile and session",
        ));
    }
    Ok(answer.start)
}

#[cfg(test)]
#[path = "managed_tests.rs"]
mod tests;
