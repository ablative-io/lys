#![cfg(test)]
//! The case table: every bundle both implementations are asked about, with
//! the verdict the specification requires, and the edits that produce the
//! refused ones.

use super::*;

/// Re-serialize a bundle after editing its JSON as a generic value, for the
/// shape attacks that a typed bundle cannot express.
pub fn edited_json(scenario: &Scenario, edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value: serde_json::Value = serde_json::to_value(&scenario.bundle).unwrap();
    edit(&mut value);
    serde_json::to_string(&value).unwrap()
}

pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();

    // ---- Accepted: the three legitimate shapes, plus the truncation control.
    cases.push(case("unnotarized_bundle", &chain(0), true));
    cases.push(case("one_link_chain", &chain(1), true));
    cases.push(case("two_link_chain", &chain(2), true));

    // Dropping the last link is a true weaker bundle, not an error: the
    // remaining chain still joins to the inclusion proof.
    let mut truncated = chain(2);
    truncated.bundle.links.pop();
    truncated.anchors.pop();
    cases.push(case("two_link_chain_truncated_to_one", &truncated, true));

    // The unrelated pair, valid on its own — the control that makes the two
    // splice refusals below demonstrably about the join rather than about a
    // broken fixture.
    cases.push(case(
        "unrelated_log_bundle_is_valid_on_its_own",
        &unrelated(),
        true,
    ));

    // ---- Refused: the join. Link 0 carries a receipt that verifies perfectly,
    // over a checkpoint that belongs to a different log than the one the
    // inclusion proof was verified against.
    let base = chain(1);
    let other = unrelated();
    let spliced = Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            base.bundle.inclusion_proof.clone(),
            other.bundle.links.clone(),
        ),
        log_key: base.log_key,
        anchors: other.anchors.clone(),
    };
    cases.push(case("link_zero_over_an_unrelated_log", &spliced, false));

    // ---- Refused: the rung. Link 1 is a valid receipt over a checkpoint that
    // is not anchor 0's own, so the ladder has a missing step.
    let mut misrung = chain(2);
    misrung.bundle.links[1] = other.bundle.links[0].clone();
    misrung.anchors[1].clone_from(&other.anchors[0]);
    cases.push(case("spliced_unrelated_second_link", &misrung, false));

    // ---- Refused: the rung again, this time reached on its own terms. Every
    // artifact is individually valid and every signature checks out; only the
    // root anchor 0 vouched for and the root anchor 0 published disagree.
    cases.push(case(
        "anchor_published_a_root_its_receipt_never_vouched_for",
        &divergent_rung(),
        false,
    ));

    // ---- Refused: equivocation at an unchanged size. Isolates the rung's root
    // comparison — nothing else in the verification can catch it.
    cases.push(case(
        "anchor_equivocates_at_the_same_tree_size",
        &equivocating_anchor(),
        false,
    ));

    // ---- Refused: a relabelled tree size, caught by the anchor's own
    // note-signed checkpoint. The only case where the rung's size comparison is
    // load-bearing alone — the root comparison passes.
    cases.push(case(
        "relabelled_tree_size_contradicts_the_anchors_checkpoint",
        &relabelled_tree_size(true),
        false,
    ));

    // ---- ACCEPTED, and it documents a real limit rather than a check: on the
    // final link there is no published checkpoint to contradict a relabelled
    // size, so the claim stands. Both implementations agree, because the
    // malleability belongs to the RFC's proof format, not to either of them.
    cases.push(case(
        "a_relabelled_size_survives_on_the_final_link",
        &relabelled_tree_size(false),
        true,
    ));

    // ---- Refused: order is load-bearing and validated, not assumed.
    let mut reordered = chain(2);
    reordered.bundle.links.swap(0, 1);
    reordered.anchors.swap(0, 1);
    cases.push(case("reordered_two_link_chain", &reordered, false));

    // ---- Refused: attribution. A receipt is only evidence about the anchor
    // whose key it names, so verifying link 0 under the wrong anchor's key must
    // fail even though both keys are real and both anchors exist.
    let mut swapped_keys = chain(2);
    swapped_keys.anchors.swap(0, 1);
    cases.push(case("anchor_keys_swapped", &swapped_keys, false));

    let mut wrong_log = chain(1);
    wrong_log.log_key = wrong_log.anchors[0].clone();
    cases.push(case("log_key_is_actually_the_anchors", &wrong_log, false));

    // ---- Refused: the key count must match exactly. Verifying a prefix would
    // report success for less than the bundle asserts.
    let mut too_few = chain(2);
    too_few.anchors.pop();
    cases.push(case("fewer_anchor_keys_than_links", &too_few, false));

    let mut too_many = chain(1);
    too_many.anchors.push(too_many.anchors[0].clone());
    cases.push(case("more_anchor_keys_than_links", &too_many, false));

    // ---- Refused: a populated counter-anchor slot. Nothing can verify one in
    // v1, and carrying an unverified time attestation is how a reader comes to
    // believe it.
    let mut countered = chain(1);
    countered.bundle.counter_anchor = Some("AAAA".to_string());
    cases.push(case("populated_counter_anchor", &countered, false));

    // ---- Refused: tampering, one field at a time.
    let mut bad_path = chain(1);
    bad_path.bundle.inclusion_proof.hashes[0] =
        flip_base64_byte(&bad_path.bundle.inclusion_proof.hashes[0]);
    cases.push(case("tampered_inclusion_path_node", &bad_path, false));

    let mut bad_receipt = chain(1);
    bad_receipt.bundle.links[0].receipt = flip_base64_byte(&bad_receipt.bundle.links[0].receipt);
    cases.push(case("tampered_receipt_byte", &bad_receipt, false));

    let mut bad_leaf = chain(1);
    bad_leaf.bundle.leaf = flip_base64_byte(&bad_leaf.bundle.leaf);
    cases.push(case("tampered_leaf", &bad_leaf, false));

    // A single flipped byte in link 0's checkpoint text breaks both the join
    // equality and the note signature over it.
    let mut bad_checkpoint = chain(1);
    bad_checkpoint.bundle.links[0].checkpoint = bad_checkpoint.bundle.links[0]
        .checkpoint
        .replace("child", "chxld");
    cases.push(case(
        "tampered_checkpoint_in_link_zero",
        &bad_checkpoint,
        false,
    ));

    // ---- Refused: container shape. A v1 bundle carrying a field v1 does not
    // define is not a v1 bundle, and an unrecognised format string is refused
    // before anything is parsed hopefully.
    let shaped = chain(1);
    cases.push(Case {
        name: "unknown_field_in_the_container",
        json: edited_json(&shaped, |value| {
            value
                .as_object_mut()
                .unwrap()
                .insert("extra".to_string(), serde_json::Value::Bool(true));
        }),
        log_key: shaped.log_key.clone(),
        anchors: shaped.anchors.clone(),
        accept: false,
    });
    cases.push(Case {
        name: "wrong_format_string",
        json: edited_json(&shaped, |value| {
            value.as_object_mut().unwrap().insert(
                "format".to_string(),
                serde_json::Value::String("lys/verification-bundle/v2".to_string()),
            );
        }),
        log_key: shaped.log_key.clone(),
        anchors: shaped.anchors.clone(),
        accept: false,
    });

    cases
}

/// Flips one bit inside a base64 payload, keeping it valid base64 of the same
/// length so the failure is the content rather than the encoding.
pub fn flip_base64_byte(encoded: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let mut bytes = STANDARD.decode(encoded).unwrap();
    assert!(!bytes.is_empty(), "nothing to tamper with");
    bytes[0] ^= 0x01;
    STANDARD.encode(bytes)
}
