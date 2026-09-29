//! What a recorded profile version gives a launch, in lys-home's neutral
//! launch fields: the declared build, the models, the instructions and the
//! MCP servers with each secret as the agent's handle id. Each harness's
//! render reads these; nothing here names a harness's program, package or
//! layout, which come only from what the profile declares.

use lys_home::harness::launch_fields::{
    EnvText, HandleEnv, KeptSkill, LaunchFields, LaunchIdentity, LaunchMcp, Transport,
};

use crate::error::ServerError;
use crate::launch_template::{HandleName, Start};
use lys_home::harness::skills::SkillFile;

use crate::provisioning_store::{McpServer, ProvisioningStore, Setting, Settings, Version};

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
    crate::launch_fields::models(&harness, &settings.model_access)?;
    crate::launch_fields::mcp(&harness, &settings.mcp_servers)?;
    let mcp_servers = settings
        .mcp_servers
        .iter()
        .map(|server| server_fields(server, handles))
        .collect::<Result<Vec<_>, _>>()?;
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

/// The text of each skill `version` pinned, as the launch carries it; a pin
/// whose text Lys no longer keeps is refused.
pub fn skill_files(
    store: &ProvisioningStore,
    version: &Version,
) -> Result<Vec<SkillFile>, ServerError> {
    version
        .settings
        .skill_pins
        .iter()
        .map(|pin| {
            store
                .skill(&pin.name, &pin.sha256)
                .map(|kept| SkillFile {
                    name: kept.name.clone(),
                    text: kept.text.clone(),
                    sha256: kept.sha256.clone(),
                })
                .ok_or_else(|| ServerError::SkillUnknown {
                    name: pin.name.clone(),
                })
        })
        .collect()
}
