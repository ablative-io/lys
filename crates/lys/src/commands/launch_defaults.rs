//! A default is a person's explicit choice, scoped to server and joined device.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;
use crate::identity::private_files;

const FORMAT: &str = "lys-launch-defaults/v1";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Defaults {
    format: String,
    choices: Vec<Choice>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Choice {
    server: String,
    person: String,
    machine: String,
    harness: String,
    agent: String,
}

/// The exact scope of a default; another person or computer never inherits it.
pub struct Scope<'a> {
    /// The server recorded at sign-in.
    pub server: &'a str,
    /// The signed-in person.
    pub person: &'a str,
    /// This device's joined machine.
    pub machine: &'a str,
}

fn path(root: &Path) -> PathBuf {
    root.join("client/launch-defaults.json")
}

fn load(root: &Path) -> CliResult<Defaults> {
    let Some(bytes) = private_files::read(&path(root))? else {
        return Ok(Defaults {
            format: FORMAT.to_owned(),
            choices: Vec::new(),
        });
    };
    let kept: Defaults = serde_json::from_slice(&bytes).map_err(|error| {
        refused(
            "launch_defaults_unreadable",
            format!("the recorded harness choices do not match their stored shape; no default was selected; JSON {:?} at {}:{}", error.classify(), error.line(), error.column()),
        )
    })?;
    let mut unique = BTreeSet::new();
    if kept.format != FORMAT
        || kept.choices.iter().any(|choice| {
            [
                &choice.server,
                &choice.person,
                &choice.machine,
                &choice.harness,
                &choice.agent,
            ]
            .iter()
            .any(|value| value.is_empty() || value.bytes().any(|byte| byte.is_ascii_control()))
                || !unique.insert((
                    &choice.server,
                    &choice.person,
                    &choice.machine,
                    &choice.harness,
                ))
        })
    {
        return Err(refused(
            "launch_defaults_unreadable",
            "the harness choices hold an unknown format, an empty value or duplicate scope; no default was selected",
        ));
    }
    Ok(kept)
}

fn matches(choice: &Choice, scope: &Scope<'_>, harness: &str) -> bool {
    choice.server == scope.server
        && choice.person == scope.person
        && choice.machine == scope.machine
        && choice.harness == harness
}

/// Read only the explicit choice for this person, server, device and harness.
///
/// # Errors
/// Names malformed or insecure state. An older install with no choices returns no default.
pub fn chosen(root: &Path, scope: &Scope<'_>, harness: &str) -> CliResult<Option<String>> {
    Ok(load(root)?
        .choices
        .into_iter()
        .find(|choice| matches(choice, scope, harness))
        .map(|choice| choice.agent))
}

/// Keep an already validated agent choice as one durable file replacement.
///
/// # Errors
/// Names invalid values and every failed read or durable write.
pub fn set(root: &Path, scope: &Scope<'_>, harness: &str, agent: &str) -> CliResult<()> {
    if [scope.server, scope.person, scope.machine, harness, agent]
        .iter()
        .any(|value| value.is_empty() || value.bytes().any(|byte| byte.is_ascii_control()))
    {
        return Err(refused(
            "launch_default_invalid",
            "a harness default requires a server, person, machine, harness and agent",
        ));
    }
    let mut kept = load(root)?;
    kept.choices
        .retain(|choice| !matches(choice, scope, harness));
    kept.choices.push(Choice {
        server: scope.server.to_owned(),
        person: scope.person.to_owned(),
        machine: scope.machine.to_owned(),
        harness: harness.to_owned(),
        agent: agent.to_owned(),
    });
    let bytes = serde_json::to_vec(&kept).map_err(|error| {
        refused(
            "launch_defaults_unwritable",
            format!(
                "the harness choices could not be encoded; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })?;
    private_files::ensure_dir(&root.join("client"))?;
    private_files::write(&path(root), &bytes)?;
    Ok(())
}

#[cfg(test)]
#[path = "launch_defaults_tests.rs"]
mod tests;
