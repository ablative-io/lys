//! A product's caller acting by the pass Lys issued it (ACCESS-002, used by
//! `POST /product-drafts` first): `Authorization: Bearer <pass>`.
//!
//! The pass is verified as every product verifies it, by lys-pass's own
//! verifier, against the keys the provider publishes now (the current key
//! and each retired key a live pass may still name), for Lys's issuer, the
//! audience the route names and the instant of the request. It must also
//! still be one Lys keeps: a pass revoked with its code's replay or its
//! person's account, or whose Lys sign-in has ended, is `TokenUnknown`.
//! Its subject is the acting identity. A pass that does not verify is
//! refused `PassRefused`, naming lys-pass's refusal, and never passed over
//! for another credential.
//!
//! The provider's own locks are taken and released here; the caller takes
//! the directory, the apps and the grants only after.

use axum::http::{HeaderMap, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_identity::IdentityId;
use lys_pass::{KeySet, VerifiedPass};
use sha2::{Digest, Sha256};

use super::endpoints::provider;
use super::{Kind, held, unavailable};
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::routes::{AppState, hex, identity_id};
use crate::session::now;

/// The pass a request presents as its bearer, when its bearer is a compact
/// JWS (its header, base64url JSON, begins `eyJ`) and not an app's or a
/// registrar's credential.
pub(crate) fn presented_pass(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| token.starts_with("eyJ") && token.split('.').count() == 3)
}

/// The identity `pass` names, verified for the app `audience` at this
/// request.
pub(crate) fn pass_holder(
    state: &AppState,
    pass: &str,
    audience: &str,
) -> Result<IdentityId, ServerError> {
    let verified = verified_pass(state, pass, audience)?;
    retained_holder(state, pass, audience, &verified)
}

fn verified_pass(
    state: &AppState,
    pass: &str,
    audience: &str,
) -> Result<VerifiedPass, ServerError> {
    let provider = provider(state)?;
    let at = now();
    let published = held(&provider.keys)?.published(at, provider.published_seconds());
    let keys = KeySet::from_json(&published.to_string()).map_err(|error| {
        unavailable(format!(
            "the provider's own key set does not read as published: {}",
            error.name()
        ))
    })?;
    VerifiedPass::verify(pass, &keys, &provider.issuer, audience, at).map_err(|error| {
        ServerError::Provider(ProviderError::PassRefused {
            refusal: error.name().to_owned(),
        })
    })
}

fn retained_holder(
    state: &AppState,
    pass: &str,
    audience: &str,
    verified: &VerifiedPass,
) -> Result<IdentityId, ServerError> {
    let provider = provider(state)?;
    let at = now();
    if let IdentityId::Agent(agent) = identity_id(&verified.claims().sub)? {
        held(&provider.agent_tokens)?.admits(pass, agent, audience, at)?;
        return Ok(IdentityId::Agent(agent));
    }
    let session = {
        let tokens = held(&provider.tokens)?;
        let kept = tokens.get(&hex(&Sha256::digest(pass.as_bytes())), at)?;
        if kept.kind == Some(Kind::Refresh)
            || kept.app != audience
            || kept.subject != verified.claims().sub
        {
            return Err(ServerError::Provider(ProviderError::TokenUnknown));
        }
        kept.session_id.clone()
    };
    if !state.sessions.is_live(&session)? {
        return Err(ServerError::Provider(ProviderError::TokenUnknown));
    }
    identity_id(&verified.claims().sub)
}

/// A person's kept access pass, verified for its own signed audience.
pub(crate) fn product_person(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<lys_identity::PersonId, ServerError> {
    let pass = presented_pass(headers).ok_or(ServerError::Provider(ProviderError::TokenUnknown))?;
    if headers.contains_key(header::COOKIE) || headers.contains_key("lys-operator") {
        return Err(ProviderError::PassRefused {
            refusal: "mixed_credentials".to_owned(),
        }
        .into());
    }
    let refused = || ProviderError::PassRefused {
        refusal: "pass_audience_invalid".to_owned(),
    };
    let payload = pass.split('.').nth(1).ok_or_else(refused)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| ProviderError::PassRefused {
            refusal: format!("pass_payload_invalid: {error}"),
        })?;
    let claims: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| ProviderError::PassRefused {
            refusal: format!(
                "pass_claims_invalid: {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        })?;
    let audience = claims["aud"]
        .as_str()
        .filter(|audience| !audience.is_empty())
        .ok_or_else(refused)?;
    // The unverified audience selects the verifier; the kept, signed pass grants authority.
    let verified = verified_pass(state, pass, audience)?;
    if verified.claims().holder.kind != "person" {
        return Err(ServerError::ProductPassPersonRequired);
    }
    match retained_holder(state, pass, audience, &verified)? {
        IdentityId::Person(person) => Ok(person),
        _ => Err(ServerError::ProductPassPersonRequired),
    }
}

/// A live person pass for Lys itself retains its issuing sign-in's actor.
pub(crate) fn personal_actor(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<lys_identity::Actor>, ServerError> {
    let Some(pass) = presented_pass(headers) else {
        return Ok(None);
    };
    let payload = pass.split('.').nth(1).ok_or(ServerError::NotSignedIn)?;
    let claims: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).map_err(|error| {
            ServerError::Provider(ProviderError::PassRefused {
                refusal: format!("person_payload_invalid: {error}"),
            })
        })?)
        .map_err(|error| {
            ServerError::Provider(ProviderError::PassRefused {
                refusal: format!("person_claims_invalid: {error}"),
            })
        })?;
    // The audience selects the verifier; only verified claims grant authority.
    if claims["aud"] != "lys"
        || !claims["sub"]
            .as_str()
            .is_some_and(|subject| subject.starts_with("person-"))
    {
        return Ok(None);
    }
    if headers.contains_key(header::COOKIE) || headers.contains_key("lys-operator") {
        return Err(ServerError::NotSignedIn);
    }
    if !matches!(pass_holder(state, pass, "lys")?, IdentityId::Person(_)) {
        return Ok(None);
    }
    let provider = provider(state)?;
    let session = held(&provider.tokens)?
        .get(&hex(&Sha256::digest(pass.as_bytes())), now())?
        .session_id
        .clone();
    state
        .sessions
        .live(|_| true)?
        .into_iter()
        .find(|entry| entry.id == session)
        .map(|entry| Some(entry.actor))
        .ok_or(ServerError::Provider(ProviderError::TokenUnknown))
}

/// An approved app's person pass retains only its issuing session's actor.
pub(crate) fn app_person_actor(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<lys_identity::Actor>, ServerError> {
    Ok(app_person(state, headers)?.map(|(actor, _app)| actor))
}

/// An approved app's person pass: its issuing session's actor and the app
/// whose audience it carries (DIRECTORY-113 names the app that admits).
pub(crate) fn app_person(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<(lys_identity::Actor, String)>, ServerError> {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|error| ProviderError::PassRefused {
        refusal: format!("authorization_header_invalid: {error}"),
    })?;
    let Some(bearer) = value.strip_prefix("Bearer ") else {
        return Ok(None);
    };
    if ["lys-app.", "lys-registrar.", "lys-cli."]
        .iter()
        .any(|scheme| bearer.starts_with(scheme))
    {
        return Ok(None);
    }
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(ProviderError::PassRefused {
            refusal: "mixed_credentials".to_owned(),
        }
        .into());
    }
    // Lys's own verified person pass keeps the sign-in's administrator authority.
    if personal_actor(state, headers)?.is_some() {
        return Ok(None);
    }
    let person = product_person(state, headers)?;
    let pass = presented_pass(headers).ok_or(ProviderError::TokenUnknown)?;
    let provider = provider(state)?;
    let (session, app) = {
        let tokens = held(&provider.tokens)?;
        let kept = tokens.get(&hex(&Sha256::digest(pass.as_bytes())), now())?;
        (kept.session_id.clone(), kept.app.clone())
    };
    let actor = state
        .sessions
        .live(|_| true)?
        .into_iter()
        .find(|entry| entry.id == session)
        .map(|entry| entry.actor)
        .ok_or(ProviderError::TokenUnknown)?;
    crate::apps_api::with_apps(state, |apps, directory| {
        let approved = apps
            .app(&app)
            .ok_or(crate::apps_error::AppError::CredentialRefused {
                reason: "no approved app holds that audience",
            })?;
        match approved.standing() {
            crate::apps_state::Standing::Approved => {}
            crate::apps_state::Standing::Pending | crate::apps_state::Standing::Declined => {
                return Err(
                    crate::apps_error::AppError::AppNotApproved { app: app.clone() }.into(),
                );
            }
            crate::apps_state::Standing::Retired => {
                return Err(crate::apps_error::AppError::AppRetired { app: app.clone() }.into());
            }
        }
        if crate::caller_admission::active_caller(directory, &actor)? != IdentityId::Person(person)
        {
            return Err(ProviderError::TokenUnknown.into());
        }
        Ok(())
    })?;
    Ok(Some((actor, app)))
}

/// An approved app's agent pass: the agent it was issued to, as its own
/// actor, and the app whose audience it carries (AGENTS-019 R2). None for
/// any other credential, a person's pass included; a pass that does not
/// verify, is no longer retained, or names an app that is not approved is
/// refused by name.
pub(crate) fn app_agent(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<(lys_identity::Actor, String)>, ServerError> {
    let Some(pass) = presented_pass(headers) else {
        return Ok(None);
    };
    let payload = pass.split('.').nth(1).ok_or(ServerError::NotSignedIn)?;
    let claims: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).map_err(|error| {
            ServerError::Provider(ProviderError::PassRefused {
                refusal: format!("agent_payload_invalid: {error}"),
            })
        })?)
        .map_err(|error| {
            ServerError::Provider(ProviderError::PassRefused {
                refusal: format!("agent_claims_invalid: {error}"),
            })
        })?;
    let (Some(audience), Some(subject)) = (claims["aud"].as_str(), claims["sub"].as_str()) else {
        return Ok(None);
    };
    if audience == "lys" || !subject.starts_with("agent-") {
        return Ok(None);
    }
    if headers.contains_key(header::COOKIE) || headers.contains_key("lys-operator") {
        return Err(ProviderError::PassRefused {
            refusal: "mixed_credentials".to_owned(),
        }
        .into());
    }
    let IdentityId::Agent(agent) = pass_holder(state, pass, audience)? else {
        return Ok(None);
    };
    let actor = lys_identity::Actor::new(
        lys_identity::LoginBinding::new(state.oidc.issuer(), &agent.to_string())?,
        lys_identity::Provenance::new(lys_identity::AuthMethod::AgentPass(agent), now()),
    );
    crate::apps_api::with_apps(state, |apps, directory| {
        let approved =
            apps.app(audience)
                .ok_or(crate::apps_error::AppError::CredentialRefused {
                    reason: "no approved app holds that audience",
                })?;
        if !matches!(approved.standing(), crate::apps_state::Standing::Approved) {
            return Err(crate::apps_error::AppError::AppNotApproved {
                app: audience.to_owned(),
            }
            .into());
        }
        crate::caller_admission::active_caller(directory, &actor)?;
        Ok(())
    })?;
    Ok(Some((actor, audience.to_owned())))
}

#[cfg(test)]
#[path = "personal_pass_authority_tests.rs"]
mod personal_pass_authority_tests;
