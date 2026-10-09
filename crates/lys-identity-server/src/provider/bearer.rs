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
    let provider = provider(state)?;
    let at = now();
    let published = held(&provider.keys)?.published(at, provider.published_seconds());
    let keys = KeySet::from_json(&published.to_string()).map_err(|error| {
        unavailable(format!(
            "the provider's own key set does not read as published: {}",
            error.name()
        ))
    })?;
    let verified =
        VerifiedPass::verify(pass, &keys, &provider.issuer, audience, at).map_err(|error| {
            ServerError::Provider(ProviderError::PassRefused {
                refusal: error.name().to_owned(),
            })
        })?;
    let session = {
        let tokens = held(&provider.tokens)?;
        let kept = tokens.get(&hex(&Sha256::digest(pass.as_bytes())), at)?;
        if kept.kind == Some(Kind::Refresh) {
            return Err(ServerError::Provider(ProviderError::TokenUnknown));
        }
        kept.session_id.clone()
    };
    if !state.sessions.is_live(&session)? {
        return Err(ServerError::Provider(ProviderError::TokenUnknown));
    }
    identity_id(&verified.claims().sub)
}
