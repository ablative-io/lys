#![cfg(test)]
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
//!   constants in `submit_tests/`, whose fixture bytes this file reuses
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
use crate::admission::AcceptAll;
use crate::keys::{FileSigner, Signer};

use super::*;

mod shape;
mod walk;

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
/// Deliberately the same bytes `submit_tests/` uses, and named for the
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
fn anchor_over(dir: &Path, statements: &[&[u8]]) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
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
fn reopen(dir: &Path) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        signer(dir),
        AcceptAll,
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
fn verifier(anchor: &Anchor<FileLeafStore, FileSigner, AcceptAll>) -> NoteVerifierKey {
    NoteVerifierKey::new(ORIGIN, anchor.signer().public_key()).unwrap()
}
