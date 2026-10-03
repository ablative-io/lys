#![cfg(test)]
//! Gates on [`bundle_for`] — the **producer's** refusals, a cascade of any
//! depth assembled whole, and the shape of what it emits.
//!
//! # What this file is deliberately not
//!
//! It is not the cascade gate. Whether a bundle actually *verifies* is
//! `tests/cascade.rs`'s question, and the judge there is `lys-core`'s
//! `verify_bundle` — code written before this crate existed, by a party that
//! does not know how the cascade was produced. Asserting acceptance here as
//! well would put the interesting claim in the file that also owns the
//! implementation, which is the arrangement where a suite agrees with itself.
//!
//! So the cases below check only what assembly owns: the container's shape, a
//! deep cascade carried whole, and the refusals this crate makes because it is
//! in a position to have caused them.
//!
//! # The duplication that exists, named rather than left to be noticed
//!
//! `verify_bundle` **also** checks the first-link join. Two checks on one rule
//! leave the rule proven by neither, so the two are isolated by construction
//! rather than by hope:
//!
//! - The cases here reach the producer's check and stop — they assert an
//!   [`AnchorError`] variant that `verify_bundle` cannot produce, so a drift in
//!   the judge cannot make them pass or fail.
//! - `tests/cascade.rs` reaches the judge's check by assembling the offending
//!   bundle **by hand**, through `lys-core`'s own constructors, never through
//!   `bundle_for` — so a drift in the producer cannot make *those* pass or
//!   fail either.
//!
//! # One rule per case
//!
//! | rule | the only case that may fail when it is drifted |
//! |---|---|
//! | a cascade of any depth assembles, every link carried | [`a_cascade_a_thousand_links_deep_assembles_whole`] |
//! | the first link must notarize the artifact's own checkpoint | [`a_checkpoint_that_moved_under_the_anchor_is_refused`] |
//! | an index the log lacks is refused, not assembled around | [`an_index_the_log_lacks_is_refused`] |
//! | the container is the frozen shape, with the leaf verbatim | [`the_container_carries_the_frozen_format_and_the_leaf`] |
//! | the time-attestation slot is never populated by production | [`assembly_never_populates_the_counter_anchor`] |
//! | an unnotarized bundle is a bundle, not a failure | [`an_empty_cascade_assembles`] |

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::bundle::VERIFICATION_BUNDLE_FORMAT;

use crate::admission::{AcceptAll, SubmitterContext};
use crate::error::{AnchorError, CascadeError, ProofError};
use crate::wire::Submission;

use super::super::pin::fixture::{Node, STATEMENT, child_with_statement, parent};
use super::super::pin::pin;
use super::*;

/// A child holding [`STATEMENT`], the index it landed at, and one real pin of
/// that child's checkpoint to a real parent.
struct Staged {
    child: Node<AcceptAll>,
    parent: Node<AcceptAll>,
    index: u64,
    pinned: UpwardPin,
}

fn staged() -> Staged {
    let (child, index) = child_with_statement();
    let mut parent = parent();
    let pinned = pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap();
    Staged {
        child,
        parent,
        index,
        pinned,
    }
}

#[test]
fn a_cascade_a_thousand_links_deep_assembles_whole() {
    let staged = staged();

    // The old cap was 32 links. A cascade far past it assembles, and every
    // link is the pin it was handed, in order, with nothing dropped.
    let deep = vec![staged.pinned.clone(); 1_000];
    let accepted = bundle_for(&staged.child.anchor, staged.index, &deep)
        .expect("a cascade of any depth must assemble");
    assert_eq!(accepted.links.len(), deep.len());
    let first = &accepted.links[0];
    assert_eq!(first.checkpoint, staged.pinned.checkpoint.note);
    for link in &accepted.links {
        assert_eq!(link.checkpoint, first.checkpoint);
        assert_eq!(link.receipt, first.receipt);
    }
}

#[test]
fn a_checkpoint_that_moved_under_the_anchor_is_refused() {
    let mut staged = staged();

    // Positive control: at this instant the pin and the artifact agree, so the
    // assembly below fails for the append and not for a join that never held.
    bundle_for(
        &staged.child.anchor,
        staged.index,
        std::slice::from_ref(&staged.pinned),
    )
    .expect("the pin and the artifact agree before anything is appended");

    // One append, and the child's next inclusion artifact carries a checkpoint
    // at a larger size than the one the parent notarized.
    staged
        .child
        .anchor
        .append(
            Submission {
                statement: b"a statement admitted between the pin and the bundle",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();

    let refused = bundle_for(
        &staged.child.anchor,
        staged.index,
        std::slice::from_ref(&staged.pinned),
    );
    match refused {
        Err(AnchorError::Cascade(CascadeError::CascadeJoinMismatch {
            pinned_tree_size,
            artifact_tree_size,
            leaf_index,
            ..
        })) => {
            assert_eq!(leaf_index, staged.index);
            assert_eq!(pinned_tree_size, staged.pinned.checkpoint.body.tree_size());
            assert_eq!(artifact_tree_size, staged.child.anchor.tree_size());
            assert!(pinned_tree_size < artifact_tree_size);
        }
        other => panic!("expected CascadeJoinMismatch, got {other:?}"),
    }
}

#[test]
fn an_index_the_log_lacks_is_refused() {
    let staged = staged();
    let past_the_end = staged.child.anchor.tree_size();

    // Positive control: an index the log *does* have assembles, so the refusal
    // below is about the index and not about the chain beside it.
    bundle_for(
        &staged.child.anchor,
        staged.index,
        std::slice::from_ref(&staged.pinned),
    )
    .expect("an in-range index assembles");

    let refused = bundle_for(&staged.child.anchor, past_the_end, &[]);
    match refused {
        Err(AnchorError::Proof(ProofError::NoSuchLeaf {
            leaf_index,
            tree_size,
            ..
        })) => {
            assert_eq!(leaf_index, past_the_end);
            assert_eq!(tree_size, staged.child.anchor.tree_size());
        }
        other => panic!("expected NoSuchLeaf, got {other:?}"),
    }
}

#[test]
fn the_container_carries_the_frozen_format_and_the_leaf() {
    let staged = staged();
    let bundle = bundle_for(
        &staged.child.anchor,
        staged.index,
        std::slice::from_ref(&staged.pinned),
    )
    .unwrap();

    assert_eq!(bundle.format, VERIFICATION_BUNDLE_FORMAT);
    // Keyed on the bytes this suite supplied to the anchor, decoded back out of
    // the container by a base64 decoder this file drives itself.
    let carried = STANDARD.decode(&bundle.leaf).unwrap();
    assert_eq!(carried, STATEMENT);

    assert_eq!(bundle.links.len(), 1);
    assert_eq!(bundle.links[0].checkpoint, staged.pinned.checkpoint.note);
    assert_eq!(
        STANDARD.decode(&bundle.links[0].receipt).unwrap(),
        staged.pinned.recorded.receipt.to_cose_bytes()
    );
    // The join the judge requires, present in what was produced.
    assert_eq!(
        bundle.links[0].checkpoint,
        bundle.inclusion_proof.checkpoint
    );
    // And the parent really is the notarizer, so the link above is not the
    // child notarizing itself.
    assert_ne!(staged.parent.origin(), staged.child.origin());
}

#[test]
fn assembly_never_populates_the_counter_anchor() {
    let staged = staged();
    // Both shapes, because a slot left empty only in the interesting case is a
    // slot that can be filled in the dull one.
    let empty: &[UpwardPin] = &[];
    let mut checked = 0;
    for chain in [std::slice::from_ref(&staged.pinned), empty] {
        let bundle = bundle_for(&staged.child.anchor, staged.index, chain).unwrap();
        assert!(
            bundle.counter_anchor.is_none(),
            "a populated slot is refused by the verifier until something can \
             check a time attestation, so production must never fill it"
        );
        checked += 1;
    }
    // Count what fired: a loop over nothing satisfies every assertion inside it.
    assert_eq!(checked, 2);
}

#[test]
fn an_empty_cascade_assembles() {
    let staged = staged();
    let bundle = bundle_for(&staged.child.anchor, staged.index, &[]).unwrap();
    assert!(
        bundle.links.is_empty(),
        "an empty chain means the leaf is in a log nobody notarized — a weaker \
         claim, not an invalid bundle"
    );
    assert_eq!(bundle.format, VERIFICATION_BUNDLE_FORMAT);
}
