//! Validate each provisioning value before its version is recorded.

use super::{SetBody, malformed};
use crate::error::ServerError;
use crate::launch_permissions::checked;
use crate::provisioning_store::{SessionSettings, Settings};
use lys_home::harness::launch_fields::{DeclaredHarness, SeatFields};
use lys_identity::OperationId;
use std::str::FromStr;

fn named(name: &str, given: &str) -> Result<String, ServerError> {
    let given = given.trim();
    if given.is_empty() {
        return Err(malformed(format!("a name in {name} is empty")));
    }
    Ok(given.to_owned())
}

fn names(name: &str, given: &[String]) -> Result<Vec<String>, ServerError> {
    let mut kept = Vec::new();
    for one in given {
        let one = named(name, one)?;
        if !kept.contains(&one) {
            kept.push(one);
        }
    }
    Ok(kept)
}

/// `given` without repeats, each `name@marketplace`, for a harness whose
/// launch writes them into its settings file; Codex has no such file.
fn plugins(
    given: &[String],
    harness: Option<&DeclaredHarness>,
) -> Result<Vec<String>, ServerError> {
    let mut kept: Vec<String> = Vec::new();
    for one in given {
        if !lys_home::harness::claude_code::template::plugin_named(one) {
            return Err(malformed(format!(
                "plugins: `{one}` is not name@marketplace"
            )));
        }
        if !kept.contains(one) {
            kept.push(one.clone());
        }
    }
    let renders = harness.is_some_and(|declared| {
        declared.description.rendering_contract == "claude-code/template-v1"
    });
    if !kept.is_empty() && !renders {
        return Err(malformed(
            "plugins require a declared Claude Code harness, whose settings file turns them on",
        ));
    }
    Ok(kept)
}

pub(super) fn settings(body: SetBody) -> Result<Settings, ServerError> {
    let declared = body.harness.map(harness).transpose()?;
    let permissions = body
        .permissions
        .map(|given| {
            let contract = declared
                .as_ref()
                .ok_or_else(|| malformed("permissions require harness.description.permissions"))?;
            checked(given, &contract.description.permissions)
        })
        .transpose()?;
    let settings = Settings {
        model_access: names("model_access", &body.model_access)?,
        tools: names("tools", &body.tools)?,
        skills: names("skills", &body.skills)?,
        mcp_servers: crate::mcp_record::servers(body.mcp_servers)?,
        plugins: plugins(&body.plugins, declared.as_ref())?,
        instructions: body.instructions.trim().to_owned(),
        instructions_mode: body.instructions_mode,
        note: body.note.trim().to_owned(),
        session: body
            .session
            .clone()
            .map(|given| session(given, declared.as_ref()))
            .transpose()?,
        harness: declared,
        skill_pins: Vec::new(),
        permissions,
        runs_on: body
            .runs_on
            .map(|id| {
                OperationId::from_str(&id)
                    .map(|machine| machine.to_string())
                    .map_err(|error| malformed(format!("runs_on is not a machine id: {error}")))
            })
            .transpose()?,
        writable: body
            .writable
            .map(|given| folder("writable", given))
            .transpose()?,
        working_folder: body
            .working_folder
            .map(|given| folder("working_folder", given))
            .transpose()?,
    };
    if let Some(declared) = &settings.harness {
        crate::launch_fields::models(declared, &settings.model_access)?;
        crate::launch_fields::mcp(declared, &settings.mcp_servers)?;
    }
    Ok(settings)
}

/// `given`, the folder named under `name`, when it is an absolute plain path:
/// no relative part, no doubled or trailing separator, no `.` or `..`.
pub(crate) fn folder(name: &str, given: String) -> Result<String, ServerError> {
    use std::path::{Component, Path};
    let path = Path::new(&given);
    if !path.is_absolute()
        || path.parent().is_none()
        || given.contains('\0')
        || given.contains("//")
        || given.contains("/./")
        || given.ends_with("/.")
        || given.ends_with('/')
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(malformed(format!(
            "{name} is not an absolute plain folder path"
        )));
    }
    Ok(given)
}

/// The declared build, refused when its program is not an absolute path or
/// it names no package to verify the program against.
fn harness(declared: DeclaredHarness) -> Result<DeclaredHarness, ServerError> {
    if !declared.program.starts_with('/') || declared.program.contains('\0') {
        return Err(malformed("harness.program is not an absolute path"));
    }
    if declared.name.trim().is_empty() {
        return Err(malformed("harness.name is empty"));
    }
    if declared.description.rendering_contract.trim().is_empty() {
        return Err(malformed("harness.description.rendering_contract is empty"));
    }
    if declared.package.trim().is_empty() {
        return Err(malformed("harness.package names no package"));
    }
    Ok(declared)
}

/// The session settings, each refused by name when a runner could not use it.
fn session(
    settings: SessionSettings,
    harness: Option<&DeclaredHarness>,
) -> Result<SessionSettings, ServerError> {
    let seat = SeatFields {
        resume: settings.resume.clone(),
        context_window: settings.context_window,
        config_dir: settings.config_dir.clone(),
        ..SeatFields::default()
    };
    seat.check(harness.map_or("", |declared| {
        declared.description.rendering_contract.as_str()
    }))
    .map_err(|error| malformed(format!("session.{error}")))?;
    if settings
        .compact
        .as_deref()
        .is_some_and(|line| line.trim().is_empty())
    {
        return Err(malformed(
            "session.compact is empty: name the command or leave it out",
        ));
    }
    if let Some(accounts) = &settings.accounts {
        lys_runner::rotation::RotationState::new(accounts.clone())
            .map_err(|refused| malformed(format!("session.accounts: {refused}")))?;
    }
    Ok(settings)
}
