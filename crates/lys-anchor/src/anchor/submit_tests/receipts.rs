#![cfg(test)]
//! What a submission produces: the leaf hash, the stored statement, and
//! receipts that reconstruct the root the checkpoint publishes, duplicates
//! included.

use super::*;

#[test]
fn the_leaf_hash_is_sha256_of_the_tag_byte_and_the_statement() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    let outcome = anchor
        .submit(
            Submission {
                statement: STATEMENT,
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    // Driven by the test, not by `raw_leaf_hash` — the helper the append path
    // used. A change to the domain-separation byte, or to what gets hashed,
    // has to be wrong in two places to survive this.
    let mut hasher = Sha256::new();
    hasher.update([0x00_u8]);
    hasher.update(STATEMENT);
    let expected: [u8; 32] = hasher.finalize().into();
    assert_eq!(outcome.leaf_hash, expected);

    // And against a value no Rust in this workspace produced.
    assert_eq!(hex(&outcome.leaf_hash), GOLDEN_STATEMENT_LEAF_HASH);

    // Genesis is leaf 0, so the submission is leaf 1 and the tree is size 2.
    assert_eq!(outcome.leaf_index, 1);
    assert_eq!(outcome.tree_size, 2);
    // The outcome and the artifact inside it must not be able to disagree.
    assert_eq!(outcome.tree_size, outcome.receipt.tree_size);
    assert_eq!(outcome.leaf_index, outcome.receipt.leaf_index);
}

#[test]
fn the_statement_is_stored_verbatim_and_the_anchor_reads_nothing_into_it() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor(dir);

    // Bytes that are not valid UTF-8, contain a NUL, and would be mangled by
    // any canonicalization, trimming or text handling on the way to storage.
    let statement: &[u8] = &[0xff, 0x00, 0x0a, b'{', 0xc3, 0x28, b'}', 0x0d];
    let outcome = anchor
        .submit(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap();

    assert_eq!(leaf_from_disk(dir, outcome.leaf_index), statement);

    // The genesis leaf is still where it was put, so the submission appended
    // rather than displacing anything.
    assert_eq!(leaf_from_disk(dir, 0), GENESIS);
    assert_eq!(
        hex(&Sha256::digest([&[0x00_u8], GENESIS].concat())),
        GOLDEN_GENESIS_LEAF_HASH
    );
}

#[test]
fn every_receipt_in_a_deeper_log_reconstructs_the_root_the_checkpoint_publishes() {
    // The two-leaf case below has an inclusion path of exactly ONE node, and a
    // one-node path is invariant under reordering — so the cross-check there
    // cannot catch a chunker that emits the right nodes in the wrong order.
    // This log is deep enough that most paths have three nodes, which makes
    // order observable.
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    for n in 0..8_u8 {
        anchor
            .submit(
                Submission {
                    statement: &[n; 11],
                },
                SubmitterContext::Unidentified,
            )
            .unwrap();
    }
    assert_eq!(anchor.tree_size(), 9);

    // The root from the other derivation: ct-merkle's accumulator, base64, a
    // signed note, and lys-core's checkpoint parser.
    let published = anchor.publish_checkpoint().unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier(&anchor))
        .expect("the checkpoint must verify");
    assert_eq!(body.tree_size(), 9);

    let key = anchor.signer().public_key();
    let mut checked = 0;
    let mut multi_node_paths = 0;
    for index in 0..9_u64 {
        let leaf = leaf_from_disk(tmp.path(), index);
        let receipt = anchor.receipt_for(index).unwrap();

        // Positive control: self-consistent, which is exactly the property
        // that would survive a wrongly ordered path.
        verify_receipt(&receipt, &leaf, &key).expect("the anchor's own receipt must verify");

        if receipt.inclusion_path.len() > 1 {
            multi_node_paths += 1;
        }
        assert_eq!(
            receipt
                .reconstructed_root(&leaf)
                .expect("the receipt's path must reconstruct"),
            body.root_hash(),
            "leaf {index}'s receipt attests to a root the anchor does not publish"
        );
        checked += 1;
    }
    // Count what fired, and count the cases that carry the property: a suite of
    // one-node paths would satisfy every assertion above while proving nothing
    // about order.
    assert_eq!(checked, 9);
    assert!(
        multi_node_paths >= 8,
        "only {multi_node_paths} of 9 receipts had a path long enough for order to matter"
    );
}

#[test]
fn the_receipt_reconstructs_the_root_the_checkpoint_publishes() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    let outcome = anchor
        .submit(
            Submission {
                statement: STATEMENT,
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    // The receipt verifies — necessary, and on its own worth nothing here: it
    // reconstructs the root from the receipt's own path, which is the very
    // thing under test. It is asserted as a positive control so the equality
    // below is not the equality of two failures.
    verify_receipt(&outcome.receipt, STATEMENT, &anchor.signer().public_key())
        .expect("the anchor's own receipt must verify");

    // The cross-check. This root came from walking the chunked inclusion path.
    let walked = outcome
        .receipt
        .reconstructed_root(STATEMENT)
        .expect("the receipt's path must reconstruct");

    // This one came from ct-merkle's accumulator, through base64, into a
    // signed note, and back out through lys-core's checkpoint parser — a
    // different derivation of the same 32 bytes, decided by neither of the two
    // functions under test.
    let published = anchor.publish_checkpoint().unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier(&anchor))
        .expect("the checkpoint must verify");
    assert_eq!(body.tree_size(), 2);
    assert_eq!(
        walked,
        body.root_hash(),
        "the root the receipt attests to is not the root the anchor publishes"
    );

    // And both agree with a value computed outside Rust from the two leaf
    // hashes, so a symmetric error shared by every Rust path here would still
    // be caught.
    assert_eq!(hex(&walked), GOLDEN_ROOT_2);
}

#[test]
fn duplicate_submissions_are_two_events_with_two_independently_verifying_receipts() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    // Identical bytes, submitted twice. Not "the same statement" — the anchor
    // has no notion of sameness, and asking it to have one is asking it to
    // decide what a statement means.
    let mut outcomes: Vec<SubmissionOutcome> = Vec::new();
    for _ in 0..2 {
        outcomes.push(
            anchor
                .submit(
                    Submission {
                        statement: STATEMENT,
                    },
                    SubmitterContext::Unidentified,
                )
                .unwrap(),
        );
    }
    // Count what fired before anything is concluded from the loop.
    assert_eq!(outcomes.len(), 2);

    let first = &outcomes[0];
    let second = &outcomes[1];

    // Two distinct indices, in order, after genesis.
    assert_eq!(first.leaf_index, 1);
    assert_eq!(second.leaf_index, 2);
    assert_ne!(first.leaf_index, second.leaf_index);

    // The same bytes, so the same leaf hash — that is not de-duplication, and
    // the log holds both.
    assert_eq!(first.leaf_hash, second.leaf_hash);
    assert_eq!(first.tree_size, 2);
    assert_eq!(second.tree_size, 3);

    // Each receipt verifies against its own index, at its own size, and the
    // two attest to different roots because they are about different trees.
    let key = anchor.signer().public_key();
    let mut verified = 0;
    for outcome in &outcomes {
        verify_receipt(&outcome.receipt, STATEMENT, &key)
            .expect("each receipt must verify on its own terms");
        verified += 1;
    }
    assert_eq!(verified, 2);
    assert_ne!(
        first.receipt.reconstructed_root(STATEMENT).unwrap(),
        second.receipt.reconstructed_root(STATEMENT).unwrap()
    );
    assert_ne!(first.receipt.signature, second.receipt.signature);

    // "Independently" means the receipts are not interchangeable. Take the
    // first receipt and claim the second's index: the walk lands on a root the
    // anchor never signed, and verification fails. Without this, a pair of
    // receipts that both verified for the *wrong* reason would pass above.
    let mut relabelled = first.receipt.clone();
    relabelled.leaf_index = second.leaf_index;
    assert!(
        verify_receipt(&relabelled, STATEMENT, &key).is_err(),
        "a receipt must not verify at an index it was not issued for"
    );

    // Both leaves are on disk, and the log holds three.
    assert_eq!(leaf_from_disk(tmp.path(), 1), STATEMENT);
    assert_eq!(leaf_from_disk(tmp.path(), 2), STATEMENT);
    assert_eq!(anchor.tree_size(), 3);
}
