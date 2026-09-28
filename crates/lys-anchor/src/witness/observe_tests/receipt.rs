#![cfg(test)]
//! The receipt an observation carries is byte-identical whatever the
//! relation.

use super::*;

// ---------------------------------------------------------------------------
// rule: the check never reaches the artifact
// ---------------------------------------------------------------------------

/// One step of the byte-identity script: the bytes offered, the consistency
/// proof offered with them, and the `(size, root)` memory *the test* computes
/// this step must be compared against.
type Step<'a> = (&'a [u8], Option<&'a [u8]>, Option<(u64, [u8; 32])>);

#[test]
fn the_receipt_is_byte_identical_whatever_the_relation() {
    // Two anchors from the same seed and the same genesis, fed the same bytes
    // in the same order. One is driven through the witness path, which computes
    // a relation at every step; the other through the plain submit path, which
    // has never heard of a relation. Every receipt must match byte-for-byte.
    //
    // The relations the seven inputs produce are asserted by the dedicated
    // cases above and are deliberately **not** re-asserted here: a relation
    // drift must fail exactly one case, and this one is about the artifact.
    let witness_dir = TempDir::new().unwrap();
    let plain_dir = TempDir::new().unwrap();
    let mut witness = witness_anchor(witness_dir.path());
    let mut plain = witness_anchor(plain_dir.path());

    let mut child = Child::new(CHILD_ORIGIN);
    child.grow(3);
    let (size_one, root_one) = (child.tree_size(), child.root());
    let first = child.checkpoint();
    let other_root = flip_root(root_one);
    let equivocation = child.checkpoint_stating(size_one, other_root);
    let rollback_root = flip_root(other_root);
    let rolled_back = child.checkpoint_stating(2, rollback_root);
    child.grow(2);
    let (size_two, root_two) = (child.tree_size(), child.root());
    let grown = child.checkpoint();
    let mut forged = child.consistency_from(size_one);
    forged[0] ^= 0xff;
    child.grow(2);
    let grown_again = child.checkpoint();
    let genuine = child.consistency_from(size_two);

    // The script, and the memory each step must find. `expect_previous` is what
    // the *test* computes the witness should be holding — the stated
    // `(size, root)` of the last checkpoint the script recorded — so it is a
    // second party for the memory chain rather than a read-back of it.
    //
    // Coverage is pinned by these two columns and by nothing else. Held against
    // the stated `(size, root)` of each note, they say what each step is:
    // unreadable; a first sighting; a repeat of the memory; the same size with a
    // different root; a smaller size; a larger size with a proof that fails; a
    // larger size with a proof that verifies. **No relation is asserted here**,
    // deliberately — the five relation rules each have exactly one case above,
    // and re-asserting one of them here would leave its drift failing two tests
    // so that neither was the case that proved it. Writing the coverage as
    // memory state instead is what keeps this case blind to every relation
    // drift while still refusing to degrade silently: an earlier draft of this
    // script never reached the same-size-different-root shape at all, and no
    // assertion in it noticed.
    //
    // For a reader checking the coverage claim, the relations the seven shapes
    // below imply are, in order: none (unreadable), none (first sighting),
    // `Identical`, `Conflicting`, `Rollback`, `Unrelated`, `Extends`. They are
    // written here as a derivation to read, not as an assertion to run.
    let script: [Step<'_>; 7] = [
        (b"an ordinary statement", None, None),
        (&first, None, None),
        (&first, None, Some((size_one, root_one))),
        (&equivocation, None, Some((size_one, root_one))),
        (&rolled_back, None, Some((size_one, other_root))),
        (&grown, Some(&forged), Some((2, rollback_root))),
        (&grown_again, Some(&genuine), Some((size_two, root_two))),
    ];
    assert_ne!(root_one, other_root, "the equivocation must differ in root");
    assert_ne!(size_one, size_two, "the extension must differ in size");

    // Positive control, first: the comparison instrument can register a
    // difference. Two receipts from one anchor at two tree sizes must not be
    // byte-equal, or `assert_eq!` on receipt bytes below proves nothing.
    let control_a = plain
        .submit(
            Submission {
                statement: b"control leaf one",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    let control_b = plain
        .submit(
            Submission {
                statement: b"control leaf two",
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    assert_ne!(
        control_a.receipt.to_cose_bytes(),
        control_b.receipt.to_cose_bytes(),
        "positive control: receipt bytes compare equal regardless, so the claim below is vacuous"
    );
    // Keep the two logs in step: the witness gets the same two control leaves.
    for statement in [
        b"control leaf one".as_slice(),
        b"control leaf two".as_slice(),
    ] {
        witness
            .submit(Submission { statement }, SubmitterContext::Unidentified)
            .unwrap();
    }

    let mut compared = 0;
    let mut with_a_memory = 0;
    for (step, (bytes, proof, expect_previous)) in script.into_iter().enumerate() {
        let mut projection = WitnessProjection::rebuild(&witness);
        let witnessed = observe(
            &mut witness,
            &mut projection,
            bytes,
            proof,
            SubmitterContext::Unidentified,
        )
        .unwrap();
        let recorded = plain
            .submit(
                Submission { statement: bytes },
                SubmitterContext::Unidentified,
            )
            .unwrap();
        assert_eq!(
            witnessed.recorded.leaf_index, recorded.leaf_index,
            "the two logs must stay in step for the comparison to mean anything"
        );
        assert_eq!(witnessed.recorded.tree_size, recorded.tree_size);
        assert_eq!(
            witnessed.recorded.receipt.to_cose_bytes(),
            recorded.receipt.to_cose_bytes(),
            "a witness receipt differs from a plain submission's, so witnessing is encodable in the artifact"
        );
        // Coverage, not a relation: the memory this step was compared against.
        // Steps with no expectation are the two that must find none, and they
        // are deliberately left unasserted so this case stays blind to the
        // no-prior-record rule as well.
        if let Some(expected) = expect_previous {
            let previous = witnessed
                .previous
                .as_ref()
                .unwrap_or_else(|| panic!("step {step} must have found a memory"));
            assert_eq!(
                (previous.tree_size, previous.root),
                expected,
                "step {step} was compared against the wrong memory, so it is not the shape this script claims"
            );
            with_a_memory += 1;
        }
        compared += 1;
    }
    assert_eq!(compared, 7, "every scripted input must have been compared");
    assert_eq!(with_a_memory, 5, "five steps must have found a memory");
}
