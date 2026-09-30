//! Process inputs bind native configuration to the session-owned directory.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::launch_template::parse;
use crate::harness::launch_fields::{InstructionsMode, Transport};
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

pub(crate) fn render(program: &str, text: &str, mode: InstructionsMode) -> Result<Launch, String> {
    let template = parse(text)?;
    let mut servers = serde_json::Map::new();
    for server in &template.fields.mcp_servers {
        let value = match &server.transport {
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
        servers.insert(server.name.clone(), value);
    }
    let config = format!("mcp_servers = {}\n", native_value(&Value::Object(servers))?);
    let mut files = vec![file("config.toml", config)];
    for skill in &template.skills {
        let held = check(skill).map_err(|error| error.to_string())?;
        files.push(file(&held.path, skill.text.clone()));
    }
    let mut arguments = vec![
        "--model".to_owned(),
        template.fields.models[0].clone(),
        "--ask-for-approval".to_owned(),
        "on-request".to_owned(),
        "-C".to_owned(),
        ".".to_owned(),
    ];
    if let Some(sandbox) = template.permissions.default_mode {
        arguments.extend(["--sandbox".to_owned(), sandbox]);
    }
    for path in template.permissions.additional_directories {
        arguments.extend(["--add-dir".to_owned(), path]);
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
        }
        InstructionsMode::Keep | InstructionsMode::Append => {}
    }
    Ok(Launch {
        program: program.to_owned(),
        arguments,
        environment: template.environment,
        files,
        argument_files: BTreeMap::new(),
        environment_paths: BTreeMap::from([("CODEX_HOME".to_owned(), String::new())]),
    })
}
