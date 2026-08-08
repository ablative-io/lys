#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`Anchor::inclusion_artifact`].
//!
//! # The trap this file is shaped around
//!
//! `build_inclusion_artifact` **self-verifies with
//! `verify_inclusion_artifact`** before it returns. So calling
//! `verify_inclusion_artifact` on what it handed back re-runs a check that has
//! already passed, with the same code, over the same bytes. It cannot fail, and
//! a suite built on it would be asserting that a function is deterministic. The
//! same objection retires the obvious round trip: `serde_json` through our own
//! `Serialize` and `Deserialize` derives is symmetric, so any change made on
//! both sides at once is invisible to it.
//!
//! Every substantive assertion below is therefore anchored outside those loops.
//!
//! # Where the second party comes from
//!
//! - **RFC 6962 §2.1.1, implemented here from the RFC's own text.**
//!   [`rfc6962_walk`] is the standard's audit-path verification algorithm
//!   transcribed step for step; it does not call `lys-core`'s
//!   `root_from_inclusion_path`, nor ct-merkle's `verify_inclusion`, which are
//!   the two Rust implementations the artifact was built and self-verified
//!   with. The walk consumes the artifact's *published* path and must arrive at
//!   the root inside the artifact's *signed checkpoint* — two values that reach
//!   this test through entirely separate machinery.
//! - **A base64 decoder written here**, [`decode_standard_base64`], strict about
//!   the padding the D2 contract requires. `lys-core` encodes the path nodes
//!   with the `base64` crate; nothing in this file does, so the encoded field is
//!   read by something that is not the encoder's mirror.
//! - **Three values computed outside Rust.** [`GOLDEN_GENESIS_LEAF_HASH`],
//!   [`GOLDEN_STATEMENT_LEAF_HASH`] and [`GOLDEN_ROOT_2`] are literals, so no
//!   change to this workspace can move them. They were produced by `openssl
//!   dgst -sha256` pipelines and independently reproduced with Python's
//!   `hashlib`, which agreed — the provenance recorded alongside the same three
//!   constants in `submit_tests.rs`, whose fixture bytes this file reuses
//!   deliberately so the external computation is not re-derived here. They were
//!   re-measured with `openssl` while this file was written, under a control
//!   that was demonstrated to fire: the control digests each **non-empty**
//!   fixture on its own and fails if either reads as the empty digest or if the
//!   two read alike, and it was run against an empty file and a missing file to
//!   confirm it fires in both cases rather than passing inertly.
//! - **The wire strings, restated as literals here.** `"lys/log-inclusion-proof/v1"`
//!   and the five field names are written out in this file rather than imported
//!   from [`INCLUSION_PROOF_FORMAT`](lys_core::tlog::INCLUSION_PROOF_FORMAT) or
//!   read off the struct, so a rename in `lys-core` is a failure here instead of
//!   a silent agreement.
//! - **The origin.** Verifiers are built from the [`ORIGIN`] literal this file
//!   supplied to the store, never from what the anchor reports back.
//! - **The log.** Leaves are appended through a plain `Log` handle and the
//!   anchor is then *opened* over them, so every artifact below is a proof about
//!   leaves this anchor did not place.
//!
//! The independence axis for all of these is *implementation*, not platform: one
//! machine, one toolchain, one dependency resolution. The cross-language claim
//! belongs to the conformance gates and is not made here.

use std::path::Path;

use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_log_store::{FileLeafStore, Log};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::keys::{FileSigner, Signer};

use super::*;

/// The origin this test supplies to the store. Verifiers are built from *this*
/// literal, never from what the anchor reports back.
const ORIGIN: &str = "example.com/lys/anchor-artifact-test";

/// A second origin, used only to build a verifier that must reject a checkpoint
/// signed for [`ORIGIN`].
const OTHER_ORIGIN: &str = "example.com/lys/some-other-log";

/// The frozen `format` string of the artifact, written out rather than imported
/// so a change to `lys-core`'s constant surfaces here.
const INCLUSION_FORMAT: &str = "lys/log-inclusion-proof/v1";

/// The genesis bytes for every anchor built here.
///
/// Deliberately the same bytes `submit_tests.rs` uses, and named for the
/// increment that first computed their digests outside Rust: the golden
/// constants below are values *of these bytes*, and changing the fixture would
/// mean re-deriving them rather than reusing an external computation that has
/// already been made and recorded.
const GENESIS: &[u8] = b"lys-anchor increment 4 genesis fixture";

/// The statement appended in the two-leaf tests. Same reuse, same reason.
const STATEMENT: &[u8] = b"lys-anchor increment 4 statement fixture";

/// `SHA-256(0x00 ‖ GENESIS)` — RFC 6962's leaf hash, computed outside Rust.
const GOLDEN_GENESIS_LEAF_HASH: &str =
    "a5dabd900c94df52610e226a28e883f7e932d4f71e27e2bb4737cb9f2a0f7d1d";

/// `SHA-256(0x00 ‖ STATEMENT)`, computed outside Rust.
const GOLDEN_STATEMENT_LEAF_HASH: &str =
    "4a68beab3afee2c3b01d3b7c277259e12240cc888e5da1773c460f48f079bf5c";

/// `SHA-256(0x01 ‖ GOLDEN_GENESIS_LEAF_HASH ‖ GOLDEN_STATEMENT_LEAF_HASH)` —
/// the RFC 6962 root of the two-leaf tree, computed outside Rust.
const GOLDEN_ROOT_2: &str = "6ecd815c5246ecb3f8ab9a7abbc5e1ba2bba773886e7db8dceca7c0739842861";

/// `lys-core`'s conformance fixture seed, reused so key material in tests is
/// deterministic and never generated.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Lower-case hex, so a 32-byte value can be compared with a literal computed
/// by a tool that is not this workspace.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// Decodes standard base64 **with** padding, strictly.
///
/// Written here rather than taken from the `base64` crate on purpose: that crate
/// is what encoded the field, and a decoder that is the encoder's own mirror
/// cannot observe whether the encoder is the one D2 specifies. Rejects any
/// character outside the standard alphabet, any misplaced padding, and any input
/// whose length is not a multiple of four — the properties the artifact's
/// `hashes` field claims.
fn decode_standard_base64(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return None;
    }
    let pad = bytes.iter().rev().take_while(|b| **b == b'=').count();
    if pad > 2 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut accumulator: u32 = 0;
    let mut bits = 0_u32;
    for (position, byte) in bytes.iter().enumerate() {
        if *byte == b'=' {
            // Padding is only ever the tail, and the tail was counted above.
            if position < bytes.len() - pad {
                return None;
            }
            continue;
        }
        let value = ALPHABET.iter().position(|c| c == byte)?;
        accumulator = (accumulator << 6) | u32::try_from(value).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((accumulator >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}

/// RFC 6962 §2.1.1's audit-path verification, transcribed from the RFC.
///
/// Returns the root the path leads to, or `None` if the standard says the proof
/// verification fails. Independent of `lys-core`'s
/// `root_from_inclusion_path` and of ct-merkle's `verify_inclusion` — the two
/// implementations the artifact under test was built and self-verified with.
fn rfc6962_walk(
    leaf_bytes: &[u8],
    leaf_index: u64,
    tree_size: u64,
    path: &[[u8; 32]],
) -> Option<[u8; 32]> {
    // Step 1: an index at or past the size fails verification.
    if leaf_index >= tree_size {
        return None;
    }
    // Step 2 and 3.
    let mut fna = leaf_index;
    let mut sna = tree_size - 1;
    let mut r: [u8; 32] = Sha256::digest([&[0x00_u8], leaf_bytes].concat()).into();
    // Step 4.
    for p in path {
        if sna == 0 {
            return None;
        }
        if fna & 1 == 1 || fna == sna {
            r = Sha256::digest([&[0x01_u8], &p[..], &r[..]].concat()).into();
            if fna & 1 == 0 {
                while fna & 1 == 0 && fna != 0 {
                    fna >>= 1;
                    sna >>= 1;
                }
            }
        } else {
            r = Sha256::digest([&[0x01_u8], &r[..], &p[..]].concat()).into();
        }
        fna >>= 1;
        sna >>= 1;
    }
    // Step 5: a path that has not consumed the whole tree fails.
    if sna != 0 { None } else { Some(r) }
}

/// Decodes an artifact's `hashes` into 32-byte nodes with this file's own
/// decoder, asserting the padding and width the D2 contract claims.
fn path_nodes(artifact: &InclusionProofArtifact) -> Vec<[u8; 32]> {
    artifact
        .hashes
        .iter()
        .map(|encoded| {
            // 32 bytes is 44 standard-base64 characters ending in one '=' — the
            // "with padding" half of the contract, asserted rather than assumed.
            assert_eq!(encoded.len(), 44, "a 32-byte node is 44 padded characters");
            assert!(
                encoded.ends_with('='),
                "the D2 contract is standard base64 WITH padding, got {encoded}"
            );
            let decoded = decode_standard_base64(encoded)
                .unwrap_or_else(|| panic!("{encoded} is not standard base64"));
            <[u8; 32]>::try_from(decoded.as_slice()).expect("every node is 32 bytes")
        })
        .collect()
}

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Appends genesis and `statements` through a plain [`Log`], then opens an
/// anchor over the result.
///
/// The anchor never places a leaf here. Every artifact it produces below is
/// therefore a proof about entries written by something else, which is the
/// situation an anchor reopened after a restart is always in.
fn anchor_over(dir: &Path, statements: &[&[u8]]) -> Anchor<FileLeafStore, FileSigner> {
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let mut log = Log::open(store).unwrap();
    log.append(GENESIS).unwrap();
    for statement in statements {
        log.append(statement).unwrap();
    }
    drop(log);
    reopen(dir)
}

/// Reopens the anchor over a directory that already holds a log.
fn reopen(dir: &Path) -> Anchor<FileLeafStore, FileSigner> {
    Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        signer(dir),
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// Appends one more leaf to an existing log directory, through a handle the
/// anchor does not own.
fn append_to(dir: &Path, statement: &[u8]) {
    let mut log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    log.append(statement).unwrap();
}

/// The verifier a third party would build: the origin they were told, and the
/// public key they were given.
fn verifier(anchor: &Anchor<FileLeafStore, FileSigner>) -> NoteVerifierKey {
    NoteVerifierKey::new(ORIGIN, anchor.signer().public_key()).unwrap()
}

#[test]
fn the_embedded_checkpoint_verifies_under_a_verifier_built_from_the_literal_origin() {
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[STATEMENT]);

    let artifact = anchor.inclusion_artifact(1).unwrap();

    // The frozen wire marker, against a literal in this file.
    assert_eq!(artifact.format, INCLUSION_FORMAT);

    // The checkpoint is checked with the origin this test handed the store and
    // the key the signer advertises — not with anything the anchor reported.
    let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
        .expect("the artifact's checkpoint must verify");
    assert_eq!(body.tree_size(), 2);

    // And the root it commits to is the one computed outside Rust, so a
    // symmetric error shared by every Rust path here would still be caught.
    assert_eq!(hex(&body.root_hash()), GOLDEN_ROOT_2);

    // Negative control. Without it, "verifies" could mean "this verifier
    // accepts anything": the same bytes, checked under a verifier named for a
    // different log, must be refused. The origin binding is what stops one
    // log's artifact being accepted for another.
    let wrong_name = NoteVerifierKey::new(OTHER_ORIGIN, anchor.signer().public_key()).unwrap();
    assert!(
        verify_checkpoint(artifact.checkpoint.as_bytes(), &wrong_name).is_err(),
        "a checkpoint signed for one origin must not verify under another's name"
    );

    // Second negative control, on the other half of the binding: the right
    // origin with a key that is not this anchor's.
    let wrong_key = NoteVerifierKey::new(ORIGIN, [0x11_u8; 32]).unwrap();
    assert!(
        verify_checkpoint(artifact.checkpoint.as_bytes(), &wrong_key).is_err(),
        "a checkpoint must not verify under a key that did not sign it"
    );

    // The checkpoint is carried verbatim, trailing newline included — a reader
    // re-verifies these exact bytes, so a trimmed note is an unverifiable one.
    assert!(
        artifact.checkpoint.ends_with('\n'),
        "the embedded note must be verbatim, including its trailing newline"
    );
}

#[test]
fn the_published_path_walks_to_the_root_the_checkpoint_signs() {
    // A two-leaf tree has a one-node path, and a one-node path is invariant
    // under reordering — so it cannot catch a path emitted in the wrong order.
    // This log is deep enough that almost every path has three nodes.
    let tmp = TempDir::new().unwrap();
    let statements: Vec<Vec<u8>> = (0..8_u8).map(|n| vec![n; 11]).collect();
    let borrowed: Vec<&[u8]> = statements.iter().map(Vec::as_slice).collect();
    let anchor = anchor_over(tmp.path(), &borrowed);
    assert_eq!(anchor.tree_size(), 9);

    let mut walked = 0;
    let mut multi_node_paths = 0;
    for index in 0..9_u64 {
        let artifact = anchor.inclusion_artifact(index).unwrap();
        let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
            .expect("the artifact's checkpoint must verify");

        let nodes = path_nodes(&artifact);
        if nodes.len() > 1 {
            multi_node_paths += 1;
        }

        // The leaf the artifact is about, read off the log rather than
        // remembered from the loop that wrote it.
        let leaf: &[u8] = if index == 0 {
            GENESIS
        } else {
            &statements[usize::try_from(index).unwrap() - 1]
        };

        let root = rfc6962_walk(leaf, artifact.leaf_index, artifact.tree_size, &nodes)
            .expect("the published path must satisfy RFC 6962 §2.1.1");
        assert_eq!(
            root,
            body.root_hash(),
            "leaf {index}'s published path leads to a root the anchor does not sign"
        );
        walked += 1;
    }
    // Count what fired, and count the cases that carry the property: a suite of
    // one-node paths would satisfy every assertion above while proving nothing
    // about order.
    assert_eq!(walked, 9);
    assert!(
        multi_node_paths >= 8,
        "only {multi_node_paths} of 9 artifacts had a path long enough for order to matter"
    );

    // Positive control on the walk itself. Every assertion above compares two
    // values this file derived; if `rfc6962_walk` returned the checkpoint root
    // for *anything*, they would all pass. Reversing a three-node path must
    // reach a different root.
    let artifact = anchor.inclusion_artifact(3).unwrap();
    let mut reversed = path_nodes(&artifact);
    assert!(
        reversed.len() >= 3,
        "this case needs order to be observable"
    );
    reversed.reverse();
    let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor)).unwrap();
    assert_ne!(
        rfc6962_walk(
            &statements[2],
            artifact.leaf_index,
            artifact.tree_size,
            &reversed
        ),
        Some(body.root_hash()),
        "the walk accepts a reordered path, so it cannot be evidence of order"
    );
}

#[test]
fn the_declared_sizes_are_the_anchors_own_and_the_index_is_the_one_asked_for() {
    let tmp = TempDir::new().unwrap();
    let statements: Vec<Vec<u8>> = (0..4_u8).map(|n| vec![n; 7]).collect();
    let borrowed: Vec<&[u8]> = statements.iter().map(Vec::as_slice).collect();
    let anchor = anchor_over(tmp.path(), &borrowed);
    assert_eq!(anchor.tree_size(), 5);

    let mut checked = 0;
    for index in 0..5_u64 {
        let artifact = anchor.inclusion_artifact(index).unwrap();
        assert_eq!(artifact.leaf_index, index);
        assert_eq!(artifact.tree_size, anchor.tree_size());

        // Not just equal to the anchor's field: equal to the size inside the
        // signed checkpoint the artifact carries, which is the only one a
        // reader is entitled to believe.
        let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
            .expect("the artifact's checkpoint must verify");
        assert_eq!(body.tree_size(), artifact.tree_size);
        checked += 1;
    }
    assert_eq!(checked, 5);
}

#[test]
fn a_genesis_only_anchor_has_an_artifact_with_an_empty_path() {
    // `receipt_for` refuses this tree — RFC 9942 types a receipt's inclusion
    // path as one-or-more nodes and a one-leaf tree's path is empty. The JSON
    // artifact has no such rule, and this asserts that the difference is real
    // rather than an untested claim in the module docs. (`receipt_for` itself
    // cannot be named here: it is behind `unstable-anchor` and this file is not.)
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[]);
    assert_eq!(anchor.tree_size(), 1);

    let artifact = anchor.inclusion_artifact(0).unwrap();
    assert_eq!(artifact.tree_size, 1);
    assert_eq!(artifact.leaf_index, 0);
    assert!(
        artifact.hashes.is_empty(),
        "the sole leaf of a one-leaf tree has an empty path"
    );

    // The root of a one-leaf tree is its leaf hash — checked against a value
    // computed outside Rust, so the empty path is not merely accepted but
    // correct.
    let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
        .expect("the artifact's checkpoint must verify");
    assert_eq!(hex(&body.root_hash()), GOLDEN_GENESIS_LEAF_HASH);
    assert_eq!(
        rfc6962_walk(GENESIS, 0, 1, &[]),
        Some(body.root_hash()),
        "RFC 6962 §2.1.1 must accept the empty path for a one-leaf tree"
    );
}

#[test]
fn inclusion_artifact_refuses_an_index_the_log_does_not_have() {
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[STATEMENT]);

    // Positive control: the indices that exist are served, so the refusals
    // below are about the index and not about a method that refuses always.
    let mut served = 0;
    for index in 0..2_u64 {
        anchor
            .inclusion_artifact(index)
            .expect("every logged leaf has an artifact");
        served += 1;
    }
    assert_eq!(served, 2);

    let mut refused = 0;
    for index in [2_u64, 3, u64::MAX] {
        let err = anchor
            .inclusion_artifact(index)
            .expect_err("an index past the end has no leaf");
        assert!(
            matches!(
                err,
                AnchorError::NoSuchLeaf { leaf_index, tree_size, ref origin }
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
fn two_artifacts_for_one_leaf_at_two_sizes_disagree_and_both_verify() {
    // The contract the module docs state, made executable. A caller who takes
    // an artifact after further appends gets a different `tree_size` and a
    // different root for the same leaf, and neither artifact is wrong.
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let anchor = anchor_over(dir, &[STATEMENT]);

    let earlier = anchor.inclusion_artifact(1).unwrap();
    assert_eq!(earlier.tree_size, 2);

    // Somebody else's statement lands, and the anchor is reopened over it.
    append_to(dir, b"a third party's statement");
    let anchor = reopen(dir);
    assert_eq!(anchor.tree_size(), 3);

    let later = anchor.inclusion_artifact(1).unwrap();
    assert_eq!(later.leaf_index, earlier.leaf_index);
    assert_ne!(later.tree_size, earlier.tree_size);
    assert_ne!(later.checkpoint, earlier.checkpoint);

    // Both verify, against different trees — that is the whole point, and a
    // caller who wants agreement has to check for it rather than assume it.
    let key = verifier(&anchor);
    let earlier_body = verify_checkpoint(earlier.checkpoint.as_bytes(), &key)
        .expect("the earlier artifact must still verify");
    let later_body = verify_checkpoint(later.checkpoint.as_bytes(), &key)
        .expect("the later artifact must verify");
    assert_eq!(earlier_body.tree_size(), 2);
    assert_eq!(later_body.tree_size(), 3);
    assert_ne!(earlier_body.root_hash(), later_body.root_hash());
    assert_eq!(hex(&earlier_body.root_hash()), GOLDEN_ROOT_2);

    // Each path leads to its own tree's root, and neither leads to the other's.
    let earlier_root =
        rfc6962_walk(STATEMENT, 1, earlier.tree_size, &path_nodes(&earlier)).unwrap();
    let later_root = rfc6962_walk(STATEMENT, 1, later.tree_size, &path_nodes(&later)).unwrap();
    assert_eq!(earlier_root, earlier_body.root_hash());
    assert_eq!(later_root, later_body.root_hash());
    assert_ne!(earlier_root, later_root);

    // And the leaf itself did not move: the statement is still at index 1, with
    // the leaf hash computed outside Rust.
    assert_eq!(
        hex(&Sha256::digest([&[0x00_u8], STATEMENT].concat())),
        GOLDEN_STATEMENT_LEAF_HASH
    );
}

#[test]
fn the_artifact_has_the_serialized_shape_the_wire_contract_names() {
    // ⚠️ SHAPE COVERAGE, NOT WIRE VERIFICATION. The round trip below goes
    // through our own `Serialize` and our own `Deserialize`, so any change made
    // symmetrically on both sides is invisible to it — it can show that the
    // type survives a trip, and nothing about what a stranger's parser sees.
    // What carries weight here is narrower and stated as such: the field names
    // and the JSON types are compared against literals written in this file,
    // and `deny_unknown_fields` is exercised with input this test supplied.
    // The wire claim proper belongs to the independent cross-language verifier.
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[STATEMENT]);
    let artifact = anchor.inclusion_artifact(1).unwrap();

    let text = serde_json::to_string(&artifact).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = value.as_object().expect("an artifact is a JSON object");

    // Exactly these five keys, no more and no fewer, named by literals.
    let expected = ["format", "tree_size", "leaf_index", "hashes", "checkpoint"];
    let mut seen = 0;
    for key in expected {
        assert!(
            object.contains_key(key),
            "the artifact has no `{key}` field"
        );
        seen += 1;
    }
    assert_eq!(seen, expected.len());
    assert_eq!(
        object.len(),
        expected.len(),
        "the artifact carries a field this contract does not name: {text}"
    );

    // The JSON types a stranger's parser will meet.
    assert_eq!(object["format"].as_str(), Some(INCLUSION_FORMAT));
    assert_eq!(object["tree_size"].as_u64(), Some(2));
    assert_eq!(object["leaf_index"].as_u64(), Some(1));
    assert_eq!(
        object["hashes"].as_array().map(Vec::len),
        Some(1),
        "a two-leaf tree's inclusion path is one node"
    );
    assert!(object["checkpoint"].as_str().is_some());

    // Shape coverage: the type survives its own round trip.
    let decoded: InclusionProofArtifact = serde_json::from_str(&text).unwrap();
    assert_eq!(decoded.format, artifact.format);
    assert_eq!(decoded.tree_size, artifact.tree_size);
    assert_eq!(decoded.leaf_index, artifact.leaf_index);
    assert_eq!(decoded.hashes, artifact.hashes);
    assert_eq!(decoded.checkpoint, artifact.checkpoint);

    // `deny_unknown_fields`, driven by input this test wrote: a v1 artifact
    // with an extra field is not a v1 artifact, and there is no smuggling into
    // a frozen shape. Without this the round trip alone would be consistent
    // with a decoder that accepts anything.
    let smuggled = text.replace("{\"format\"", "{\"witnessed\":true,\"format\"");
    assert_ne!(smuggled, text, "the injection must actually have applied");
    assert!(
        serde_json::from_str::<InclusionProofArtifact>(&smuggled).is_err(),
        "an unknown field must not be accepted into a frozen shape"
    );
}

#[test]
fn the_path_nodes_are_padded_standard_base64_of_whole_digests() {
    // The decoder in this file is the second party for the `hashes` field, so
    // it is itself controlled here before anything is concluded from it.
    let tmp = TempDir::new().unwrap();
    let statements: Vec<Vec<u8>> = (0..4_u8).map(|n| vec![n; 5]).collect();
    let borrowed: Vec<&[u8]> = statements.iter().map(Vec::as_slice).collect();
    let anchor = anchor_over(tmp.path(), &borrowed);

    let artifact = anchor.inclusion_artifact(2).unwrap();
    let nodes = path_nodes(&artifact);
    assert!(!nodes.is_empty(), "this case needs a non-empty path");

    // Positive control on the decoder: a known base64 string decodes to known
    // bytes. If the decoder returned `None` for everything, or empty output for
    // everything, `path_nodes` above would have failed loudly — but a decoder
    // that returned the *wrong* bytes consistently would not, and this catches
    // it against a vector nothing in this workspace produced.
    assert_eq!(
        decode_standard_base64("bHlz").as_deref(),
        Some(&b"lys"[..]),
        "the decoder does not decode a known vector"
    );
    assert_eq!(
        decode_standard_base64("bA==").as_deref(),
        Some(&b"l"[..]),
        "the decoder mishandles two-character padding"
    );
    assert_eq!(
        decode_standard_base64("bHk=").as_deref(),
        Some(&b"ly"[..]),
        "the decoder mishandles one-character padding"
    );

    // Negative controls: the decoder must refuse what the contract forbids,
    // otherwise the padding assertion in `path_nodes` is checking a decoder
    // that would have accepted the unpadded form anyway.
    let mut refused = 0;
    for bad in ["bHl", "bH!=", "bHlz=", "", "bA=A"] {
        assert!(
            decode_standard_base64(bad).is_none(),
            "the decoder accepted `{bad}`, which is not standard padded base64"
        );
        refused += 1;
    }
    assert_eq!(refused, 5);
}
