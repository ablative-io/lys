#![cfg(test)]
//! Gates on [`Anchor::submit`] and [`Anchor::receipt_for`].
//!
//! # The trap this file is shaped around
//!
//! `verify_receipt` reconstructs the root **from the receipt's own inclusion
//! path** and checks the anchor's signature against that reconstruction. The
//! anchor derived and signed the root from that same path. So a receipt whose
//! path this crate chunked *wrongly* is still internally self-consistent, and
//! `verify_receipt` accepts it: a test that only asserts `is_ok()` proves
//! nothing whatever about the path. Anything this crate encodes and decodes
//! with its own pair of functions is invisible to its own suite.
//!
//! Every receipt assertion below is therefore anchored outside that loop.
//!
//! # Where the second party comes from
//!
//! - **The anchor's published checkpoint.** Its root comes from
//!   `AppendOnlyTree::root` — ct-merkle's own accumulator — and travels
//!   through base64 into a signed note that `lys-core`'s `verify_checkpoint`
//!   parses back out. The receipt's root comes from walking a chunked
//!   inclusion path. Two different derivations of the same 32 bytes, and
//!   [`the_receipt_reconstructs_the_root_the_checkpoint_publishes`] holds them
//!   against each other.
//! - **A root and two leaf hashes computed outside Rust.** [`GOLDEN_ROOT_2`],
//!   [`GOLDEN_GENESIS_LEAF_HASH`] and [`GOLDEN_STATEMENT_LEAF_HASH`] were
//!   produced by `openssl dgst -sha256` shell pipelines and independently
//!   reproduced with Python's `hashlib`, which agreed. **That agreement between
//!   two implementations is the whole of the evidence** — the literals are
//!   written out here, so no change to this workspace can move them.
//!
//!   ⚠️ **These constants were first justified by a positive control that
//!   cannot fail, and the claim is retracted here rather than quietly
//!   dropped.** The control was: hash the empty fixture with and without the
//!   `0x00` prefix, expect `e3b0c442…b855` and `6e340b9c…fa01d`. Both values
//!   are indeed correct, and the check is nevertheless **inert**, because on an
//!   *empty* fixture a working `cat` and a failed `cat` contribute the same
//!   thing — nothing. Measured: with the file replaced by a non-existent path,
//!   both legs return byte-identical digests to the working case, and the
//!   pipeline's exit status stays `0` because a pipeline reports its *last*
//!   command. A control fixture must be **non-empty**, or it cannot observe
//!   whether the file was read at all. Restated as house rule: *a control whose
//!   expected value is what a broken instrument returns anyway is not a
//!   control* — and this repo's own law that a check which never fires is
//!   indistinguishable from one that passed.
//! - **`sha2` driven by the test itself**, for the leaf hash — never
//!   `raw_leaf_hash`, which is the helper the implementation used.
//! - **The disk.** Leaf bytes are read back through a `Log` handle opened
//!   fresh over the same directory, which never saw the submission happen.
//!
//! The independence axis for all of these is *implementation*, not platform:
//! one machine, one toolchain, one dependency resolution. The cross-language
//! claim belongs to the Go conformance gate and is not made here.

use std::path::Path;
use std::time::Duration;

use lys_core::Ed25519Identity;
use lys_core::ca::CertificateAuthority;
use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_core::receipt::verify_receipt;
use lys_log_store::{FileLeafStore, Log};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{
    AcceptAll, AdmissionPolicy, AuthenticatedPeer, MaxSize, RecognisedCertificate, SubmitterContext,
};
use crate::keys::{FileSigner, Signer};

use super::*;

mod receipts;
mod refusals;

/// The origin this test supplies to the store. Verifiers are built from *this*
/// literal, never from what the anchor reports back.
const ORIGIN: &str = "example.com/lys/anchor-submit-test";

/// The genesis bytes for every anchor built here.
const GENESIS: &[u8] = b"lys-anchor increment 4 genesis fixture";

/// The statement submitted in the single-submission tests.
const STATEMENT: &[u8] = b"lys-anchor increment 4 statement fixture";

/// `SHA-256(0x00 ‖ GENESIS)` — RFC 6962's leaf hash, computed outside Rust.
const GOLDEN_GENESIS_LEAF_HASH: &str =
    "a5dabd900c94df52610e226a28e883f7e932d4f71e27e2bb4737cb9f2a0f7d1d";

/// `SHA-256(0x00 ‖ STATEMENT)`, computed outside Rust.
const GOLDEN_STATEMENT_LEAF_HASH: &str =
    "4a68beab3afee2c3b01d3b7c277259e12240cc888e5da1773c460f48f079bf5c";

/// `SHA-256(0x01 ‖ GOLDEN_GENESIS_LEAF_HASH ‖ GOLDEN_STATEMENT_LEAF_HASH)` —
/// the RFC 6962 root of the two-leaf tree an anchor holds after exactly one
/// submission. Computed outside Rust from the two literals above.
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

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a store at `dir` under [`ORIGIN`] and an anchor over it.
fn create_anchor(dir: &Path) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    create_anchor_with(dir, AcceptAll)
}

/// Creates a store at `dir` under [`ORIGIN`] and an anchor over it, under a
/// named admission policy.
fn create_anchor_with<P: AdmissionPolicy>(
    dir: &Path,
    policy: P,
) -> Anchor<FileLeafStore, FileSigner, P> {
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(
        store,
        GENESIS,
        signer(dir),
        policy,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// The verifier a third party would build: the origin they were told, and the
/// public key they were given.
fn verifier(anchor: &Anchor<FileLeafStore, FileSigner, AcceptAll>) -> NoteVerifierKey {
    NoteVerifierKey::new(ORIGIN, anchor.signer().public_key()).unwrap()
}

/// Reads leaf `index` back off the disk through a handle that never saw the
/// submission.
fn leaf_from_disk(dir: &Path, index: u64) -> Vec<u8> {
    let log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    log.leaf_bytes(index)
        .expect("the leaf must be on disk")
        .to_vec()
}
