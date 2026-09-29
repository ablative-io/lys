//! What a recorded profile version gives a launch, in lys-home's neutral
//! launch fields: the declared build, the models, the instructions and the
//! MCP servers with each secret as the agent's handle id. Each harness's
//! render reads these; nothing here names a harness's program, package or
//! layout, which come only from what the profile declares.

use lys_home::harness::launch_fields::{
    EnvText, HandleEnv, LaunchFields, LaunchIdentity, LaunchMcp, Transport,
};

use crate::error::ServerError;
use crate::launch_template::{HandleName, Start};
use crate::provisioning_store::{McpServer, Setting};

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
        skills: Vec::new(),
    })
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
