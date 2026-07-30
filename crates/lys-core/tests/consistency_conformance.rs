//! Conformance gate for consistency-path root derivation: lys's iterative walk
//! against RFC 6962's own **recursive** definitions, transcribed in Go.
//!
//! `merkle::root_from_consistency_path` is hand-written, and a hand-written
//! Merkle walk is the one thing in this crate whose failure mode is silent and
//! total: a walk that disagreed with the RFC would derive roots no anchor ever
//! signed, so every consistency receipt ever issued would be unverifiable, and
//! nothing would reveal it until a stranger tried to check one.
//!
//! The in-crate sweep already requires the walk to agree with `ct-merkle` over
//! every size pair. **That is independence of algorithm and not of much else** —
//! same language, same process, same dependency resolution, and ct-merkle's
//! consistency proof is the source of the path being walked, so a shared
//! misunderstanding of the RFC would go unseen. This gate closes that from a
//! second direction: the Go side builds the tree, the path and both roots from
//! `MTH` / `SUBPROOF` as the recursions the RFC writes, splitting at `k`, then
//! runs §2.1.4.2 itself to derive the newer root.
//!
//! Four values must agree per case, and each catches something different:
//!
//! | value | what disagreement would mean |
//! |---|---|
//! | older root | the two implementations disagree about tree *shape* |
//! | newer root | the same, at the size a receipt would be signed over |
//! | path bytes | ct-merkle's `SUBPROOF` is not the RFC's `SUBPROOF` |
//! | derived root | the two *derivations* disagree even given one proof |
//!
//! The last is the one this gate exists for: it is Go's derivation over Go's own
//! proof, so agreement is not two implementations sharing one input.
//!
//! # Environment contract
//!
//! See [`harness`] — vendored, network-free, and a hard failure rather than a
//! skip when `LYS_REQUIRE_GO` is set.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::{build_go_tool, go_or_skip, run_built_tool};
use lys_core::merkle::root_from_consistency_path;
use lys_core::merkle::tree::{AppendOnlyTree, RawLeaf};

/// Leaf *i* of the shared fixture tree.
fn leaf(index: u64) -> Vec<u8> {
    format!("consistency-leaf-{index}").into_bytes()
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

fn from_hex(text: &str) -> Vec<u8> {
    assert!(
        text.len() % 2 == 0,
        "odd-length hex from the Go tool: {text}"
    );
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex digit"))
        .collect()
}

/// The raw-leaf tree of `size` leaves.
fn tree_of(size: u64) -> AppendOnlyTree<RawLeaf> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for index in 0..size {
        tree.append_raw(&leaf(index));
    }
    tree
}

fn root_of(size: u64) -> [u8; 32] {
    let (root, _size) = tree_of(size).root().to_parts();
    root
}

#[test]
fn the_rfcs_recursive_subproof_derives_the_same_newer_root() {
    let Some(go) = go_or_skip("consistency conformance") else {
        return;
    };
    let cache = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let bin = out.path().join("cosetool");
    build_go_tool(&go, cache.path(), &bin);

    let mut cases = 0;
    let mut prepended = 0;
    for new_size in 2u64..=17 {
        // Leaves are passed as hex so the Go side hashes the same preimages lys
        // does: raw bytes, RFC 6962 `SHA-256(0x00 || leaf)`.
        let mut sizes_and_leaves = vec![String::new(), new_size.to_string()];
        sizes_and_leaves.extend((0..new_size).map(|index| to_hex(&leaf(index))));

        for old_size in 1u64..new_size {
            sizes_and_leaves[0] = old_size.to_string();
            let mut invocation = vec!["consistency".to_string()];
            invocation.extend(sizes_and_leaves.iter().cloned());
            let (ok, stdout) = run_built_tool(&bin, &invocation, b"");
            assert!(
                ok,
                "({old_size} -> {new_size}) go tool failed: {}",
                String::from_utf8_lossy(&stdout)
            );
            let text = String::from_utf8(stdout).unwrap();
            let fields: Vec<&str> = text.split_whitespace().collect();
            assert_eq!(fields.len(), 4, "expected four fields, got {text:?}");

            let (go_first, go_second) = (from_hex(fields[0]), from_hex(fields[1]));
            let (go_path, go_derived) = (from_hex(fields[2]), from_hex(fields[3]));

            // 1 and 2: the two implementations agree about tree shape.
            assert_eq!(
                go_first,
                root_of(old_size).to_vec(),
                "({old_size} -> {new_size}) older root disagrees with ct-merkle"
            );
            assert_eq!(
                go_second,
                root_of(new_size).to_vec(),
                "({old_size} -> {new_size}) newer root disagrees with ct-merkle"
            );

            // 3: ct-merkle's consistency proof IS the RFC's SUBPROOF, byte for
            // byte. If this ever diverges the in-crate sweep would still pass
            // while conforming verifiers rejected every receipt lys issued.
            let ct_path = tree_of(new_size)
                .prove_consistency(old_size, new_size)
                .unwrap()
                .as_bytes()
                .to_vec();
            assert_eq!(
                go_path, ct_path,
                "({old_size} -> {new_size}) SUBPROOF differs from ct-merkle's path"
            );

            // 4: Go's own derivation over Go's own proof.
            assert_eq!(
                go_derived,
                root_of(new_size).to_vec(),
                "({old_size} -> {new_size}) the recursive derivation missed the root"
            );

            // And lys's iterative walk over the Go-built path — so the walk is
            // fed a proof it did not help construct.
            let lys_derived =
                root_from_consistency_path(&root_of(old_size), old_size, new_size, &go_path)
                    .unwrap_or_else(|err| panic!("({old_size} -> {new_size}): {err}"));
            assert_eq!(
                lys_derived.to_vec(),
                go_derived,
                "({old_size} -> {new_size}) iterative and recursive derivations disagree"
            );

            cases += 1;
            prepended += u32::from(old_size.is_power_of_two());
        }
    }
    // A loop that ran zero times would satisfy every assertion inside it, and a
    // gate that skipped the prepend branch would leave RFC step 1 unexercised.
    assert_eq!(cases, 136, "16*17/2 pairs with 1 <= old < new <= 17");
    assert_eq!(prepended, 54, "the power-of-two prepend branch");
}
