//! Who an MCP message is from, before any call it carries is relayed: the
//! agent that signed it, verified once over the message exactly as it
//! arrived, or the agent a connected app's bearer token names.

use std::sync::Arc;

use axum::http::header;
use axum::http::request::Parts;
use lys_identity::AgentId;

use crate::error::ServerError;
use crate::routes::AppState;

/// The verified signer and the connected app of one MCP message.
pub(crate) type Callers = (Option<AgentId>, Option<(AgentId, String)>);

/// The agent that signed this MCP message; none when it carries no
/// signature.
pub(crate) fn signer(
    state: Option<&Arc<AppState>>,
    parts: &Parts,
    bytes: &[u8],
) -> Result<Option<AgentId>, ServerError> {
    if !parts.headers.contains_key(crate::agent_signature::HEADER) {
        return Ok(None);
    }
    let state = state.ok_or(ServerError::AgentSignatureRefused {
        reason: "MCP has no signature authority state",
    })?;
    let path = parts.uri.path();
    let path = path.strip_prefix("/api").unwrap_or(path);
    crate::routes::with_directory(state, |directory| {
        crate::agent_signature::signed_agent(
            state,
            directory.projection()?,
            &parts.headers,
            ("POST", path, bytes),
        )
    })
}

/// The agent and app a connected app's bearer token names, refused when the
/// token travels with any other credential.
pub(crate) fn connected_app(
    apps: Option<&crate::mcp_oauth::Apps>,
    parts: &Parts,
) -> Result<Option<(AgentId, String)>, ServerError> {
    let Some(apps) = apps else {
        return Ok(None);
    };
    let Some(app) = apps.bearer(&parts.headers)? else {
        return Ok(None);
    };
    if [
        header::COOKIE,
        header::HeaderName::from_static(crate::agent_signature::HEADER),
        header::HeaderName::from_static(crate::agent_pass::HEADER),
        header::HeaderName::from_static(crate::grant_tokens::HEADER),
    ]
    .iter()
    .any(|name| parts.headers.contains_key(name))
    {
        return Err(ServerError::AgentSignatureRefused {
            reason: "a connected app's token cannot carry another credential",
        });
    }
    Ok(Some(app))
}
