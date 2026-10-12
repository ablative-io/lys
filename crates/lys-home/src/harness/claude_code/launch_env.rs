//! The environment file of a launch (HOME-002 R6): a Claude Code settings
//! file whose member is `env`, holding each of the template's variables
//! with its value and each use-only secret's variable with its handle, and
//! the template's `permissions` beside it when it sets them (HOME-037 R5). No
//! secret's value is read, the process environment is not read, and a
//! readable secret never reaches here (the parser refuses it). Keys are
//! written in sorted order, so one template writes one sequence of bytes.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde_json::json;

use crate::error::HomeError;
use crate::harness::claude_code::template::Template;
use crate::record::blocks::Hash;

/// Refuse a template whose own flags would load the person's own Claude Code
/// setup again. Every Lys start carries `--setting-sources=` (see
/// `rendering_launch`), and a later `--setting-sources` with a value, or the
/// two-word form whose value is the next flag, would take its place: the
/// person's settings, hooks and plugins would be back while the screens say
/// they are left out. The empty one-word form is the one Lys writes itself
/// and is admitted.
///
/// # Errors
/// `TemplateShape` naming `flags`.
pub fn own_setup_left_out(template: &Template) -> Result<(), HomeError> {
    let asks = template.flags.iter().map(String::as_str).any(|flag| {
        flag == "--setting-sources"
            || (flag.starts_with("--setting-sources=") && flag != "--setting-sources=")
    });
    if asks {
        return Err(HomeError::TemplateShape {
            field: "flags".to_owned(),
            reason: "names --setting-sources: a Lys start does not load the person's own Claude Code setup",
        });
    }
    Ok(())
}

/// Refuse a template that does not say how the agent is confined: a
/// permissions slot that is absent, names no `defaultMode`, or names an empty
/// one. Checked before anything is written, so nothing is ever rendered
/// unconfined and the operator's file is never rewritten.
///
/// # Errors
/// `TemplateShape` naming `slots.permissions.defaultMode`.
pub fn confinement_named(template: &Template) -> Result<(), HomeError> {
    if template
        .permissions
        .as_ref()
        .and_then(|permissions| permissions.get("defaultMode"))
        .and_then(serde_json::Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(HomeError::TemplateShape {
            field: "slots.permissions.defaultMode".to_owned(),
            reason: "names no mode: choose how this agent is confined",
        });
    }
    Ok(())
}

/// The settings file's bytes: `{"env": {...}}`, keys sorted, with the
/// template's `permissions` beside `env` when it sets them, and the
/// `statusLine` every run is given; one trailing newline.
pub fn env_settings(template: &Template) -> Result<Vec<u8>, HomeError> {
    settings(template, None)
}

fn settings_value(
    template: &Template,
    judge: Option<&Judge>,
) -> Result<serde_json::Value, HomeError> {
    let mut seat = crate::harness::rendering::filtered_seat(&template.seat);
    seat.check("claude-code/template-v1")
        .map_err(|error| HomeError::TemplateShape {
            field: format!("slots.{}", error.field),
            reason: error.reason,
        })?;
    let mut env: BTreeMap<&str, &str> = template
        .env
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    for secret in &template.use_only {
        env.insert(secret.env.as_str(), secret.handle.as_str());
    }
    let mut value = serde_json::Value::Object(match seat.settings.take() {
        Some(settings) => settings,
        None => serde_json::Map::new(),
    });
    value["env"] = json!(env);
    value["statusLine"] = json!({
        "type": "command",
        "command": crate::harness::rendering::status_line_command(seat.status_line.as_deref()),
    });
    if let Some(permissions) = &template.permissions {
        value["permissions"] = serde_json::Value::Object(permissions.clone());
        if permissions
            .get("defaultMode")
            .and_then(serde_json::Value::as_str)
            == Some("workspace-only")
        {
            workspace_settings(&mut value)?;
        }
    }
    if !template.plugins.is_empty() {
        value["enabledPlugins"] = template
            .plugins
            .iter()
            .map(|name| (name.clone(), serde_json::Value::Bool(true)))
            .collect::<serde_json::Map<_, _>>()
            .into();
    }
    let mut hooks: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    if judge.is_some_and(|judge| judge.collect) {
        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "Stop",
            "SessionEnd",
            "PreCompact",
            "PostCompact",
            "PostToolUse",
            "PostToolUseFailure",
        ] {
            let mut entry = json!({"hooks": [{
                "type": "command",
                "command": r#""$LYS_PROGRAM" runner hook --socket "$LYS_RUNNER_SOCKET""#,
                "timeout": 5
            }]});
            if !matches!(event, "UserPromptSubmit" | "Stop") {
                entry["matcher"] = json!("*");
            }
            hooks.insert(event.to_owned(), vec![entry]);
        }
    }
    if let Some(judge) = judge {
        hooks.insert(
            "PreToolUse".to_owned(),
            vec![json!({
                "matcher": "*", "hooks": [{"type": "command", "command": judge.command()}]
            })],
        );
    }
    if let Some(own) = &seat.hooks {
        for (event, commands) in own {
            for command in commands {
                let mut hook = json!({"type": "command", "command": command.command});
                if let Some(timeout) = command.timeout {
                    hook["timeout"] = json!(timeout);
                }
                let mut entry = json!({"hooks": [hook]});
                if let Some(matcher) = &command.matcher {
                    entry["matcher"] = json!(matcher);
                }
                hooks.entry(event.clone()).or_default().push(entry);
            }
        }
    }
    if !hooks.is_empty() {
        value["hooks"] = json!(hooks);
    }
    Ok(value)
}

fn workspace_settings(value: &mut serde_json::Value) -> Result<(), HomeError> {
    let permissions = &mut value["permissions"];
    for field in ["allow", "additionalDirectories"] {
        if permissions
            .get(field)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.is_empty())
        {
            return Err(HomeError::TemplateShape {
                field: format!("slots.permissions.{field}"),
                reason: "workspace-only cannot carry additional allowed tools or directories",
            });
        }
    }
    let mut deny = permissions
        .get("deny")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for tool in ["WebFetch", "WebSearch"] {
        let rule = json!(tool);
        if !deny.contains(&rule) {
            deny.push(rule);
        }
    }
    permissions["defaultMode"] = json!("dontAsk");
    permissions["allow"] = json!(["Read(./**)", "Edit(./**)"]);
    permissions["deny"] = json!(deny);
    permissions["blockReadsOutsideWorkingDirectories"] = json!(true);
    value["sandbox"] = json!({
        "enabled": true, "failIfUnavailable": true,
        "allowUnsandboxedCommands": false, "excludedCommands": [],
        "filesystem": {"allowWrite": []},
        "network": {"allowedDomains": [], "strictAllowlist": true, "allowLocalBinding": false, "allowAllUnixSockets": false}
    });
    Ok(())
}

/// The hook that sends each of the session's tool calls to the runner's
/// judge before it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judge {
    command: String,
    collect: bool,
}

impl Judge {
    /// The hook running `program` against the runner on `socket`. Both must
    /// be absolute, so the hook never depends on the session's directory or
    /// search path, and neither may hold a character the hook's shell would
    /// read as more than a path.
    pub fn new(program: &Path, socket: &Path) -> Result<Self, HomeError> {
        let program = hook_path("--judge-program", program)?;
        let socket = hook_path("--judge-socket", socket)?;
        Ok(Self {
            command: format!("'{program}' runner judge --harness claude --socket '{socket}'"),
            collect: false,
        })
    }

    /// The judge and collector for a runner launch, using its published bindings.
    ///
    /// # Errors
    /// Refuses a supplied program or socket that is not an absolute hook path.
    pub fn runner(program: Option<&str>, socket: Option<&str>) -> Result<Self, HomeError> {
        if let Some(program) = program {
            hook_path("--judge-program", Path::new(program))?;
        }
        if let Some(socket) = socket {
            hook_path("--judge-socket", Path::new(socket))?;
        }
        let mut judge = match (program, socket) {
            (Some(program), Some(socket)) => Self::new(Path::new(program), Path::new(socket))?,
            _ => Self {
                command:
                    "\"$LYS_PROGRAM\" runner judge --harness claude --socket \"$LYS_RUNNER_SOCKET\""
                        .to_owned(),
                collect: true,
            },
        };
        judge.collect = true;
        Ok(judge)
    }

    /// The hook's command line.
    pub fn command(&self) -> &str {
        &self.command
    }
}

fn hook_path<'a>(flag: &'static str, path: &'a Path) -> Result<&'a str, HomeError> {
    let text = path
        .to_str()
        .filter(|text| path.is_absolute() && !text.chars().any(|c| c == '\'' || c.is_control()));
    text.ok_or_else(|| HomeError::JudgePath {
        flag,
        path: path.to_path_buf(),
    })
}

/// The settings file's bytes with the judge's `PreToolUse` hook beside
/// `env` when there is a judge; without one, exactly [`env_settings`].
pub fn settings(template: &Template, judge: Option<&Judge>) -> Result<Vec<u8>, HomeError> {
    let value = settings_value(template, judge)?;
    let mut bytes = serde_json::to_vec_pretty(&value).map_err(|source| HomeError::Json {
        context: "the environment file could not be serialised",
        source,
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Write the environment file at `path`, which must not exist.
pub fn write_env_file(
    template: &Template,
    judge: Option<&Judge>,
    path: &Path,
) -> Result<Hash, HomeError> {
    write_new(path, &settings(template, judge)?)
}

/// Write a launch file exclusively (an existing path is refused by name),
/// sync it, and return the hash of the bytes written.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<Hash, HomeError> {
    let mut file = match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(HomeError::LaunchTargetExists {
                path: path.to_path_buf(),
            });
        }
        Err(e) => return Err(HomeError::io("creating a launch file", path, e)),
    };
    file.write_all(bytes)
        .map_err(|e| HomeError::io("writing a launch file", path, e))?;
    file.sync_all()
        .map_err(|e| HomeError::io("syncing a launch file", path, e))?;
    Ok(Hash::of(bytes))
}

#[cfg(test)]
#[path = "../../../tests/rendering/runner_launch.rs"]
mod runner_rendering_tests;
