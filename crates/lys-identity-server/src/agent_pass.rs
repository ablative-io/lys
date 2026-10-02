//! Run-pass verification and event-driven invalidation, without directory locks.
use crate::agent_pass_store::Passes;
use crate::error::ServerError;
use crate::routes::AppState;
use axum::http::{HeaderMap, header};
use lys_identity::{AgentId, Provenance};
use std::sync::MutexGuard;

/// The sole header carrying a pass for a Lys-started run.
pub const HEADER: &str = "lys-agent-pass";

fn refused(reason: &str) -> ServerError {
    ServerError::AgentPassRefused {
        reason: reason.to_owned(),
    }
}

pub(crate) fn store(state: &AppState) -> Result<MutexGuard<'_, Passes>, ServerError> {
    state
        .agent_passes
        .lock()
        .map_err(|error| refused(&format!("the agent pass store lock is poisoned: {error}")))
}

/// Resolve only the cached pass; lifecycle and grants are checked by admission.
/// No directory lock is taken, including when admission already holds it.
pub fn holder(state: &AppState, headers: &HeaderMap) -> Result<Option<AgentId>, ServerError> {
    let Some(pass) = value(headers)? else {
        return Ok(None);
    };
    let (agent, session) = {
        let passes = store(state)?;
        (passes.lookup(pass)?, passes.session_of(pass)?)
    };
    crate::agent_seat::check(state, headers, agent, &session, pass)?;
    Ok(Some(agent))
}

/// The run a pass is for, once its seat signed it.
pub(crate) fn verified(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<(AgentId, Provenance)>, ServerError> {
    let Some(pass) = value(headers)? else {
        return Ok(None);
    };
    let (run, session) = {
        let passes = store(state)?;
        (passes.lookup_run(pass)?, passes.session_of(pass)?)
    };
    crate::agent_seat::check(state, headers, run.0, &session, pass)?;
    Ok(Some(run))
}

fn value(headers: &HeaderMap) -> Result<Option<&str>, ServerError> {
    if !headers.contains_key(HEADER) {
        return Ok(None);
    }
    if headers.contains_key(header::COOKIE)
        || headers.contains_key(header::AUTHORIZATION)
        || headers.contains_key("lys-agent-signature")
        || headers.contains_key(crate::grant_tokens::HEADER)
    {
        return Err(refused(
            "a run pass cannot be combined with another credential",
        ));
    }
    if headers.get_all(HEADER).iter().count() != 1 {
        return Err(refused("exactly one run pass is required"));
    }
    headers
        .get(HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|pass| pass.len() == 43)
        .map(Some)
        .ok_or_else(|| refused("the run pass is malformed"))
}

pub(crate) fn end_agent(state: &AppState, agent: AgentId) -> Result<(), ServerError> {
    store(state)?.end_agent(agent)
}
pub(crate) fn end_session(state: &AppState, session: &str) -> Result<(), ServerError> {
    store(state)?.end_session(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    #[test]
    fn agent_pass_refuses_mixed_and_duplicate_credentials_without_echoing_them()
    -> Result<(), Box<dyn Error>> {
        for other in [
            header::COOKIE.as_str(),
            header::AUTHORIZATION.as_str(),
            "lys-agent-signature",
            crate::grant_tokens::HEADER,
        ] {
            let mut headers = HeaderMap::new();
            let pass = "a".repeat(43);
            headers.insert(HEADER, pass.parse()?);
            headers.insert(
                axum::http::HeaderName::from_bytes(other.as_bytes())?,
                "other".parse()?,
            );
            let error = value(&headers).err().ok_or("mixed credentials accepted")?;
            assert!(matches!(error, ServerError::AgentPassRefused { .. }));
            assert!(!error.to_string().contains(&pass));
        }
        let mut headers = HeaderMap::new();
        headers.append(HEADER, "a".repeat(43).parse()?);
        headers.append(HEADER, "b".repeat(43).parse()?);
        assert!(value(&headers).is_err());
        assert!(value(&HeaderMap::new())?.is_none());
        Ok(())
    }
}
