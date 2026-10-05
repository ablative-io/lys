//! The token exchange's two acts on the provider's own state, kept apart so a
//! test can drive the seam between them.
//!
//! Lock order: codes, then tokens, never the other way round. `take_grant`
//! holds codes alone, and takes tokens under it only to revoke a replayed
//! code's token. `issue` holds codes then tokens. The apps store, the
//! directory and the sessions are never taken under either; the endpoint
//! reads them between the two acts with no provider guard held.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_identity::PersonId;
use sha2::{Digest, Sha256};

use super::{Access, OpenIdProvider, held, same};
use crate::error::ServerError;
use crate::error_provider::ProviderError;

/// What an exchange takes from a code once it is verified and marked used.
pub(super) struct Taken {
    pub(super) subject: String,
    pub(super) nonce: Option<String>,
    pub(super) authenticated_at: u64,
    pub(super) expires_at: u64,
    pub(super) session_id: String,
    pub(super) person: PersonId,
    /// Whether the authorization asked for the name and was granted it then.
    pub(super) profile: bool,
}

impl OpenIdProvider {
    /// The first act: verify `code` for `client_id`, `redirect` and
    /// `verifier` at `at`, mark it used and take what the token needs. A code
    /// already used is a replay: the token it issued is revoked, and one not
    /// issued yet is marked so that `issue` keeps none.
    pub(super) fn take_grant(
        &self,
        code: &str,
        client_id: &str,
        redirect: &str,
        verifier: &str,
        at: u64,
    ) -> Result<Taken, ServerError> {
        let mut codes = held(&self.codes)?;
        let grant = codes
            .get_mut(code)
            .ok_or(ServerError::Provider(ProviderError::CodeUnknown))?;
        if grant.client_id != client_id {
            return Err(ServerError::Provider(ProviderError::CodeUnknown));
        }
        if grant.used {
            match &grant.issued_access {
                Some(key) => held(&self.tokens)?.revoke(key)?,
                None => grant.replayed = true,
            }
            return Err(ServerError::Provider(ProviderError::CodeUsed));
        }
        if at >= grant.expires_at || at >= grant.sign_in_ends_at {
            return Err(ServerError::Provider(ProviderError::CodeExpired));
        }
        if grant.redirect_uri != redirect {
            return Err(ServerError::Provider(ProviderError::RedirectUnregistered));
        }
        grant.used = true;
        let hashed = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        if !same(&hashed, &grant.challenge) {
            return Err(ServerError::Provider(ProviderError::VerifierWrong));
        }
        Ok(Taken {
            subject: grant.subject.clone(),
            nonce: grant.nonce.clone(),
            authenticated_at: grant.authenticated_at,
            expires_at: grant.sign_in_ends_at,
            session_id: grant.session_id.clone(),
            person: grant.person,
            profile: grant.profile,
        })
    }

    /// The second act: keep the token under `lookup` and let `code` remember
    /// it, so a later replay revokes it. A code replayed between the two acts
    /// gets no token and the exchange answers `CodeUsed`. A code retired
    /// meanwhile as expired is unknown to any replay; its token is kept and
    /// ends at its own instant.
    pub(super) fn issue(
        &self,
        code: &str,
        lookup: String,
        access: Access,
        at: u64,
    ) -> Result<(), ServerError> {
        let mut codes = held(&self.codes)?;
        let mut tokens = held(&self.tokens)?;
        match codes.get_mut(code) {
            Some(grant) if grant.replayed => Err(ServerError::Provider(ProviderError::CodeUsed)),
            Some(grant) => {
                tokens.insert(lookup.clone(), access, at)?;
                grant.issued_access = Some(lookup);
                Ok(())
            }
            None => tokens.insert(lookup, access, at),
        }
    }
}
