//! Admission of the exact profile's account handles before its first spawn.

/// The model proxy transport used by account bindings.
pub use lys_home::proxy as model_proxy;

use axum::http::StatusCode;
use lys_runner::Rotation;
use lys_runner::rotation::RotationState;
use std::str::FromStr;

use crate::ServerError;
use crate::launch_template::HandleName;
use crate::provisioning_store::Version;

/// Check every account against the agent's live handle inventory.
///
/// The caller supplies the inventory read for this agent from the broker.
/// Secret names cannot stand in for handle ids, including later accounts
/// that will only be used after a usage limit.
///
/// # Errors
/// Returns `account_handle_unheld` for an account outside that inventory,
/// or the runner's named refusal for a malformed rotation declaration.
pub fn accounts(version: &Version, held: &[HandleName]) -> Result<Option<Rotation>, ServerError> {
    let Some(rotation) = version
        .settings
        .session
        .as_ref()
        .and_then(|session| session.accounts.as_ref())
    else {
        return Ok(None);
    };
    RotationState::new(rotation.clone())?;
    for account in &rotation.accounts {
        if !held.iter().any(|handle| handle.id == *account) {
            return Err(ServerError::SecretsRefused {
                status: StatusCode::FORBIDDEN,
                refusal: "account_handle_unheld".to_owned(),
                reason: format!("the agent does not hold account handle `{account}`"),
            });
        }
    }
    Ok(Some(rotation.clone()))
}

/// Admit account ownership and provider routing before review or spawn.
///
/// # Errors
/// Refuses unheld or misrouted handles and unavailable broker inventory.
pub(crate) async fn admit(
    state: &crate::routes::AppState,
    headers: &axum::http::HeaderMap,
    agent: &str,
    version: &Version,
) -> Result<Option<String>, ServerError> {
    if version
        .settings
        .session
        .as_ref()
        .and_then(|s| s.accounts.as_ref())
        .is_none()
    {
        return Ok(None);
    }
    let held = crate::start_checks::handles(state, headers, agent).await?;
    let Some(rotation) = accounts(version, &held)? else {
        return Ok(None);
    };
    let inventory = crate::start_checks::inventory(
        state,
        headers,
        agent,
        crate::start_checks::Inventory::ProviderRoutes,
    )
    .await?;
    routed(version, &rotation, &held, &inventory).map(Some)
}

/// Check each held account against the declared harness's provider.
///
/// # Errors
/// Names the first misrouted account or unsupported account harness.
pub fn routed(
    version: &Version,
    rotation: &Rotation,
    held: &[HandleName],
    inventory: &serde_json::Value,
) -> Result<String, ServerError> {
    let contract = version
        .settings
        .harness
        .as_ref()
        .map_or("", |h| h.description.rendering_contract.as_str());
    let providers: &[&str] = match contract {
        "claude-code/template-v1" => &["https://api.anthropic.com"],
        "codex/template-v1" => &[
            "https://api.openai.com",
            "https://chatgpt.com/backend-api/codex",
        ],
        _ => &[],
    };
    let entries = inventory
        .get("secrets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ServerError::SecretsUnavailable {
            reason: "the broker account route inventory is unreadable".to_owned(),
        })?;
    let mut selected = None;
    for source in &rotation.accounts {
        let handle =
            held.iter()
                .find(|h| h.id == *source)
                .ok_or_else(|| ServerError::SecretsRefused {
                    status: StatusCode::FORBIDDEN,
                    refusal: "account_handle_unheld".to_owned(),
                    reason: format!("the agent does not hold account handle `{source}`"),
                })?;
        let entry = entries
            .iter()
            .find(|e| e["name"].as_str() == Some(handle.secret.as_str()));
        let route = entry.and_then(|e| e["upstream"].as_str());
        let valid = entry
            .is_some_and(|e| e["class"] == "credential" && e["header"] == "authorization")
            && route.is_some_and(|r| providers.contains(&r));
        if !valid || selected.is_some_and(|s| Some(s) != route) {
            return Err(ServerError::SecretsRefused {
                status: StatusCode::FORBIDDEN,
                refusal: "account_provider_mismatch".to_owned(),
                reason: format!(
                    "account handle `{source}` is not routed to the profile harness's provider"
                ),
            });
        }
        selected = route;
    }
    selected
        .map(str::to_owned)
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "account rotation names no provider".to_owned(),
        })
}

pub(crate) async fn bind(
    state: &crate::routes::AppState,
    agent: &str,
    caller: &str,
    launch: &mut lys_runner::Launch,
) -> Result<(), ServerError> {
    let Some(rotation) = launch.rotation.as_ref() else {
        return Ok(());
    };
    let run = launch
        .environment
        .get("LYS_RUN")
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "accounts require the model proxy and a run key".to_owned(),
        })?;
    let number = launch
        .environment
        .get("LYS_PROVISIONING_VERSION")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "account launch names no profile version".to_owned(),
        })?;
    let version = crate::provisioning_api::with_provisioning(state, |store| {
        store
            .version(agent, number)
            .cloned()
            .ok_or(ServerError::ProfileVersionUnknown { version: number })
    })?;
    let actor = lys_identity::PersonId::from_str(caller)?;
    let handles = held(state, &actor.to_string(), agent).await?;
    drop(accounts(&version, &handles)?);
    let inventory = crate::secrets_api::ask_as(
        state,
        &actor.to_string(),
        axum::http::Method::GET,
        "/_lys/secrets",
        axum::body::Bytes::new(),
    )
    .await?;
    let provider = routed(&version, rotation, &handles, &inventory)?;
    let proxy = state
        .model_proxy
        .as_ref()
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: "account model proxy absent".to_owned(),
        })?;
    let origin = proxy
        .strip_suffix("/anthropic")
        .unwrap_or(proxy)
        .trim_end_matches('/');
    crate::secrets_api::bind_proxy(
        state,
        &actor.to_string(),
        origin,
        run,
        agent,
        rotation,
        &provider,
    )
    .await?;
    let variable = match version
        .settings
        .harness
        .as_ref()
        .map(|h| h.description.rendering_contract.as_str())
    {
        Some("claude-code/template-v1") => "ANTHROPIC_AUTH_TOKEN",
        Some("codex/template-v1") => "CODEX_API_KEY",
        _ => {
            return Err(ServerError::LaunchUnrenderable {
                reason: "account harness unsupported".to_owned(),
            });
        }
    };
    if variable == "ANTHROPIC_AUTH_TOKEN" {
        launch
            .environment
            .insert("CLAUDE_CODE_OAUTH_TOKEN".to_owned(), String::new());
        launch
            .environment
            .insert("ANTHROPIC_API_KEY".to_owned(), String::new());
    }
    launch
        .environment
        .insert("LYS_ACCOUNT_VARIABLE".to_owned(), variable.to_owned());
    launch.environment.insert(
        variable.to_owned(),
        format!(
            "lys-account.{}",
            rotation
                .accounts
                .first()
                .ok_or_else(|| ServerError::LaunchUnrenderable {
                    reason: "account rotation empty".to_owned()
                })?
        ),
    );
    Ok(())
}

async fn held(
    state: &crate::routes::AppState,
    person: &str,
    agent: &str,
) -> Result<Vec<HandleName>, ServerError> {
    let held = crate::secrets_api::ask_as(
        state,
        person,
        axum::http::Method::GET,
        &format!("/_lys/handles?holder={agent}"),
        axum::body::Bytes::new(),
    )
    .await?;
    held["handles"]
        .as_array()
        .ok_or_else(|| ServerError::SecretsUnavailable {
            reason: "account handle inventory unreadable".to_owned(),
        })?
        .iter()
        .filter(|h| h["dropped"] == false)
        .map(|h| {
            let id = h["id"]
                .as_str()
                .ok_or_else(|| ServerError::SecretsUnavailable {
                    reason: "account handle id absent".to_owned(),
                })?;
            let secret = h["secret"]
                .as_str()
                .ok_or_else(|| ServerError::SecretsUnavailable {
                    reason: "account handle secret absent".to_owned(),
                })?;
            Ok(HandleName {
                id: id.to_owned(),
                secret: secret.to_owned(),
                env: String::new(),
            })
        })
        .collect::<Result<Vec<_>, ServerError>>()
}
