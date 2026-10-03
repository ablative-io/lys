//! A fresh launch carries the native template as files and process inputs.

use std::collections::BTreeMap;

use super::claude_code::launch_env::{confinement_named, env_settings};
use super::claude_code::template::parse_template;
use super::launch_fields::InstructionsMode;
use crate::record::blocks::Hash;

/// One file, relative to the session's config directory.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// Its relative location.
    pub path: String,
    /// Its exact contents.
    pub text: String,
    /// The contents' digest.
    pub sha256: String,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("path", &self.path)
            .field("sha256", &self.sha256)
            .finish_non_exhaustive()
    }
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

/// Loads none of the person's, the folder's or the folder's local settings.
pub const NO_SETTING_SOURCES: &str = "--setting-sources=";
/// Uses only the MCP servers in the file Lys passes.
pub const ONLY_GIVEN_MCP: &str = "--strict-mcp-config";

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
/// Refuses unknown contracts, invalid templates, invalid declared programs,
/// and a Claude template that does not say how the agent is confined: it is
/// never rendered unconfined, and the operator's file is never rewritten.
pub fn render(
    contract: &str,
    program: &str,
    text: &str,
    instructions_mode: InstructionsMode,
) -> Result<Launch, String> {
    if !matches!(contract, "claude-code/template-v1" | "codex/template-v1") {
        return Err(format!("unknown rendering contract `{contract}`"));
    }
    if !program.starts_with('/') || program.chars().any(char::is_control) {
        return Err("the declared program is not an absolute plain path".to_owned());
    }
    if contract == "codex/template-v1" {
        return super::codex::launch::render(program, text, instructions_mode);
    }
    let template = parse_template(text.as_bytes()).map_err(|error| error.to_string())?;
    confinement_named(&template).map_err(|error| error.to_string())?;
    let settings = String::from_utf8(env_settings(&template).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let mcp =
        serde_json::to_string_pretty(&template.mcp).map_err(|error| error.to_string())? + "\n";
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
    // The run keeps the machine's sign-in and nothing else of the person's
    // own Claude Code setup (Tom, 3 Oct 2026 20:47): no config folder is set,
    // because the sign-in lives under the login's own one, and two flags
    // leave the rest out. Sources are in
    // docs/harness/reference/claude-code/CLEAN-START.md.
    let mut arguments = vec![
        "--mcp-config".to_owned(),
        "mcp.json".to_owned(),
        "--settings".to_owned(),
        "settings.json".to_owned(),
        // cli-reference.md line 129: "Comma-separated list of setting
        // sources to load (`user`, `project`, `local`)". An empty list loads
        // none of the three, so the person's settings, hooks, installed
        // plugins and their skills and agents stay out; the `--settings`
        // file above still applies. Written as one word so no empty
        // argument has to survive a command line.
        NO_SETTING_SOURCES.to_owned(),
        // cli-reference.md line 131: "Only use MCP servers from
        // `--mcp-config`, ignoring all other MCP configurations". That
        // leaves out the person's MCP servers, plugin servers and claude.ai
        // connectors; the servers in Lys's own mcp.json stay.
        ONLY_GIVEN_MCP.to_owned(),
    ];
    let mut argument_files =
        BTreeMap::from([(1, "mcp.json".to_owned()), (3, "settings.json".to_owned())]);
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
        environment_paths: BTreeMap::new(),
    })
}
