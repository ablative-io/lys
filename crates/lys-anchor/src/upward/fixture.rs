#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Scaffolding shared by the cascade gates: anchors that can be pinned to one
//! another.
//!
//! Nothing here asserts anything. It exists so the two test files below build
//! the same shapes rather than each hand-rolling an anchor, and so a cascade of
//! three is as cheap to stage as a cascade of one.
//!
//! Unlike the witness fixture, whose child is a plain `Log` — a witness must
//! work against any log that publishes checkpoints, so building the
//! counterparty out of the type under test would hide assumptions — **every
//! party here is a real [`Anchor`]**, because that is precisely the claim:
//! DP14's cascade is anchors pinning to anchors, and a fixture that faked
//! either end would be testing something else.

use std::path::Path;

use lys_core::checkpoint::NoteVerifierKey;
use lys_log_store::FileLeafStore;
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::{AcceptAll, AdmissionPolicy, SubmitterContext};
use crate::anchor::Anchor;
use crate::keys::{FileSigner, Signer};
use crate::wire::Submission;

/// Genesis bytes for every anchor built here.
pub const GENESIS: &[u8] = b"lys-anchor increment 8 cascade genesis";

/// Writes `seed` into `dir` under `name` and loads a signer over it.
fn signer_at(dir: &Path, name: &str, seed: &[u8; 32]) -> FileSigner {
    let path = dir.join(name);
    std::fs::write(&path, seed).unwrap();
    FileSigner::load(&path).unwrap()
}

/// One anchor in a cascade, with its storage directory held so it outlives the
/// anchor.
///
/// Generic over the admission policy so a parent that *refuses* can be staged
/// without a second fixture type — the refusal path is a rule of its own and
/// needs a parent that has one.
pub struct Node<P: AdmissionPolicy> {
    /// The anchor itself.
    pub anchor: Anchor<FileLeafStore, FileSigner, P>,
    origin: String,
    _dir: TempDir,
}

impl<P: AdmissionPolicy> Node<P> {
    /// Builds an anchor under `origin`, keyed by `seed`, admitting by `policy`.
    ///
    /// `seed` is supplied per node rather than derived, so a note signed by one
    /// anchor can never verify under another's key by accident.
    pub fn with_policy(origin: &str, seed: &[u8; 32], policy: P) -> Self {
        let dir = TempDir::new().unwrap();
        let store = FileLeafStore::create(dir.path(), origin).unwrap();
        let anchor = Anchor::create(
            store,
            GENESIS,
            signer_at(dir.path(), "anchor.key", seed),
            policy,
            AnchorConfig::unconfigured(),
        )
        .unwrap();
        Self {
            anchor,
            origin: origin.to_string(),
            _dir: dir,
        }
    }

    /// The origin this node was created under, as **this fixture** supplied it
    /// — never read back off the anchor, so a verifier built from it is keyed
    /// on the test's own value.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The verifier key a third party would be given for this node: the origin
    /// literal above, and the public key the signer advertises.
    pub fn verifier(&self) -> NoteVerifierKey {
        NoteVerifierKey::new(&self.origin, self.anchor.signer().public_key()).unwrap()
    }
}

impl Node<AcceptAll> {
    /// An anchor that refuses nothing — the right default for these gates,
    /// because a cascade must add no admission rule of its own and an anchor
    /// that refuses nothing is the one that exposes any rule this crate
    /// smuggled in.
    pub fn new(origin: &str, seed: &[u8; 32]) -> Self {
        Self::with_policy(origin, seed, AcceptAll)
    }
}

/// The origin of the anchor at the bottom of a cascade — the one whose leaf a
/// bundle is about.
pub const CHILD_ORIGIN: &str = "example.com/lys/cascade-child";

/// The origin of the anchor the child pins to.
pub const PARENT_ORIGIN: &str = "example.com/lys/cascade-parent";

/// `lys-core`'s conformance fixture seed, so key material is deterministic and
/// never generated.
pub const CHILD_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// A distinct seed per node, so one anchor's note can never verify under
/// another's key by accident.
pub const PARENT_SEED: &[u8; 32] = b"lys-anchor-increment-8-parent-ke";

/// A bare child anchor, holding only its genesis leaf.
pub fn child() -> Node<AcceptAll> {
    Node::new(CHILD_ORIGIN, CHILD_SEED)
}

/// A child anchor with [`STATEMENT`] logged, and **the index the anchor put it
/// at**.
///
/// The index is returned rather than published as a constant, so every check
/// downstream is keyed on what the anchor reported and not on a value this
/// fixture chose. Past genesis the tree is size 2, so a receipt over any leaf
/// has a non-empty inclusion path.
pub fn child_with_statement() -> (Node<AcceptAll>, u64) {
    let mut node = child();
    let appended = node
        .anchor
        .append(
            Submission {
                statement: STATEMENT,
            },
            SubmitterContext::Unidentified,
        )
        .unwrap();
    (node, appended.leaf_index)
}

/// The anchor a child pins to.
pub fn parent() -> Node<AcceptAll> {
    Node::new(PARENT_ORIGIN, PARENT_SEED)
}

/// The statement a cascade is assembled about.
pub const STATEMENT: &[u8] = b"the statement whose provenance the bundle carries";
