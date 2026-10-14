//! Process inputs bind native configuration to the session-owned directory.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::launch_template::{Template, parse};
use crate::harness::launch_fields::{InstructionsMode, LaunchFields, LaunchMcp, Transport};
use crate::harness::rendering_launch::{File, Launch};
use crate::harness::skills::check;
use crate::record::blocks::Hash;

fn file(path: &str, text: String) -> File {
    File {
        path: path.to_owned(),
        sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
        text,
    }
}

fn native_value(value: &Value) -> Result<String, String> {
    match value {
        Value::String(_) | Value::Bool(_) | Value::Number(_) => serde_json::to_string(value)
            .map(|encoded| encoded.replace('\u{7f}', "\\u007f"))
            .map_err(|error| format!("config: cannot encode native value: {error}")),
        Value::Array(items) => Ok(format!(
            "[{}]",
            items
                .iter()
                .map(native_value)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        Value::Object(members) => {
            let entries = members
                .iter()
                .map(|(key, value)| {
                    Ok(format!(
                        "{} = {}",
                        native_value(&json!(key))?,
                        native_value(value)?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(format!("{{{}}}", entries.join(", ")))
        }
        Value::Null => Err("config: native values cannot be null".to_owned()),
    }
}

fn runner_path(environment: &BTreeMap<String, String>, name: &str) -> Result<String, String> {
    match environment.get(name) {
        Some(value)
            if std::path::Path::new(value).is_absolute()
                && !value.chars().any(char::is_control) =>
        {
            Ok(value.clone())
        }
        Some(_) => Err(format!(
            "environment.{name}: must be an absolute plain path"
        )),
        None => Ok(format!("${name}")),
    }
}

pub(crate) fn render(program: &str, text: &str, mode: InstructionsMode) -> Result<Launch, String> {
    let template = parse(text)?;
    let servers = server_values(&template.fields.mcp_servers);
    let mut files = Vec::new();
    let mut argument_files = BTreeMap::new();
    for skill in &template.skills {
        let held = check(skill).map_err(|error| error.to_string())?;
        files.push(file(&held.path, skill.text.clone()));
    }
    // Each profile server augments the native setup rather than replacing its table.
    let mut arguments = vec!["--model".to_owned(), template.fields.models[0].clone()];
    native_settings(&template, &servers, &mut arguments)?;
    let program_path = runner_path(
        &template.environment,
        crate::harness::rendering::PROGRAM_VARIABLE,
    )?;
    let socket = runner_path(
        &template.environment,
        crate::harness::rendering::SOCKET_VARIABLE,
    )?;
    arguments.extend([
        "-c".to_owned(),
        format!(
            "notify={}",
            native_value(&json!([
                program_path,
                "runner",
                "notify",
                "--socket",
                socket
            ]))?
        ),
    ]);
    model_proxy_arguments(&template.fields, &mut arguments)?;
    if let Some(sandbox) = template.permissions.default_mode {
        if sandbox == "workspace-write" {
            // Command-line settings keep project configuration from widening the boundary.
            for setting in [
                "sandbox_workspace_write.network_access=false",
                "sandbox_workspace_write.writable_roots=[]",
                "sandbox_workspace_write.exclude_tmpdir_env_var=true",
                "sandbox_workspace_write.exclude_slash_tmp=true",
                "web_search=\"disabled\"",
            ] {
                arguments.extend(["-c".to_owned(), setting.to_owned()]);
            }
        }
        arguments.extend(["--sandbox".to_owned(), sandbox]);
    }
    for path in template.permissions.additional_directories {
        arguments.extend(["--add-dir".to_owned(), path]);
    }
    let mut environment = template.environment;
    if let Some(directory) = template.seat.config_dir {
        environment.insert("CODEX_HOME".to_owned(), directory.clone());
        arguments.extend(["--add-dir".to_owned(), directory]);
    }
    if !template.skills.is_empty() {
        arguments.push("--add-dir".to_owned());
        argument_files.insert(arguments.len(), String::new());
        arguments.push(String::new());
    }
    match mode {
        InstructionsMode::Append if !template.fields.instructions.trim().is_empty() => {
            arguments.extend([
                "-c".to_owned(),
                format!(
                    "developer_instructions={}",
                    native_value(&json!(template.fields.instructions))?
                ),
            ]);
        }
        InstructionsMode::Replace => {
            if template.fields.instructions.trim().is_empty() {
                return Err("instructions: Replace requires nonblank text".to_owned());
            }
            files.push(file("instructions.txt", template.fields.instructions));
            arguments.extend([
                "-c".to_owned(),
                "model_instructions_file=\"instructions.txt\"".to_owned(),
            ]);
            argument_files.insert(arguments.len() - 1, "instructions.txt".to_owned());
        }
        InstructionsMode::Keep | InstructionsMode::Append => {}
    }
    if let Some(resume) = template.seat.resume {
        arguments.extend(["resume".to_owned(), resume]);
    }
    Ok(Launch {
        program: program.to_owned(),
        arguments,
        environment,
        files,
        argument_files,
        environment_paths: BTreeMap::new(),
    })
}

fn server_values(servers: &[LaunchMcp]) -> serde_json::Map<String, Value> {
    let mut values = serde_json::Map::new();
    for server in servers {
        let mut value = match &server.transport {
            Transport::Http { url } => json!({"url": url}),
            Transport::Stdio {
                program,
                args,
                cwd,
                env,
                handles,
            } => {
                let variables: BTreeMap<_, _> = env
                    .iter()
                    .map(|one| (one.name.clone(), one.text.clone()))
                    .chain(
                        handles
                            .iter()
                            .map(|one| (one.name.clone(), one.handle_id.clone())),
                    )
                    .collect();
                let mut value = json!({"command": program, "args": args, "env": variables});
                if let Some(cwd) = cwd {
                    value["cwd"] = json!(cwd);
                }
                value
            }
        };
        value["channel"] = json!(server.channel);
        values.insert(server.name.clone(), value);
    }
    values
}

fn native_settings(
    template: &Template,
    servers: &serde_json::Map<String, Value>,
    arguments: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(settings) = &template.seat.settings {
        for (key, value) in settings {
            if key == "notify"
                || (template.fields.model_proxy.is_some()
                    && matches!(key.as_str(), "model_provider" | "model_providers"))
            {
                continue;
            }
            if key.is_empty()
                || !key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            {
                return Err(format!("settings.{key}: is not a bare native config key"));
            }
            arguments.extend([
                "-c".to_owned(),
                format!(
                    "{key}={}",
                    native_value(value).map_err(|error| format!("settings.{key}: {error}"))?
                ),
            ]);
        }
    }
    for (name, value) in servers {
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(format!(
                "mcp_servers: `{name}` is not a bare TOML key; use letters, digits, _ and -"
            ));
        }
        arguments.extend([
            "-c".to_owned(),
            format!("mcp_servers.{name}={}", native_value(value)?),
        ]);
    }
    Ok(())
}

fn model_proxy_arguments(fields: &LaunchFields, arguments: &mut Vec<String>) -> Result<(), String> {
    if let Some(proxy) = &fields.model_proxy {
        // Codex is pointed at Lys's proxy as a provider of its own, signed
        // in as the machine's Codex is, over plain HTTP event streams: the
        // proxy reads a call as it passes, and a WebSocket it cannot read.
        // (Measured on codex-cli 0.159.2, 4 Oct 2026: OPENAI_BASE_URL in the
        // environment is ignored; the built-in provider opens a WebSocket.)
        let base = crate::harness::rendering::openai_base(proxy, fields.run.as_deref())
            .ok_or_else(|| {
                "model_proxy: the model proxy's address has no host a Codex run can be given"
                    .to_owned()
            })?;
        arguments.extend([
            "-c".to_owned(),
            format!(
                "model_providers.lys={{name=\"OpenAI through Lys\",base_url={},wire_api=\"responses\",requires_openai_auth=true,supports_websockets=false}}",
                native_value(&json!(base))?
            ),
            "-c".to_owned(),
            "model_provider=\"lys\"".to_owned(),
        ]);
    }
    Ok(())
}
