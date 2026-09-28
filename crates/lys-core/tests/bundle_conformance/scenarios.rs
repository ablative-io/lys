#![cfg(test)]
//! The logs and chains every case is cut from: anchors notarizing a child
//! log, chains of any depth, and the divergent, equivocating, relabelled and
//! unrelated scenarios.

use super::*;

/// An anchor that has notarized `notarized`, appended after a genesis leaf so
/// its tree size is never 1 — the receipt format cannot express size 1.
pub fn anchor_over(origin: &str, seed: &[u8; 32], notarized: &[u8]) -> Party {
    let mut anchor = Party::new(origin, seed);
    anchor.tree.append_raw(b"genesis");
    anchor.tree.append_raw(notarized);
    anchor
}

/// The index the notarized checkpoint sits at in every anchor built above.
pub const NOTARIZED_INDEX: u64 = 1;

pub const CHILD_LEAF: &[u8] = b"entry-1";

pub const CHILD_LEAF_INDEX: u64 = 1;

/// A log with three entries, the middle one to be proven.
pub fn log_with_entries(origin: &str, seed: &[u8; 32]) -> Party {
    let mut log = Party::new(origin, seed);
    for leaf in [b"entry-0".as_slice(), CHILD_LEAF, b"entry-2".as_slice()] {
        log.tree.append_raw(leaf);
    }
    log
}

/// A log, plus `depth` anchors each notarizing the one below it.
///
/// `depth` 0 is an unnotarized bundle — a leaf proven in a log nobody has
/// countersigned, which is weaker but perfectly valid.
pub fn chain(depth: usize) -> Scenario {
    let log = log_with_entries("child.example", b"lys-bundle-conformance-child-01a");
    let artifact = build_inclusion_artifact(
        &log.tree,
        CHILD_LEAF,
        &log.origin,
        &log.identity,
        CHILD_LEAF_INDEX,
    )
    .unwrap();

    let mut links = Vec::new();
    let mut anchors = Vec::new();
    // Each anchor notarizes the checkpoint below it: the log's own for the
    // first, then each anchor's own published checkpoint in turn.
    let mut notarized = artifact.checkpoint.clone();
    for level in 0..depth {
        let anchor = anchor_over(
            &format!("anchor-{level}.example"),
            &anchor_seed(level),
            notarized.as_bytes(),
        );
        let receipt = anchor.receipt_over(notarized.as_bytes(), NOTARIZED_INDEX);
        links.push(BundleLink::new(&notarized, &receipt));
        anchors.push(anchor.spec());
        notarized = anchor.checkpoint();
    }

    Scenario {
        bundle: VerificationBundle::new(CHILD_LEAF, artifact, links),
        log_key: log.spec(),
        anchors,
    }
}

/// Distinct 32-byte seeds per chain level, checked for length at the source so
/// a mistyped literal cannot silently become a different key.
pub fn anchor_seed(level: usize) -> [u8; 32] {
    let seed = format!("lys-bundle-conformance-anchor-{level:02}");
    let bytes = seed.into_bytes();
    assert_eq!(bytes.len(), 32, "anchor seed must be exactly 32 bytes");
    bytes.try_into().unwrap()
}

/// A log and its inclusion artifact, the starting point every scenario shares.
pub fn child_and_artifact() -> (Party, lys_core::tlog::InclusionProofArtifact) {
    let log = log_with_entries("child.example", b"lys-bundle-conformance-child-01a");
    let artifact = build_inclusion_artifact(
        &log.tree,
        CHILD_LEAF,
        &log.origin,
        &log.identity,
        CHILD_LEAF_INDEX,
    )
    .unwrap();
    (log, artifact)
}

/// A two-link chain in which anchor 0 grew **after** issuing its receipt and
/// then published a checkpoint that receipt never vouched for.
///
/// Every artifact here is individually valid: the receipt verifies over the
/// log's checkpoint, anchor 0's later checkpoint carries anchor 0's own
/// signature over its own origin, and anchor 1's receipt validly notarizes that
/// checkpoint. Only the *rung* is broken — the root anchor 0 vouched for is not
/// the root anchor 0 published — which is the case that lands squarely on the
/// root comparison rather than tripping a signature check on the way there.
///
/// This is the "history moved between vouching and publishing" attack, and it is
/// the one a chain of individually-valid receipts cannot detect without the
/// comparison.
pub fn divergent_rung() -> Scenario {
    let (log, artifact) = child_and_artifact();
    let log_note = artifact.checkpoint.clone();

    let mut anchor_0 = anchor_over("anchor-0.example", &anchor_seed(0), log_note.as_bytes());
    let receipt_0 = anchor_0.receipt_over(log_note.as_bytes(), NOTARIZED_INDEX);
    anchor_0
        .tree
        .append_raw(b"appended-after-the-receipt-was-issued");
    let diverged_note = anchor_0.checkpoint();

    let anchor_1 = anchor_over(
        "anchor-1.example",
        &anchor_seed(1),
        diverged_note.as_bytes(),
    );
    let receipt_1 = anchor_1.receipt_over(diverged_note.as_bytes(), NOTARIZED_INDEX);

    Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            artifact,
            vec![
                BundleLink::new(&log_note, &receipt_0),
                BundleLink::new(&diverged_note, &receipt_1),
            ],
        ),
        log_key: log.spec(),
        anchors: vec![anchor_0.spec(), anchor_1.spec()],
    }
}

/// A two-link chain in which anchor 0 **equivocates**: it vouches for one root
/// in its receipt and publishes a different root, at the same tree size, in its
/// own checkpoint.
///
/// This is the case that isolates the rung's *root* comparison. Growth after
/// issuing a receipt (see [`divergent_rung`]) changes the tree size too, so the
/// size comparison catches it either way; equivocation at an unchanged size is
/// caught by nothing but the root check. Both checkpoints here carry anchor 0's
/// genuine signature over its own origin, and both are size 2 — the only
/// disagreement is which history the anchor is describing, which is exactly the
/// dishonesty a notarization chain exists to expose.
pub fn equivocating_anchor() -> Scenario {
    let (log, artifact) = child_and_artifact();
    let log_note = artifact.checkpoint.clone();

    // The tree anchor 0 actually vouched for.
    let vouched = anchor_over("anchor-0.example", &anchor_seed(0), log_note.as_bytes());
    let receipt_0 = vouched.receipt_over(log_note.as_bytes(), NOTARIZED_INDEX);

    // A second history under the SAME key and origin, at the same size.
    let published = anchor_over(
        "anchor-0.example",
        &anchor_seed(0),
        b"a-second-entry-that-was-never-notarized",
    );
    let published_note = published.checkpoint();

    let anchor_1 = anchor_over(
        "anchor-1.example",
        &anchor_seed(1),
        published_note.as_bytes(),
    );
    let receipt_1 = anchor_1.receipt_over(published_note.as_bytes(), NOTARIZED_INDEX);

    Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            artifact,
            vec![
                BundleLink::new(&log_note, &receipt_0),
                BundleLink::new(&published_note, &receipt_1),
            ],
        ),
        log_key: log.spec(),
        anchors: vec![vouched.spec(), anchor_1.spec()],
    }
}

/// A chain whose first receipt **relabels its tree size**, exercising the one
/// half of the rung check that nothing else reaches.
///
/// The receipt format's `tree_size` is authenticated only as far as it changes
/// the reconstruction, and sizes 3 and 4 share a decision sequence at index 1 —
/// so a receipt carrying a size-4 path while claiming size 3 reconstructs the
/// same root and carries a valid signature. That malleability is documented in
/// `receipt::sign`, and it is a property of the RFC's proof format rather than
/// of either implementation, which is why both sides here accept the relabelled
/// receipt in isolation.
///
/// The bundle is what discharges it: the anchor's own note-signed checkpoint
/// says size 4, and the rung requires the two signed statements to agree. With
/// `links` truncated to one there is nothing to compare against, and the lie
/// survives — the honest limit, pinned below as an accepted case rather than
/// left as a claim.
pub fn relabelled_tree_size(with_second_link: bool) -> Scenario {
    let (log, artifact) = child_and_artifact();
    let log_note = artifact.checkpoint.clone();

    let mut anchor_0 = Party::new("anchor-0.example", &anchor_seed(0));
    anchor_0.tree.append_raw(b"genesis");
    anchor_0.tree.append_raw(log_note.as_bytes());
    anchor_0.tree.append_raw(b"later-0");
    anchor_0.tree.append_raw(b"later-1");

    let proof = anchor_0.tree.prove_inclusion(NOTARIZED_INDEX).unwrap();
    let path: Vec<[u8; 32]> = proof
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect();
    // Claiming 3 while carrying the size-4 path: same walk, same root, valid
    // signature.
    let relabelled = sign_receipt(
        log_note.as_bytes(),
        NOTARIZED_INDEX,
        3,
        &path,
        &anchor_0.identity,
    )
    .unwrap()
    .to_cose_bytes();

    let mut links = vec![BundleLink::new(&log_note, &relabelled)];
    let mut anchors = vec![anchor_0.spec()];
    if with_second_link {
        let a0_note = anchor_0.checkpoint();
        let anchor_1 = anchor_over("anchor-1.example", &anchor_seed(1), a0_note.as_bytes());
        links.push(BundleLink::new(
            &a0_note,
            &anchor_1.receipt_over(a0_note.as_bytes(), NOTARIZED_INDEX),
        ));
        anchors.push(anchor_1.spec());
    }

    Scenario {
        bundle: VerificationBundle::new(CHILD_LEAF, artifact, links),
        log_key: log.spec(),
        anchors,
    }
}

/// An entirely separate log-and-anchor pair, used to build the splice attacks:
/// artifacts that are individually valid but about the wrong log.
pub fn unrelated() -> Scenario {
    let log = log_with_entries("other.example", b"lys-bundle-conformance-other-01a");
    let artifact = build_inclusion_artifact(
        &log.tree,
        CHILD_LEAF,
        &log.origin,
        &log.identity,
        CHILD_LEAF_INDEX,
    )
    .unwrap();
    let checkpoint = artifact.checkpoint.clone();
    let anchor = anchor_over(
        "anchor-x.example",
        b"lys-bundle-conformance-anchor-xx",
        checkpoint.as_bytes(),
    );
    let receipt = anchor.receipt_over(checkpoint.as_bytes(), NOTARIZED_INDEX);

    Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            artifact,
            vec![BundleLink::new(&checkpoint, &receipt)],
        ),
        log_key: log.spec(),
        anchors: vec![anchor.spec()],
    }
}
