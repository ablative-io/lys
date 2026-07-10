#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::attestation::encoding;
use crate::attestation::sign::verify_attestation;
use crate::keys::identity::Ed25519Identity;

/// Fixed golden seed: the 32 ASCII bytes `"lys-cose-conformance-test-seed01"`.
const GOLDEN_SEED: &[u8; 32] = b"lys-cose-conformance-test-seed01";

const GOLDEN_PAYLOAD: &[u8] = b"lys attestation conformance payload\n";

const GOLDEN_TIMESTAMP: i64 = 1_700_000_000_000;

/// The complete 199-byte golden artifact from the D4 design (§1.5).
const GOLDEN_ARTIFACT_HEX: &str = "d2845850a301270378276170706c69636174696f6e2f766e642e6c79732e6174746573746174696f6e2e76322b63626f72045820214e41cc8475f5bc9af68dbf33fa56e8adf5144a4a81134268f4ae379e103bfda0582ea20158209404f8b8cec8ad98a88b106d9345d518c273f012c1306b8af5103e865997191e021b0000018bcfe568005840ba6eae66b3a04f25e6116e7278ae8b0e2174a2964bb0917ae53ab2ef326c22a706e90cae6886b20ecd086fee4b515de046d450f90de81f9aa46a9b34bd7f1a09";

/// Golden artifact byte layout, used to splice mutants:
/// `d2` (0) `84` (1) `58 50` (2–3) protected (4–83) `a0` (84)
/// `58 2e` (85–86) claims (87–132) `58 40` (133–134) signature (135–198).
const OFFSET_UNPROTECTED: usize = 84;
const OFFSET_CLAIMS_HEAD: usize = 85;

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn golden_artifact() -> Vec<u8> {
    hex_to_bytes(GOLDEN_ARTIFACT_HEX)
}

fn golden_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("golden.key");
    std::fs::write(&path, GOLDEN_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

fn golden_attestation() -> Attestation {
    Attestation::from_cose_bytes(&golden_artifact()).unwrap()
}

/// Append a definite-length bstr (head + content) for test assembly.
fn push_bstr(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u8::try_from(bytes.len()).unwrap();
    if len < 24 {
        out.push(0x40 | len);
    } else {
        out.push(0x58);
        out.push(len);
    }
    out.extend_from_slice(bytes);
}

/// Assemble a tagged `COSE_Sign1` from raw parts, so tests can build
/// artifacts the canonical encoder would never emit.
fn assemble(protected: &[u8], unprotected: &[u8], claims: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut out = vec![0xd2, 0x84];
    push_bstr(&mut out, protected);
    out.extend_from_slice(unprotected);
    push_bstr(&mut out, claims);
    push_bstr(&mut out, signature);
    out
}

/// Sign arbitrary protected/claims parts with the golden identity and
/// assemble the artifact — the signature is internally consistent even for
/// shapes the verifier must reject.
fn signed_custom(protected: &[u8], claims: &[u8]) -> Vec<u8> {
    let (_dir, identity) = golden_identity();
    let signature = identity.sign(&encoding::sig_structure_bytes(protected, claims));
    assemble(protected, &[0xa0], claims, &signature)
}

// ------------------------------------------------------------ golden vector

#[test]
fn golden_artifact_parses_to_expected_fields() {
    let (_dir, identity) = golden_identity();
    let att = golden_attestation();
    assert_eq!(att.signer_public_key, identity.public_key_bytes());
    assert_eq!(att.timestamp, GOLDEN_TIMESTAMP);
    verify_attestation(&att, GOLDEN_PAYLOAD).unwrap();
}

#[test]
fn to_cose_bytes_reproduces_the_input_bytes_exactly() {
    let att = golden_attestation();
    assert_eq!(att.to_cose_bytes(), golden_artifact());
}

#[test]
fn round_trip_from_struct_is_identity() {
    let att = golden_attestation();
    let restored = Attestation::from_cose_bytes(&att.to_cose_bytes()).unwrap();
    assert_eq!(restored, att);
}

// -------------------------------------------------------------- mutants A–F

/// Mutant A — unprotected-header smuggling: content in the unprotected map
/// leaves the signature valid (it is outside `Sig_structure`) and vanilla
/// COSE verifiers accept it; lys rejects via the empty-map requirement and
/// the canonical byte-compare.
#[test]
fn mutant_a_unprotected_header_smuggling_is_rejected() {
    let golden = golden_artifact();
    let mut mutant = golden[..OFFSET_UNPROTECTED].to_vec();
    mutant.extend_from_slice(&[0xa1, 0x0a, 0x0a]); // {10: 10}
    mutant.extend_from_slice(&golden[OFFSET_UNPROTECTED + 1..]);
    assert!(Attestation::from_cose_bytes(&mutant).is_err());
}

/// Mutant B — tag stripping: the verifier requires tag 18; the bare
/// `COSE_Sign1` array is rejected.
#[test]
fn mutant_b_untagged_artifact_is_rejected() {
    let golden = golden_artifact();
    assert!(Attestation::from_cose_bytes(&golden[1..]).is_err());
    // A different tag number over the same array is equally dead.
    let mut retagged = golden;
    retagged[0] = 0xd3; // tag(19)
    assert!(Attestation::from_cose_bytes(&retagged).is_err());
}

/// Mutant C — indefinite-length re-encoding of the outer array: the four
/// items are untouched, so the signature stays cryptographically valid;
/// the canonical byte-compare rejects it anyway.
#[test]
fn mutant_c_indefinite_length_array_is_rejected() {
    let golden = golden_artifact();
    let mut mutant = vec![0xd2, 0x9f];
    mutant.extend_from_slice(&golden[2..]);
    mutant.push(0xff);
    assert!(Attestation::from_cose_bytes(&mutant).is_err());
}

/// Mutant D — oversized integer head on the claims bstr length (`59 00 2e`
/// for `58 2e`): the claims bytes are unchanged, so the signature over
/// `Sig_structure` remains valid; the canonical byte-compare rejects.
#[test]
fn mutant_d_oversized_length_head_is_rejected() {
    let golden = golden_artifact();
    let mut mutant = golden[..OFFSET_CLAIMS_HEAD].to_vec();
    mutant.push(0x59);
    mutant.push(0x00);
    mutant.extend_from_slice(&golden[OFFSET_CLAIMS_HEAD + 1..]);
    assert!(Attestation::from_cose_bytes(&mutant).is_err());
}

/// Mutant D (payload variant) — a non-canonical timestamp int head inside
/// the claims, properly signed: internally consistent signature, rejected
/// on canonicality.
#[test]
fn mutant_d_noncanonical_timestamp_head_is_rejected() {
    let (_dir, identity) = golden_identity();
    let protected = encoding::protected_bytes(&identity.public_key_bytes());
    let canonical_claims = encoding::claims_bytes(&[0x11; 32], 1);
    // Re-encode timestamp 1 with an oversized 8-byte head: `02 1b …01`.
    let mut claims = canonical_claims[..canonical_claims.len() - 1].to_vec();
    claims.extend_from_slice(&[0x1b, 0, 0, 0, 0, 0, 0, 0, 1]);
    let mutant = signed_custom(&protected, &claims);
    assert!(Attestation::from_cose_bytes(&mutant).is_err());
}

/// Mutant E — algorithm substitution: an artifact whose protected bucket
/// pins a different `alg` is rejected even when its signature is
/// internally consistent under that bucket.
#[test]
fn mutant_e_alg_substitution_is_rejected() {
    let (_dir, identity) = golden_identity();
    let mut protected = encoding::protected_bytes(&identity.public_key_bytes());
    assert_eq!(protected[2], 0x27); // -8
    protected[2] = 0x26; // -7 (ES256)
    let claims = encoding::claims_bytes(&[0x22; 32], GOLDEN_TIMESTAMP);
    let mutant = signed_custom(&protected, &claims);
    assert!(Attestation::from_cose_bytes(&mutant).is_err());
}

/// Mutant F — old-preimage splice: a signature over the deleted v1
/// preimage spliced into an otherwise-valid v2 artifact parses (the bytes
/// are canonical) but fails signature verification — cross-form confusion
/// is dead. The reverse direction is structurally impossible (§1.2 of the
/// design: a `Sig_structure` begins 0x84, which cannot start a v1 preimage).
#[test]
fn mutant_f_old_v1_preimage_signature_parses_but_fails_verification() {
    let (_dir, identity) = golden_identity();
    let payload_hash: [u8; 32] = {
        use sha2::{Digest, Sha256};
        Sha256::digest(GOLDEN_PAYLOAD).into()
    };
    let mut preimage = b"lys/attestation/v1".to_vec();
    preimage.extend_from_slice(&GOLDEN_TIMESTAMP.to_le_bytes());
    preimage.extend_from_slice(&payload_hash);
    let spliced_signature = identity.sign(&preimage);

    let artifact = encoding::artifact_bytes(
        &identity.public_key_bytes(),
        &payload_hash,
        GOLDEN_TIMESTAMP,
        &spliced_signature,
    );
    // Structurally canonical: parsing succeeds…
    let att = Attestation::from_cose_bytes(&artifact).unwrap();
    // …but the v1-preimage signature can never verify as a v2 attestation.
    assert!(verify_attestation(&att, GOLDEN_PAYLOAD).is_err());
}

// --------------------------------------------------------- structural edges

#[test]
fn trailing_garbage_is_rejected() {
    let mut padded = golden_artifact();
    padded.push(0x00);
    assert!(Attestation::from_cose_bytes(&padded).is_err());
}

#[test]
fn oversize_input_is_rejected_before_parsing() {
    let mut oversize = golden_artifact();
    oversize.resize(1025, 0x00);
    assert!(Attestation::from_cose_bytes(&oversize).is_err());
    let zeros = vec![0u8; 4096];
    assert!(Attestation::from_cose_bytes(&zeros).is_err());
}

#[test]
fn empty_and_truncated_inputs_are_rejected() {
    let golden = golden_artifact();
    assert!(Attestation::from_cose_bytes(&[]).is_err());
    assert!(Attestation::from_cose_bytes(&golden[..golden.len() - 1]).is_err());
    assert!(Attestation::from_cose_bytes(&golden[..10]).is_err());
}

/// Claims-shape pins: an empty claims map and an extra-claim map are both
/// rejected even with internally consistent signatures.
#[test]
fn empty_claims_and_extra_claims_are_rejected() {
    let (_dir, identity) = golden_identity();
    let protected = encoding::protected_bytes(&identity.public_key_bytes());

    let empty_claims = vec![0xa0];
    assert!(Attestation::from_cose_bytes(&signed_custom(&protected, &empty_claims)).is_err());

    let mut extra_claims = encoding::claims_bytes(&[0x33; 32], GOLDEN_TIMESTAMP);
    extra_claims[0] = 0xa3; // three entries
    extra_claims.extend_from_slice(&[0x03, 0x00]); // 3: 0
    assert!(Attestation::from_cose_bytes(&signed_custom(&protected, &extra_claims)).is_err());
}

/// A kid that is not exactly 32 bytes is rejected.
#[test]
fn wrong_kid_length_is_rejected() {
    let (_dir, identity) = golden_identity();
    let canonical = encoding::protected_bytes(&identity.public_key_bytes());
    // Rebuild the protected bucket with a 31-byte kid: shrink the bstr head
    // and drop the final key byte.
    let mut protected = canonical[..canonical.len() - 33].to_vec();
    assert_eq!(protected.pop(), Some(0x58)); // strip the 2-byte bstr head…
    protected.extend_from_slice(&[0x58, 31]); // …and write a 31-byte one
    protected.extend_from_slice(&[0x44; 31]);
    let claims = encoding::claims_bytes(&[0x55; 32], GOLDEN_TIMESTAMP);
    assert!(Attestation::from_cose_bytes(&signed_custom(&protected, &claims)).is_err());
}

/// Duplicate keys in the claims map are rejected (the positional shape pin
/// requires exactly `{1: …, 2: …}`).
#[test]
fn duplicate_claims_keys_are_rejected() {
    let (_dir, identity) = golden_identity();
    let protected = encoding::protected_bytes(&identity.public_key_bytes());
    let mut claims = vec![0xa2];
    for _ in 0..2 {
        claims.push(0x01);
        claims.push(0x58);
        claims.push(32);
        claims.extend_from_slice(&[0x66; 32]);
    }
    assert!(Attestation::from_cose_bytes(&signed_custom(&protected, &claims)).is_err());
}

/// Reordered protected-header keys are rejected (canonical order is a pin,
/// enforced positionally at decode and again by the byte-compare).
#[test]
fn reordered_protected_keys_are_rejected() {
    let (_dir, identity) = golden_identity();
    let canonical = encoding::protected_bytes(&identity.public_key_bytes());
    // Canonical layout: a3 | 01 27 | 03 78 27 <39 CT> | 04 58 20 <32 pk>.
    // Reorder to {3, 1, 4}.
    let mut protected = vec![0xa3];
    protected.extend_from_slice(&canonical[3..45]); // 03 78 27 <39 CT>
    protected.extend_from_slice(&canonical[1..3]); // 01 27
    protected.extend_from_slice(&canonical[45..]); // 04 58 20 <32 pk>
    assert_eq!(protected.len(), canonical.len());
    let claims = encoding::claims_bytes(&[0x77; 32], GOLDEN_TIMESTAMP);
    assert!(Attestation::from_cose_bytes(&signed_custom(&protected, &claims)).is_err());
}

/// A signature bstr that is not exactly 64 bytes is rejected.
#[test]
fn wrong_signature_length_is_rejected() {
    let (_dir, identity) = golden_identity();
    let protected = encoding::protected_bytes(&identity.public_key_bytes());
    let claims = encoding::claims_bytes(&[0x88; 32], GOLDEN_TIMESTAMP);
    let artifact = assemble(&protected, &[0xa0], &claims, &[0xab; 63]);
    assert!(Attestation::from_cose_bytes(&artifact).is_err());
}

/// A wrong content type is rejected even when everything else is valid and
/// the signature is internally consistent — the content type is the v2
/// domain discriminator.
#[test]
fn wrong_content_type_is_rejected() {
    let (_dir, identity) = golden_identity();
    let mut protected = encoding::protected_bytes(&identity.public_key_bytes());
    // Uppercase one content-type byte (offset 6 is inside the CT string).
    protected[6] = protected[6].to_ascii_uppercase();
    let claims = encoding::claims_bytes(&[0x99; 32], GOLDEN_TIMESTAMP);
    assert!(Attestation::from_cose_bytes(&signed_custom(&protected, &claims)).is_err());
}

// ----------------------------------------------------------- timestamp range

/// The frozen claims contract is `2: int unix-millisecond timestamp (i64,
/// pre-epoch representable)` — the decode path must accept every i64, not
/// just post-epoch values. Pre-epoch (negative), zero, and both extremes
/// parse, round-trip byte-identically, and cryptographically verify.
#[test]
fn full_i64_timestamp_range_parses_round_trips_and_verifies() {
    let (_dir, identity) = golden_identity();
    let payload_hash: [u8; 32] = {
        use sha2::{Digest, Sha256};
        Sha256::digest(GOLDEN_PAYLOAD).into()
    };
    let protected = encoding::protected_bytes(&identity.public_key_bytes());
    for ts in [i64::MIN, -1, 0, i64::MAX] {
        let claims = encoding::claims_bytes(&payload_hash, ts);
        let artifact = signed_custom(&protected, &claims);
        let att = Attestation::from_cose_bytes(&artifact).unwrap();
        assert_eq!(att.timestamp, ts, "timestamp {ts} did not round-trip");
        assert_eq!(att.to_cose_bytes(), artifact);
        verify_attestation(&att, GOLDEN_PAYLOAD).unwrap();
    }
}

/// A claims timestamp outside the i64 range is rejected at decode, even
/// with an internally consistent signature: the canonical CBOR unsigned
/// 2^63 (one past `i64::MAX`) and its negative mirror -2^63 - 1 (one past
/// `i64::MIN`) are both valid CBOR integers the contract excludes.
#[test]
fn out_of_i64_range_timestamp_is_rejected() {
    let (_dir, identity) = golden_identity();
    let protected = encoding::protected_bytes(&identity.public_key_bytes());
    // Canonical claims for timestamp 0 end `02 00` (key 2, value 0); swap
    // the value byte for a canonical 9-byte integer head outside i64.
    let base = encoding::claims_bytes(&[0xaa; 32], 0);
    let overflow_heads: &[[u8; 9]] = &[
        [0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0], // unsigned 2^63
        [0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0], // negative -1 - 2^63
    ];
    for head in overflow_heads {
        let mut claims = base[..base.len() - 1].to_vec();
        claims.extend_from_slice(head);
        let mutant = signed_custom(&protected, &claims);
        assert!(Attestation::from_cose_bytes(&mutant).is_err());
    }
}

/// Every rejection is the same non-oracle error value.
#[test]
fn all_rejections_collapse_to_invalid_signature() {
    let golden = golden_artifact();
    let mut retagged = golden.clone();
    retagged[0] = 0xd3;
    let mut padded = golden.clone();
    padded.push(0x00);
    for bad in [retagged.as_slice(), &padded, &golden[1..], b"junk"] {
        let err = Attestation::from_cose_bytes(bad).unwrap_err();
        assert!(matches!(err, crate::error::TrustError::InvalidSignature));
        assert_eq!(err.to_string(), "invalid signature");
    }
}
