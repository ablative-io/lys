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

/// The settings file's bytes: `{"env": {...}}`, keys sorted, with the
/// template's `permissions` beside `env` when it sets them; one trailing
/// newline.
pub fn env_settings(template: &Template) -> Result<Vec<u8>, HomeError> {
    let mut env: BTreeMap<&str, &str> = template
        .env
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    for secret in &template.use_only {
        env.insert(secret.env.as_str(), secret.handle.as_str());
    }
    let mut value = json!({ "env": env });
    if let Some(permissions) = &template.permissions {
        value["permissions"] = serde_json::Value::Object(permissions.clone());
    }
    let mut bytes = serde_json::to_vec_pretty(&value).map_err(|source| HomeError::Json {
        context: "the environment file could not be serialised",
        source,
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// The hook that sends each of the session's tool calls to the runner's
/// judge before it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judge {
    command: String,
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
        })
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
    let Some(judge) = judge else {
        return env_settings(template);
    };
    let mut value: serde_json::Value =
        serde_json::from_slice(&env_settings(template)?).map_err(|source| HomeError::Json {
            context: "the environment file could not be read back",
            source,
        })?;
    value["hooks"] = json!({
        "PreToolUse": [{
            "matcher": "*",
            "hooks": [{"type": "command", "command": judge.command()}]
        }]
    });
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
