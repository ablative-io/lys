#![cfg(test)]
//! Receipts and attestations sign different bytes, so neither passes as
//! the other.

use super::*;

/// Cross-protocol confusion, in both directions and at the signature level —
/// not merely at the parser level.
#[test]
fn an_attestation_signature_is_never_a_valid_receipt_signature() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);

    // Sign an attestation over the receipt's root, the most favourable case for
    // an attacker: same key, same 32 bytes of "meaning".
    let receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    let root = receipt.reconstructed_root(&leaf).unwrap();
    let attestation = attestation::sign_attestation(&root, &key);

    // Graft the attestation's signature onto the receipt.
    let mut grafted = receipt.clone();
    grafted.signature = attestation.signature;
    assert!(
        verify_receipt(&grafted, &leaf, &key.public_key_bytes()).is_err(),
        "an attestation signature must not satisfy a receipt"
    );

    // And the reverse: the receipt's signature must not satisfy the attestation.
    let mut grafted_back = attestation;
    grafted_back.signature = receipt.signature;
    assert!(attestation::verify_attestation(&grafted_back, &root).is_err());
}

#[test]
fn the_two_artifact_families_sign_different_bytes_for_the_same_key() {
    // The COSE `Sig_structure` prefix is identical for both — `0x84 0x6A
    // "Signature1"` — so byte-0 disjointness does not separate them. What does
    // is the protected bucket inside the signed bytes.
    let key = primary();

    // Read each family's protected bucket out of a real artifact, through the
    // public API only — the buckets are what the signatures actually cover.
    let attestation_bytes = attestation::sign_attestation(b"payload", &key).to_cose_bytes();
    let attestation_protected = protected_bucket_of(&attestation_bytes);

    let leaf = leaf_bytes(3);
    let receipt_bytes = sign_receipt(&leaf, 3, 8, &path_of(8, 3), &key)
        .unwrap()
        .to_cose_bytes();
    let receipt_protected = protected_bucket_of(&receipt_bytes);

    assert_ne!(attestation_protected, receipt_protected);
    assert_eq!(attestation_protected[0], 0xa3, "attestation: 3-entry map");
    assert_eq!(receipt_protected[0], 0xa4, "receipt: 4-entry map");

    // Both signed preimages nonetheless start identically, which is exactly why
    // the separation has to live in the bucket rather than in the prefix.
    let attestation_preimage =
        crate::cbor::sig_structure_bytes(&attestation_protected, b"whatever");
    let receipt_preimage = crate::cbor::sig_structure_bytes(&receipt_protected, b"whatever");
    assert_eq!(&attestation_preimage[..12], &receipt_preimage[..12]);
    assert_eq!(&attestation_preimage[..12], b"\x84\x6aSignature1");
    assert_ne!(attestation_preimage, receipt_preimage);
}

/// Extract the protected bucket bstr from a tagged `COSE_Sign1`.
fn protected_bucket_of(cose: &[u8]) -> Vec<u8> {
    let ciborium::value::Value::Tag(18, boxed) = ciborium::de::from_reader(cose).unwrap() else {
        panic!("tagged COSE_Sign1")
    };
    let ciborium::value::Value::Array(items) = *boxed else {
        panic!("4-array")
    };
    let ciborium::value::Value::Bytes(protected) = &items[0] else {
        panic!("protected is a bstr")
    };
    protected.clone()
}
