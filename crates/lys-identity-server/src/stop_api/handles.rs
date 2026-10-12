//! End credential handles through the authenticated caller or recorded owner.

use super::{AgentId, AppState, IdentityId, ServerError, part, with_directory};
use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde_json::{Value, json};
use std::str::FromStr;

/// End every handle the agent holds at the broker, each under an operation
/// id made from the stop's, so a stop cut off and sent again ends nothing
/// twice; a handle ended by this stop counts, one ended by anyone else does
/// not.
pub(crate) async fn end_handles(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
    operation: &str,
) -> Result<Vec<String>, ServerError> {
    end_handles_by(state, HandleCaller::Session(headers), agent, operation).await
}

/// A console stop speaks to the broker for the agent's recorded responsible person.
pub(crate) async fn end_handles_as_owner(
    state: &AppState,
    agent: &str,
    operation: &str,
) -> Result<Vec<String>, ServerError> {
    let id = AgentId::from_str(agent)?;
    let person = with_directory(state, |directory| {
        directory
            .projection()?
            .record(IdentityId::Agent(id))
            .ok_or(ServerError::AgentNotVisible)?
            .responsible()
            .map(|person| person.to_string())
            .ok_or(ServerError::NoPerson)
    })?;
    end_handles_by(state, HandleCaller::Owner(&person), agent, operation).await
}

#[derive(Clone, Copy)]
enum HandleCaller<'a> {
    Session(&'a HeaderMap),
    Owner(&'a str),
}

impl HandleCaller<'_> {
    async fn ask(
        self,
        state: &AppState,
        method: Method,
        path: &str,
        body: Bytes,
    ) -> Result<Value, ServerError> {
        match self {
            Self::Session(headers) => {
                crate::secrets_api::ask(state, headers, method, path, body).await
            }
            Self::Owner(person) => {
                crate::secrets_api::ask_as(state, person, method, path, body).await
            }
        }
    }
}

async fn end_handles_by(
    state: &AppState,
    caller: HandleCaller<'_>,
    agent: &str,
    operation: &str,
) -> Result<Vec<String>, ServerError> {
    let path = format!("/_lys/handles?holder={agent}");
    let held = caller.ask(state, Method::GET, &path, Bytes::new()).await?;
    let handles = held["handles"]
        .as_array()
        .ok_or_else(|| ServerError::SecretsUnavailable {
            reason: "the broker's handles answer lists no handles member".to_owned(),
        })?;
    let mut ended = Vec::new();
    for handle in handles {
        let Some(id) = handle["id"].as_str() else {
            return Err(ServerError::SecretsUnavailable {
                reason: "the broker listed a handle without an id".to_owned(),
            });
        };
        let body = json!({ "handle": id, "operation": part(operation, "handle", id) });
        let answer: Value = caller
            .ask(
                state,
                Method::POST,
                "/_lys/drop",
                Bytes::from(body.to_string()),
            )
            .await?;
        if answer["handle"].as_str() != Some(id) {
            return Err(ServerError::SecretsUnavailable {
                reason: format!("the broker did not confirm ending handle {id}"),
            });
        }
        match answer["outcome"].as_str() {
            Some("ended" | "repeated") => ended.push(id.to_owned()),
            Some("already_ended") => {}
            other => {
                return Err(ServerError::SecretsUnavailable {
                    reason: format!("the broker answered handle {id} with outcome {other:?}"),
                });
            }
        }
    }
    Ok(ended)
}
