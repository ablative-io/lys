//! Personal launches ask the server to render and start its reviewed agent.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::cli::{AgentsArgs, HarnessArgs};
use crate::commands::error::{CliError, CliResult};
use crate::commands::launch_catalog::{self, Agent};
use crate::commands::launch_context::{self, Context, surface};
use crate::commands::launch_defaults::{self, Scope};
use crate::commands::launch_device;
use crate::commands::launch_terminal;
use crate::commands::seat_client::{operation, refused, segment};

#[path = "launch/family.rs"]
mod family;
#[path = "launch/managed.rs"]
mod managed;

#[derive(Deserialize)]
struct Started {
    agent: String,
    machine: String,
    session: String,
    provisioning_version: u32,
    harness: String,
    template: String,
    template_sha256: String,
    runner: Running,
}

#[derive(Deserialize)]
struct Running {
    session: String,
    state: String,
}

fn scope<'a>(context: &'a Context, machine: &'a str) -> Scope<'a> {
    Scope {
        server: &context.base,
        person: &context.person,
        machine,
    }
}

fn name(error: &CliError) -> String {
    match error {
        CliError::Runner(error) => error.name(),
        CliError::Identity(error) => error.kind().name().to_owned(),
        _ => "launch_failed".to_owned(),
    }
}

fn action(context: &Context, command: &str, error: CliError) -> CliError {
    let reason = name(&error);
    let words = match error {
        CliError::Runner(lys_runner::RunnerError::Refused { words, .. }) => words,
        other => other.to_string(),
    };
    if words.contains("; run `") || words.contains("; open ") || words.contains("; inspect ") {
        return refused(&reason, words);
    }
    let fix = match reason.as_str() {
        "agent_for_harness_missing" | "agent_unknown" => format!(
            "open {} and add your {command} agent on this computer",
            surface(&context.base, "agents/new")
        ),
        "agent_default_missing" => format!(
            "run `lys agents --harness {} --default <agent-name>`",
            shell_word(command)
        ),
        "agent_name_ambiguous" | "agent_harness_mismatch" | "agent_default_unavailable" => {
            "run `lys agents` and choose an unambiguous matching id with `--agent`".to_owned()
        }
        "launch_command_invalid" => {
            "run `lys agents --json` and use a listed declared program".to_owned()
        }
        "person_pass_expired"
        | "person_pass_wrong_server"
        | "person_sign_in_refused"
        | "Unauthenticated"
        | "NotSignedIn" => format!("run `lys sign-in --server {}`", context.base),
        "machine_not_joined"
        | "machine_retired"
        | "MachineRetired"
        | "MachineNotForAgent"
        | "runner_not_dialled_in"
        | "runner_unreachable"
        | "MachineWithoutRunner" => format!(
            "open {} > Computer > Connect",
            surface(&context.base, "network")
        ),
        "launch_arguments_unavailable" => format!(
            "open {} to set supported harness choices, then launch without extra arguments",
            surface(&context.base, "agents")
        ),
        _ => format!(
            "open {} and correct the named refusal on the agent's page",
            surface(&context.base, "agents")
        ),
    };
    refused(&reason, format!("{words}; {fix}"))
}

fn shell_word(value: &str) -> String {
    lys_home::harness::rendering::shell_word(value)
}

/// Launch a harness or reviewed command as the signed-in person on this device.
///
/// # Errors
/// Names sign-in, device, profile, server and terminal failures with a next act.
pub fn run(command: &str, args: &HarnessArgs, json: bool) -> CliResult<()> {
    let context = launch_context::read(args.options.server.as_deref())?;
    launch(&context, command, args, json).map_err(|error| action(&context, command, error))
}

fn launch(context: &Context, command: &str, args: &HarnessArgs, json: bool) -> CliResult<()> {
    valid_command(command)?;
    if json {
        return Err(refused(
            "launch_terminal_required",
            "an interactive launch cannot emit JSON; remove `--json`",
        ));
    }
    arguments(&args.arguments)?;
    let machine = launch_device::find(&context.server, &context.root, &context.base)?;
    let agents = launch_catalog::read(&context.server, &context.person, &machine)?;
    let default = if args.options.agent.is_none() {
        launch_defaults::chosen(
            &context.root,
            &scope(context, &machine),
            family::for_command(&agents, command)?,
        )?
    } else {
        None
    };
    let selected = launch_catalog::select(
        &agents,
        command,
        args.options.agent.as_deref(),
        default.as_deref(),
    )?;
    ready(selected)?;
    let seat = selected
        .controlled
        .then(|| managed::select(selected, &machine, &context.base))
        .transpose()?;
    let terminal = launch_terminal::prepare()?;
    let session = operation()?;
    let agent = segment("agent_id_invalid", "the agent id", &selected.id)?;
    eprintln!(
        "starting {} on this computer through Lys; session {session}",
        selected.name
    );
    let (route, body) = match seat {
        Some(seat) => (
            format!(
                "/seats/{}/start",
                segment("seat_name_invalid", "the selected seat", &seat.name)?
            ),
            json!({"operation":session}),
        ),
        None => (
            format!("/agents/{agent}/start-command"),
            json!({"machine":machine,"operation":session}),
        ),
    };
    let answer = context.server.post(&route, &body);
    let answer = match answer {
        Ok(answer) => answer,
        Err(error) if uncertain(&error) => return cleanup(context, &session, error),
        Err(error) => return Err(error),
    };
    let validated = match seat {
        Some(seat) => managed::start(answer, selected, seat, &machine, &session),
        None => Ok(answer),
    }
    .and_then(|answer| started(answer, selected, &machine, &session));
    if let Err(error) = validated {
        return cleanup(context, &session, error);
    }
    eprintln!(
        "terminal: {}; usage: {}",
        surface(&context.base, &format!("canvas/{agent}")),
        surface(&context.base, &format!("canvas?view=proxy&agent={agent}"))
    );
    match seat {
        Some(seat) => {
            launch_terminal::attach_managed(&context.server, &seat.name, &session, terminal)
        }
        None => launch_terminal::attach(&context.server, &session, terminal),
    }
}

fn uncertain(error: &CliError) -> bool {
    matches!(
        name(error).as_str(),
        "identity_server_answer_unreadable"
            | "identity_server_outcome_uncertain"
            | "runner_dial_failed"
            | "runner_unreachable"
            | "runner_reply_malformed"
            | "RuntimeUnavailable"
    )
}

fn cleanup(context: &Context, session: &str, error: CliError) -> CliResult<()> {
    match launch_terminal::end(&context.server, session) {
        Ok(()) => Err(error),
        Err(ending) => Err(refused(
            "launch_cleanup_failed",
            format!(
                "{error}; ending session {session} also failed: {ending}; open {}",
                surface(&context.base, "canvas?view=sessions")
            ),
        )),
    }
}

fn arguments(given: &[String]) -> CliResult<()> {
    if given.is_empty() {
        return Ok(());
    }
    Err(refused(
        "launch_arguments_unavailable",
        "this server's start request has no argument field; no arguments were discarded and no session was started",
    ))
}

fn valid_command(command: &str) -> CliResult<()> {
    if lys_home::harness::rendering_launch::command_or_absolute_path(command) {
        return Ok(());
    }
    Err(refused(
        "launch_command_invalid",
        "name one plain command or an absolute executable path; no launch request was sent",
    ))
}

fn ready(agent: &Agent) -> CliResult<()> {
    if agent.state != "active" {
        return Err(refused(
            "AgentNotActive",
            format!("agent {} is {}", agent.name, agent.state),
        ));
    }
    if !agent.reviewed {
        return Err(refused(
            "ProfileNotReviewed",
            "the selected agent's latest profile has not been reviewed",
        ));
    }
    family::of(agent)?;
    if !agent.controlled && agent.working_folder.is_none() {
        return Err(refused(
            "WorkingFolderUnnamed",
            "the agent's profile names no working folder",
        ));
    }
    Ok(())
}

fn started(answer: Value, selected: &Agent, machine: &str, session: &str) -> CliResult<()> {
    let reply: Started = serde_json::from_value(answer).map_err(|error| {
        refused(
            "launch_answer_unreadable",
            format!(
                "Lys did not answer in its recorded start shape; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })?;
    if reply.agent != selected.id
        || reply.machine != machine
        || reply.session != session
        || reply.runner.session != session
    {
        return Err(refused(
            "launch_answer_mismatch",
            "Lys's answer does not name the requested agent, machine and session",
        ));
    }
    if Some(reply.provisioning_version) != selected.profile_version {
        return Err(refused(
            "launch_profile_changed",
            "the profile changed after selection; review the current profile before starting again",
        ));
    }
    if !matches!(reply.runner.state.as_str(), "running" | "ended") {
        return Err(refused(
            "launch_runner_unconfirmed",
            "Lys did not confirm the runner started or ended the requested process",
        ));
    }
    let template: Value = serde_json::from_str(&reply.template).map_err(|error| {
        refused(
            "launch_answer_unreadable",
            format!(
                "the kept launch template does not read; JSON {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        )
    })?;
    if lys_home::record::blocks::Hash::of(reply.template.as_bytes()).as_str()
        != reply.template_sha256
    {
        return Err(refused(
            "launch_template_mismatch",
            "the server's launch template does not match its recorded hash",
        ));
    }
    let (harness, proxy_path, run_path) = match selected.harness.as_deref() {
        Some("claude") => (
            "claude-code",
            "/slots/env/ANTHROPIC_BASE_URL",
            "/slots/env/LYS_RUN",
        ),
        Some("codex") => ("codex", "/fields/model_proxy", "/fields/run"),
        _ => {
            return Err(refused(
                "launch_template_unsupported",
                "the selected harness has no recorded proxy validation contract",
            ));
        }
    };
    if reply.harness != harness || template["harness"].as_str() != Some(harness) {
        return Err(refused(
            "launch_harness_mismatch",
            "the server supplied another harness's launch template",
        ));
    }
    if !template
        .pointer(run_path)
        .and_then(Value::as_str)
        .is_some_and(lys_home::proxy::usage::is_run_key)
    {
        return Err(refused(
            "launch_run_missing",
            "the server supplied no valid per-run proxy identity",
        ));
    }
    let proxy = template
        .pointer(proxy_path)
        .and_then(Value::as_str)
        .filter(|base| !base.is_empty())
        .ok_or_else(|| {
            refused(
                "model_proxy_missing",
                "the server supplied no model proxy for this run; the session is being ended",
            )
        })?;
    crate::commands::sign_in::base(proxy)?;
    Ok(())
}

/// List the person's agents admitted on this device and explicitly record a default.
///
/// # Errors
/// Names sign-in, device, catalog, selection and durable default-write failures.
pub fn agents(args: &AgentsArgs, json_output: bool) -> CliResult<()> {
    let context = launch_context::read(args.server.as_deref())?;
    list(&context, args, json_output)
        .map_err(|error| action(&context, args.harness.as_deref().unwrap_or("agent"), error))
}

fn list(context: &Context, args: &AgentsArgs, json_output: bool) -> CliResult<()> {
    if let Some(harness) = &args.harness {
        valid_command(harness)?;
    }
    let machine = launch_device::listed(&context.server, &context.root, &context.base)?;
    let agents = launch_catalog::read(&context.server, &context.person, &machine)?;
    let scope = scope(context, &machine);
    if let Some(chosen) = &args.default {
        let harness = args.harness.as_deref().ok_or_else(|| {
            refused(
                "launch_default_invalid",
                "a default requires `--harness <command>`",
            )
        })?;
        let agent = launch_catalog::select(&agents, harness, Some(chosen), None)?;
        launch_defaults::set(&context.root, &scope, family::of(agent)?, &agent.id)?;
    }
    let mut rows = Vec::new();
    let mut human = Vec::new();
    for agent in &agents {
        let matches = args.harness.as_deref().is_none_or(|harness| {
            agent.harness.as_deref() == Some(harness) || agent.program.as_deref() == Some(harness)
        });
        if !matches {
            continue;
        }
        let harness = agent.harness.as_deref();
        let default = match harness {
            Some(harness) => {
                launch_defaults::chosen(&context.root, &scope, harness)?.as_deref()
                    == Some(agent.id.as_str())
            }
            None => false,
        };
        rows.push(json!({"id":agent.id, "name":agent.name, "harness":agent.harness, "program":agent.program,
            "state":agent.state, "runtime":agent.runtime_state(), "profile_version":agent.profile_version,
            "reviewed":agent.reviewed, "managed":agent.controlled, "default":default, "reasons":agent.unknown}));
        human.push((agent, default));
    }
    if json_output {
        println!(
            "{}",
            json!({"ok":true,"person":context.person,"machine":machine,"agents":rows})
        );
    } else if rows.is_empty() {
        println!(
            "no agents on this machine; add one at {}",
            surface(&context.base, "agents/new")
        );
    } else {
        for (agent, default) in human {
            println!(
                "{}  {}  {}  {}  {}{}",
                agent.name,
                agent.id,
                agent.harness.as_deref().unwrap_or("harness undeclared"),
                agent.state,
                agent.runtime_state(),
                if default { "  default" } else { "" }
            );
            for reason in &agent.unknown {
                println!("  {reason}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
