//! The admission and neutral launch-field checks shared by start and readiness.

use std::collections::BTreeSet;

use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use lys_home::harness::launch_fields::{
    EnvText, HandleEnv, KeptSkill, LaunchFields, LaunchIdentity, LaunchMcp, ProfileLaunchFields,
    Transport,
};
use lys_identity::LifecycleState;
use serde_json::Value;

use crate::launch_template::{HandleName, Start, handle_variable};
use crate::routes::AppState;

use crate::error::ServerError;
use crate::network_store::{Machine, NetworkStore};
use crate::provisioning_store::{McpServer, ProvisioningStore, Setting, Settings, Version};
use std::str::FromStr;

#[cfg(test)]
#[path = "start_proxy_tests.rs"]
mod proxy_tests;

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
        let host = server_host(server)?;
        if !machine.may_reach.contains(&host) {
            return Err(ServerError::MachineCannotReach { host });
        }
    }
    Ok(())
}

/// Every host a server of `version` is reached at, in the profile's order:
/// what a machine's egress list must name for [`reaches`] to pass.
pub(crate) fn needed_hosts(version: &Version) -> Result<Vec<String>, ServerError> {
    version
        .settings
        .mcp_servers
        .iter()
        .filter(|server| server.command.is_none())
        .map(server_host)
        .collect()
}

/// The host `server` is reached at, or its refusal by name.
fn server_host(server: &McpServer) -> Result<String, ServerError> {
    url_host(&server.url).ok_or_else(|| ServerError::LaunchUnrenderable {
        reason: format!(
            "server `{}` is reached at `{}`, which names no host",
            server.name, server.url
        ),
    })
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

/// A kept start's review, read again when its launch is rebuilt: the
/// version's own, or the carried review the start was admitted with
/// (`carried_review` in the kept view) while that version still holds a
/// person's review of the same settings. Kinship was proved at admission.
pub(crate) fn kept_reviewed(
    store: &ProvisioningStore,
    view: &Value,
    version: &Version,
) -> Result<(), ServerError> {
    if version.reviewed.is_some() {
        return Ok(());
    }
    let source = view.get("carried_review").and_then(|carried| {
        let agent = carried.get("agent")?.as_str()?;
        let number = u32::try_from(carried.get("version")?.as_u64()?).ok()?;
        store
            .profile(agent)?
            .versions
            .iter()
            .find(|kept| kept.number == number)
    });
    let digest = version.settings.sha256();
    let holds = source.is_some_and(|source| {
        source
            .reviewed
            .as_ref()
            .is_some_and(|review| lys_identity::PersonId::from_str(&review.by).is_ok())
            && digest.is_some()
            && source.settings.sha256() == digest
    });
    if holds { Ok(()) } else { reviewed(version) }
}

/// A review carried to a version without its own (AGENTS-017 R3): a person's
/// review of the same settings on another agent answering to the same person.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct CarriedReview {
    /// The agent whose version was reviewed.
    pub agent: String,
    /// That version's number.
    pub version: u32,
    /// The person who reviewed it: the responsible person or the administrator.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// An agent and every agent answering to the same person, read from the
/// directory before any profile is read, so the directory is never asked
/// while the profiles are held.
pub(crate) struct Kin {
    agent: String,
    agents: BTreeSet<String>,
}

/// `agent`'s kin in `projection`: none beside it when it answers to no one.
pub(crate) fn kin_in(
    projection: &lys_identity::projection::Projection,
    agent: &str,
) -> Result<Kin, ServerError> {
    let parsed: lys_identity::AgentId = agent.parse().map_err(|error| {
        drop(error);
        ServerError::AgentNotVisible
    })?;
    let person = projection
        .record(lys_identity::IdentityId::Agent(parsed))
        .and_then(lys_identity::projection::Record::responsible);
    let mut agents = BTreeSet::new();
    if let Some(person) = person {
        for held in projection.agents_of(person) {
            let (identity, _) = held.map_err(|error| ServerError::DirectoryUnavailable {
                reason: error.to_string(),
            })?;
            agents.insert(identity.to_string());
        }
    }
    Ok(Kin {
        agent: agent.to_owned(),
        agents,
    })
}

/// `agent`'s kin, read from the directory alone.
pub(crate) fn kin(state: &AppState, agent: &str) -> Result<Kin, ServerError> {
    crate::routes::with_directory(state, |directory| kin_in(directory.projection()?, agent))
}

/// The review that carries to `version`, which has none of its own: the
/// latest review, by a person, of a version with the same settings digest on
/// another agent answering to the same person. A review an agent made never
/// carries; nor does any review to an agent that answers to no one.
pub(crate) fn carried(
    store: &ProvisioningStore,
    kin: &Kin,
    version: &Version,
) -> Option<CarriedReview> {
    let digest = version.settings.sha256()?;
    store
        .reviewed_with(&digest)
        .filter(|(agent, _)| *agent != kin.agent && kin.agents.contains(*agent))
        .filter_map(|(agent, kept)| {
            let review = kept.reviewed.as_ref()?;
            lys_identity::PersonId::from_str(&review.by).ok()?;
            Some(CarriedReview {
                agent: agent.to_owned(),
                version: kept.number,
                by: review.by.clone(),
                at: review.at,
            })
        })
        .max_by_key(|review| review.at)
}

/// A new start's review: the version's own (none carried), or one carried to
/// it by content; refused `ProfileNotReviewed` when neither stands.
pub(crate) fn reviewed_for(
    store: &ProvisioningStore,
    kin: &Kin,
    version: &Version,
) -> Result<Option<CarriedReview>, ServerError> {
    if version.reviewed.is_some() {
        return Ok(None);
    }
    carried(store, kin, version)
        .map(Some)
        .ok_or(ServerError::ProfileNotReviewed {
            version: version.number,
        })
}

/// The broker metadata a launch needs, without any credential value.
pub(crate) enum Inventory {
    /// The handles held by the selected agent.
    Handles,
    /// Provider metadata used to check its reviewed accounts.
    ProviderRoutes,
}

/// Keep the responsible person check at every app pass broker read.
pub(crate) async fn inventory(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
    kind: Inventory,
) -> Result<Value, ServerError> {
    let path = match kind {
        Inventory::Handles => format!("/_lys/handles?holder={agent}"),
        Inventory::ProviderRoutes => "/_lys/secrets".to_owned(),
    };
    if let Some(person) = crate::provider::app_person_agent(state, headers, agent)? {
        crate::secrets_api::ask_as(state, &person.to_string(), Method::GET, &path, Bytes::new())
            .await
    } else {
        crate::secrets_api::ask(state, headers, Method::GET, &path, Bytes::new()).await
    }
}

/// The handles `agent` holds that are not dropped, as the broker lists them
/// to the signed-in person, each with the variable the launch sets.
pub(crate) async fn handles(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
) -> Result<Vec<HandleName>, ServerError> {
    let answer = inventory(state, headers, agent, Inventory::Handles).await?;
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
///
/// # Errors
/// Names a missing harness, unsupported model or server, unknown skill or invalid seat declaration.
pub fn fields(
    start: &Start<'_>,
    handles: &[HandleName],
) -> Result<ProfileLaunchFields, ServerError> {
    let settings = &start.version.settings;
    let harness = settings
        .harness
        .clone()
        .ok_or(ServerError::HarnessUndeclared {
            version: start.version.number,
        })?;
    drop(crate::model_account_start::accounts(
        start.version,
        handles,
    )?);
    crate::launch_fields::models(&harness, &settings.model_access)?;
    crate::launch_fields::mcp(&harness, &settings.mcp_servers)?;
    let mcp_servers = settings
        .mcp_servers
        .iter()
        .map(|server| server_fields(server, handles))
        .collect::<Result<Vec<_>, _>>()?;
    let supports_proxy = matches!(
        harness.description.rendering_contract.as_str(),
        "claude-code/template-v1" | "codex/template-v1" | "seat-program/v1"
    );
    if supports_proxy && start.model_proxy.is_none() {
        return Err(lys_runner::RunnerError::refused(
            "model_proxy_unset",
            "the server has no model proxy setting; configure its model proxy before starting this reviewed program",
        ).into());
    }
    let model_proxy = start
        .model_proxy
        .filter(|_| supports_proxy)
        .map(str::to_owned);
    // Every start is rendered from these fields, so every start that is
    // given the proxy is given it under a key minted here for it alone: no
    // start path can give a run the proxy without one.
    let run = model_proxy.as_ref().map(|_| lys_home::record::fresh_id());
    let seat = settings
        .seat_fields()
        .map_err(|reason| ServerError::LaunchUnrenderable { reason })?;
    seat.check(&harness.description.rendering_contract)
        .map_err(|error| ServerError::LaunchUnrenderable {
            reason: error.to_string(),
        })?;
    Ok(ProfileLaunchFields {
        seat,
        launch: LaunchFields {
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
            plugins: settings.plugins.clone(),
            model_proxy,
            run,
        },
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
