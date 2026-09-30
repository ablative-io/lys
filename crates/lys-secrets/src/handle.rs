//! Handles and presentations. A handle is 32 random bytes bound to one
//! identity; the broker keeps only its digest. Whoever presents it signs a
//! presentation naming the handle id, the call's operation id, the time and
//! the digest of the one request it is for.

use std::fmt;

use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, sign_attestation, verify_attestation_by_signer};

use crate::encoding::{Canonical, hex, random_bytes, sha256, unhex};
use crate::error::SecretsError;
use crate::secret::Secret;

/// The domain every presentation payload opens with.
pub const PRESENTATION_DOMAIN: &str = "lys-secrets/presentation/v2";
/// The shortest operation id the broker takes.
pub const MIN_OPERATION_ID: usize = 16;
/// The domain a request digest opens with.
pub const REQUEST_DOMAIN: &str = "lys-secrets/request/v1";

/// The digest of one request: its method, its path with query, and its body.
/// A presentation signs it, so a presentation is good for that request only.
///
/// # Errors
///
/// `Encoding` when a part is too long to encode.
pub fn request_digest(method: &str, path: &str, body: &[u8]) -> Result<[u8; 32], SecretsError> {
    let mut encoding = Canonical::new(REQUEST_DOMAIN)?;
    encoding
        .field(method.as_bytes())?
        .field(path.as_bytes())?
        .field(&sha256(body))?;
    Ok(sha256(&encoding.into_bytes()))
}

/// A handle's id: random, never derived from the handle, and what every
/// record and audit line names.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HandleId(pub(crate) String);

impl HandleId {
    pub(crate) fn generate() -> Result<Self, SecretsError> {
        Ok(Self(hex(&random_bytes::<16>()?)))
    }

    /// A handle id as a person or a file writes it.
    pub fn from_text(text: &str) -> Self {
        Self(text.to_owned())
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
    /// The digest of the request it is for. It never travels: the broker
    /// computes it from the request it is given.
    pub request: [u8; 32],
    /// The presenter's signature over the payload; none when the handle
    /// was presented unsigned, which the broker refuses by name.
    pub attestation: Option<Attestation>,
}

impl Presentation {
    /// The presentation as the four text values it travels in: handle id,
    /// operation id in hex, signing time, and the signature in hex.
    pub fn to_wire(&self) -> [String; 4] {
        [
            self.handle_id.as_str().to_owned(),
            hex(&self.operation_id),
            self.signed_at_ms.to_string(),
            self.attestation
                .as_ref()
                .map(|attestation| hex(&attestation.to_cose_bytes()))
                .unwrap_or_default(),
        ]
    }

    /// Reads a presentation from its four text values and the digest of
    /// the request that carried them. A value that does not read is the one
    /// `PresentationInvalid`. No signature, or an empty one, reads as a
    /// presentation sent unsigned, which the broker refuses as
    /// `PresentationUnsigned` once it has found the handle.
    ///
    /// # Errors
    ///
    /// `PresentationInvalid`.
    pub fn from_wire(
        handle_id: &str,
        operation: &str,
        signed_at: &str,
        signature: Option<&str>,
        request: [u8; 32],
    ) -> Result<Self, SecretsError> {
        let invalid = || SecretsError::PresentationInvalid {
            handle: handle_id.to_owned(),
        };
        let operation_id = unhex(operation).ok_or_else(invalid)?;
        let signed_at_ms = signed_at.parse::<i64>().map_err(|_number| invalid())?;
        let attestation = match signature.filter(|text| !text.is_empty()) {
            None => None,
            Some(text) => {
                let cose = unhex(text).ok_or_else(invalid)?;
                Some(Attestation::from_cose_bytes(&cose).map_err(|_cose| invalid())?)
            }
        };
        Ok(Self {
            handle_id: HandleId(handle_id.to_owned()),
            operation_id,
            signed_at_ms,
            request,
            attestation,
        })
    }

    /// Signs a presentation of `handle_id` for `operation_id` at
    /// `signed_at_ms`, for the request whose digest is `request`, with `key`.
    ///
    /// # Errors
    ///
    /// `OperationIdTooShort` and `Encoding`.
    pub fn sign(
        handle_id: &HandleId,
        operation_id: &[u8],
        signed_at_ms: i64,
        request: [u8; 32],
        key: &Ed25519Identity,
    ) -> Result<Self, SecretsError> {
        check_operation_id(operation_id)?;
        let payload = payload(handle_id, operation_id, signed_at_ms, &request)?;
        Ok(Self {
            handle_id: handle_id.clone(),
            operation_id: operation_id.to_vec(),
            signed_at_ms,
            request,
            attestation: Some(sign_attestation(&payload, key)),
        })
    }

    /// A presentation of `handle_id` for `operation_id` at `signed_at_ms`
    /// that carries no signature, as a presenter who skipped signing sends
    /// it. The broker refuses it as `PresentationUnsigned`.
    pub fn unsigned(
        handle_id: &HandleId,
        operation_id: &[u8],
        signed_at_ms: i64,
        request: [u8; 32],
    ) -> Self {
        Self {
            handle_id: handle_id.clone(),
            operation_id: operation_id.to_vec(),
            signed_at_ms,
            request,
            attestation: None,
        }
    }

    /// Verifies the signature against `registered`, the key of the identity
    /// the handle is bound to. No signature is `PresentationUnsigned`; every
    /// failure of a signature is the one `PresentationInvalid`.
    pub(crate) fn verify(&self, registered: &[u8; 32]) -> Result<(), SecretsError> {
        let Some(attestation) = &self.attestation else {
            return Err(SecretsError::PresentationUnsigned {
                handle: self.handle_id.to_string(),
            });
        };
        let payload = payload(
            &self.handle_id,
            &self.operation_id,
            self.signed_at_ms,
            &self.request,
        )?;
        verify_attestation_by_signer(attestation, &payload, registered).map_err(|_invalid| {
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
    request: &[u8; 32],
) -> Result<Vec<u8>, SecretsError> {
    let mut encoding = Canonical::new(PRESENTATION_DOMAIN)?;
    encoding
        .field(handle_id.as_str().as_bytes())?
        .field(operation_id)?
        .field(&signed_at_ms.to_be_bytes())?
        .field(request)?;
    Ok(encoding.into_bytes())
}
