//! Rendering contracts live in the home. The server passes neutral fields
//! and never selects a product; the registry refuses an unknown identifier.
use serde_json::{Map, Value, json};

use super::claude_code::HARNESS;
/// Quote one shell word without changing its bytes.
pub use super::claude_code::launch::shell_word;
use super::claude_code::template::{FILL_RESUME_BY_PATH, parse_template};
use super::description::FurtherModels;
use super::launch_fields::{Channel, LaunchFields, LaunchMcp, Transport};
use super::skills::SkillFile;

/// A resolved secret, carried only as a handle identifier.
pub struct SecretBinding {
    /// The environment variable assigned the handle.
    pub env: String,
    /// The opaque handle identifier, never its value.
    pub handle: String,
}

/// A template and its content identity after native-parser validation.
pub struct RenderedTemplate {
    /// The native template format selected by this rendering contract.
    pub harness: String,
    /// The exact template bytes, as text.
    pub text: String,
    /// The hash checked by the home.
    pub sha256: String,
}

/// A refusal names the contract member that cannot be represented.
#[derive(Debug, thiserror::Error)]
#[error("rendering contract {contract}: {member}: {reason}")]
pub struct RenderRefusal {
    /// The registry identifier.
    pub contract: String,
    /// The member the refusal concerns.
    pub member: String,
    /// Why the member cannot be represented.
    pub reason: String,
}

pub(super) fn refused(
    fields: &LaunchFields,
    member: &str,
    reason: &(impl ToString + ?Sized),
) -> RenderRefusal {
    RenderRefusal {
        contract: fields.harness.description.rendering_contract.clone(),
        member: member.to_owned(),
        reason: reason.to_string(),
    }
}

enum Contract {
    Native,
    Codex,
}

fn resolve(contract: &str) -> Option<Contract> {
    match contract {
        "claude-code/template-v1" => Some(Contract::Native),
        "codex/template-v1" => Some(Contract::Codex),
        _ => None,
    }
}

/// Whether the rendering registry can resolve this explicit contract.
pub fn registered(contract: &str) -> bool {
    resolve(contract).is_some()
}

/// Resolve the explicit rendering contract; display names are never examined.
pub fn render(
    fields: &LaunchFields,
    skills: &[SkillFile],
    permissions: &Value,
    secrets: &[SecretBinding],
) -> Result<RenderedTemplate, RenderRefusal> {
    match resolve(&fields.harness.description.rendering_contract) {
        Some(Contract::Native) => native_template(fields, skills, permissions, secrets),
        Some(Contract::Codex) => {
            super::codex::launch_template::render(fields, skills, permissions, secrets)
        }
        None => Err(refused(
            fields,
            "description.rendering_contract",
            "unknown identifier",
        )),
    }
}

fn flags(fields: &LaunchFields) -> Result<Vec<String>, RenderRefusal> {
    let mut flags = Vec::new();
    if let Some((model, further)) = fields.models.split_first() {
        flags.extend(["--model".to_owned(), model.clone()]);
        if !further.is_empty() {
            match &fields.harness.description.models.further_encoding {
                FurtherModels::Delimited { separator } if separator == "," => {
                    flags.extend(["--fallback-model".to_owned(), further.join(separator)]);
                }
                _ => {
                    return Err(refused(
                        fields,
                        "description.models.further_encoding",
                        "this rendering contract requires comma-separated further models",
                    ));
                }
            }
        }
    }
    let waking: Vec<String> = fields
        .mcp_servers
        .iter()
        .filter(|server| server.channel == Channel::Wake)
        .map(|server| format!("server:{}", server.name))
        .collect();
    if !waking.is_empty() {
        flags.push("--channels".to_owned());
        flags.extend(waking);
    }
    Ok(flags)
}

fn server_entry(fields: &LaunchFields, server: &LaunchMcp) -> Result<Value, RenderRefusal> {
    match &server.transport {
        Transport::Http { url } => Ok(json!({"type": "http", "url": url})),
        Transport::Stdio {
            program,
            args,
            cwd,
            env,
            handles,
        } => {
            if cwd.is_some() {
                return Err(refused(
                    fields,
                    &format!("mcp_servers.{}.cwd", server.name),
                    "this rendering contract has no command working-directory slot",
                ));
            }
            let mut vars = Map::new();
            for one in env {
                vars.insert(one.name.clone(), json!(one.text));
            }
            for one in handles {
                vars.insert(one.name.clone(), json!(one.handle_id));
            }
            Ok(json!({"type": "stdio", "command": program, "args": args, "env": vars}))
        }
    }
}

fn native_template(
    fields: &LaunchFields,
    skills: &[SkillFile],
    permissions: &Value,
    secrets: &[SecretBinding],
) -> Result<RenderedTemplate, RenderRefusal> {
    if permissions
        .get("default_mode")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(refused(
            fields,
            "permissions",
            "choose how this agent is confined",
        ));
    }
    let mut servers = Map::new();
    for server in &fields.mcp_servers {
        servers.insert(server.name.clone(), server_entry(fields, server)?);
    }
    let mut body = json!({
        "harness": HARNESS, "flags": flags(fields)?,
        "slots": {
            "transcript": {"fill": FILL_RESUME_BY_PATH, "canon": null},
            "mcp": {"mcpServers": servers},
            "env": {"LYS_AGENT": fields.identity.agent, "LYS_SESSION": fields.identity.session,
                "LYS_MACHINE": fields.identity.machine, "LYS_PROVISIONING_VERSION": fields.identity.version.to_string()},
            "secrets": {"use_only": secrets.iter().map(|held| json!({"env": held.env, "handle": held.handle})).collect::<Vec<_>>(),
                "readable": [], "reader": ""},
            "instructions": fields.instructions
        }
    });
    if !skills.is_empty() {
        body["slots"]["skills"] = json!(skills);
    }
    let native = super::rendering_permissions::render(permissions)
        .map_err(|error| refused(fields, "permissions", &error))?;
    let empty = native.as_object().is_some_and(|members| {
        members
            .values()
            .all(|value| value.as_array().is_some_and(Vec::is_empty))
    });
    if !empty {
        body["slots"]["permissions"] = native;
    }
    let bytes =
        serde_json::to_vec_pretty(&body).map_err(|error| refused(fields, "template", &error))?;
    let parsed = parse_template(&bytes).map_err(|error| refused(fields, "template", &error))?;
    let text = String::from_utf8(bytes).map_err(|error| refused(fields, "template", &error))?;
    Ok(RenderedTemplate {
        harness: HARNESS.to_owned(),
        text,
        sha256: parsed.hash.as_str().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::registered;

    #[test]
    fn only_registered_contracts_are_answered_as_registered() {
        assert!(registered("claude-code/template-v1"));
        assert!(registered("codex/template-v1"));
        assert!(!registered("Claude Code"));
        assert!(!registered(""));
    }
}
