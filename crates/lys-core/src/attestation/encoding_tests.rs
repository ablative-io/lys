#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::keys::identity::Ed25519Identity;

/// Fixed golden seed: the 32 ASCII bytes `"lys-cose-conformance-test-seed01"`.
const GOLDEN_SEED: &[u8; 32] = b"lys-cose-conformance-test-seed01";

/// Public key for the golden seed.
const GOLDEN_PUBKEY_HEX: &str = "214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfd";

/// SHA-256 of the golden payload `"lys attestation conformance payload\n"`.
const GOLDEN_HASH_HEX: &str = "9404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e";

const GOLDEN_TIMESTAMP: i64 = 1_700_000_000_000;

/// The 46-byte golden claims payload.
const GOLDEN_CLAIMS_HEX: &str =
    "a20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe56800";

/// The 80-byte golden protected header bucket.
const GOLDEN_PROTECTED_HEX: &str = "a301270378276170706c69636174696f6e2f766e642e6c79732e6174746573746174696f6e2e76322b63626f72045820214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfd";

/// The 143-byte golden `Sig_structure` (the exact Ed25519 input).
const GOLDEN_SIG_STRUCTURE_HEX: &str = "846a5369676e6174757265315850a301270378276170706c69636174696f6e2f766e642e6c79732e6174746573746174696f6e2e76322b63626f72045820214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfd40582ea20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe56800";

/// The golden 64-byte Ed25519 signature.
const GOLDEN_SIGNATURE_HEX: &str = "ba6eae66b3a04f25e6116e7278ae8b0e2174a2964bb0917ae53ab2ef326c22a706e90cae6886b20ecd086fee4b515de046d450f90de81f9aa46a9b34bd7f1a09";

/// The complete 199-byte golden artifact — 4-way cross-verified (coset,
/// hand assembly, pycose, go-cose) during the D4 design.
const GOLDEN_ARTIFACT_HEX: &str = "d2845850a301270378276170706c69636174696f6e2f766e642e6c79732e6174746573746174696f6e2e76322b63626f72045820214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfda0582ea20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe568005840ba6eae66b3a04f25e6116e7278ae8b0e2174a2964bb0917ae53ab2ef326c22a706e90cae6886b20ecd086fee4b515de046d450f90de81f9aa46a9b34bd7f1a09";

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn golden_pubkey() -> [u8; 32] {
    hex_to_bytes(GOLDEN_PUBKEY_HEX)
        .as_slice()
        .try_into()
        .unwrap()
}

fn golden_hash() -> [u8; 32] {
    hex_to_bytes(GOLDEN_HASH_HEX).as_slice().try_into().unwrap()
}

fn golden_signature() -> [u8; 64] {
    hex_to_bytes(GOLDEN_SIGNATURE_HEX)
        .as_slice()
        .try_into()
        .unwrap()
}

fn golden_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden.key");
    std::fs::write(&path, GOLDEN_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

// ------------------------------------------------------------ head encoding

/// Canonical shortest-form heads at every width boundary (RFC 8949 §4.2.1).
#[test]
fn write_head_uses_shortest_form_at_every_boundary() {
    let cases: &[(u64, &[u8])] = &[
        (0, &[0x00]),
        (23, &[0x17]),
        (24, &[0x18, 24]),
        (255, &[0x18, 0xff]),
        (256, &[0x19, 0x01, 0x00]),
        (65_535, &[0x19, 0xff, 0xff]),
        (65_536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
        (u64::from(u32::MAX), &[0x1a, 0xff, 0xff, 0xff, 0xff]),
        (
            u64::from(u32::MAX) + 1,
            &[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
        ),
        (
            u64::MAX,
            &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
    ];
    for (value, expected) in cases {
        let mut out = Vec::new();
        write_head(&mut out, MAJOR_UNSIGNED, *value);
        assert_eq!(out.as_slice(), *expected, "value {value}");
    }
}

/// Signed encoding across zero, both extremes, and the negative boundaries.
#[test]
fn write_i64_covers_full_range_canonically() {
    let cases: &[(i64, &[u8])] = &[
        (0, &[0x00]),
        (1, &[0x01]),
        (-1, &[0x20]),
        (-24, &[0x37]),
        (-25, &[0x38, 0x18]),
        (-256, &[0x38, 0xff]),
        (-257, &[0x39, 0x01, 0x00]),
        (
            i64::MAX,
            &[0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (
            i64::MIN,
            &[0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
    ];
    for (value, expected) in cases {
        let mut out = Vec::new();
        write_i64(&mut out, *value);
        assert_eq!(out.as_slice(), *expected, "value {value}");
    }
}

// ------------------------------------------------------------- golden pins

#[test]
fn protected_bytes_matches_golden_and_is_80_bytes() {
    let protected = protected_bytes(&golden_pubkey());
    assert_eq!(protected, hex_to_bytes(GOLDEN_PROTECTED_HEX));
    assert_eq!(protected.len(), 80);
}

#[test]
fn claims_bytes_matches_golden() {
    let claims = claims_bytes(&golden_hash(), GOLDEN_TIMESTAMP);
    assert_eq!(claims, hex_to_bytes(GOLDEN_CLAIMS_HEX));
}

#[test]
fn sig_structure_matches_golden() {
    let protected = protected_bytes(&golden_pubkey());
    let claims = claims_bytes(&golden_hash(), GOLDEN_TIMESTAMP);
    let sig_structure = sig_structure_bytes(&protected, &claims);
    assert_eq!(sig_structure, hex_to_bytes(GOLDEN_SIG_STRUCTURE_HEX));
    assert_eq!(sig_structure.len(), 143);
}

#[test]
fn artifact_bytes_matches_golden() {
    let artifact = artifact_bytes(
        &golden_pubkey(),
        &golden_hash(),
        GOLDEN_TIMESTAMP,
        &golden_signature(),
    );
    assert_eq!(artifact, hex_to_bytes(GOLDEN_ARTIFACT_HEX));
    assert_eq!(artifact.len(), 199);
}

/// The golden signature is what the golden identity produces over the
/// golden `Sig_structure` — Ed25519 is deterministic, so this pins the
/// signing path end to end.
#[test]
fn golden_identity_reproduces_golden_signature() {
    let (_dir, identity) = golden_identity();
    assert_eq!(identity.public_key_bytes(), golden_pubkey());
    let signature = identity.sign(&hex_to_bytes(GOLDEN_SIG_STRUCTURE_HEX));
    assert_eq!(signature, golden_signature());
}

/// The artifact size window is exactly 191–199 bytes: the timestamp head is
/// the only variable part (1, 2, 3, 5, or 9 bytes).
#[test]
fn artifact_size_window_is_191_to_199() {
    let pk = golden_pubkey();
    let hash = golden_hash();
    let sig = golden_signature();
    let smallest = artifact_bytes(&pk, &hash, 0, &sig);
    assert_eq!(smallest.len(), 191);
    let largest = artifact_bytes(&pk, &hash, i64::MAX, &sig);
    assert_eq!(largest.len(), 199);
    for ts in [0, 23, 24, 255, 256, 65_535, 65_536, i64::MAX, i64::MIN, -1] {
        let len = artifact_bytes(&pk, &hash, ts, &sig).len();
        assert!((191..=199).contains(&len), "ts {ts} gave length {len}");
    }
}

// ------------------------------------------------------------ decode_fields

#[test]
fn decode_fields_extracts_golden_fields() {
    let fields = decode_fields(&hex_to_bytes(GOLDEN_ARTIFACT_HEX)).unwrap();
    assert_eq!(fields.signer_public_key, golden_pubkey());
    assert_eq!(fields.payload_hash, golden_hash());
    assert_eq!(fields.timestamp, GOLDEN_TIMESTAMP);
    assert_eq!(fields.signature, golden_signature());
}

#[test]
fn decode_fields_rejects_oversize_input() {
    let oversize = vec![0u8; MAX_ARTIFACT_LEN + 1];
    assert!(decode_fields(&oversize).is_err());
}

#[test]
fn decode_fields_rejects_non_cbor_input() {
    assert!(decode_fields(b"not cbor at all").is_err());
    assert!(decode_fields(&[]).is_err());
}

// --------------------------------------------------- coset cross-check (dev)

/// An independent Rust COSE library (coset, dev-dependency only) builds a
/// byte-identical artifact from the same inputs — the in-tree half of the
/// two-library conformance obligation.
#[test]
fn coset_builds_byte_identical_artifact() {
    use coset::{CoseSign1Builder, HeaderBuilder, TaggedCborSerializable, iana};

    let (_dir, identity) = golden_identity();
    let claims = claims_bytes(&golden_hash(), GOLDEN_TIMESTAMP);
    let protected = HeaderBuilder::new()
        .algorithm(iana::Algorithm::EdDSA)
        .content_type(CONTENT_TYPE.to_string())
        .key_id(golden_pubkey().to_vec())
        .build();
    let sign1 = CoseSign1Builder::new()
        .protected(protected)
        .payload(claims)
        .create_signature(b"", |data| identity.sign(data).to_vec())
        .build();
    let coset_artifact = sign1.to_tagged_vec().unwrap();
    assert_eq!(coset_artifact, hex_to_bytes(GOLDEN_ARTIFACT_HEX));
}

/// coset parses and cryptographically verifies the hand-assembled artifact.
#[test]
fn coset_verifies_hand_assembled_artifact() {
    use coset::{CoseSign1, TaggedCborSerializable};

    let artifact = artifact_bytes(
        &golden_pubkey(),
        &golden_hash(),
        GOLDEN_TIMESTAMP,
        &golden_signature(),
    );
    let parsed = CoseSign1::from_tagged_slice(&artifact).unwrap();
    parsed
        .verify_signature(b"", |signature, data| {
            Ed25519Identity::verify(&golden_pubkey(), data, signature)
        })
        .unwrap();
}
