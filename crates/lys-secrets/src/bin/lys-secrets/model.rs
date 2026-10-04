//! Model accounts: a long-lived token a person registers for the AIs Lys
//! starts, sealed here like any secret and never read back out.
//!
//! A run never holds the token. When a run is started on an account the
//! identity service asks for a draw: a handle on the account, issued to the
//! agent and bound to the model proxy's key, so only the model proxy can
//! present it. The run is given that handle as its placeholder; the model
//! proxy presents it here with each call, the broker judges the agent's
//! grant on the account at every use, writes the sealed token into the one
//! header the account's route names, and takes it out of whatever comes
//! back (see `serve`). Who drew from which account, and how many calls, is
//! what the audit log counts on each draw.
//!
//! The record of each account (its name, its program, who registered it and
//! when, and when it was retired) is `model-accounts.json` beside the
//! routes; it holds no value.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use lys_secrets::{EntryClass, Secret, SecretsError, from_hex, token_text};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::callers::{caller, refused};
use crate::files::{Layout, now_ms};
use crate::serve::{Shared, on_broker};
use crate::values::{Answer, checked_route, repeated, retired, route_words, vouched};

/// The name the model proxy is trusted by.
pub const PROXY_SERVICE: &str = "lys-proxy";

/// Every model account, by the secret it is sealed as.
pub type Models = BTreeMap<String, ModelAccount>;

/// One model account as its record keeps it. No value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelAccount {
    /// The name a person gave it.
    pub name: String,
    /// The program its token is for, as the harness catalogue names it.
    pub harness: String,
    /// Who registered it.
    pub registered_by: String,
    /// When, in milliseconds since the epoch.
    pub registered_at: i64,
    /// The operation id it was registered under.
    pub operation: String,
    /// When it was retired, once it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired_at: Option<i64>,
}

fn models(layout: &Layout) -> Result<Arc<Models>, SecretsError> {
    layout.models.get(|bytes| {
        serde_json::from_slice(bytes).map_err(|error| SecretsError::Encoding {
            context: "model accounts file",
            reason: error.to_string(),
        })
    })
}

fn put(layout: &Layout, secret: &str, account: ModelAccount) -> Result<(), SecretsError> {
    let mut all = Models::clone(&models(layout)?);
    all.insert(secret.to_owned(), account);
    let bytes = serde_json::to_vec_pretty(&all).map_err(|error| SecretsError::Encoding {
        context: "model accounts file",
        reason: error.to_string(),
    })?;
    layout.models.write(&bytes)
}

fn refusal(status: StatusCode, name: &str, words: &str) -> (StatusCode, String) {
    (status, format!("{name}: {words}\n"))
}

/// The secret an account registered under `operation` is sealed as: made
/// from the operation id alone, so it is a path segment of the proxy's.
fn secret_of(operation: &str) -> String {
    format!("model-account-{operation}")
}

const REGISTER: &str = "{operation, name, harness, upstream, header, prefix, token}, each a string";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Register {
    operation: String,
    name: String,
    harness: String,
    upstream: String,
    header: String,
    prefix: String,
    token: String,
}

fn plain(text: &str) -> bool {
    !text.trim().is_empty() && !text.chars().any(char::is_control)
}

/// Registers a model account as the person: its token sealed as a
/// credential they own, routed where the program's calls go, and recorded.
/// A resend of the same operation answers the account and writes its route
/// and record again.
pub async fn register(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<Register>(&shared, request, REGISTER).await?;
    let token = Zeroizing::new(asked.token);
    if token.is_empty() {
        return Err(refusal(
            StatusCode::BAD_REQUEST,
            "ValueEmpty",
            "a model account is registered with its token",
        ));
    }
    if !plain(&asked.name) || !plain(&asked.harness) {
        return Err(refusal(
            StatusCode::BAD_REQUEST,
            "RequestMalformed",
            "an account's name and program are plain text",
        ));
    }
    let route = checked_route(&asked.upstream, &asked.header, &asked.prefix)?;
    let secret = secret_of(&asked.operation);
    let held = models(&shared.layout).map_err(|error| refused(&error))?;
    let taken = held.iter().any(|(other, account)| {
        *other != secret && account.retired_at.is_none() && account.name == asked.name.trim()
    });
    if taken {
        return Err(refusal(
            StatusCode::CONFLICT,
            "ModelAccountNameTaken",
            &format!(
                "a model account named {} is registered already; choose another name",
                asked.name.trim()
            ),
        ));
    }
    let words = format!(
        "{} for {} named {}",
        route_words(&route),
        asked.harness,
        asked.name.trim()
    );
    let layout = shared.layout.clone();
    let Register {
        operation,
        name,
        harness,
        ..
    } = asked;
    on_broker(&shared, move |broker| {
        let sealed = Secret::from_slice(token.as_bytes());
        let changed = broker.add_secret(
            &who.identity,
            (&secret, EntryClass::Credential, &sealed),
            &words,
            (who.via.as_deref(), Some(operation.as_str())),
        )?;
        layout.add_route(&secret, route)?;
        let account = match models(&layout)?.get(&secret) {
            Some(kept) => kept.clone(),
            None => ModelAccount {
                name: name.trim().to_owned(),
                harness,
                registered_by: who.identity.clone(),
                registered_at: now_ms(),
                operation: operation.clone(),
                retired_at: None,
            },
        };
        put(&layout, &secret, account.clone())?;
        Ok::<_, SecretsError>(Json(json!({
            "account": secret,
            "name": account.name,
            "harness": account.harness,
            "registered_by": account.registered_by,
            "registered_at": account.registered_at,
            "operation": operation,
            "repeated": repeated(&changed),
        })))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

/// Every model account the caller may discover, or registered themselves,
/// with who drew from it and how many calls the log counts for each.
pub async fn accounts(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, _body) = request.into_parts();
    let who = caller(&shared, &parts, &[]).await?;
    let held = models(&shared.layout).map_err(|error| refused(&error))?;
    on_broker(&shared, move |broker| {
        let listed: Vec<Value> = held
            .iter()
            .filter(|(secret, account)| {
                account.registered_by == who.identity || broker.discovers(&who.identity, secret)
            })
            .map(|(secret, account)| {
                let mut drawn: BTreeMap<String, (u64, u64)> = BTreeMap::new();
                for draw in broker.draws(secret) {
                    let counted = drawn.entry(draw.identity).or_default();
                    counted.0 = counted.0.saturating_add(draw.uses);
                    if !draw.dropped {
                        counted.1 = counted.1.saturating_add(1);
                    }
                }
                let draws: Vec<Value> = drawn
                    .into_iter()
                    .map(|(identity, (calls, live))| {
                        json!({ "identity": identity, "calls": calls, "live": live })
                    })
                    .collect();
                json!({
                    "account": secret,
                    "name": account.name,
                    "harness": account.harness,
                    "registered_by": account.registered_by,
                    "registered_at": account.registered_at,
                    "retired_at": account.retired_at,
                    "draws": draws,
                })
            })
            .collect();
        Json(json!({ "accounts": listed }))
    })
    .await
    .map_err(|error| refused(&error))
}

const RETIRE: &str = "{operation, account}, each a string";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetireAccount {
    operation: String,
    account: String,
}

/// Retires a model account, as its owner: every draw on it ends, its token
/// leaves the store, and its record says when.
pub async fn retire(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = vouched::<RetireAccount>(&shared, request, RETIRE).await?;
    let known = models(&shared.layout)
        .map_err(|error| refused(&error))?
        .get(&asked.account)
        .cloned()
        .ok_or_else(|| unknown(&asked.account))?;
    let (changed, at) = retired(
        &shared,
        who,
        asked.account.clone(),
        asked.operation.clone(),
    )
    .await?;
    let account = ModelAccount {
        retired_at: Some(known.retired_at.unwrap_or(at)),
        ..known
    };
    put(&shared.layout, &asked.account, account.clone()).map_err(|error| refused(&error))?;
    Ok(Json(json!({
        "account": asked.account,
        "name": account.name,
        "retired_at": account.retired_at,
        "operation": asked.operation,
        "repeated": repeated(&changed),
    })))
}

fn unknown(account: &str) -> (StatusCode, String) {
    refusal(
        StatusCode::NOT_FOUND,
        "ModelAccountUnknown",
        &format!("no model account {account} is registered"),
    )
}

const DRAW: &str = "{account, agent}, each a string";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DrawAsked {
    account: String,
    agent: String,
}

/// Issues a draw for a run about to start: a handle on the account for the
/// agent, bound to the model proxy's key. Answers the handle's id and the
/// handle, which is the run's placeholder; it opens nothing without the
/// model proxy's key, and the account's token is never in the answer.
pub async fn draw(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (_who, asked) = vouched::<DrawAsked>(&shared, request, DRAW).await?;
    let account = models(&shared.layout)
        .map_err(|error| refused(&error))?
        .get(&asked.account)
        .cloned()
        .ok_or_else(|| unknown(&asked.account))?;
    if account.retired_at.is_some() {
        return Err(refusal(
            StatusCode::CONFLICT,
            "ModelAccountRetired",
            &format!("model account {} was retired", account.name),
        ));
    }
    let services = shared
        .layout
        .services()
        .map_err(|error| refused(&error))?;
    let proxy_key: [u8; 32] = services
        .iter()
        .find(|service| service.name == PROXY_SERVICE)
        .and_then(|service| from_hex(&service.public_key))
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| {
            refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "ModelProxyUntrusted",
                "the secrets broker trusts no model proxy, so no run can draw from a model account; install Lys again so the broker trusts its model proxy",
            )
        })?;
    let (agent, secret, name) = (asked.agent, asked.account, account.name);
    on_broker(&shared, move |broker| {
        broker
            .issue_draw(&agent, proxy_key, &secret)
            .map(|issued| {
                Json(json!({
                    "account": secret,
                    "agent": agent,
                    "handle": issued.id.as_str(),
                    "token": token_text(&issued),
                }))
            })
            .map_err(|error| {
                refusal(
                    StatusCode::FORBIDDEN,
                    "ModelAccountNotDrawable",
                    &format!(
                        "agent {agent} may not draw from model account {name}: {} ({error})",
                        error.name()
                    ),
                )
            })
    })
    .await
    .map_err(|error| refused(&error))?
}
