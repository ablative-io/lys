//! The grant binding of an issued pass (DIRECTORY-089 R2).
//!
//! A product asks for it in its token request (`grant_binding`); without
//! the ask the answer is exactly as before bindings existed. With it, the
//! pass's rights are decided as they always are, through the grants' one
//! decision, under one hold, at the revision the grants stand at once the
//! log is settled, each decision required to reflect that revision; a
//! decision from a degraded reading, a projection behind the revision, an
//! unresolved revocation on a path, or grants that moved while the rights
//! were read refuse the whole issue by name. The binding is then signed,
//! after the hold, beside the pass and by the same issuer, naming the pass
//! by the SHA-256 of its exact bytes. The pass and its binding are one
//! answer: when either cannot be made, neither is kept or answered.
//!
//! Its bounds are the pass's own: one dependency per grant the pass's
//! rights name (bounded by `provider.rights_bytes` when set), each path the
//! grant's existing checked ancestry; no new limit is invented.

use std::collections::{BTreeMap, BTreeSet};

use lys_pass::binding::{
    BINDING_REQUEST, BINDING_TYPE, BINDING_VERSION, BindingClaims, Dependency, pass_digest,
};
use lys_pass::{Claims, GrantLog};

use super::{OpenIdProvider, unavailable};
use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;

/// The coherent decision a binding states: the revision the rights were
/// decided at, and each grant's ancestry as the decision read it.
pub(super) struct Decided {
    pub(super) revision: u64,
    pub(super) paths: BTreeMap<String, Vec<String>>,
}

/// Whether the token request asks for a binding: absent, none; the version
/// produced, one; any other version is refused by name before anything is
/// taken or issued.
pub(super) fn asked(version: Option<&str>) -> Result<bool, ServerError> {
    match version {
        None => Ok(false),
        Some(BINDING_REQUEST) => Ok(true),
        Some(other) => Err(GrantStreamError::BindingUnsupported {
            asked: other.to_owned(),
            served: BINDING_VERSION,
        }
        .into()),
    }
}

/// The signed binding of the pass `token`, whose claims are `claims`,
/// decided as `decided` from the grant log `log`.
pub(super) fn binding(
    provider: &OpenIdProvider,
    claims: &Claims,
    token: &str,
    log: GrantLog,
    decided: &Decided,
) -> Result<String, ServerError> {
    let grants: BTreeSet<&String> = claims.rights.iter().map(|right| &right.grant).collect();
    let dependencies = grants
        .into_iter()
        .map(|grant| {
            let path = decided.paths.get(grant).cloned().ok_or_else(|| {
                unavailable(format!(
                    "the decision read no ancestry for the right's grant {grant}"
                ))
            })?;
            Ok(Dependency {
                grant: grant.clone(),
                path,
            })
        })
        .collect::<Result<Vec<_>, ServerError>>()?;
    let bound = BindingClaims {
        binding: BINDING_VERSION,
        iss: claims.iss.clone(),
        sub: claims.sub.clone(),
        aud: claims.aud.clone(),
        iat: claims.iat,
        exp: claims.exp,
        pass: pass_digest(token),
        log,
        revision: decided.revision,
        dependencies,
    };
    let value = serde_json::to_value(&bound)
        .map_err(|error| unavailable(format!("the grant binding could not be encoded: {error}")))?;
    provider.signed_as(&value, BINDING_TYPE)
}

/// `claims` signed as a grant binding with the provider's current key, for
/// a pass Lys did not issue itself (`grant_bindings_api`).
pub(crate) fn signed_binding(
    state: &crate::routes::AppState,
    claims: &BindingClaims,
) -> Result<String, ServerError> {
    let provider = super::endpoints::provider(state)?;
    let value = serde_json::to_value(claims)
        .map_err(|error| unavailable(format!("the grant binding could not be encoded: {error}")))?;
    provider.signed_as(&value, BINDING_TYPE)
}

#[cfg(test)]
mod tests {
    use super::asked;

    /// No ask is no binding; the produced version is one; any other is
    /// refused by name.
    #[test]
    fn only_the_produced_binding_version_is_asked_for() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!asked(None)?);
        assert!(asked(Some("1"))?);
        let refused = asked(Some("2"))
            .err()
            .ok_or("another version was accepted")?;
        assert_eq!(refused.name(), "grant_binding_unsupported");
        Ok(())
    }
}
