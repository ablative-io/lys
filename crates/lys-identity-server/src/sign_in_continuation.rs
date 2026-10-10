//! What a provider sign-in carries through its flight: the continuation a
//! person returns to once signed in, the refusal that keeps it for the
//! browser that answered, and the PKCE verifier the flight is started with.
//! A child of `sign_in_upstream`, kept beside it so that file stays under the
//! length gate (ADR-111).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::TryRngCore;
use rand::rngs::OsRng;

use super::failed;
use crate::error::ServerError;

/// Where a sign-in continues once the person is signed in: a bounded
/// authorize request on Lys's own origin, the only target Lys accepts. It is
/// made only by [`Continuation::accepted`], so nothing else can be carried.
#[derive(Clone)]
pub(super) struct Continuation(pub(super) String);

impl Continuation {
    /// `target` as a continuation, refused by name unless it is a bounded
    /// authorize request on this origin.
    pub(super) fn accepted(target: String) -> Result<Self, ServerError> {
        if target.len() > 8192
            || !["/oauth/authorize?", "/oauth/mcp/authorize?"]
                .iter()
                .any(|path| target.starts_with(path))
            || !target.is_ascii()
            || target
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
            || target.contains(['#', '\\'])
        {
            return Err(failed(
                "the provider continuation is not a bounded authorize request on this origin",
            ));
        }
        Ok(Self(target))
    }
}

/// A provider sign-in refused once its flight was found: the refusal, and
/// the continuation the flight carried when it may be kept for the browser
/// that answered.
pub(super) struct Refused {
    pub(super) error: ServerError,
    pub(super) continuation: Option<Continuation>,
}

impl Refused {
    pub(super) fn dropping(error: ServerError) -> Self {
        Self {
            error,
            continuation: None,
        }
    }
}

/// A PKCE verifier: 32 bytes from the secure random source, base64url.
pub(super) fn verifier() -> Result<String, ServerError> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| failed(format!("the secure random source failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
