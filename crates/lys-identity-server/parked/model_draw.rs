//! A run started on a model account. An agent's settings name a model
//! account only when a person chose one; with none named, the run uses the
//! machine's own login as it always has (Tom, 3 October 2026).
//!
//! When one is named, the start is checked before it is kept: the declared
//! program must say how it is run from a model account, its model calls
//! must go to Lys's model proxy, and this service must have one. Each is
//! refused by name, as `ModelAccountUnsupported`, saying the program cannot
//! be run from a model account and why.
//!
//! The kept start names the account and the variable, never a token. When
//! the start is sent to its runner, the secrets broker is asked for a draw
//! for the agent (refused there, by name, when the agent may not draw from
//! the account), and the run's environment is given, in the program's
//! variable, the placeholder that names the draw. The account's token stays
//! sealed in the broker: the model proxy presents the draw with its own key
//! on each call, and the broker puts the token in (see `lys-home`'s
//! `proxy::account`). The placeholder is never kept: a start sent again
//! that its runner already holds draws nothing.

use std::sync::Arc;

use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::provisioning_store::Version;
use crate::routes::AppState;

/// The refusal a start on a model account its program cannot use is given.
pub(crate) const UNSUPPORTED: &str = "ModelAccountUnsupported";

fn unsupported(reason: String) -> ServerError {
    ServerError::ModelAccount {
        status: StatusCode::CONFLICT,
        refusal: UNSUPPORTED.to_owned(),
        reason,
    }
}

/// What a start on `version` draws from, as the kept start names it: the
/// account and the variable the program reads its token from; none when
/// its settings name no account.
///
/// # Errors
///
/// `HarnessUndeclared`, and `ModelAccountUnsupported` naming why the
/// program cannot be run from a model account here.
pub(crate) fn chosen(state: &AppState, version: &Version) -> Result<Option<Value>, ServerError> {
    let Some(account) = version.settings.model_account.clone() else {
        return Ok(None);
    };
    let harness = version
        .settings
        .harness
        .as_ref()
        .ok_or(ServerError::HarnessUndeclared {
            version: version.number,
        })?;
    let using = harness.description.model_account.as_ref().ok_or_else(|| {
        unsupported(format!(
            "{} cannot be run from a model account: its description names no variable its token is read from; declare it again from the harness catalogue",
            harness.name
        ))
    })?;
    if !crate::start_checks::through_proxy(harness) {
        return Err(unsupported(format!(
            "{} cannot be run from a model account: its model calls are not sent to Lys's model proxy",
            harness.name
        )));
    }
    if state.model_proxy.is_none() {
        return Err(unsupported(format!(
            "{} cannot be run from a model account here: this service is configured with no model proxy to send its calls through",
            harness.name
        )));
    }
    Ok(Some(json!({ "account": account, "variable": using.variable })))
}

/// Give `launch` its draw on the model account the kept start `view`
/// names, when it names one, asked as the person `headers` sign in.
///
/// # Errors
///
/// As [`drawn`], and `LaunchUnrenderable` for a kept start whose account
/// does not read.
pub(crate) async fn draw(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    view: &Value,
    launch: &mut lys_runner::Launch,
) -> Result<(), ServerError> {
    let Some(drawing) = view.get("model_account").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let agent = text(view, "agent")?;
    let person = crate::secrets_api::person(state, headers)?;
    drawn(state, &person, (drawing, &agent), launch).await
}

/// Give `launch`, started from launch record `record` by `caller`, its draw
/// on the model account the record's profile version names, when it names
/// one.
///
/// # Errors
///
/// `LaunchUnrenderable` when the record names no kept version,
/// `ModelAccountUnsupported`, and as [`drawn`].
pub(crate) async fn draw_for_record(
    state: &Arc<AppState>,
    caller: &str,
    record: &lys_identity::start::LaunchRecord,
    launch: &mut lys_runner::Launch,
) -> Result<(), ServerError> {
    let drawing = crate::provisioning_api::with_provisioning(state, |store| {
        let (_agent, version) = store.named(&record.profile_version).ok_or_else(|| {
            ServerError::LaunchUnrenderable {
                reason: "the launch record names no kept provisioning operation".to_owned(),
            }
        })?;
        chosen(state, version)
    })?;
    let Some(drawing) = drawing else {
        return Ok(());
    };
    drawn(state, caller, (&drawing, &record.agent), launch).await
}

fn text(from: &Value, member: &str) -> Result<String, ServerError> {
    from.get(member)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ServerError::LaunchUnrenderable {
            reason: format!("the kept start's model account names no {member}"),
        })
}

/// Ask the secrets broker, as `person`, for a draw on `drawing`'s account
/// for `agent`, and set its placeholder in `drawing`'s variable of
/// `launch`; nothing is drawn when the runner holds the session already.
///
/// # Errors
///
/// The broker's refusal by name (`ModelAccountNotDrawable` when the agent
/// may not draw from the account, `ModelAccountRetired`,
/// `ModelAccountUnknown`, `ModelProxyUntrusted`), and `SecretsUnavailable`.
async fn drawn(
    state: &Arc<AppState>,
    person: &str,
    (drawing, agent): (&Value, &str),
    launch: &mut lys_runner::Launch,
) -> Result<(), ServerError> {
    let (account, variable) = (text(drawing, "account")?, text(drawing, "variable")?);
    let held = crate::agent_pass::store(state)?.has_session(&launch.session)?;
    if held {
        return Ok(());
    }
    let body = serde_json::to_vec(&json!({ "account": account, "agent": agent })).map_err(
        |error| ServerError::SecretsUnavailable {
            reason: format!("the draw could not be written: {error}"),
        },
    )?;
    let answer = crate::secrets_api::ask_as(
        state,
        person,
        Method::POST,
        "/_lys/model/draw",
        Bytes::from(body),
    )
    .await?;
    let unread = || ServerError::SecretsUnavailable {
        reason: "the secrets broker's draw names no handle".to_owned(),
    };
    let handle = answer.get("handle").and_then(Value::as_str).ok_or_else(unread)?;
    let token = answer.get("token").and_then(Value::as_str).ok_or_else(unread)?;
    launch.environment.insert(
        variable,
        lys_home::proxy::account::placeholder(&account, handle, token),
    );
    Ok(())
}
