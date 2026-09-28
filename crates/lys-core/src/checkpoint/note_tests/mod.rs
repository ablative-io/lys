#![cfg(test)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

use super::*;
use crate::error::TrustError;
use crate::keys::Ed25519Identity;

mod golden;
mod malformed;
mod signature_lines;

/// Fixed test seed: the 32 ASCII bytes `"lys-go-conformance-test-seed-01!"`.
const GOLDEN_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

const GOLDEN_NAME: &str = "example.com/lys/test";

/// Golden checkpoint body (size-3 raw-leaf tree), byte-exact.
const GOLDEN_BODY: &str = "example.com/lys/test\n3\nz3Y6BByBzu8VeKYIP3XGG+8uABTyo+aDqX/Pylvn8Zo=\n";

/// Golden signature blob: base64(key ID ‖ Ed25519 signature over the body
/// including its trailing newline). Byte-identical to Go `note.Sign`
/// output for the same inputs (verified during design).
const GOLDEN_SIG_BLOB_B64: &str =
    "UlgM2S4MVZwL9PUGADbPhidG6yKCC0hCE+sx7iXFboC6/rex00vtEy4d33ODa1g0afYmx36opQUAXnwdUl9E7eE28QU=";

/// Golden Ed25519 signature (hex) over the golden body.
const GOLDEN_SIG_HEX: &str = "2e0c559c0bf4f5060036cf862746eb22820b484213eb31ee25c56e80bafeb7b1d34bed132e1ddf73836b583469f626c77ea8a505005e7c1d525f44ede136f105";

const GOLDEN_KEY_ID: [u8; 4] = [0x52, 0x58, 0x0c, 0xd9];

fn golden_note() -> String {
    format!("{GOLDEN_BODY}\n\u{2014} {GOLDEN_NAME} {GOLDEN_SIG_BLOB_B64}\n")
}

fn golden_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden.key");
    std::fs::write(&path, GOLDEN_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

fn golden_verifier() -> NoteVerifierKey {
    let (_dir, identity) = golden_identity();
    NoteVerifierKey::new(GOLDEN_NAME, identity.public_key_bytes()).unwrap()
}

// --- Envelope tampers: every failure collapses to NoteVerification ---

fn assert_rejected(note_bytes: &[u8], label: &str) {
    let err = verify_note(note_bytes, &golden_verifier()).unwrap_err();
    assert!(
        matches!(err, TrustError::NoteVerification),
        "tamper {label} must collapse to NoteVerification"
    );
}
