#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`FileSigner`].
//!
//! # Where the second party comes from
//!
//! - **A key whose public half Go and Rust already agreed on.** The seed below
//!   is `lys-core`'s Go-conformance fixture, and
//!   [`FIXTURE_VERIFIER_SPEC`] is the verifier-key string that
//!   `golang.org/x/mod/sumdb/note` and `lys-core` were cross-checked to
//!   produce for it. It is written here as a **literal**, not imported: a
//!   constant pulled in from the code under test moves with it, and then the
//!   check is the implementation agreeing with itself through an extra import.
//! - **`lys-core`'s strict Ed25519 verifier**, written long before this crate,
//!   as the judge of anything this type signs.
//! - **The trait contract, as prose.** `Signer` promises that a signature
//!   verifies under the key `public_key` reports, and `InProcessSigner`
//!   promises `identity` is *that* key's identity. Both are stated in
//!   `signer.rs` where the implementation cannot quietly agree with them, and
//!   are asserted here.
//!
//! # Every refusal opens with a positive control
//!
//! A test made only of refusals cannot tell a working check from one that
//! rejects everything. Each negative case below first asserts the unmodified
//! setup is accepted.

use std::path::{Path, PathBuf};

use lys_core::Ed25519Identity;
use lys_core::checkpoint::NoteVerifierKey;
use tempfile::TempDir;

use super::*;

/// The 32 ASCII bytes `lys-go-conformance-test-seed-01!`, `lys-core`'s
/// Go-conformance fixture seed.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// The signed-note key name the fixture verifier string was computed under.
const FIXTURE_NAME: &str = "example.com/lys/test";

/// The verifier-key text form for `(FIXTURE_NAME, FIXTURE_SEED)`, as Go
/// `sumdb/note` and `lys-core` were cross-checked to agree on. Hardcoded, so it
/// cannot move with the code it is checking.
const FIXTURE_VERIFIER_SPEC: &str =
    "example.com/lys/test+52580cd9+AQz9D9gbFqzLxSMM9Fy6nUuTfYJ8bI29RKFE5aulcbni";

/// Lowercase hex of the fixture's 32-byte public key — the trailing 32 bytes of
/// the base64 blob in [`FIXTURE_VERIFIER_SPEC`], after its `0x01` algorithm
/// byte. Used as the positive control for the redaction case.
const FIXTURE_PUBLIC_KEY_HEX: &str =
    "0cfd0fd81b16accbc5230cf45cba9d4b937d827c6c8dbd44a144e5aba571b9e2";

/// Lowercase hex of the fixture seed — the bytes that must never appear.
const FIXTURE_SEED_HEX: &str = "6c79732d676f2d636f6e666f726d616e63652d746573742d736565642d303121";

/// Writes the fixture seed to a key file and loads a signer from it.
fn fixture_signer(dir: &Path) -> (PathBuf, FileSigner) {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    let signer = FileSigner::load(&path).unwrap();
    (path, signer)
}

#[test]
fn the_public_key_is_the_one_the_seed_on_disk_determines() {
    let tmp = TempDir::new().unwrap();
    let (_path, signer) = fixture_signer(tmp.path());

    // Keyed on a string produced by Go and lys-core agreeing, not on anything
    // this crate computed.
    assert_eq!(
        NoteVerifierKey::new(FIXTURE_NAME, signer.public_key())
            .unwrap()
            .to_spec(),
        FIXTURE_VERIFIER_SPEC
    );
}

#[test]
fn the_identity_it_signs_with_is_the_key_it_advertises() {
    let tmp = TempDir::new().unwrap();
    let (_path, signer) = fixture_signer(tmp.path());

    // `InProcessSigner`'s written contract: `identity` is the identity whose
    // public key is `Signer::public_key`. An implementation that returned a
    // different one would publish under one key while advertising another.
    assert_eq!(
        signer.identity().public_key_bytes(),
        signer.public_key(),
        "InProcessSigner::identity must be the identity behind Signer::public_key"
    );
}

#[test]
fn a_signature_verifies_under_the_public_key_it_reports() {
    let tmp = TempDir::new().unwrap();
    let (_path, signer) = fixture_signer(tmp.path());
    let message = b"bytes the signer did not choose";

    let signature = signer.sign(message).unwrap();

    // Positive control: the judge accepts the untouched triple, so the two
    // refusals below are about the one thing each changes and not about a
    // verifier that rejects everything.
    Ed25519Identity::verify(&signer.public_key(), message, &signature)
        .expect("a signature must verify under the key the signer reports");

    // One difference: a different message.
    assert!(
        Ed25519Identity::verify(&signer.public_key(), b"different bytes", &signature).is_err(),
        "a signature must not verify over a message it does not cover"
    );

    // One difference: one flipped bit of the public key.
    let mut other_key = signer.public_key();
    other_key[0] ^= 0x01;
    assert!(
        Ed25519Identity::verify(&other_key, message, &signature).is_err(),
        "a signature must not verify under a key that did not produce it"
    );
}

#[test]
fn a_missing_key_file_is_refused_and_no_key_is_minted() {
    let tmp = TempDir::new().unwrap();

    // Positive control: this loader accepts a key file that exists, so the
    // refusal below is about absence and not about `load` refusing everything.
    let (present, _signer) = fixture_signer(tmp.path());
    assert!(present.exists());

    let absent = tmp.path().join("not-created-by-anyone.key");
    assert!(!absent.exists());
    match FileSigner::load(&absent) {
        Err(AnchorError::SignerKey { path, .. }) => {
            assert_eq!(path, absent.display().to_string());
        }
        other => panic!("expected SignerKey for a missing key file, got {other:?}"),
    }

    // The claim that matters is not that it errored but that it wrote nothing:
    // a loader that minted a key would leave one here and report success.
    assert!(
        !absent.exists(),
        "loading must never create a key file at the path it was asked to read"
    );
}

#[test]
fn the_debug_output_carries_the_public_key_and_not_the_seed() {
    let tmp = TempDir::new().unwrap();
    let (_path, signer) = fixture_signer(tmp.path());

    let rendered = format!("{signer:?}");

    // Positive control: the rendering is real — it contains key material
    // derived from this seed — so the absence asserted next is not the absence
    // of any output at all.
    assert!(
        rendered.contains(FIXTURE_PUBLIC_KEY_HEX),
        "Debug should identify the signer by its public key, got: {rendered}"
    );
    assert!(
        rendered.contains("REDACTED"),
        "Debug must mark the private half as redacted, got: {rendered}"
    );
    assert!(
        !rendered.contains(FIXTURE_SEED_HEX),
        "Debug must never render the private seed"
    );
}
