//! Handles and presentations. A handle is 32 random bytes bound to one
//! identity; the broker keeps only its digest. Whoever presents it signs a
//! presentation naming the handle id, the call's operation id and the time.

use std::fmt;

use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, sign_attestation, verify_attestation_by_signer};

use crate::encoding::{Canonical, hex, random_bytes, sha256};
use crate::error::SecretsError;
use crate::secret::Secret;

/// The domain every presentation payload opens with.
pub const PRESENTATION_DOMAIN: &str = "lys-secrets/presentation/v1";
/// The shortest operation id the broker takes.
pub const MIN_OPERATION_ID: usize = 16;

/// A handle's id: random, never derived from the handle, and what every
/// record and audit line names.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HandleId(pub(crate) String);

impl HandleId {
    pub(crate) fn generate() -> Result<Self, SecretsError> {
        Ok(Self(hex(&random_bytes::<16>()?)))
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HandleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The raw handle a holder carries. It prints nothing.
#[derive(Debug)]
pub struct HandleToken(Secret);

impl HandleToken {
    pub(crate) fn generate() -> Result<Self, SecretsError> {
        Ok(Self(Secret::new(random_bytes::<32>()?.to_vec())))
    }

    /// Wraps handle bytes a holder presents.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(Secret::from_slice(bytes))
    }

    /// The handle's digest, which is all the broker keeps.
    pub(crate) fn digest(&self) -> [u8; 32] {
        sha256(self.0.expose())
    }

    /// The handle bytes, for the holder to carry.
    pub fn expose(&self) -> &[u8] {
        self.0.expose()
    }
}

/// An identity a handle can be bound to: its id and the Ed25519 key
/// registered for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The identity's id.
    pub identity: String,
    /// The public key its presentations are verified against.
    pub key: [u8; 32],
}

/// What issuing gives back: the id every record names, and the token that is
/// shown to the holder once and never kept.
#[derive(Debug)]
pub struct IssuedHandle {
    /// The handle's id.
    pub id: HandleId,
    /// The raw handle.
    pub token: HandleToken,
}

/// A signed presentation of a handle for one operation.
#[derive(Debug, Clone)]
pub struct Presentation {
    /// The handle id the presenter claims.
    pub handle_id: HandleId,
    /// The call's operation id.
    pub operation_id: Vec<u8>,
    /// When it was signed, in milliseconds since the epoch.
    pub signed_at_ms: i64,
    /// The presenter's signature over the payload.
    pub attestation: Attestation,
}

impl Presentation {
    /// Signs a presentation of `handle_id` for `operation_id` at
    /// `signed_at_ms` with `key`.
    ///
    /// # Errors
    ///
    /// `OperationIdTooShort` and `Encoding`.
    pub fn sign(
        handle_id: &HandleId,
        operation_id: &[u8],
        signed_at_ms: i64,
        key: &Ed25519Identity,
    ) -> Result<Self, SecretsError> {
        check_operation_id(operation_id)?;
        let payload = payload(handle_id, operation_id, signed_at_ms)?;
        Ok(Self {
            handle_id: handle_id.clone(),
            operation_id: operation_id.to_vec(),
            signed_at_ms,
            attestation: sign_attestation(&payload, key),
        })
    }

    /// Verifies the signature against `registered`, the key of the identity
    /// the handle is bound to. Every failure is the one `PresentationInvalid`.
    pub(crate) fn verify(&self, registered: &[u8; 32]) -> Result<(), SecretsError> {
        let payload = payload(&self.handle_id, &self.operation_id, self.signed_at_ms)?;
        verify_attestation_by_signer(&self.attestation, &payload, registered).map_err(|_invalid| {
            SecretsError::PresentationInvalid {
                handle: self.handle_id.to_string(),
            }
        })
    }
}

/// A random operation id of the shortest length the broker takes.
///
/// # Errors
///
/// `Random` when the operating system's source fails.
pub fn new_operation_id() -> Result<Vec<u8>, SecretsError> {
    Ok(random_bytes::<MIN_OPERATION_ID>()?.to_vec())
}

pub(crate) fn check_operation_id(operation_id: &[u8]) -> Result<(), SecretsError> {
    if operation_id.len() < MIN_OPERATION_ID {
        return Err(SecretsError::OperationIdTooShort {
            len: operation_id.len(),
        });
    }
    Ok(())
}

fn payload(
    handle_id: &HandleId,
    operation_id: &[u8],
    signed_at_ms: i64,
) -> Result<Vec<u8>, SecretsError> {
    let mut encoding = Canonical::new(PRESENTATION_DOMAIN)?;
    encoding
        .field(handle_id.as_str().as_bytes())?
        .field(operation_id)?
        .field(&signed_at_ms.to_be_bytes())?;
    Ok(encoding.into_bytes())
}
