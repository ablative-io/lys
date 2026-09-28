#![cfg(test)]
//! Loading the identity from the environment variable.

use super::*;

// ─── from_env ─────────────────────────────────────────────────────

#[test]
fn from_env_loads_valid_base64_standard() {
    let seed = [9u8; 32];
    let encoded = STANDARD.encode(seed);
    let id = Ed25519Identity::from_env_value(Ok(encoded)).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);

    let sig = id.sign(b"test");
    Ed25519Identity::verify(&id.public_key_bytes(), b"test", &sig).unwrap();
}

#[test]
fn from_env_loads_valid_base64_urlsafe() {
    let seed = [3u8; 32];
    let encoded = URL_SAFE_NO_PAD.encode(seed);
    let id = Ed25519Identity::from_env_value(Ok(encoded)).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

#[test]
fn from_env_missing_var_returns_key_management_error() {
    let err = Ed25519Identity::from_env_value(Err(std::env::VarError::NotPresent)).unwrap_err();
    assert!(matches!(err, TrustError::KeyManagement { .. }));
    let msg = err.to_string();
    assert!(msg.contains(TEST_ENV_VAR), "got: {msg}");
    assert!(msg.contains("not set"), "got: {msg}");
}

#[test]
fn from_env_invalid_base64_returns_key_management_error() {
    let err = Ed25519Identity::from_env_value(Ok("not-base64!!!@@".to_owned())).unwrap_err();
    assert!(matches!(err, TrustError::KeyManagement { .. }));
    let msg = err.to_string();
    assert!(msg.contains("invalid base64"), "got: {msg}");
}

#[test]
fn from_env_wrong_length_decoded_returns_key_management_error() {
    let encoded = STANDARD.encode([1u8; 16]);
    let err = Ed25519Identity::from_env_value(Ok(encoded)).unwrap_err();
    assert!(matches!(err, TrustError::KeyManagement { .. }));
    let msg = err.to_string();
    assert!(
        msg.contains("decoded to 16 bytes, expected 32"),
        "got: {msg}"
    );
}
