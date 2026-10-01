#![cfg(test)]
//! Go refuses the mis-chunked receipts lys accepts.

use super::*;

#[test]
fn go_refuses_the_mis_chunked_receipts_lys_accepts() {
    let Some(go) = go_or_skip("lys-anchor receipt negatives") else {
        return;
    };
    let (_bin_dir, bin) = cose_tool(&go);

    // Grown here rather than in a helper: a helper would have to name
    // `Anchor<..>`'s type parameters, and this file has no business pinning
    // those.
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
    for leaf_index in 1..LOG_SIZE {
        let statement = leaf(leaf_index);
        let outcome = anchor
            .submit(submission(&statement), SubmitterContext::Unidentified)
            .unwrap();
        assert_eq!(
            outcome.leaf_index, leaf_index,
            "submit appended out of order"
        );
    }
    assert_eq!(anchor.tree_size(), LOG_SIZE);

    let anchor_key = anchor.signer().public_key();
    let pubkey_hex = to_hex(&anchor_key);
    let (index, size) = (NEGATIVE_INDEX, LOG_SIZE);
    let statement = leaf(index);

    // ---- Positive control, and it is the FIRST assertion on purpose. Every
    // case below asserts that the Go tool *refused* something, and a verifier
    // that refuses everything satisfies all of them at once. That is not
    // hypothetical in this repository: drifting a Go content-type constant by
    // one character left a conformance test made of refusals entirely green.
    let honest = anchor.receipt_for(index).unwrap();
    let (ok, stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &honest.to_cose_bytes(),
    );
    assert!(
        ok,
        "the control failed: go-cose refused an honest receipt, so no refusal \
         below means anything"
    );
    assert_eq!(
        String::from_utf8(stdout).unwrap(),
        format!("{} {size} {index}\n", to_hex(&root_of(size))),
        "the control failed: go-cose exited 0 without agreeing about the root"
    );
    // And the permutation below is a real one. A one-node path reversed is the
    // same path, and the refusals would then be measuring nothing.
    assert!(
        honest.inclusion_path.len() >= 2,
        "the control failed: a path of {} node(s) cannot be reordered",
        honest.inclusion_path.len()
    );

    let identity = anchor.signer().identity();

    // ---- The hole itself: an anchor that chunked its proof in the wrong ORDER.
    // The receipt is signed over the root that wrong path reconstructs to, so
    // the artifact is internally consistent and describes a tree that does not
    // exist.
    let mut reversed = honest.inclusion_path.clone();
    reversed.reverse();
    assert_ne!(
        reversed, honest.inclusion_path,
        "the reversal must change the path"
    );
    let mis_ordered = sign_receipt(&statement, index, size, &reversed, identity).unwrap();
    assert_ne!(
        mis_ordered.signature, honest.signature,
        "a different path must reconstruct to a different signed root"
    );
    assert!(
        verify_receipt(&mis_ordered, &statement, &anchor_key).is_ok(),
        "lys was expected to ACCEPT its own mis-ordered receipt — it reconstructs \
         the root from the receipt's own path, so it has nothing to disagree \
         with. If this ever fails, lys grew a second party and this gate's \
         premise needs rewriting, not deleting"
    );
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &mis_ordered.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose accepted a receipt whose inclusion path is in the wrong order"
    );

    // ---- The same hole, one flipped bit rather than a permutation. Separated
    // from the reordering because an injection cannot tell two rules apart when
    // one case guards both: a chunker can get the order right and the offset
    // wrong, and vice versa.
    let mut flipped = honest.inclusion_path.clone();
    flipped[0][0] ^= 0x01;
    let mis_hashed = sign_receipt(&statement, index, size, &flipped, identity).unwrap();
    assert!(
        verify_receipt(&mis_hashed, &statement, &anchor_key).is_ok(),
        "lys was expected to ACCEPT its own receipt over a corrupted path node"
    );
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &mis_hashed.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose accepted a receipt carrying a corrupted inclusion-path node"
    );

    // ---- The mirror image, so the two acceptances above are attributable to
    // the anchor having signed the wrong root rather than to `verify_receipt`
    // accepting anything. Here the path is altered *after* signing, and both
    // implementations must refuse.
    let mut tampered = honest.clone();
    tampered.inclusion_path[0][0] ^= 0x01;
    assert!(
        verify_receipt(&tampered, &statement, &anchor_key).is_err(),
        "lys accepted a receipt whose path was altered after signing"
    );
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &tampered.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose accepted a receipt whose path was altered after signing"
    );

    // ---- And the leaves Go is handed are load-bearing: the same honest
    // receipt, offered against a log this verifier does not hold. Without this,
    // every acceptance above could be explained by a tool that ignores its leaf
    // arguments entirely.
    let mut wrong_log = vec!["receipt-verify".to_string(), pubkey_hex, index.to_string()];
    wrong_log.extend((0..size).map(|i| to_hex(&leaf(i + 100))));
    let (ok, _stdout) = run_built_tool(&bin, &wrong_log, &honest.to_cose_bytes());
    assert!(
        !ok,
        "go-cose accepted a receipt against leaves it does not describe"
    );
}
