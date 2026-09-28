#![cfg(test)]
//! go-cose signs over exactly the bytes `delegation_preimage` builds, across
//! the sweep of claims.

use super::*;

/// go-cose accepts what lys signs, and — the assertion this file exists for —
/// the bytes go-cose signed over are byte-identical to `delegation_preimage`'s.
#[test]
fn go_cose_signs_over_exactly_the_bytes_delegation_preimage_builds() {
    let Some(go) = go_or_skip("go-cose delegation conformance") else {
        return;
    };
    let (workdir, bin) = cose_tool(&go);

    let (_, root) = identity(&ROOT_SEED);
    let (_, delegated) = identity(&DELEGATED_SEED);
    let root_public_key = root.public_key_bytes();
    let root_hex = to_hex(&root_public_key);
    let delegated_public_key = delegated.public_key_bytes();

    let claims = sweep_claims(delegated_public_key);
    let mut cases = 0usize;
    let mut wide_timestamp_cases = 0usize;
    let mut long_subject_cases = 0usize;
    let mut distinct_sequences = std::collections::BTreeSet::new();
    let mut distinct_pairs = std::collections::BTreeSet::new();

    for claim in &claims {
        let artifact = sign_delegation(&root, claim).unwrap();
        let preimage = delegation_preimage(&root_public_key, claim);

        let (ok, stdout) =
            run_built_tool(&bin, &["delegation-verify", root_hex.as_str()], &artifact);
        assert!(
            ok,
            "go-cose rejected a delegation lys signed (subject {} bytes, \
             not_before {})",
            claim.subject_value.len(),
            claim.not_before_unix_ms
        );
        let report = parse_report(&stdout);

        // ---- THE assertion. Not "go-cose said yes" — which bytes it said yes
        // to. An implementation that has never seen lys reassembled the
        // Sig_structure from the artifact and reached the same 165-or-so bytes
        // `delegation_preimage` produced.
        assert_eq!(
            report["sig_structure"],
            to_hex(&preimage),
            "go-cose signed over different bytes than delegation_preimage built \
             (subject {} bytes, not_before {})",
            claim.subject_value.len(),
            claim.not_before_unix_ms
        );

        // ---- The preimage's own shape, read out of the bytes go-cose returned
        // rather than restated here. `protected` comes back bstr-wrapped,
        // exactly as it sits inside the Sig_structure.
        let protected = hex_bytes(&report["protected"]);
        let payload = hex_bytes(&report["payload"]);
        // 12 = 0x84 (4-array) + 0x6A + "Signature1".
        assert!(
            preimage[12..].starts_with(&protected),
            "the protected bucket go-cose read is not the one inside our preimage"
        );
        // RFC 9052 §4.4's third element. Pinned here because an unpinned
        // `external_aad` sits INSIDE the signed bytes: two conforming
        // implementations would produce different signatures over one claim and
        // neither would be wrong.
        assert_eq!(
            preimage[12 + protected.len()],
            0x40,
            "external_aad must be the empty byte string h''"
        );
        assert!(
            preimage.ends_with(&payload),
            "the payload go-cose read is not the one inside our preimage"
        );

        // ---- Every payload field the scaffold found, with its CBOR type, so a
        // type or a dropped field surfaces as a value disagreement rather than
        // as a parse that happens to work. The label list is compared too: a
        // field this format is supposed to carry and does not is a
        // `payload_labels` mismatch here, not a lookup nobody made.
        let expected = expected_payload_report(claim);
        let expected_labels = expected
            .iter()
            .map(|(name, _)| name.trim_start_matches("payload_"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            report["payload_labels"], expected_labels,
            "the payload does not carry the labels this format version defines"
        );
        let mut payload_fields_checked = 0usize;
        for (name, value) in &expected {
            assert_eq!(report[name], *value, "payload field {name} disagrees");
            payload_fields_checked += 1;
        }
        assert_eq!(
            payload_fields_checked, 6,
            "the payload comparison must cover every field this version defines"
        );
        // The two fields whose wire values must differ, read out of what the Go
        // decoder reported rather than out of our own claim. If the vocabularies
        // were renumbered so that `subject_kind == role`, a transposition of the
        // two labels would become undetectable and this assertion is where the
        // second party notices.
        assert_ne!(
            report["payload_1"], report["payload_4"],
            "subject_kind and role must not share a wire value, or transposing \
             them is invisible on the wire"
        );

        // ---- And `kid`: compared HERE, by this test, against the key it named.
        // The scaffold never consults it — see the module docs.
        assert_eq!(report["kid"], root_hex);

        if claim.not_before_unix_ms > 65_535 {
            wide_timestamp_cases += 1;
        }
        if claim.subject_value.len() > 255 {
            long_subject_cases += 1;
        }
        distinct_sequences.insert(claim.sequence);
        distinct_pairs.insert((claim.subject_kind.wire_value(), claim.role.wire_value()));
        cases += 1;
    }

    // Count what fired. A loop that ran zero times satisfies every assertion
    // inside it, and a Go gate that never spawned looks exactly like one that
    // passed.
    assert_eq!(cases, 72, "12 timestamps x 6 subject values");
    // Both valid pairs reached the Go gate. A sweep that only ever emitted the
    // anchor's pair would leave the seat one unread by any second party.
    assert_eq!(
        distinct_pairs,
        std::collections::BTreeSet::from([(1u64, 2u64), (2, 3)]),
        "both valid (subject_kind, role) pairs must have been read by go-cose"
    );
    // And the sweep reached every `sequence` head width, so the newest payload
    // field is covered on the same axis as the rest rather than only at 300.
    assert_eq!(
        distinct_sequences.len(),
        7,
        "every sequence head width must have appeared in the sweep"
    );
    // And the sweep reached the shapes where a head width can be wrong at all.
    // A four- or eight-byte head written one width too wide is still perfectly
    // decodable, so nothing but a byte comparison against another
    // implementation discriminates — and a sweep confined to small values would
    // leave that undiscriminated while looking thorough.
    assert_eq!(
        wide_timestamp_cases, 30,
        "cases whose not_before needs a four- or eight-byte CBOR head"
    );
    assert_eq!(
        long_subject_cases, 12,
        "cases whose subject value needs a two-byte tstr length"
    );
    drop(workdir);
}
