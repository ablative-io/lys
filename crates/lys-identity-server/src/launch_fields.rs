//! A profile's models checked against its recorded description, never its name.
use lys_home::harness::description::FurtherModels;
use lys_home::harness::launch_fields::DeclaredHarness;

use crate::error::ServerError;

/// Refuse an absent model or the first model the described build cannot carry.
pub fn models(harness: &DeclaredHarness, models: &[String]) -> Result<(), ServerError> {
    let contract = &harness.description.models;
    let refused = |model: &str, reason: &str| ServerError::ModelUnrepresentable {
        harness: harness.name.clone(),
        model: model.to_owned(),
        reason: reason.to_owned(),
    };
    if models.len() < contract.minimum {
        return Err(refused(
            "",
            "description.models.minimum exceeds the supplied model count",
        ));
    }
    if let Some(maximum) = contract.maximum {
        if maximum < contract.minimum {
            return Err(refused(
                "",
                "description.models.maximum is below its minimum",
            ));
        }
        if let Some(model) = models.get(maximum) {
            return Err(refused(
                model,
                "description.models.maximum does not admit this model",
            ));
        }
    }
    if let FurtherModels::Delimited { separator } = &contract.further_encoding {
        if separator.is_empty() {
            return Err(refused(
                "",
                "description.models.further_encoding.separator is empty",
            ));
        }
        if let Some(model) = models.iter().find(|model| model.contains(separator)) {
            return Err(refused(
                model,
                "the model contains description.models.further_encoding.separator",
            ));
        }
    }
    Ok(())
}

/// Refuse MCP members absent from the recorded capability description.
pub fn mcp(
    harness: &DeclaredHarness,
    servers: &[crate::provisioning_store::McpServer],
) -> Result<(), ServerError> {
    use crate::provisioning_store::Setting;
    let contract = &harness.description.mcp;
    for server in servers {
        let refused = |member: &str| ServerError::McpSettingUnrepresentable {
            server: server.name.clone(),
            member: member.to_owned(),
            reason: format!("harness description does not admit {member}"),
        };
        let transport = if server.command.is_some() {
            "stdio"
        } else {
            "http"
        };
        if !contract.transports.iter().any(|value| value == transport) {
            return Err(refused("description.mcp.transports"));
        }
        if !contract.channel_policies.contains(&server.channel) {
            return Err(refused("description.mcp.channel_policies"));
        }
        if let Some(command) = &server.command {
            if command.cwd.is_some() && !contract.working_directory {
                return Err(refused("cwd"));
            }
            if !contract.handle_variables
                && command
                    .env
                    .values()
                    .any(|value| matches!(value, Setting::Handle { .. }))
            {
                return Err(refused("description.mcp.handle_variables"));
            }
        }
    }
    Ok(())
}
