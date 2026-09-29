//! The directory service's attestation over each event: `lys/identity-event/v1`.
//!
//! An event is a `COSE_Sign1` message (RFC 9052, tag 18) in the shape lys-core's
//! attestation v2 uses. The protected header names `EdDSA` (`1: -8`), the
//! content type `application/vnd.lys.identity-event.v1+cbor` (`3`) and the
//! directory service's Ed25519 public key as the key id (`4`). The unprotected
//! header is empty, the payload is the canonical event body from
//! [`crate::encoding`], and the signature covers the `Signature1` structure
//! with empty external data. The whole message is the log leaf.
//!
//! The signature is the service's, and it attests that the service
//! authenticated the actor the event names. It is never a person's signature
//! (P8). Verification is strict: a message is refused unless it is the exact
//! canonical message over its three parts, so each event has one byte string.

use ciborium::Value;
use lys_core::Ed25519Identity;

use crate::encoding::{
    MAJOR_ARRAY, MAJOR_NEGATIVE, MAJOR_TAG, as_bytes, as_uint, bytes, cbor, decode_body,
    encode_body, head, malformed, map, payload_commitment, text, uint,
};
use crate::error::IdentityError;
use crate::event::IdentityEvent;

/// The content type the protected header names.
pub const CONTENT_TYPE: &str = "application/vnd.lys.identity-event.v1+cbor";
/// New envelope for events authenticated by a service account's bearer.
pub const SERVICE_ACCOUNT_CONTENT_TYPE: &str = "application/vnd.lys.identity-event.v2+cbor";

fn content_type(event: &IdentityEvent) -> &'static str {
    if event.version() == 2 {
        SERVICE_ACCOUNT_CONTENT_TYPE
    } else {
        CONTENT_TYPE
    }
}

/// The largest event this directory reads or writes, in bytes.
pub const MAX_EVENT_BYTES: usize = 64 * 1024;

const COSE_SIGN1_TAG: u64 = 18;
const SIGNATURE_LEN: usize = 64;
const KEY_LEN: usize = 32;

/// The protected header naming `EdDSA`, this envelope's content type and `kid`.
fn protected_header(kid: &[u8; KEY_LEN], media_type: &str) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 3);
    uint(&mut out, 1);
    head(&mut out, MAJOR_NEGATIVE, 7);
    uint(&mut out, 3);
    text(&mut out, media_type);
    uint(&mut out, 4);
    bytes(&mut out, kid);
    out
}

/// The `Signature1` structure the signature covers, with empty external data.
fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_ARRAY, 4);
    text(&mut out, "Signature1");
    bytes(&mut out, protected);
    bytes(&mut out, &[]);
    bytes(&mut out, payload);
    out
}

/// The tagged `COSE_Sign1` message over its three parts, with an empty unprotected header.
fn cose_sign1(protected: &[u8], payload: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_TAG, COSE_SIGN1_TAG);
    head(&mut out, MAJOR_ARRAY, 4);
    bytes(&mut out, protected);
    map(&mut out, 0);
    bytes(&mut out, payload);
    bytes(&mut out, signature);
    out
}

/// An event with the exact bytes that were signed, ready to append or just verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedEvent {
    bytes: Vec<u8>,
    event: IdentityEvent,
    commitment: [u8; 32],
}

impl SignedEvent {
    /// The whole `COSE_Sign1` message: the log leaf.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The event the message carries.
    pub fn event(&self) -> &IdentityEvent {
        &self.event
    }

    /// SHA-256 over the body bytes, named by [`crate::encoding::PAYLOAD_COMMITMENT_HASH`].
    pub fn payload_commitment(&self) -> [u8; 32] {
        self.commitment
    }
}

/// Sign `event` with the directory service's key, refusing an event larger
/// than any event this directory reads back, so nothing is logged that
/// [`verify_event`] would refuse for ever.
pub fn sign_event(
    event: IdentityEvent,
    service_key: &Ed25519Identity,
) -> Result<SignedEvent, IdentityError> {
    let body = encode_body(&event);
    let protected = protected_header(&service_key.public_key_bytes(), content_type(&event));
    let signature = service_key.sign(&sig_structure(&protected, &body));
    let bytes = cose_sign1(&protected, &body, &signature);
    if bytes.len() > MAX_EVENT_BYTES {
        return Err(IdentityError::EventTooLarge {
            len: bytes.len(),
            limit: MAX_EVENT_BYTES,
        });
    }
    Ok(SignedEvent {
        bytes,
        commitment: payload_commitment(&body),
        event,
    })
}

/// Verify `message` against the directory service's public key and return the event it carries.
pub fn verify_event(
    message: &[u8],
    service_key: &[u8; KEY_LEN],
) -> Result<SignedEvent, IdentityError> {
    if message.len() > MAX_EVENT_BYTES {
        return Err(IdentityError::EventTooLarge {
            len: message.len(),
            limit: MAX_EVENT_BYTES,
        });
    }
    let parts = decode_message(message)?;
    let Ok(kid) = <[u8; KEY_LEN]>::try_from(decode_kid(&parts.protected)?.as_slice()) else {
        return Err(IdentityError::EventMalformed {
            reason: "the key id is not a 32-byte Ed25519 public key",
        });
    };
    if parts.protected != protected_header(&kid, CONTENT_TYPE)
        && parts.protected != protected_header(&kid, SERVICE_ACCOUNT_CONTENT_TYPE)
    {
        return Err(IdentityError::EventMalformed {
            reason: "the protected header is not the identity-event header",
        });
    }
    if &kid != service_key {
        return Err(IdentityError::SignerMismatch);
    }
    if parts.signature.len() != SIGNATURE_LEN
        || Ed25519Identity::verify(
            service_key,
            &sig_structure(&parts.protected, &parts.payload),
            &parts.signature,
        )
        .is_err()
    {
        return Err(IdentityError::SignatureInvalid);
    }
    let event = decode_body(&parts.payload)?;
    if parts.protected != protected_header(&kid, content_type(&event)) {
        return Err(malformed(
            "the identity event body version differs from its envelope",
        ));
    }
    if cose_sign1(&parts.protected, &parts.payload, &parts.signature) != message {
        return Err(IdentityError::EventNotCanonical);
    }
    Ok(SignedEvent {
        bytes: message.to_vec(),
        commitment: payload_commitment(&parts.payload),
        event,
    })
}

/// The three signed parts of a `COSE_Sign1` message.
struct Parts {
    protected: Vec<u8>,
    payload: Vec<u8>,
    signature: Vec<u8>,
}

fn decode_message(message: &[u8]) -> Result<Parts, IdentityError> {
    const SHAPE: &str = "the message is not a tagged COSE_Sign1 of four parts";
    let Value::Tag(COSE_SIGN1_TAG, inner) = cbor(message, SHAPE)? else {
        return Err(malformed(SHAPE));
    };
    let Value::Array(items) = *inner else {
        return Err(malformed(SHAPE));
    };
    let Ok([protected, unprotected, payload, signature]) = <[Value; 4]>::try_from(items) else {
        return Err(malformed(SHAPE));
    };
    if unprotected != Value::Map(Vec::new()) {
        return Err(malformed("the unprotected header is not empty"));
    }
    Ok(Parts {
        protected: as_bytes(protected, SHAPE)?,
        payload: as_bytes(payload, SHAPE)?,
        signature: as_bytes(signature, SHAPE)?,
    })
}

fn decode_kid(protected: &[u8]) -> Result<Vec<u8>, IdentityError> {
    const SHAPE: &str = "the protected header is not a map of keys 1, 3 and 4";
    let Value::Map(pairs) = cbor(protected, SHAPE)? else {
        return Err(malformed(SHAPE));
    };
    pairs
        .into_iter()
        .find(|(key, _)| matches!(as_uint(key, SHAPE), Ok(4)))
        .map_or_else(|| Err(malformed(SHAPE)), |(_, kid)| as_bytes(kid, SHAPE))
}

/// Load the service's event signing key from the key file the operator supplies.
///
/// The key is held by lys-core's `Ed25519Identity`, in zeroizing memory, and
/// never printed: its debug form is redacted.
pub fn load_service_key(path: &std::path::Path) -> Result<Ed25519Identity, IdentityError> {
    Ed25519Identity::load(path).map_err(|error| IdentityError::KeyUnavailable {
        reason: error.to_string(),
    })
}
