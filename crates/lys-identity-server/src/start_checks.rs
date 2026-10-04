//! The admission and neutral launch-field checks shared by start and readiness.

use std::collections::BTreeSet;

use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use lys_home::harness::launch_fields::{
    EnvText, HandleEnv, KeptSkill, LaunchFields, LaunchIdentity, LaunchMcp, Transport,
};
use lys_identity::LifecycleState;
use serde_json::Value;

use crate::launch_template::{HandleName, Start, handle_variable};
use crate::routes::AppState;

use crate::error::ServerError;
use crate::network_store::{Machine, NetworkStore};
use crate::provisioning_store::{McpServer, Setting, Settings, Version};

/// The machine `id`, when it takes `agent`: known, in use, with a runtime,
/// and listing the agent among those that may run on it.
pub(crate) fn placed<'a>(
    store: &'a NetworkStore,
    id: &str,
    (agent, held): (&str, &[String]),
) -> Result<&'a Machine, ServerError> {
    let machine = store.machine(id).ok_or(ServerError::MachineUnknown)?;
    if machine.retired.is_some() {
        return Err(ServerError::MachineRetired);
    }
    if machine.runtime.is_none() {
        return Err(ServerError::MachineWithoutRuntime);
    }
    let by_role = machine.may_run_roles.iter().any(|role| held.contains(role));
    if !by_role && !machine.may_run.iter().any(|named| named == agent) {
        return Err(ServerError::MachineNotForAgent);
    }
    Ok(machine)
}

/// Refuse by name the first host a server of `version` is reached at that
/// `machine`'s egress list does not name. A command server is started on
/// the machine and reached over its own streams, so it names no host here.
pub(crate) fn reaches(machine: &Machine, version: &Version) -> Result<(), ServerError> {
    for server in version
        .settings
        .mcp_servers
        .iter()
        .filter(|server| server.command.is_none())
    {
        let host = url_host(&server.url).ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: format!(
                "server `{}` is reached at `{}`, which names no host",
                server.name, server.url
            ),
        })?;
        if !machine.may_reach.contains(&host) {
            return Err(ServerError::MachineCannotReach { host });
        }
    }
    Ok(())
}

/// The host of `url`, lower-cased, without scheme, credentials, port or path.
fn url_host(url: &str) -> Option<String> {
    let (_scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let located = authority
        .rsplit_once('@')
        .map_or(authority, |(_user, host)| host);
    let host = located.split(':').next()?.to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// A new start requires an active agent.
pub(crate) fn active(state: LifecycleState) -> Result<(), ServerError> {
    if state != LifecycleState::Active {
        return Err(ServerError::AgentNotActive {
            state: state.to_string(),
        });
    }
    Ok(())
}

/// A new start requires the exact profile version to have been reviewed.
pub(crate) fn reviewed(version: &Version) -> Result<(), ServerError> {
    if version.reviewed.is_none() {
        return Err(ServerError::ProfileNotReviewed {
            version: version.number,
        });
    }
    Ok(())
}

/// The handles `agent` holds that are not dropped, as the broker lists them
/// to the signed-in person, each with the variable the launch sets.
pub(crate) async fn handles(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
) -> Result<Vec<HandleName>, ServerError> {
    let path = format!("/_lys/handles?holder={agent}");
    let answer = crate::secrets_api::ask(state, headers, Method::GET, &path, Bytes::new()).await?;
    let unread = |reason: &str| ServerError::SecretsUnavailable {
        reason: format!("the broker's handle list does not read: {reason}"),
    };
    let listed = answer
        .get("handles")
        .and_then(Value::as_array)
        .ok_or_else(|| unread("it holds no handles list"))?;
    let mut held: Vec<(String, String)> = Vec::new();
    for handle in listed {
        let dropped = handle
            .get("dropped")
            .and_then(Value::as_bool)
            .ok_or_else(|| unread("a handle does not say whether it was dropped"))?;
        if dropped {
            continue;
        }
        let text = |name: &str| {
            handle
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| unread(&format!("a handle has no {name}")))
        };
        held.push((text("id")?, text("secret")?));
    }
    held.sort();
    let mut taken = BTreeSet::new();
    Ok(held
        .into_iter()
        .map(|(id, secret)| HandleName {
            env: handle_variable(&secret, &mut taken),
            id,
            secret,
        })
        .collect())
}

/// The launch fields `start` gives, with each secret resolved to one of the
/// agent's `handles`; a profile that declares no harness is refused.
pub fn fields(start: &Start<'_>, handles: &[HandleName]) -> Result<LaunchFields, ServerError> {
    let settings = &start.version.settings;
    let harness = settings
        .harness
        .clone()
        .ok_or(ServerError::HarnessUndeclared {
            version: start.version.number,
        })?;
    // Only Claude Code's model calls go through Lys's proxy for now.
    let harness_is_claude = harness.description.rendering_contract == "claude-code/template-v1";
    crate::launch_fields::models(&harness, &settings.model_access)?;
    crate::launch_fields::mcp(&harness, &settings.mcp_servers)?;
    let mcp_servers = settings
        .mcp_servers
        .iter()
        .map(|server| server_fields(server, handles))
        .collect::<Result<Vec<_>, _>>()?;
    let model_proxy = start
        .model_proxy
        .filter(|_| harness_is_claude)
        .map(str::to_owned);
    // Every start is rendered from these fields, so every start that is
    // given the proxy is given it under a key minted here for it alone: no
    // start path can give a run the proxy without one.
    let run = model_proxy.as_ref().map(|_| lys_home::record::fresh_id());
    Ok(LaunchFields {
        identity: LaunchIdentity {
            agent: start.agent.to_owned(),
            session: start.session.to_owned(),
            machine: start.machine.to_owned(),
            version: start.version.number,
        },
        harness,
        instructions: settings.instructions.clone(),
        models: settings.model_access.clone(),
        mcp_servers,
        skills: skills(settings)?,
        model_proxy,
        run,
    })
}

/// Each skill the profile names, as the text the version pinned; a name
/// with no pin was recorded before Lys kept its text and is refused.
fn skills(settings: &Settings) -> Result<Vec<KeptSkill>, ServerError> {
    settings
        .skills
        .iter()
        .map(|name| {
            settings
                .skill_pins
                .iter()
                .find(|pin| &pin.name == name)
                .map(|pin| KeptSkill {
                    name: pin.name.clone(),
                    path: format!("skills/{}/SKILL.md", pin.name),
                    len: pin.len,
                    sha256: pin.sha256.clone(),
                })
                .ok_or_else(|| ServerError::SkillUnknown { name: name.clone() })
        })
        .collect()
}

fn server_fields(server: &McpServer, handles: &[HandleName]) -> Result<LaunchMcp, ServerError> {
    let transport = match &server.command {
        None => Transport::Http {
            url: server.url.clone(),
        },
        Some(command) => {
            let mut env = Vec::new();
            let mut held = Vec::new();
            for (name, setting) in &command.env {
                match setting {
                    Setting::Literal(literal) => env.push(EnvText {
                        name: name.clone(),
                        text: literal.text(),
                    }),
                    Setting::Handle { handle: secret } => held.push(HandleEnv {
                        name: name.clone(),
                        handle_id: handle_id(server, name, secret, handles)?,
                    }),
                }
            }
            Transport::Stdio {
                program: command.program.clone(),
                args: command.args.clone(),
                cwd: command.cwd.clone(),
                env,
                handles: held,
            }
        }
    };
    Ok(LaunchMcp {
        name: server.name.clone(),
        transport,
        channel: server.channel,
    })
}

fn handle_id(
    server: &McpServer,
    variable: &str,
    secret: &str,
    handles: &[HandleName],
) -> Result<String, ServerError> {
    handles
        .iter()
        .find(|held| held.secret == secret)
        .map(|held| held.id.clone())
        .ok_or_else(|| ServerError::McpHandleUnsupported {
            server: server.name.clone(),
            member: format!("env `{variable}`"),
            secret: secret.to_owned(),
        })
}
