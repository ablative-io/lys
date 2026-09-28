#![cfg(test)]
//! The seeds derive the named keys, the preimage matches the frozen
//! `Sig_structure`, signing reproduces the frozen artifacts, and they verify
//! and parse back.

use super::*;

/// Both key derivations reach the public keys every vector names.
///
/// If this fails, nothing below means anything — the two sides are not talking
/// about the same keys — so it is a test of its own rather than a preamble
/// inside another.
#[test]
fn the_seeds_derive_the_public_keys_the_vectors_name() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);
    assert_eq!(to_hex(&root.public_key_bytes()), HEX_ROOT_PUBLIC_KEY);
    assert_eq!(
        to_hex(&delegated.public_key_bytes()),
        HEX_DELEGATED_PUBLIC_KEY
    );
}

/// The `Sig_structure` this crate builds equals the one derived from the
/// specification independently, for every vector.
#[test]
fn delegation_preimage_matches_the_frozen_sig_structure() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);

    let mut checked = 0;
    for v in all_vectors() {
        let name = v.name;
        let claim = v.claim(delegated.public_key_bytes());

        assert_eq!(
            claim.role.wire_value(),
            v.role_wire,
            "{name}: the role's wire encoding has moved from the frozen value"
        );
        assert_eq!(
            claim.subject_kind.wire_value(),
            v.subject_kind_wire,
            "{name}: the subject kind's wire encoding has moved from the frozen value"
        );

        let preimage = delegation_preimage(&root.public_key_bytes(), &claim);
        assert_eq!(
            to_hex(&preimage),
            v.hex_preimage,
            "{name}: the signing preimage no longer matches the frozen vector — \
             every signature ever produced under this format is over these bytes, \
             so a change here breaks all historical verification"
        );
        checked += 1;
    }
    assert_eq!(checked, 5, "every frozen vector must have been reproduced");
}

/// Every whole artifact, and the signature inside it, equals the frozen vector.
///
/// Ed25519 is deterministic, so `sign_delegation` under the vector's seed has
/// exactly one correct output; this is byte-identity, not mutual acceptance.
#[test]
fn sign_delegation_reproduces_the_frozen_artifacts() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);

    let mut checked = 0;
    for v in all_vectors() {
        let name = v.name;
        let claim = v.claim(delegated.public_key_bytes());

        let artifact = sign_delegation(&root, &claim).unwrap();
        assert_eq!(to_hex(&artifact), v.hex_artifact, "{name}: artifact");

        // The signature on its own, so a failure says which half moved rather
        // than only that the artifact differs.
        assert_eq!(
            to_hex(&artifact[artifact.len() - 64..]),
            v.hex_signature,
            "{name}: signature"
        );

        // And the two-phase path must reach the same bytes as the convenience
        // wrapper: an operator signing on an air-gapped machine gets the artifact
        // a single-machine signer would have produced, or the split is a second
        // format.
        let signature: [u8; 64] = from_hex(v.hex_signature).try_into().unwrap();
        let assembled = assemble_delegation(&root.public_key_bytes(), &claim, &signature).unwrap();
        assert_eq!(
            to_hex(&assembled),
            v.hex_artifact,
            "{name}: assemble_delegation and sign_delegation disagree about the envelope"
        );
        checked += 1;
    }
    assert_eq!(checked, 5, "every frozen vector must have been signed");
}

/// The positive control: every frozen artifact verifies, and parses back to the
/// claim it was built from.
///
/// Without this, every assertion above could be satisfied by an implementation
/// broken in the same direction as the literals — the literals would then be
/// pinning a defect rather than a format.
///
/// Each vector is verified under **its own** subject kind, which is also the
/// cross-kind control: B is a seat delegation and A and C are domain ones, so a
/// verifier that ignored the kind would be the only way all three could pass
/// under one.
#[test]
fn the_frozen_artifacts_verify_and_parse_back() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);
    let root_public_key: [u8; 32] = from_hex(HEX_ROOT_PUBLIC_KEY).try_into().unwrap();

    let mut checked = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for v in all_vectors() {
        let name = v.name;
        let artifact = from_hex(v.hex_artifact);

        let verified = verify_delegation(
            &artifact,
            &root_public_key,
            v.subject_kind,
            &v.subject_value,
        )
        .unwrap_or_else(|err| panic!("{name}: the frozen artifact must verify, got {err:?}"));

        assert_eq!(verified.root_public_key, root_public_key, "{name}");
        assert_eq!(
            verified.claim,
            v.claim(delegated.public_key_bytes()),
            "{name}"
        );
        assert_eq!(to_hex(&verified.signature), v.hex_signature, "{name}");

        // Re-encoding the parsed delegation returns the same bytes, which is what
        // makes "canonical" mean the frozen encoding rather than merely a
        // self-consistent one.
        assert_eq!(to_hex(&verified.to_cose_bytes()), v.hex_artifact, "{name}");

        // The parse-only route reaches the same value. Named for parsing because
        // it is NOT verification: it takes no expected key and no expected
        // subject, so the delegation it returns vouches for nothing.
        let parsed = Delegation::from_cose_bytes(&artifact).unwrap();
        assert_eq!(parsed, verified, "{name}");

        kinds.insert(v.subject_kind_wire);
        checked += 1;
    }
    assert_eq!(checked, 5, "every frozen vector must have been verified");
    assert_eq!(
        kinds.len(),
        2,
        "both subject kinds must appear among the frozen vectors, or the seat \
         arm has no golden coverage at all"
    );

    // And the identity the vector's seed produces is the one the artifacts name,
    // so the frozen `kid` is not merely 32 plausible bytes.
    assert_eq!(root.public_key_bytes(), root_public_key);
}
