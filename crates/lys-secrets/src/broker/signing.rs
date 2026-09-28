//! Signing keys: a secret the broker signs with and never gives out.
//!
//! A signing key is sealed once, as an Ed25519 seed of 32 bytes with the one
//! purpose it signs for, from a closed list ([`SigningPurpose`]). The sealed
//! value binds the purpose beside the seed, and the index records the
//! purpose and the public key, so the listing shows both and never the seed.
//!
//! **Invariants.**
//!
//! - The seed leaves the store only inside [`Broker::sign_for`], into an
//!   identity that signs and is dropped there. No answer, audit line, error
//!   or log carries a byte of it, and no value-bearing use serves it: the
//!   proxy, a login at spawn and a sealed read each refuse a signing key
//!   `not_a_value_secret`.
//! - The broker builds what it signs from typed members ([`Signable`]) under
//!   the purpose's domain label, with the one function the verifier calls
//!   ([`lys_core::agent_request::payload`]). It never signs a digest or bytes
//!   a caller composed.
//! - A signature is a use like any other: admitted through the one
//!   admission, with its use lines, counted against the lease's uses,
//!   refused after a drop, a revoke or the lease's window, and settled once
//!   it is made or refused, so no operation stays open.
//! - A signature is made only for a signing time within
//!   [`super::PRESENTATION_SKEW_MS`] of the broker's clock, so a holder
//!   cannot sign requests dated ahead that would outlive its lease's end.

use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::audit::AuditKind;
use crate::encoding::{Canonical, Reader, hex, unhex};
use crate::error::{SecretsError, SigningRefusal};
use crate::handle::{HandleToken, Presentation};
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::EntryClass;

use super::using::UseFor;
use super::{Admitted, Broker, PRESENTATION_SKEW_MS, Ticket};

/// The domain the sealed value of a signing key opens with.
const SEALED_DOMAIN: &str = "lys-secrets/signing-key/v1";
/// The length of an Ed25519 seed.
const SEED_LEN: usize = 32;
/// The fewest bytes an agent request's nonce carries.
const NONCE_MIN: usize = 16;
/// What a mismatch names as the purpose of a secret that is no signing key.
const NO_PURPOSE: &str = "none";

/// The one purpose a signing key signs for. Each has its own domain label,
/// and the list is closed: a purpose outside it is refused by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningPurpose {
    /// An agent's signed request to the identity service, under the domain
    /// label that service verifies.
    AgentRequest,
}

impl SigningPurpose {
    /// The purpose as it is named.
    pub fn label(self) -> &'static str {
        match self {
            Self::AgentRequest => "agent_request",
        }
    }

    /// The domain label every signature for this purpose is made under.
    pub fn domain(self) -> &'static str {
        match self {
            Self::AgentRequest => lys_core::agent_request::DOMAIN,
        }
    }

    /// The purpose named `text`.
    ///
    /// # Errors
    ///
    /// `signing_purpose_unknown`, naming `text`, for a purpose outside the
    /// list.
    pub fn parse(text: &str) -> Result<Self, SecretsError> {
        match text {
            "agent_request" => Ok(Self::AgentRequest),
            other => Err(SecretsError::from(SigningRefusal::PurposeUnknown {
                purpose: other.to_owned(),
            })),
        }
    }
}

/// The typed members of an agent's request, from which the broker builds
/// the bytes it signs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    method: String,
    path: String,
    body_digest: String,
    signed_at_ms: u64,
    nonce: String,
}

impl AgentRequest {
    /// The request `method` `path` whose body has the SHA-256 digest
    /// `body_digest`, signed at `signed_at_ms` under `nonce`.
    ///
    /// # Errors
    ///
    /// `Encoding` when the method is not a word of visible characters,
    /// the path does not start with `/` or holds a space or a control
    /// character, the digest is not 64 lower-case hex characters, or the
    /// nonce is not at least 16 bytes in hex. So no member can carry a line
    /// of the payload that is not its own.
    pub fn new(
        method: &str,
        path: &str,
        body_digest: &str,
        signed_at_ms: u64,
        nonce: &str,
    ) -> Result<Self, SecretsError> {
        let malformed = |reason: &str| SecretsError::Encoding {
            context: "agent request",
            reason: reason.to_owned(),
        };
        if method.is_empty() || !method.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(malformed("the method is not a word of visible characters"));
        }
        if !path.starts_with('/') || !path.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(malformed(
                "the path does not start with / or holds a space or a control character",
            ));
        }
        if body_digest.len() != 64 || !body_digest.bytes().all(lower_hex_digit) {
            return Err(malformed(
                "the body digest is not 64 lower-case hex characters",
            ));
        }
        if unhex(nonce).is_none_or(|bytes| bytes.len() < NONCE_MIN) {
            return Err(malformed("the nonce is not at least 16 bytes in hex"));
        }
        Ok(Self {
            method: method.to_owned(),
            path: path.to_owned(),
            body_digest: body_digest.to_owned(),
            signed_at_ms,
            nonce: nonce.to_owned(),
        })
    }
}

/// One thing to sign: a purpose and its typed members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signable {
    /// An agent's request to the identity service.
    AgentRequest(AgentRequest),
}

impl Signable {
    /// The purpose it is signed for.
    pub fn purpose(&self) -> SigningPurpose {
        match self {
            Self::AgentRequest(_) => SigningPurpose::AgentRequest,
        }
    }

    /// The signing time it names, in milliseconds since the epoch.
    pub fn signed_at_ms(&self) -> u64 {
        match self {
            Self::AgentRequest(request) => request.signed_at_ms,
        }
    }

    /// The bytes signed for it, built under its purpose's domain label by
    /// the function the verifier calls.
    pub fn payload(&self) -> Vec<u8> {
        match self {
            Self::AgentRequest(request) => lys_core::agent_request::payload(
                &request.method,
                &request.path,
                &request.body_digest,
                request.signed_at_ms,
                &request.nonce,
            ),
        }
    }
}

/// A signature the broker made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    /// The tagged `COSE_Sign1` signature.
    pub cose: Vec<u8>,
    /// The signing key's Ed25519 public key.
    pub public_key: [u8; 32],
    /// Uses left on the lease after this one.
    pub uses_left: u64,
}

/// What a signing use answers.
#[derive(Debug)]
pub enum Signing {
    /// The signature, made for a fresh operation.
    Made(Signed),
    /// The same operation was presented before; its recorded outcome is
    /// answered and nothing is signed again.
    Retried {
        /// The recorded outcome.
        outcome: String,
    },
}

impl<P: PermissionCheck> Broker<P> {
    /// Seals `seed`, an Ed25519 seed of 32 bytes, as the signing key `name`
    /// owned by `owner`, signing for `purpose` alone. Answers its public
    /// key; the seed is never answered.
    ///
    /// # Errors
    ///
    /// `signing_key_invalid`, naming the length, for a seed of any other
    /// length, every refusal of [`crate::SecretStore::add`], and the audit
    /// log's.
    pub fn seal_signing_key(
        &mut self,
        name: &str,
        owner: &str,
        purpose: SigningPurpose,
        seed: &Secret,
    ) -> Result<[u8; 32], SecretsError> {
        let seed = seed_of(name, seed.expose())?;
        let public_key = Ed25519Identity::from_seed(&seed).public_key_bytes();
        let mut sealed = Canonical::new(SEALED_DOMAIN)?;
        sealed.field(purpose.label().as_bytes())?.field(&seed[..])?;
        let sealed = Secret::new(sealed.into_bytes());
        let signs = (purpose, hex(&public_key));
        self.store
            .add_signing_key(&self.store_key, (name, owner), signs, &sealed)?;
        self.record(
            AuditKind::Seal,
            (None, Some(owner), Some(name)),
            None,
            None,
            "sealed",
        )?;
        Ok(public_key)
    }

    /// Signs `signable` with the signing key `key`, for the holder of
    /// `token` presenting `presentation`. The use is admitted as every use
    /// is: the presentation is signed, the handle is live, the lease covers
    /// `key` and the holder is permitted. Then the purpose asked must be the
    /// key's own. The broker builds the bytes it signs from `signable`'s
    /// typed members and answers the `COSE_Sign1` signature and the public
    /// key, never the seed. The use is settled once the signature is made,
    /// and as failed when signing is refused after admission. A signing time
    /// more than [`PRESENTATION_SKEW_MS`] from the broker's clock is refused
    /// before admission, as a malformed request is.
    ///
    /// # Errors
    ///
    /// `PresentationStale` for a signing time that far from the broker's
    /// clock; every refusal of admission, each on its use line;
    /// `signing_purpose_mismatch`, naming both purposes, when the key signs
    /// for another purpose or is no signing key; `EntryBindingMismatch`
    /// when the sealed key does not match what the index records; and the
    /// audit log's.
    pub fn sign_for(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        key: &str,
        signable: &Signable,
    ) -> Result<Signing, SecretsError> {
        let now = (self.clock)();
        let skew = i64::try_from(signable.signed_at_ms()).map_or(u64::MAX, |at| now.abs_diff(at));
        if skew > PRESENTATION_SKEW_MS.unsigned_abs() {
            return Err(SecretsError::PresentationStale {
                skew_ms: i64::try_from(skew).unwrap_or(i64::MAX),
                limit_ms: PRESENTATION_SKEW_MS,
            });
        }
        let asked = (Some(key), None);
        let ticket = match self.admit_use_as(token, presentation, 0, asked, UseFor::Signature)? {
            Admitted::Retried { outcome } => return Ok(Signing::Retried { outcome }),
            Admitted::Fresh(ticket) => ticket,
        };
        match self.signature(&ticket, signable) {
            Ok((cose, public_key)) => {
                let uses_left = ticket.uses_left;
                self.settle_unmetered(ticket)?;
                Ok(Signing::Made(Signed {
                    cose,
                    public_key,
                    uses_left,
                }))
            }
            Err(refusal) => {
                self.settle_failed(ticket, 0)?;
                Err(refusal)
            }
        }
    }

    /// The signature over `signable` with the key `ticket` opened, and the
    /// key's public key.
    fn signature(
        &self,
        ticket: &Ticket,
        signable: &Signable,
    ) -> Result<(Vec<u8>, [u8; 32]), SecretsError> {
        let asked = signable.purpose();
        let view = self
            .store
            .entry(&ticket.entry)
            .ok_or_else(|| SecretsError::SecretUnknown {
                name: ticket.entry.clone(),
            })?;
        let held = if view.class == EntryClass::SigningKey {
            view.purpose
        } else {
            None
        };
        if held != Some(asked) {
            return Err(SecretsError::from(SigningRefusal::PurposeMismatch {
                secret: ticket.secret.clone(),
                held: held.map_or(NO_PURPOSE, SigningPurpose::label),
                asked: asked.label(),
            }));
        }
        let unbound = || SecretsError::EntryBindingMismatch {
            entry: ticket.entry.clone(),
        };
        let mut reader = Reader::new(ticket.credential.expose(), "sealed signing key");
        if reader.field()? != SEALED_DOMAIN.as_bytes()
            || reader.field()? != asked.label().as_bytes()
        {
            return Err(unbound());
        }
        let seed = seed_of(&ticket.secret, reader.field()?)?;
        if !reader.is_done() {
            return Err(unbound());
        }
        let identity = Ed25519Identity::from_seed(&seed);
        let public_key = identity.public_key_bytes();
        if view.public_key.as_deref() != Some(hex(&public_key).as_str()) {
            return Err(unbound());
        }
        let attestation = sign_attestation(&signable.payload(), &identity);
        Ok((attestation.to_cose_bytes(), public_key))
    }

    /// Refuses `not_a_value_secret` when `secret`, or `entry`, the entry a
    /// value-bearing use of it would open, is a signing key: its seed is
    /// handed to no forward, login or read, whatever account is current.
    pub(super) fn gives_value(&self, entry: &str, secret: &str) -> Result<(), SecretsError> {
        let signing_key = |name: &str| {
            self.store
                .entry(name)
                .is_some_and(|view| view.class == EntryClass::SigningKey)
        };
        if signing_key(secret) || signing_key(entry) {
            return Err(SecretsError::from(SigningRefusal::NotAValueSecret {
                secret: secret.to_owned(),
            }));
        }
        Ok(())
    }
}

/// Whether `byte` is a lower-case hex digit.
fn lower_hex_digit(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'a'..=b'f')
}

/// `bytes` as the seed of the signing key `secret`, in a buffer overwritten
/// when it is dropped.
fn seed_of(secret: &str, bytes: &[u8]) -> Result<Zeroizing<[u8; SEED_LEN]>, SecretsError> {
    if bytes.len() != SEED_LEN {
        return Err(SecretsError::from(SigningRefusal::KeyInvalid {
            secret: secret.to_owned(),
            reason: format!("holds a seed of {} bytes, not {SEED_LEN}", bytes.len()),
        }));
    }
    let mut seed = Zeroizing::new([0u8; SEED_LEN]);
    seed.copy_from_slice(bytes);
    Ok(seed)
}

#[cfg(test)]
#[path = "signing_tests.rs"]
mod tests;
