//! A person's declared agents on the computer their client joined.

use serde_json::Value;

use super::error::CliResult;
use super::seat_client::{Server, refused, segment};

#[path = "launch_catalog/wire.rs"]
mod wire;

#[cfg(test)]
#[path = "launch_catalog_tests.rs"]
mod tests;

/// One agent owned by the signed-in person and admitted on this computer.
#[derive(Debug, Clone)]
pub struct Agent {
    /// Its Lys identity.
    pub id: String,
    /// Its display name.
    pub name: String,
    /// Its directory lifecycle state.
    pub state: String,
    /// Its declared harness, canonicalized from the rendering contract.
    pub harness: Option<String>,
    /// Its latest declared program, never a replacement command.
    pub program: Option<String>,
    /// The latest profile version, absent until one is recorded.
    pub profile_version: Option<u32>,
    /// Whether the latest profile has a recorded review.
    pub reviewed: bool,
    /// Whether that profile requires managed session controls.
    pub controlled: bool,
    /// The reviewed default computer, absent when no default was recorded.
    pub runs_on: Option<String>,
    /// The single writable folder, absent when no confinement was recorded.
    pub writable: Option<String>,
    /// The declared starting folder, absent when none was recorded.
    pub working_folder: Option<String>,
    /// Live sessions on this computer, with unanswered runner reads retained.
    pub sessions: Vec<Session>,
    /// Seats bound to this computer under this agent.
    pub seats: Vec<Seat>,
    /// Reasons the current runtime state cannot be confirmed.
    pub unknown: Vec<String>,
}

/// One session's reported state and the runner's refusal, when unread.
#[derive(Debug, Clone)]
pub struct Session {
    /// The server's session identity.
    pub session: String,
    /// Its reported state; an unanswered runner is explicitly `unknown`.
    pub state: String,
    /// The server's named refusal from an unanswered runner.
    pub refusal: Option<String>,
    /// The server's reason from an unanswered runner.
    pub reason: Option<String>,
}

/// A seat pinned to a reviewed profile on this computer.
#[derive(Debug, Clone)]
pub struct Seat {
    /// The seat's name.
    pub name: String,
    /// Its runner's state.
    pub state: String,
    /// The reviewed profile version the seat holds.
    pub profile_version: u32,
    /// Its latest session, absent until the seat has started.
    pub session: Option<String>,
    /// Why a runner could not confirm its state.
    pub reason: Option<String>,
}

impl Agent {
    /// The strongest currently reported runtime state, preserving unread runners.
    #[must_use]
    pub fn runtime_state(&self) -> &str {
        if !self.unknown.is_empty() {
            return "unknown";
        }
        if let Some(session) = self
            .sessions
            .iter()
            .find(|session| session.state == "running")
        {
            return &session.state;
        }
        if let Some(session) = self.sessions.first() {
            return &session.state;
        }
        if let Some(seat) = self
            .seats
            .iter()
            .find(|seat| seat.state.starts_with("online-"))
        {
            return &seat.state;
        }
        self.seats.first().map_or("not-seen", |seat| &seat.state)
    }
}

/// Read only the signed-in person's agents on the explicitly joined machine.
///
/// # Errors
/// Propagates each server refusal. Refuses malformed or wrongly scoped answers
/// as `launch_catalog_invalid`, and a missing machine as `machine_not_joined`.
pub fn read(server: &Server, person: &str, machine: &str) -> CliResult<Vec<Agent>> {
    read_with(|path| server.get(path), person, machine)
}

fn read_with(
    mut get: impl FnMut(&str) -> CliResult<Value>,
    person: &str,
    machine: &str,
) -> CliResult<Vec<Agent>> {
    segment("launch_catalog_invalid", "person identity", person)?;
    segment("launch_catalog_invalid", "joined machine identity", machine)?;
    let people: wire::People = wire::decode(get("/people")?, "/people")?;
    if people.scope != "personal" || people.people.len() != 1 {
        return Err(wire::invalid(
            "/people",
            "the answer is not one personal view",
        ));
    }
    let owner = people
        .people
        .into_iter()
        .next()
        .ok_or_else(|| wire::invalid("/people", "the answer names no signed-in person"))?;
    if owner.id != person {
        return Err(wire::invalid(
            "/people",
            "the answer belongs to a different person",
        ));
    }
    let network: wire::Network = wire::decode(get("/network")?, "/network")?;
    let mut machines = network.machines.into_iter().filter(|row| row.id == machine);
    let computer = machines.next().ok_or_else(|| {
        refused(
            "machine_not_joined",
            "the signed-in server does not list this joined computer",
        )
    })?;
    if machines.next().is_some() {
        return Err(wire::invalid(
            "/network",
            "the joined machine appears more than once",
        ));
    }
    computer.validate()?;
    if computer.state == "retired" {
        return Err(refused(
            "machine_retired",
            "the joined computer has been retired",
        ));
    }
    let seats: wire::Seats = wire::decode(get("/seats")?, "/seats")?;
    seats.validate()?;
    let runtime: wire::Runtime = wire::decode(get("/runtime/live")?, "/runtime/live")?;
    runtime.validate()?;
    let held = roles(&mut get, &computer.may_run_roles)?;
    let mut agents = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for summary in owner.agents {
        summary.validate("/people")?;
        if !ids.insert(summary.id.clone()) {
            return Err(wire::invalid("/people", "an agent appears more than once"));
        }
        let bound: Vec<_> = seats
            .seats
            .iter()
            .filter(|seat| seat.agent == summary.id && seat.machine == machine)
            .collect();
        if bound
            .iter()
            .any(|seat| seat.responsible.as_deref() != Some(person))
        {
            return Err(wire::invalid(
                "/seats",
                "a bound seat answers to a different person",
            ));
        }
        let admitted = computer
            .may_run
            .iter()
            .any(|allowed| allowed.id == summary.id)
            || held.contains(&summary.id)
            || !bound.is_empty();
        if admitted {
            agents.push(agent(&mut get, summary, bound, &runtime, machine)?);
        }
    }
    Ok(agents)
}

fn roles(
    get: &mut impl FnMut(&str) -> CliResult<Value>,
    admitted: &[String],
) -> CliResult<std::collections::BTreeSet<String>> {
    if admitted.is_empty() {
        return Ok(std::collections::BTreeSet::new());
    }
    let roles: wire::Roles = wire::decode(get("/roles")?, "/roles")?;
    roles.validate()?;
    Ok(roles
        .roles
        .into_iter()
        .filter(|role| admitted.contains(&role.id))
        .flat_map(|role| role.holders)
        .filter(|holder| holder.state == "holding")
        .map(|holder| holder.holder)
        .collect())
}

fn agent(
    get: &mut impl FnMut(&str) -> CliResult<Value>,
    summary: wire::Summary,
    bound: Vec<&wire::Seat>,
    runtime: &wire::Runtime,
    machine: &str,
) -> CliResult<Agent> {
    let path = format!("/agents/{}/provisioning", summary.id);
    let provisioning: wire::Provisioning = wire::decode(get(&path)?, &path)?;
    provisioning.validate(&summary.id, &path)?;
    let profile = provisioning.profile;
    let harness = profile
        .as_ref()
        .and_then(|profile| profile.harness.as_ref());
    let sessions: Vec<_> = runtime
        .sessions
        .iter()
        .filter(|session| {
            session.agent.as_deref() == Some(summary.id.as_str()) && session.machine == machine
        })
        .map(|session| {
            let unread = runtime
                .unanswered
                .iter()
                .find(|unread| unread.session == session.session && unread.machine == machine);
            Session {
                session: session.session.clone(),
                state: unread.map_or_else(|| session.shown.clone(), |_| "unknown".to_owned()),
                refusal: unread.map(|unread| unread.refusal.clone()),
                reason: unread.map(|unread| unread.reason.clone()),
            }
        })
        .collect();
    let seats: Vec<_> = bound
        .into_iter()
        .map(|seat| Seat {
            name: seat.name.clone(),
            state: seat.state.clone(),
            profile_version: seat.profile_version,
            session: seat.session.clone(),
            reason: seat.reason.clone(),
        })
        .collect();
    let unknown = sessions
        .iter()
        .filter_map(|session| {
            session
                .reason
                .as_ref()
                .zip(session.refusal.as_ref())
                .map(|(reason, refusal)| format!("{refusal}: {reason}"))
        })
        .chain(seats.iter().filter_map(|seat| seat.reason.clone()))
        .collect();
    Ok(Agent {
        id: summary.id,
        name: summary.display_name,
        state: summary.state,
        harness: harness.map(
            |harness| match harness.description.rendering_contract.as_str() {
                "claude-code/template-v1" => "claude".to_owned(),
                "codex/template-v1" => "codex".to_owned(),
                _ => harness.description.rendering_contract.clone(),
            },
        ),
        program: harness.map(|harness| harness.program.clone()),
        profile_version: profile.as_ref().map(|profile| profile.version),
        reviewed: profile
            .as_ref()
            .is_some_and(|profile| profile.reviewed_by.is_some()),
        controlled: profile
            .as_ref()
            .and_then(|profile| profile.session.as_ref())
            .is_some_and(|session| session.requires_controls),
        runs_on: profile.as_ref().and_then(|profile| profile.runs_on.clone()),
        writable: profile
            .as_ref()
            .and_then(|profile| profile.writable.clone()),
        working_folder: profile.and_then(|profile| profile.working_folder),
        sessions,
        seats,
        unknown,
    })
}

/// Select only an explicit name or the person's explicit harness default.
///
/// # Errors
/// Names a missing harness, default or agent, an ambiguous name, or a selected
/// agent whose declared command does not match. Never infers a default.
pub fn select<'a>(
    agents: &'a [Agent],
    command: &str,
    named: Option<&str>,
    default: Option<&str>,
) -> CliResult<&'a Agent> {
    let matches = |agent: &Agent| {
        (matches!(agent.harness.as_deref(), Some("claude" | "codex"))
            && agent.harness.as_deref() == Some(command))
            || agent.program.as_deref() == Some(command)
    };
    if !agents.iter().any(matches) {
        return Err(refused(
            "agent_for_harness_missing",
            "no agent on this computer declares that harness or program",
        ));
    }
    let name = named.or(default).ok_or_else(|| {
        refused(
            "agent_default_missing",
            "no default agent has been chosen for this harness on this computer",
        )
    })?;
    let agent = if let Some(agent) = agents.iter().find(|agent| agent.id == name) {
        agent
    } else {
        let mut selected = agents.iter().filter(|agent| agent.name == name);
        let agent = selected.next().ok_or_else(|| {
            refused(
                "agent_unknown",
                "the named agent is not yours on this computer",
            )
        })?;
        if selected.next().is_some() {
            return Err(refused(
                "agent_name_ambiguous",
                "more than one agent has that name; name its Lys id",
            ));
        }
        agent
    };
    if !matches(agent) {
        return Err(refused(
            "agent_harness_mismatch",
            "the named agent declares a different harness and program",
        ));
    }
    Ok(agent)
}
