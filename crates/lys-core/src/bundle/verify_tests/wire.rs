#![cfg(test)]
//! The serialized shape of a bundle: its five fields, unknown and duplicate
//! keys, the JSON round trip, the constructor and the debug form.

use super::*;

// ----------------------------------------------------------- shape contract

#[test]
fn the_serialized_shape_is_exactly_the_five_wire_fields_in_order() {
    let s = one_link();
    let json = serde_json::to_string(&s.bundle).unwrap();

    let keys: Vec<&str> = [
        "format",
        "leaf",
        "inclusion_proof",
        "links",
        "counter_anchor",
    ]
    .to_vec();
    let mut cursor = 0;
    for key in &keys {
        let needle = format!("\"{key}\":");
        let at = json[cursor..]
            .find(&needle)
            .unwrap_or_else(|| panic!("missing field {key}"));
        cursor += at + needle.len();
    }

    // And nothing that could be mistaken for a signature over the container.
    for forbidden in ["signature", "sig", "signed", "mac"] {
        assert!(
            !json.contains(&format!("\"{forbidden}\"")),
            "the bundle must carry no {forbidden} field: a signature over packaging \
             invites checking the wrapper and skipping the contents"
        );
    }
    // Nor any key material.
    assert!(!json.contains("anchor_key"));
    assert!(!json.contains("verifier"));
}

#[test]
fn an_unknown_field_is_not_a_valid_v1_bundle() {
    let s = one_link();
    let json = serde_json::to_string(&s.bundle).unwrap();
    let smuggled = json.replace(
        "\"counter_anchor\":null",
        "\"counter_anchor\":null,\"extra\":1",
    );
    assert!(serde_json::from_str::<VerificationBundle>(&smuggled).is_err());
}

#[test]
fn a_duplicate_key_is_not_a_valid_v1_bundle() {
    let s = one_link();
    let json = serde_json::to_string(&s.bundle).unwrap();
    let duplicated = json.replace(
        "\"counter_anchor\":null",
        "\"counter_anchor\":null,\"format\":\"lys/verification-bundle/v1\"",
    );
    assert!(serde_json::from_str::<VerificationBundle>(&duplicated).is_err());
}

#[test]
fn a_bundle_round_trips_through_json_and_still_verifies() {
    // The bundle is a file people move around; a round trip through the codec
    // must not change what it establishes.
    let s = one_link();
    let json = serde_json::to_string_pretty(&s.bundle).unwrap();
    let restored: VerificationBundle = serde_json::from_str(&json).unwrap();
    let verified = verify_bundle(&restored, &s.child.verifier(), &[s.anchor.verifier()]).unwrap();
    assert_eq!(verified.leaf(), CHILD_LEAF);
    assert_eq!(verified.notarizations().len(), 1);
}

#[test]
fn the_constructor_sets_the_frozen_format_and_an_empty_slot() {
    let s = one_link();
    assert_eq!(s.bundle.format, VERIFICATION_BUNDLE_FORMAT);
    assert_eq!(s.bundle.format, "lys/verification-bundle/v1");
    assert!(s.bundle.counter_anchor.is_none());
}

#[test]
fn the_debug_form_carries_no_private_material() {
    let s = one_link();
    let verified = verify_bundle(&s.bundle, &s.child.verifier(), &[s.anchor.verifier()]).unwrap();
    let rendered = format!("{verified:?}");
    assert!(rendered.contains("VerifiedBundle"));
    for forbidden in ["seed", "secret", "private", "signingkey"] {
        assert!(!rendered.to_lowercase().contains(forbidden));
    }
}
