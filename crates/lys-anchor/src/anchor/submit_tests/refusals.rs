#![cfg(test)]
//! Refusals: indices the log does not have, a genesis-only log, a refused
//! submission appending nothing, and every refusal being one value.

use super::*;

#[test]
fn receipt_for_refuses_an_index_the_log_does_not_have() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());
    anchor
        .submit(
            Submission {
                statement: STATEMENT,
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    // Positive control: the indices that exist are served, so the refusals
    // below are about the index and not about a method that refuses always.
    let mut served = 0;
    for index in 0..2_u64 {
        anchor
            .receipt_for(index)
            .expect("every logged leaf has a receipt");
        served += 1;
    }
    assert_eq!(served, 2);

    let mut refused = 0;
    for index in [2_u64, 3, u64::MAX] {
        let err = anchor
            .receipt_for(index)
            .expect_err("an index past the end has no leaf");
        assert!(
            matches!(
                err,
                AnchorError::Proof(ProofError::NoSuchLeaf { leaf_index, tree_size, ref origin })
                    if leaf_index == index && tree_size == 2 && origin == ORIGIN
            ),
            "an absent index must be refused by name, got: {err}"
        );
        refused += 1;
    }
    assert_eq!(refused, 3);

    // Refusing must not have appended anything on the caller's behalf.
    assert_eq!(anchor.tree_size(), 2);
}

#[test]
fn receipt_for_refuses_a_log_that_holds_only_its_genesis_leaf() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());
    assert_eq!(anchor.tree_size(), 1);

    // Named by this layer, not forwarded from lys-core's CDDL-cardinality
    // message, and reported for index 0 — the one index that does exist.
    let err = anchor
        .receipt_for(0)
        .expect_err("a one-leaf tree has no conforming receipt");
    assert!(
        matches!(
            err,
            AnchorError::Proof(ProofError::TreeTooSmallForReceipt { tree_size, ref origin })
                if tree_size == 1 && origin == ORIGIN
        ),
        "a one-leaf log must be refused by name, got: {err}"
    );

    // Positive control, and the proof that the refusal is about the size and
    // nothing else: one submission removes the condition permanently.
    anchor
        .submit(
            Submission {
                statement: STATEMENT,
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    anchor
        .receipt_for(0)
        .expect("genesis has a receipt once a second leaf exists");
}

#[test]
fn a_refused_submission_appends_nothing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor_with(dir, MaxSize::new(4));

    // Positive control first: the policy admits something, so the refusal below
    // is the rule firing rather than an anchor that refuses everything.
    let admitted = anchor
        .submit(
            Submission { statement: b"four" },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    assert_eq!(admitted.leaf_index, 1);
    assert_eq!(anchor.tree_size(), 2);

    let err = anchor
        .submit(
            Submission {
                statement: b"five!",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap_err();
    assert!(
        matches!(err, AnchorError::NotAdmitted),
        "a refused submission must produce NotAdmitted, got: {err}"
    );

    // The claim is about what is on disk, so it is read back through a store
    // handle that never saw either call.
    assert_eq!(anchor.tree_size(), 2);
    let store = FileLeafStore::open(dir).unwrap();
    assert_eq!(store.extent(), 2, "a refused submission must not append");
    assert_eq!(store.leaf(0).unwrap().as_deref(), Some(GENESIS));
    assert_eq!(store.leaf(1).unwrap().as_deref(), Some(&b"four"[..]));
}

#[test]
fn refusals_from_different_rules_and_different_policies_are_one_value() {
    // Three anchors, three admission rules, two policy types. A submitter who
    // could tell any of these apart would be reading the rule out of the
    // refusal, which is the disclosure the collapse exists to prevent.
    let tmp_a = TempDir::new().unwrap();
    let mut by_length = create_anchor_with(tmp_a.path(), MaxSize::new(0));

    let ca_dir = TempDir::new().unwrap();
    let ca_key = ca_dir.path().join("ca.key");
    std::fs::write(&ca_key, b"lys-anchor-submit-gate-ca-seed1!").unwrap();
    let ours = CertificateAuthority::new(Ed25519Identity::load(&ca_key).unwrap());
    let other_key = ca_dir.path().join("other-ca.key");
    std::fs::write(&other_key, b"lys-anchor-submit-gate-ca-seed2!").unwrap();
    let theirs = CertificateAuthority::new(Ed25519Identity::load(&other_key).unwrap());
    let foreign = theirs
        .issue_certificate("submitter", Duration::from_secs(3600), vec![])
        .unwrap();

    let tmp_b = TempDir::new().unwrap();
    let mut by_absence = create_anchor_with(
        tmp_b.path(),
        RecognisedCertificate::issued_by(ours.public_key_bytes()),
    );
    let tmp_c = TempDir::new().unwrap();
    let mut by_issuer = create_anchor_with(
        tmp_c.path(),
        RecognisedCertificate::issued_by(ours.public_key_bytes()),
    );

    let refusals = [
        // Tripped by length.
        by_length
            .submit(
                Submission {
                    statement: STATEMENT,
                },
                SubmitterContext::Unidentified,
            )
            .unwrap_err(),
        // Tripped by there being no credential at all.
        by_absence
            .submit(
                Submission {
                    statement: STATEMENT,
                },
                SubmitterContext::Unidentified,
            )
            .unwrap_err(),
        // Tripped by a valid certificate from the wrong authority — and
        // reached through the arm a transport would use, so the provenance
        // carrying the strongest claim does not soften the refusal.
        by_issuer
            .submit(
                Submission {
                    statement: STATEMENT,
                },
                SubmitterContext::AuthenticatedByTransport(
                    AuthenticatedPeer::verified_by_transport(&foreign.der_bytes),
                ),
            )
            .unwrap_err(),
    ];

    let mut compared = 0;
    for refusal in &refusals {
        assert!(matches!(refusal, AnchorError::NotAdmitted));
        assert_eq!(refusal.to_string(), refusals[0].to_string());
        assert_eq!(format!("{refusal:?}"), format!("{:?}", refusals[0]));
        assert_eq!(
            std::mem::discriminant(refusal),
            std::mem::discriminant(&refusals[0])
        );
        compared += 1;
    }
    // Count what fired: an empty array satisfies the loop without comparing
    // anything at all.
    assert_eq!(compared, 3, "every refusal case must have been submitted");

    // Positive control, and it is the part that can fail today. The three
    // assertions above are all *equalities*, and equalities between values a
    // broken harness produced identically would also pass. So the same three
    // comparisons are run against an error that genuinely differs: if
    // `to_string`, `Debug` or `discriminant` were blind, this would pass too.
    let different = by_length.receipt_for(9_999).unwrap_err();
    assert_ne!(different.to_string(), refusals[0].to_string());
    assert_ne!(format!("{different:?}"), format!("{:?}", refusals[0]));
    assert_ne!(
        std::mem::discriminant(&different),
        std::mem::discriminant(&refusals[0])
    );

    // And the refusal really does say nothing: no origin, no size, no rule.
    let message = refusals[0].to_string();
    assert!(!message.contains(ORIGIN), "the refusal named the origin");
    assert!(
        !message.chars().any(|c| c.is_ascii_digit()),
        "the refusal carried a number: {message}"
    );
}
