#![cfg(test)]
//! go-cose derives the inclusion path the anchor emitted, node for node.

use super::*;

#[test]
fn go_cose_derives_the_inclusion_path_the_anchor_emitted_node_for_node() {
    let Some(go) = go_or_skip("lys-anchor receipt conformance") else {
        return;
    };
    let (_bin_dir, bin) = cose_tool(&go);

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let mut anchor = Anchor::create(
        store,
        GENESIS,
        signer(dir),
        // The admission surface is not what this gate measures: a policy that
        // could refuse would put a second rule on every case.
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap();
    let pubkey_hex = to_hex(&anchor.signer().public_key());
    let seed_hex = to_hex(ANCHOR_SEED);

    let mut submissions = 0u64;
    let mut cases = 0usize;
    let mut multi_node = 0usize;

    for tree_size in 2..=LOG_SIZE {
        // ---- What `submit` reports, against what the artifact it returned
        // actually proves. Go is handed this test's own counter as the index,
        // never `outcome.leaf_index` — a check keyed on the value under test is
        // no check at all.
        let expected_index = tree_size - 1;
        let statement = leaf(expected_index);
        let outcome = anchor
            .submit(submission(&statement), SubmitterContext::Unidentified)
            .unwrap();
        submissions += 1;

        let (ok, stdout) = run_built_tool(
            &bin,
            &verify_args(&pubkey_hex, expected_index, tree_size),
            &outcome.receipt.to_cose_bytes(),
        );
        assert!(
            ok,
            "go-cose rejected the receipt `submit` returned for index \
             {expected_index} of {tree_size}"
        );
        // Not `ok` alone: a tool that exited 0 without doing anything satisfies
        // that. The root is Go's own recursive MTH over the supplied leaves.
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{} {tree_size} {expected_index}\n",
                to_hex(&root_of(tree_size))
            ),
            "the Go derivation disagreed with the anchor at index \
             {expected_index} of {tree_size}"
        );
        assert_eq!(
            outcome.leaf_index, expected_index,
            "`submit` reported an index the emitted receipt does not prove"
        );
        assert_eq!(
            outcome.tree_size, tree_size,
            "`submit` reported a size the emitted receipt does not prove"
        );

        // ---- Every index at this size. Incomplete trees are where a recursion
        // and an iterative walk part company, so the whole triangle runs rather
        // than a sample.
        for leaf_index in 0..tree_size {
            let receipt = anchor.receipt_for(leaf_index).unwrap();

            let (ok, stdout) = run_built_tool(
                &bin,
                &verify_args(&pubkey_hex, leaf_index, tree_size),
                &receipt.to_cose_bytes(),
            );
            assert!(
                ok,
                "go-cose rejected the anchor's receipt for index {leaf_index} \
                 of {tree_size}"
            );
            assert_eq!(
                String::from_utf8(stdout).unwrap(),
                format!("{} {tree_size} {leaf_index}\n", to_hex(&root_of(tree_size))),
                "the Go derivation disagreed with the anchor for index \
                 {leaf_index} of {tree_size}"
            );

            // Go builds the same artifact from the same seed, index and leaves,
            // and the two must be the same bytes.
            let (ok, go_receipt) =
                run_built_tool(&bin, &sign_args(&seed_hex, leaf_index, tree_size), &[]);
            assert!(
                ok,
                "go-cose receipt-sign failed for index {leaf_index} of {tree_size}"
            );
            assert_eq!(
                go_receipt,
                receipt.to_cose_bytes(),
                "go-cose and the anchor built different receipts for index \
                 {leaf_index} of {tree_size} — check the encoders before the crypto"
            );

            if receipt.inclusion_path.len() >= 2 {
                multi_node += 1;
            }
            cases += 1;
        }
    }

    // Count what fired. A loop that ran zero times satisfies every assertion
    // inside it, and a Go gate that never spawned looks exactly like one that
    // passed.
    assert_eq!(submissions, LOG_SIZE - 1, "one submission per size above 1");
    assert_eq!(cases, 77, "sum of n for n in 2..=12");

    // And the sweep reached the shapes where element-wise comparison can
    // discriminate at all: a one-node path is invariant under reordering, so
    // without this the ordering rule would be pinned by nothing.
    assert_eq!(
        multi_node, 72,
        "cases whose inclusion path has two or more nodes"
    );
}
