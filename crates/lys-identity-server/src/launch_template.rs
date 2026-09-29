//! An agent's kept provisioning profile rendered as the home's launch
//! template, and the start command that names it.
//!
//! The template is the one the home keeps by hash and renders a harness
//! launch from; it is checked by the home's own parser, so a machine's home
//! takes these bytes as they are. Secrets ride only as use-only handles,
//! named by handle id, never a value. The start command is text: the agent's
//! identity, the session it reports under, the machine, the template's hash
//! and the handle ids, then the machine's runtime. Nothing here runs it.

use std::collections::BTreeSet;

use lys_home::harness::claude_code::HARNESS;
use lys_home::harness::claude_code::launch::shell_word;
use lys_home::harness::claude_code::template::{FILL_RESUME_BY_PATH, parse_template};
use lys_home::harness::launch_fields::{Channel, HarnessKind, LaunchFields, LaunchMcp, Transport};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::ServerError;
use crate::launch_harness::fields;
use crate::provisioning_store::Version;

/// One handle the agent holds, as the start command names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct HandleName {
    /// The handle's id, which is not the handle.
    pub id: String,
    /// The secret it stands for.
    pub secret: String,
    /// The variable the launch sets to the handle id.
    pub env: String,
}

/// What a start is rendered for.
pub struct Start<'a> {
    /// The agent.
    pub agent: &'a str,
    /// The session the started agent reports under.
    pub session: &'a str,
    /// The machine it starts on.
    pub machine: &'a str,
    /// The runtime installed on that machine.
    pub runtime: &'a str,
    /// The profile version it starts from.
    pub version: &'a Version,
}

/// A rendered start.
pub struct Rendered {
    /// The template's bytes, exactly as hashed.
    pub template: String,
    /// The SHA-256 of those bytes, as the home keeps the template by.
    pub template_sha256: String,
    /// The start command, as text.
    pub command: String,
    /// What of the profile no slot of the template carries.
    pub left_out: Vec<String>,
}

/// The variable a handle on `secret` is set in, unique among `taken`.
pub fn handle_variable(secret: &str, taken: &mut BTreeSet<String>) -> String {
    let stem: String = secret
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let base = format!("LYS_HANDLE_{stem}");
    let mut name = base.clone();
    let mut next = 2_u32;
    while taken.contains(&name) {
        name = format!("{base}_{next}");
        next = next.saturating_add(1);
    }
    taken.insert(name.clone());
    name
}

fn flags(version: &Version) -> Vec<String> {
    let settings = &version.settings;
    let mut flags = Vec::new();
    if let Some((model, further)) = settings.model_access.split_first() {
        flags.push("--model".to_owned());
        flags.push(model.clone());
        if !further.is_empty() {
            flags.push("--fallback-model".to_owned());
            flags.push(further.join(","));
        }
    }
    if !settings.tools.is_empty() {
        flags.push("--allowedTools".to_owned());
        flags.push(settings.tools.join(","));
    }
    let waking: Vec<String> = settings
        .mcp_servers
        .iter()
        .filter(|server| server.channel == Channel::Wake)
        .map(|server| format!("server:{}", server.name))
        .collect();
    if !waking.is_empty() {
        flags.push("--channels".to_owned());
        flags.extend(waking);
    }
    flags
}

fn left_out(version: &Version) -> Vec<String> {
    let settings = &version.settings;
    let mut left = Vec::new();
    if !settings.skills.is_empty() {
        left.push(format!(
            "skills, which no template slot carries: {}",
            settings.skills.join(", ")
        ));
    }
    left
}

/// A server's Claude Code entry: its address, or its stdio command with
/// each setting as its text and each secret as the agent's handle id on it.
fn server_entry(server: &LaunchMcp) -> Result<Value, ServerError> {
    match &server.transport {
        Transport::Http { url } => Ok(json!({ "type": "http", "url": url })),
        Transport::Stdio {
            program,
            args,
            cwd,
            env,
            handles,
        } => {
            if cwd.is_some() {
                return Err(ServerError::McpSettingUnrepresentable {
                    server: server.name.clone(),
                    member: "cwd".to_owned(),
                    reason: "Claude Code starts a stdio server with no directory of its own"
                        .to_owned(),
                });
            }
            let mut vars = Map::new();
            for one in env {
                vars.insert(one.name.clone(), Value::String(one.text.clone()));
            }
            for one in handles {
                vars.insert(one.name.clone(), Value::String(one.handle_id.clone()));
            }
            Ok(json!({ "type": "stdio", "command": program, "args": args, "env": vars }))
        }
    }
}

fn template(
    start: &Start<'_>,
    fields: &LaunchFields,
    handles: &[HandleName],
) -> Result<Value, ServerError> {
    let mut servers = Map::new();
    for server in &fields.mcp_servers {
        servers.insert(server.name.clone(), server_entry(server)?);
    }
    Ok(json!({
        "harness": HARNESS,
        "flags": flags(start.version),
        "slots": {
            "transcript": { "fill": FILL_RESUME_BY_PATH, "canon": null },
            "mcp": { "mcpServers": servers },
            "env": {
                "LYS_AGENT": start.agent,
                "LYS_SESSION": start.session,
                "LYS_MACHINE": start.machine,
                "LYS_PROVISIONING_VERSION": start.version.number.to_string(),
            },
            "secrets": {
                "use_only": handles
                    .iter()
                    .map(|handle| json!({ "env": handle.env, "handle": handle.id }))
                    .collect::<Vec<_>>(),
                "readable": [],
                "reader": "",
            },
            "instructions": fields.instructions,
        },
    }))
}

/// Render `start` with the agent's `handles`: the template the home checks
/// and keeps by hash, and the command a machine's runtime is given.
pub fn render(start: &Start<'_>, handles: &[HandleName]) -> Result<Rendered, ServerError> {
    let unrenderable = |reason: String| ServerError::LaunchUnrenderable { reason };
    let fields = fields(start, handles)?;
    if fields.harness.kind != HarnessKind::ClaudeCode {
        return Err(unrenderable(
            "the declared harness is Codex, whose render this start does not take yet".to_owned(),
        ));
    }
    let bytes = serde_json::to_vec_pretty(&template(start, &fields, handles)?)
        .map_err(|error| unrenderable(format!("the template does not write: {error}")))?;
    let parsed = parse_template(&bytes).map_err(|error| unrenderable(error.to_string()))?;
    let template = String::from_utf8(bytes)
        .map_err(|error| unrenderable(format!("the template is not text: {error}")))?;
    let template_sha256 = parsed.hash.as_str().to_owned();
    let handle_ids: Vec<&str> = handles.iter().map(|handle| handle.id.as_str()).collect();
    let words = [
        "env".to_owned(),
        format!("LYS_AGENT={}", start.agent),
        format!("LYS_SESSION={}", start.session),
        format!("LYS_MACHINE={}", start.machine),
        format!("LYS_LAUNCH_TEMPLATE={template_sha256}"),
        format!("LYS_HANDLES={}", handle_ids.join(",")),
        start.runtime.to_owned(),
    ];
    let command = words
        .iter()
        .map(|word| shell_word(word))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Rendered {
        template,
        template_sha256,
        command,
        left_out: left_out(start.version),
    })
}
