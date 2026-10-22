//! Sealing grant events and verifying their signed envelopes.

use super::{
    CONNECTOR_ENVELOPE, COSE_SIGN1_TAG, EXERCISED_ENVELOPE, GRANT_ENVELOPE, GrantError, GrantEvent,
    HELD_MODE_ENVELOPE, KEY_LEN, MACHINE_ENVELOPE, ORGANISATION_ENVELOPE, ROLE_ENVELOPE,
    SERVICE_ACCOUNT_ENVELOPE, SIGNATURE_LEN, Value, as_uint, cbor, decode_event_body,
    encode_event_body, malformed,
};
use crate::encoding::{
    MAJOR_ARRAY, MAJOR_NEGATIVE, MAJOR_TAG, bytes, head, map, payload_commitment, text, uint,
};
use lys_core::Ed25519Identity;

fn protected_header(kid: &[u8; KEY_LEN], content_type: &str) -> Vec<u8> {
    let mut out = Vec::new();
    map(&mut out, 3);
    uint(&mut out, 1);
    head(&mut out, MAJOR_NEGATIVE, 7);
    uint(&mut out, 3);
    text(&mut out, content_type);
    uint(&mut out, 4);
    bytes(&mut out, kid);
    out
}

fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    head(&mut out, MAJOR_ARRAY, 4);
    text(&mut out, "Signature1");
    bytes(&mut out, protected);
    bytes(&mut out, &[]);
    bytes(&mut out, payload);
    out
}

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

/// A grant event with the exact bytes that were signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedGrantEvent {
    bytes: Vec<u8>,
    event: GrantEvent,
    commitment: [u8; 32],
}

impl SignedGrantEvent {
    /// The whole `COSE_Sign1` message: the log leaf.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The event the message carries.
    #[must_use]
    pub fn event(&self) -> &GrantEvent {
        &self.event
    }

    /// SHA-256 over the body bytes.
    #[must_use]
    pub fn payload_commitment(&self) -> [u8; 32] {
        self.commitment
    }
}

/// Sign `event` with the service key. Every event is signed whole, however
/// large: each CBOR part carries its own length, so no size is refused.
///
/// # Errors
/// This implementation is infallible; the result type is retained for callers of the signing API.
pub fn sign_grant_event(
    event: GrantEvent,
    service_key: &Ed25519Identity,
) -> Result<SignedGrantEvent, GrantError> {
    let body = encode_event_body(&event);
    let protected = protected_header(&service_key.public_key_bytes(), event.content_type());
    let signature = service_key.sign(&sig_structure(&protected, &body));
    let bytes = cose_sign1(&protected, &body, &signature);
    Ok(SignedGrantEvent {
        bytes,
        commitment: payload_commitment(&body),
        event,
    })
}

fn as_bytes(value: Value, reason: &'static str) -> Result<Vec<u8>, GrantError> {
    match value {
        Value::Bytes(raw) => Ok(raw),
        _ => Err(malformed(reason)),
    }
}

/// The protected header's content type and key id.
fn header(protected: &[u8]) -> Result<(String, Vec<u8>), GrantError> {
    const SHAPE: &str = "the protected header is not a map naming a content type and a key id";
    let Value::Map(pairs) = cbor(protected, SHAPE)? else {
        return Err(malformed(SHAPE));
    };
    let mut content_type = None;
    let mut kid = None;
    for (key, value) in pairs {
        match (as_uint(&key, SHAPE), value) {
            (Ok(3), Value::Text(found)) => content_type = Some(found),
            (Ok(4), Value::Bytes(found)) => kid = Some(found),
            _ => {}
        }
    }
    Ok((
        content_type.ok_or_else(|| malformed(SHAPE))?,
        kid.ok_or_else(|| malformed(SHAPE))?,
    ))
}

/// Verify `message` against the service key and return the grant event it carries.
///
/// # Errors
/// - `GrantError::EventMalformed` if the event CBOR or wire codes have the wrong shape.
/// - `GrantError::VersionUnsupported` if the event version is unsupported.
/// - `GrantError::GrantMalformed` if a grant member, identity or optional mark has the wrong shape.
/// - `GrantError::MemberUnknown` if an encoded grant names an unsupported member.
/// - `GrantError::MemberMissing` if an encoded grant omits a required member.
/// - `GrantError::RecipientKindUnknown` if a pass-on recipient code is unsupported.
/// - `GrantError::TokenInvalid` if an action, relation or resource token is invalid.
/// - `GrantError::AuthorityAbsent` if exercise or pass-on actions or recipients are empty.
/// - `GrantError::PassOnOutside` if pass-on authority exceeds exercise authority.
/// - `GrantError::WindowInvalid` if the grant ends at or before its start.
/// - `GrantError::ModelInvalid` if the model version is zero.
/// - `GrantError::LineageMalformed` if the source or root holder is invalid.
/// - `GrantError::UseOnly` if a one-time grant allows pass-on.
/// - `GrantError::EventMismatch` if the issued grant disagrees with its event or a revocation reason is blank.
/// - `GrantError::EventNotCanonical` if re-encoding does not reproduce the input bytes.
/// - `GrantError::EnvelopeMismatch` if the message names another envelope.
/// - `GrantError::SignerMismatch` if the envelope names another service key.
/// - `GrantError::SignatureInvalid` if the signature has the wrong length or does not verify.
pub fn verify_grant_event(
    message: &[u8],
    service_key: &[u8; KEY_LEN],
) -> Result<SignedGrantEvent, GrantError> {
    const SHAPE: &str = "the message is not a tagged COSE_Sign1 of four parts";
    let Value::Tag(COSE_SIGN1_TAG, inner) = cbor(message, SHAPE)? else {
        return Err(malformed(SHAPE));
    };
    let Value::Array(items) = *inner else {
        return Err(malformed(SHAPE));
    };
    let [protected, _, payload, signature] = <[Value; 4]>::try_from(items)
        .ok()
        .ok_or_else(|| malformed(SHAPE))?;
    let (protected, payload, signature) = (
        as_bytes(protected, SHAPE)?,
        as_bytes(payload, SHAPE)?,
        as_bytes(signature, SHAPE)?,
    );
    let (content_type, kid) = header(&protected)?;
    if ![
        GRANT_ENVELOPE,
        SERVICE_ACCOUNT_ENVELOPE,
        CONNECTOR_ENVELOPE,
        HELD_MODE_ENVELOPE,
        ROLE_ENVELOPE,
        MACHINE_ENVELOPE,
        ORGANISATION_ENVELOPE,
        EXERCISED_ENVELOPE,
    ]
    .contains(&content_type.as_str())
    {
        return Err(GrantError::EnvelopeMismatch {
            reason: format!("it names {content_type}"),
        });
    }
    let kid = <[u8; KEY_LEN]>::try_from(kid.as_slice())
        .ok()
        .ok_or_else(|| malformed("the key id is not a 32-byte Ed25519 public key"))?;
    if protected != protected_header(&kid, &content_type) {
        return Err(malformed(
            "the protected header is not the grant-event header",
        ));
    }
    if &kid != service_key {
        return Err(GrantError::SignerMismatch);
    }
    if signature.len() != SIGNATURE_LEN
        || Ed25519Identity::verify(
            service_key,
            &sig_structure(&protected, &payload),
            &signature,
        )
        .is_err()
    {
        return Err(GrantError::SignatureInvalid);
    }
    let event = decode_event_body(&payload)?;
    if content_type != event.content_type() {
        return Err(malformed(
            "the grant body version differs from its envelope",
        ));
    }
    if cose_sign1(&protected, &payload, &signature) != message {
        return Err(GrantError::EventNotCanonical);
    }
    Ok(SignedGrantEvent {
        bytes: message.to_vec(),
        commitment: payload_commitment(&payload),
        event,
    })
}
