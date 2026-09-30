//! A fresh launch carries the native template as files and process inputs.

use std::collections::BTreeMap;

use super::claude_code::launch_env::env_settings;
use super::claude_code::template::parse_template;
use super::launch_fields::InstructionsMode;
use crate::record::blocks::Hash;

/// One file, relative to the session's config directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// Its relative location.
    pub path: String,
    /// Its exact contents.
    pub text: String,
    /// The contents' digest.
    pub sha256: String,
}

/// Process inputs and files rebuilt deterministically from a kept template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The operator-declared executable.
    pub program: String,
    /// Arguments before the runner binds config locations.
    pub arguments: Vec<String>,
    /// Literal variables, with credentials represented only by handle ids.
    pub environment: BTreeMap<String, String>,
    /// Argument positions bound to config-relative files.
    pub argument_files: BTreeMap<usize, String>,
    /// Variables bound to config-relative locations; empty names its root.
    pub environment_paths: BTreeMap<String, String>,
    /// Every native config file and skill.
    pub files: Vec<File>,
}

fn file(path: &str, text: String) -> File {
    File {
        path: path.to_owned(),
        sha256: Hash::of(text.as_bytes()).as_str().to_owned(),
        text,
    }
}

/// Rebuild a fresh launch without inspecting any machine settings.
///
/// # Errors
/// Refuses unknown contracts, invalid templates and invalid declared programs.
pub fn render(
    contract: &str,
    program: &str,
    text: &str,
    instructions_mode: InstructionsMode,
) -> Result<Launch, String> {
    if contract != "claude-code/template-v1" {
        return Err(format!("unknown rendering contract `{contract}`"));
    }
    if !program.starts_with('/') || program.chars().any(char::is_control) {
        return Err("the declared program is not an absolute plain path".to_owned());
    }
    let template = parse_template(text.as_bytes()).map_err(|error| error.to_string())?;
    let settings = String::from_utf8(env_settings(&template).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let mcp = serde_json::to_string_pretty(&serde_json::json!({"mcpServers": template.mcp}))
        .map_err(|error| error.to_string())?
        + "\n";
    let mut files = vec![
        file("mcp.json", mcp),
        file("settings.json", settings),
        file("instructions.txt", template.instructions.clone()),
    ];
    for skill in &template.skills {
        let checked = super::skills::check(skill).map_err(|error| error.to_string())?;
        files.push(file(&checked.path, skill.text.clone()));
    }
    let mut environment = template.env;
    for secret in template.use_only {
        environment.insert(secret.env, secret.handle);
    }
    let mut arguments = vec![
        "--mcp-config".to_owned(),
        "mcp.json".to_owned(),
        "--strict-mcp-config".to_owned(),
        "--settings".to_owned(),
        "settings.json".to_owned(),
        "--setting-sources".to_owned(),
        String::new(),
    ];
    let mut argument_files =
        BTreeMap::from([(1, "mcp.json".to_owned()), (4, "settings.json".to_owned())]);
    let prompt_flag = match instructions_mode {
        InstructionsMode::Keep => None,
        InstructionsMode::Append => Some("--append-system-prompt-file"),
        InstructionsMode::Replace => Some("--system-prompt-file"),
    };
    if let Some(flag) = prompt_flag {
        arguments.push(flag.to_owned());
        argument_files.insert(arguments.len(), "instructions.txt".to_owned());
        arguments.push("instructions.txt".to_owned());
    }
    arguments.extend(template.flags);
    Ok(Launch {
        program: program.to_owned(),
        arguments,
        environment,
        files,
        argument_files,
        environment_paths: BTreeMap::from([("CLAUDE_CONFIG_DIR".to_owned(), String::new())]),
    })
}
