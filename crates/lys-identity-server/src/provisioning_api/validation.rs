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
        hooks: body.hooks,
        status_line: body.status_line,
        settings: body.settings,
        resume: body.resume,
        context_window: body.context_window,
        config_dir: body.config_dir,
        instructions: body.instructions.trim().to_owned(),
        instructions_mode: body.instructions_mode,
        note: body.note.trim().to_owned(),
        session: body
            .session
            .clone()
            .map(|given| session(given.settings, declared.as_ref()))
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
    let seat = settings.seat_fields().map_err(malformed)?;
    if settings.harness.as_ref().is_some_and(|harness| {
        harness.description.rendering_contract == lys_home::harness::seat_program::CONTRACT
    }) {
        lys_home::harness::seat_program::check_seat(&seat).map_err(malformed)?;
        if settings
            .session
            .as_ref()
            .and_then(|session| session.accounts.as_ref())
            .is_some()
            && settings
                .harness
                .as_ref()
                .and_then(|h| h.description.seat_program.as_ref())
                .and_then(|s| s.model_wire)
                .is_none()
        {
            return Err(ServerError::SecretsRefused {
                status: axum::http::StatusCode::BAD_REQUEST,
                refusal: "ModelWireMissing".to_owned(),
                reason: "seat_program.model_wire must name openai-responses or anthropic-messages when session.accounts is set".to_owned(),
            });
        }
        if settings.permissions.is_some() {
            return Err(malformed(
                "permissions: native confinement is unavailable for seat-program/v1",
            ));
        }
    }
    seat.check(settings.harness.as_ref().map_or("", |declared| {
        declared.description.rendering_contract.as_str()
    }))
    .map_err(|error| malformed(error.to_string()))?;
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

fn checked_program(program: &str) -> Result<(), ServerError> {
    if lys_home::harness::rendering_launch::command_or_absolute_path(program) {
        Ok(())
    } else {
        Err(malformed(lys_home::harness::rendering_launch::PROGRAM_RULE))
    }
}

/// The declared build, refused when its program is neither a plain command
/// the computer resolves at every start (`claude`, `codex`, `cdx`) nor an
/// absolute path, or it names no package to verify the program against.
fn harness(declared: DeclaredHarness) -> Result<DeclaredHarness, ServerError> {
    checked_program(&declared.program)?;
    if declared.description.rendering_contract == lys_home::harness::seat_program::CONTRACT {
        let seat = declared
            .description
            .seat_program
            .as_ref()
            .ok_or_else(|| malformed("harness.description.seat_program is missing"))?;
        seat.check().map_err(malformed)?;
        if seat.executable != declared.program {
            return Err(malformed(
                "harness.program differs from seat_program.executable",
            ));
        }
    } else if declared.description.seat_program.is_some() {
        return Err(malformed(
            "seat_program requires the seat-program/v1 rendering contract",
        ));
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

pub(super) async fn on_machine(
    state: &std::sync::Arc<crate::routes::AppState>,
    body: &SetBody,
) -> Result<(), ServerError> {
    let (Some(machine), Some(harness)) = (&body.runs_on, &body.harness) else {
        return Ok(());
    };
    checked_program(&harness.program)?;
    let command = crate::launch_template::launch_command(&harness.program);
    if std::path::Path::new(&command).is_absolute() {
        return Ok(());
    }
    let found = crate::runner_client::programs(state, machine).await?;
    let adapter = match harness.description.rendering_contract.as_str() {
        "claude-code/template-v1" => "claude-stream-json/1",
        "codex/template-v1" => "codex-app-server/1",
        _ => "",
    };
    if found.programs.iter().any(|program| {
        crate::launch_template::launch_command(match program.command.as_deref() {
            Some(command) => command,
            None => &program.program,
        }) == command
            && program.adapter == adapter
    }) {
        return Ok(());
    }
    Err(ServerError::Runner {
        refusal: "harness_program_unknown".to_owned(),
        words: format!(
            "computer {machine} did not report {command}; {} It reported: {}",
            lys_home::harness::rendering_launch::PROGRAM_RULE,
            found
                .programs
                .iter()
                .map(|program| format!(
                    "{} version {} ({})",
                    program.program, program.version, program.adapter
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    })
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
