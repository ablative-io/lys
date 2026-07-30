#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on newer-root derivation from a consistency path.
//!
//! # The sweep is the reason this walk can be trusted at all
//!
//! `ct-merkle` verifies a consistency proof against two supplied roots and
//! exposes no root-derivation entry point, so RFC 6962 §2.1.4.2 is written out
//! by hand here. A hand-written walk that disagreed with ct-merkle's tree shape
//! would derive roots no anchor ever signed, and **every consistency receipt
//! ever issued would be unverifiable** — the failure would appear only when a
//! stranger tried to check one.
//!
//! So the walk is not reviewed, it is *disagreed with*: every `(old, new)` pair
//! over sizes 1..=17 has its path produced by ct-merkle and its expected newer
//! root produced by ct-merkle, and this walk must land on that root. Two
//! independently written implementations of the same RFC must agree 136 times,
//! and the case count is asserted because a loop that silently ran zero times
//! satisfies every assertion inside it.
//!
//! **Axis of independence, stated rather than implied:** this is independence of
//! *algorithm* — ct-merkle's recursive tree construction against this iterative
//! walk, by different authors. It is not independence of platform or of
//! toolchain; one machine, one dependency resolution. The Go gate in
//! `tests/cose-conformance` adds a third algorithm in another language, and that
//! is where the RFC's recursive `SUBPROOF` definition is exercised directly.

use super::*;
use crate::merkle::{AppendOnlyTree, RawLeaf};

/// The raw-leaf tree of `size` leaves, leaf *i* being `leaf-i`.
fn tree_of(size: u64) -> AppendOnlyTree<RawLeaf> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for leaf in 0..size {
        tree.append_raw(format!("leaf-{leaf}").as_bytes());
    }
    tree
}

/// The root of the raw-leaf tree of `size` leaves.
fn root_of(size: u64) -> [u8; 32] {
    let (root, _size) = tree_of(size).root().to_parts();
    root
}

/// The ct-merkle consistency path between two sizes of the same tree.
fn path_between(old_size: u64, new_size: u64) -> Vec<u8> {
    tree_of(new_size)
        .prove_consistency(old_size, new_size)
        .unwrap()
        .as_bytes()
        .to_vec()
}

#[test]
fn every_size_pair_derives_the_root_ct_merkle_built() {
    let mut swept = 0;
    let mut prepended = 0;
    for new_size in 2u64..=17 {
        let expected = root_of(new_size);
        for old_size in 1u64..new_size {
            let derived = root_from_consistency_path(
                &root_of(old_size),
                old_size,
                new_size,
                &path_between(old_size, new_size),
            )
            .unwrap_or_else(|err| panic!("({old_size} -> {new_size}): {err}"));
            assert_eq!(
                derived, expected,
                "({old_size} -> {new_size}) derived a root ct-merkle did not build"
            );
            swept += 1;
            // The power-of-two branch omits the older root from the proof and
            // the verifier prepends it. Counted so the sweep cannot quietly
            // exercise only one side of that split.
            prepended += u32::from(old_size.is_power_of_two());
        }
    }
    assert_eq!(
        swept, 136,
        "16*17/2 pairs with 1 <= old_size < new_size <= 17"
    );
    // Sizes 1, 2, 4, 8, 16 are powers of two; each is an older size for every
    // larger new_size up to 17: 16 + 15 + 13 + 9 + 1 = 54.
    assert_eq!(prepended, 54, "the prepend branch must be exercised");
    assert_eq!(
        swept - prepended,
        82,
        "and so must the branch that reads the older root out of the path"
    );
}

#[test]
fn a_derived_root_is_rejected_when_the_older_root_is_not_the_callers() {
    // The whole point of the `fr` comparison: a well-formed path for a DIFFERENT
    // history must not silently derive a newer root. Without this check the
    // signature would appear to endorse a chain the caller never held.
    // Power-of-two older sizes are EXCLUDED here and covered by the next test.
    // There the caller's root is the seed, so `fr` is trivially equal to it and
    // this check cannot fire — asserting a refusal there would be asserting a
    // control that cannot fire, which is a different defect from a missing one.
    let mut refused = 0;
    for new_size in 2u64..=9 {
        for old_size in (1u64..new_size).filter(|size| !size.is_power_of_two()) {
            let path = path_between(old_size, new_size);
            let mut wrong = root_of(old_size);
            wrong[0] ^= 0x01;
            let err = root_from_consistency_path(&wrong, old_size, new_size, &path).unwrap_err();
            assert!(
                format!("{err}").contains("does not descend from the supplied older root"),
                "({old_size} -> {new_size}): {err}"
            );
            refused += 1;
        }
    }
    assert_eq!(
        refused, 15,
        "non-power-of-two older sizes below each new_size in 2..=9"
    );
}

#[test]
fn a_power_of_two_older_size_still_binds_the_callers_root() {
    // The `fr` check cannot catch a wrong older root when the older size is a
    // power of two, because the caller's root IS the seed. Authentication is by
    // consequence there instead: the newer root comes out different. Asserted
    // separately so the previous test's `||` cannot hide a real gap.
    let mut cases = 0;
    for new_size in 3u64..=17 {
        for old_size in [1u64, 2, 4, 8, 16] {
            if old_size >= new_size {
                continue;
            }
            let path = path_between(old_size, new_size);
            let honest = root_from_consistency_path(&root_of(old_size), old_size, new_size, &path)
                .expect("honest derivation");
            let mut wrong = root_of(old_size);
            wrong[31] ^= 0x80;
            match root_from_consistency_path(&wrong, old_size, new_size, &path) {
                Ok(derived) => assert_ne!(
                    derived, honest,
                    "({old_size} -> {new_size}) a wrong older root derived the honest newer root"
                ),
                Err(_refused) => {}
            }
            cases += 1;
        }
    }
    assert_eq!(
        cases, 53,
        "every power-of-two older size below each new size"
    );
}

#[test]
fn tampering_with_any_path_node_changes_the_derived_root_or_is_refused() {
    // Authentication by consequence, asserted rather than assumed: no single-bit
    // change to a path may leave the derived root untouched, or a tampered proof
    // would carry a signature that still verified.
    let (old_size, new_size) = (5u64, 13u64);
    let honest_path = path_between(old_size, new_size);
    let old_root = root_of(old_size);
    let honest = root_from_consistency_path(&old_root, old_size, new_size, &honest_path).unwrap();
    assert_eq!(honest, root_of(new_size));

    let mut flipped = 0;
    for byte in 0..honest_path.len() {
        for bit in 0..8u32 {
            let mut path = honest_path.clone();
            path[byte] ^= 1u8 << bit;
            match root_from_consistency_path(&old_root, old_size, new_size, &path) {
                Ok(derived) => assert_ne!(derived, honest, "byte {byte} bit {bit} kept the root"),
                Err(_refused) => {}
            }
            flipped += 1;
        }
    }
    assert!(!honest_path.is_empty(), "the path must have nodes to flip");
    assert_eq!(flipped, honest_path.len() * 8, "every bit flipped");
}

#[test]
fn equal_sizes_are_refused_even_though_the_statement_would_be_true() {
    // Reverses the wire draft's first ruling. With equal sizes the "derived"
    // root is the caller's own argument, so the signature check degenerates to
    // "has this anchor signed this 32-byte value?" with the value entirely
    // attacker-chosen — and nothing but the content type would stand behind it.
    let mut refused = 0;
    for size in 1u64..=17 {
        let err = root_from_consistency_path(&root_of(size), size, size, &[]).unwrap_err();
        assert!(
            format!("{err}").contains("strictly below"),
            "size {size}: {err}"
        );
        refused += 1;
    }
    assert_eq!(refused, 17, "every equal-size pair refused");
}

#[test]
fn a_backwards_or_zero_older_size_is_refused() {
    let err = root_from_consistency_path(&root_of(1), 0, 4, &[]).unwrap_err();
    assert!(format!("{err}").contains("size zero"), "{err}");

    let err = root_from_consistency_path(&root_of(5), 5, 3, &path_between(3, 5)).unwrap_err();
    assert!(format!("{err}").contains("strictly below"), "{err}");
}

/// Sizes with a multi-node path whose older size is not a power of two, so the
/// older root is read out of the path rather than seeded by the caller.
const RAGGED: (u64, u64) = (3, 11);

/// One case per length defect, deliberately NOT bundled into a single
/// wrong-length test. Four checks behind one test name would all fail together,
/// and a drift injection cannot tell a bundled case apart from a specific one —
/// which is the defect this crate's own `pin` hid behind for five injections.
#[test]
fn a_path_that_is_not_whole_digests_is_refused() {
    let (old_size, new_size) = RAGGED;
    let honest = path_between(old_size, new_size);
    let ragged = &honest[..honest.len() - 1];
    let err =
        root_from_consistency_path(&root_of(old_size), old_size, new_size, ragged).unwrap_err();
    assert!(format!("{err}").contains("whole number"), "{err}");
}

#[test]
fn a_path_one_node_short_is_refused() {
    let (old_size, new_size) = RAGGED;
    let honest = path_between(old_size, new_size);
    assert!(honest.len() >= 64, "need at least two nodes to shorten");
    let short = &honest[..honest.len() - 32];
    let err =
        root_from_consistency_path(&root_of(old_size), old_size, new_size, short).unwrap_err();
    assert!(format!("{err}").contains("shorter than"), "{err}");
}

#[test]
fn a_path_one_node_too_long_is_refused() {
    let (old_size, new_size) = RAGGED;
    let mut long = path_between(old_size, new_size);
    long.extend_from_slice(&[0x5a; 32]);
    let err =
        root_from_consistency_path(&root_of(old_size), old_size, new_size, &long).unwrap_err();
    assert!(format!("{err}").contains("longer than"), "{err}");
}

#[test]
fn an_empty_path_is_refused_when_the_older_size_is_not_a_power_of_two() {
    // 3 is not a power of two, so there is no seed to fall back on and the
    // absent first node must be named as such rather than reported as a length
    // mismatch further down.
    let (old_size, new_size) = RAGGED;
    assert!(!old_size.is_power_of_two(), "the branch under test");
    let err = root_from_consistency_path(&root_of(old_size), old_size, new_size, &[]).unwrap_err();
    assert!(format!("{err}").contains("empty"), "{err}");
}

#[test]
fn the_derivation_agrees_with_verify_consistency_on_the_same_inputs() {
    // A second party of a different kind: ct-merkle's own verifier is handed the
    // root this walk derived. If the walk drifted, the value would still be
    // self-consistent here while failing there.
    let mut agreed = 0;
    for new_size in 2u64..=13 {
        for old_size in 1u64..new_size {
            let derived = root_from_consistency_path(
                &root_of(old_size),
                old_size,
                new_size,
                &path_between(old_size, new_size),
            )
            .unwrap();
            let proof = tree_of(new_size)
                .prove_consistency(old_size, new_size)
                .unwrap();
            crate::merkle::verify_consistency(
                &crate::merkle::RootHash::from_parts(root_of(old_size), old_size),
                &crate::merkle::RootHash::from_parts(derived, new_size),
                &proof,
            )
            .unwrap_or_else(|err| panic!("({old_size} -> {new_size}): {err}"));
            agreed += 1;
        }
    }
    assert_eq!(agreed, 78, "12*13/2 pairs with 1 <= old < new <= 13");
}
