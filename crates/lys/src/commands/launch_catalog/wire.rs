//! Decode public read views without returning response values in refusals.

use lys_home::harness::launch_fields::DeclaredHarness;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::commands::error::{CliError, CliResult};
use crate::commands::seat_client::{refused, segment};

pub(super) fn invalid(path: &str, reason: &str) -> CliError {
    refused("launch_catalog_invalid", format!("GET {path}: {reason}"))
}

pub(super) fn decode<T: DeserializeOwned>(value: Value, path: &str) -> CliResult<T> {
    serde_json::from_value(value).map_err(|error| {
        invalid(
            path,
            &format!(
                "the public view is malformed or incomplete; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })
}

#[derive(Deserialize)]
pub(super) struct People {
    pub scope: String,
    pub people: Vec<Person>,
}
#[derive(Deserialize)]
pub(super) struct Person {
    pub id: String,
    pub agents: Vec<Summary>,
}
#[derive(Deserialize)]
pub(super) struct Summary {
    pub id: String,
    pub display_name: String,
    pub state: String,
}

impl Summary {
    pub fn validate(&self, path: &str) -> CliResult<()> {
        segment("launch_catalog_invalid", "agent identity", &self.id).map_err(
            |error| match error {
                CliError::Runner(error) => invalid(
                    path,
                    &format!("an agent identity is malformed: {}", error.name()),
                ),
                other => other,
            },
        )?;
        if self.display_name.trim().is_empty() || self.state.trim().is_empty() {
            return Err(invalid(path, "an agent has no name or lifecycle state"));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub(super) struct Network {
    pub machines: Vec<Machine>,
}
#[derive(Deserialize)]
pub(super) struct Machine {
    pub id: String,
    pub state: String,
    pub may_run: Vec<Summary>,
    pub may_run_roles: Vec<String>,
}

impl Machine {
    pub fn validate(&self) -> CliResult<()> {
        if !matches!(self.state.as_str(), "in_use" | "retired") {
            return Err(invalid(
                "/network",
                "the joined machine has no recognized state",
            ));
        }
        for allowed in &self.may_run {
            allowed.validate("/network")?;
        }
        for role in &self.may_run_roles {
            segment("launch_catalog_invalid", "admitted role identity", role).map_err(|error| {
                match error {
                    CliError::Runner(error) => invalid(
                        "/network",
                        &format!("an admitted role identity is malformed: {}", error.name()),
                    ),
                    other => other,
                }
            })?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub(super) struct Provisioning {
    pub agent: String,
    #[serde(deserialize_with = "Option::<Profile>::deserialize")]
    pub profile: Option<Profile>,
}
#[derive(Deserialize)]
pub(super) struct Profile {
    pub version: u32,
    #[serde(deserialize_with = "Option::<DeclaredHarness>::deserialize")]
    pub harness: Option<DeclaredHarness>,
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub reviewed_by: Option<String>,
    #[serde(deserialize_with = "Option::<u64>::deserialize")]
    pub reviewed_at: Option<u64>,
    #[serde(deserialize_with = "Option::<Controls>::deserialize")]
    pub session: Option<Controls>,
    pub working_folder: Option<String>,
    pub runs_on: Option<String>,
    pub writable: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Controls {
    // The served session shape omits this member precisely when controls are off.
    #[serde(default)]
    pub requires_controls: bool,
}

impl Provisioning {
    pub fn validate(&self, agent: &str, path: &str) -> CliResult<()> {
        if self.agent != agent {
            return Err(invalid(path, "the profile belongs to a different agent"));
        }
        if let Some(profile) = &self.profile {
            if profile.version == 0
                || profile.reviewed_by.is_some() != profile.reviewed_at.is_some()
                || profile
                    .reviewed_by
                    .as_ref()
                    .is_some_and(|name| name.trim().is_empty())
            {
                return Err(invalid(path, "the profile version or review is incomplete"));
            }
            if let Some(harness) = &profile.harness {
                if [
                    &harness.name,
                    &harness.program,
                    &harness.package,
                    &harness.description.rendering_contract,
                ]
                .iter()
                .any(|word| word.trim().is_empty())
                {
                    return Err(invalid(path, "the harness declaration is incomplete"));
                }
            }
            if profile
                .working_folder
                .as_ref()
                .is_some_and(|folder| folder.trim().is_empty())
            {
                return Err(invalid(path, "the starting folder was declared empty"));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub(super) struct Runtime {
    pub sessions: Vec<RuntimeSession>,
    pub unanswered: Vec<Unanswered>,
}
#[derive(Deserialize)]
pub(super) struct RuntimeSession {
    pub session: String,
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub agent: Option<String>,
    pub machine: String,
    pub shown: String,
}
#[derive(Deserialize)]
pub(super) struct Unanswered {
    pub session: String,
    pub machine: String,
    pub refusal: String,
    pub reason: String,
}

impl Runtime {
    pub fn validate(&self) -> CliResult<()> {
        let mut seen = std::collections::BTreeSet::new();
        for session in &self.sessions {
            if session.session.trim().is_empty()
                || session.machine.trim().is_empty()
                || !matches!(
                    session.shown.as_str(),
                    "unconfirmed" | "running" | "stopped"
                )
                || !seen.insert((&session.session, &session.machine))
            {
                return Err(invalid(
                    "/runtime/live",
                    "a session has an invalid identity or state",
                ));
            }
        }
        let mut unread = std::collections::BTreeSet::new();
        for row in &self.unanswered {
            if row.refusal.trim().is_empty()
                || row.reason.trim().is_empty()
                || !seen.contains(&(&row.session, &row.machine))
                || !unread.insert((&row.session, &row.machine))
            {
                return Err(invalid(
                    "/runtime/live",
                    "an unanswered runner has no unique session, refusal or reason",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub(super) struct SeatViews {
    pub seats: Vec<Seat>,
    pub runner: String,
    pub reason: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct Seat {
    pub name: String,
    pub agent: String,
    pub machine: String,
    pub profile_version: u32,
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub responsible: Option<String>,
    pub state: String,
    pub reason: Option<String>,
    pub session: Option<String>,
}

impl SeatViews {
    pub fn validate(&self) -> CliResult<()> {
        if !matches!(self.runner.as_str(), "read" | "unknown")
            || (self.runner == "unknown"
                && self
                    .reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty()))
        {
            return Err(invalid(
                "/seats",
                "the runner read state has no recognized result or reason",
            ));
        }
        let mut names = std::collections::BTreeSet::new();
        for seat in &self.seats {
            if seat.name.trim().is_empty()
                || !names.insert(&seat.name)
                || seat.profile_version == 0
                || !matches!(
                    seat.state.as_str(),
                    "online-working" | "online-idle" | "offline" | "not-seen" | "unknown"
                )
                || (seat.state == "unknown"
                    && seat
                        .reason
                        .as_ref()
                        .is_none_or(|reason| reason.trim().is_empty()))
            {
                return Err(invalid(
                    "/seats",
                    "a seat has no unique name, profile, state or unknown reason",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub(super) struct Roles {
    pub roles: Vec<Role>,
}
#[derive(Deserialize)]
pub(super) struct Role {
    pub id: String,
    pub holders: Vec<Holder>,
}
#[derive(Deserialize)]
pub(super) struct Holder {
    pub holder: String,
    pub state: String,
}

impl Roles {
    pub fn validate(&self) -> CliResult<()> {
        let mut ids = std::collections::BTreeSet::new();
        for role in &self.roles {
            if role.id.trim().is_empty()
                || !ids.insert(&role.id)
                || role.holders.iter().any(|holder| {
                    holder.holder.trim().is_empty()
                        || !matches!(holder.state.as_str(), "holding" | "lapsed" | "ended")
                })
            {
                return Err(invalid(
                    "/roles",
                    "a role or holding has no unique identity or recognized state",
                ));
            }
        }
        Ok(())
    }
}
