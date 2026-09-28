//! The signature this service puts on a request to the secrets broker on a
//! signed-in person's behalf, as the secrets contract's amendment A7 writes
//! it: over `lys-secrets/on-behalf/v1`, then the service, the person, the
//! operation id, the signing time and the request's digest, each field
//! length-prefixed. The digest is over `lys-secrets/request/v1`, then the
//! method, the path with its query and the SHA-256 of the body. The broker's
//! own tests check that it admits what this signs.

use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use rand::TryRngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::hex;

const SERVICE_DOMAIN: &str = "lys-secrets/on-behalf/v1";
const REQUEST_DOMAIN: &str = "lys-secrets/request/v1";

/// One request the service asks for a person.
pub struct Asked<'a> {
    /// The name the broker trusts the service by.
    pub service: &'a str,
    /// The person, as the directory names them.
    pub person: &'a str,
    /// The request's method.
    pub method: &'a str,
    /// The request's path with its query.
    pub path: &'a str,
    /// The request's body.
    pub body: &'a [u8],
    /// When it is signed, in milliseconds since the epoch.
    pub signed_at_ms: i64,
}

/// The five header values that carry `asked`, signed with `key` under a
/// fresh operation id: `lys-service`, `lys-on-behalf-of`, `lys-operation`,
/// `lys-signed-at` and `lys-service-signature`, in that order.
///
/// # Errors
///
/// `SecretsUnavailable` when the random source fails or a field is too long
/// to encode.
pub fn headers(asked: &Asked<'_>, key: &Ed25519Identity) -> Result<[String; 5], ServerError> {
    let mut operation = [0_u8; 16];
    OsRng
        .try_fill_bytes(&mut operation)
        .map_err(|error| ServerError::SecretsUnavailable {
            reason: format!("the secure random source failed: {error}"),
        })?;
    let mut request = Fields::new(REQUEST_DOMAIN)?;
    request
        .field(asked.method.as_bytes())?
        .field(asked.path.as_bytes())?
        .field(&Sha256::digest(asked.body))?;
    let digest = Sha256::digest(request.0);
    let mut payload = Fields::new(SERVICE_DOMAIN)?;
    payload
        .field(asked.service.as_bytes())?
        .field(asked.person.as_bytes())?
        .field(&operation)?
        .field(&asked.signed_at_ms.to_be_bytes())?
        .field(&digest)?;
    let signature = sign_attestation(&payload.0, key);
    Ok([
        asked.service.to_owned(),
        asked.person.to_owned(),
        hex(&operation),
        asked.signed_at_ms.to_string(),
        hex(&signature.to_cose_bytes()),
    ])
}

struct Fields(Vec<u8>);

impl Fields {
    fn new(domain: &str) -> Result<Self, ServerError> {
        let mut fields = Self(Vec::new());
        fields.field(domain.as_bytes())?;
        Ok(fields)
    }

    fn field(&mut self, bytes: &[u8]) -> Result<&mut Self, ServerError> {
        let len = u32::try_from(bytes.len())
            .ok()
            .ok_or_else(|| ServerError::SecretsUnavailable {
                reason: format!("a field of {} bytes is too long to sign", bytes.len()),
            })?;
        self.0.extend_from_slice(&len.to_be_bytes());
        self.0.extend_from_slice(bytes);
        Ok(self)
    }
}
