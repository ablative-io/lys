//! An agent's pass to an app (AGENTS-006 R1): the pass a person or a
//! machine is issued, issued to an agent.
//!
//! An agent signs in nowhere: it proves itself with its run pass or with a
//! grant credential for a live grant it holds (`agent_app_pass.rs` judges
//! that proof before anything here is asked). It is then issued the same
//! pass a person and a machine are, from the same reading
//! (`rights_claim::pass`): its own rights on the audience app's kinds as its
//! live grants give them at issue, never the rights of the person who
//! answers for it, its holder asserted as `agent` with that person named,
//! and a retired agent refused. The audience is an app approved on the Apps
//! screen, judged from the apps' record as it stands at this request; any
//! other is refused `audience_not_approved`, carrying the apps' own words.
//!
//! The pass lives the provider's `pass_seconds` and no longer. No refresh
//! token is issued: the agent asks again. Its digest is retained for issuer
//! admission and audience revocation. Products verifying offline against
//! the published keys still admit a signed copy until its expiry.
//!
//! The apps lock is taken and released before the grants are read; no
//! provider lock is held under either.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use lys_identity::{AgentId, IdentityId};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use super::endpoints::{apps, provider};
use super::exchange_timing::{Act, Timing};
use super::held;
use super::rights_claim::pass;
use crate::apps_binding::sign_in_app;
use crate::error::ServerError;
use crate::error_app_pass::AppPassError;
use crate::routes::AppState;
use crate::session::now;

/// An agent's pass to an app, answered once to the agent that asked.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub(crate) struct AgentAppPass {
    /// The pass: a compact JWS whose claims are lys-pass's.
    pub(crate) pass: String,
    /// Its `exp`, in seconds since the Unix epoch: the first instant it is
    /// no longer good.
    pub(crate) expires_at: u64,
    /// The app the pass is for, as its `aud` names it.
    pub(crate) audience: String,
}

/// The pass of `agent` to the approved app `app`, at this request. The
/// agent is the caller's to have proved.
pub(crate) fn agent_app_pass(
    state: &AppState,
    agent: AgentId,
    app: &str,
) -> Result<AgentAppPass, ServerError> {
    let mut timing = Timing::new("agent", app);
    let provider = provider(state)?;
    let audience = timing.measure(Act::IsLive, || approved_audience(state, app))?;
    let at = now();
    let issued = timing.measure(Act::Pass, || {
        pass(
            state,
            provider,
            (IdentityId::Agent(agent), audience.as_str()),
            (at, at.saturating_add(provider.pass_seconds)),
            false,
        )
    })?;
    held(&provider.agent_tokens)?.issue(&issued.token, agent, &audience, issued.expires_at, at)?;
    timing.cache(issued.cache_hit);
    Ok(timing.measure(Act::Issue, || AgentAppPass {
        pass: issued.token,
        expires_at: issued.expires_at,
        audience,
    }))
}

/// The currently approved audience, shared by issue and revoke.
pub(super) fn approved_audience(state: &AppState, app: &str) -> Result<String, ServerError> {
    let mut apps = apps(state)?;
    apps.settle()?;
    Ok(sign_in_app(apps.held(), app)
        .map_err(|error| AppPassError::AudienceNotApproved {
            audience: app.to_owned(),
            reason: error.to_string(),
        })?
        .registered
        .app
        .clone())
}

/// Remove every retained pass for this agent and approved audience.
pub(crate) fn revoke(
    state: &AppState,
    agent: AgentId,
    app: &str,
) -> Result<(String, usize), ServerError> {
    let provider = provider(state)?;
    let audience = approved_audience(state, app)?;
    let removed = held(&provider.agent_tokens)?.revoke(agent, &audience)?;
    Ok((audience, removed))
}

/// Snapshot only the agent's current signing certificates, releasing their store before decoding.
pub(super) fn key_thumbprints(
    state: &AppState,
    agent: AgentId,
    at: u64,
) -> Result<Vec<String>, ServerError> {
    let Some(store) = state.certificates.as_ref() else {
        // Without a certificate log there are no certificate keys this issuer can attest.
        return Ok(Vec::new());
    };
    let certificates = store
        .lock()
        .map_err(|error| ServerError::CertificatesUnavailable {
            reason: format!("the certificate store for {agent} is unavailable: {error}"),
        })?
        .signing_certificates(&agent.to_string())?;
    let mut keys = Vec::with_capacity(certificates.len());
    for certificate in certificates {
        if let Some(key) = certificate.key_at(at)? {
            keys.push(URL_SAFE_NO_PAD.encode(Sha256::digest(key)));
        }
    }
    keys.sort_unstable();
    keys.dedup();
    Ok(keys)
}
