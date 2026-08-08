#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Scaffolding shared by the witness gates: a witness anchor, and a *child*
//! log whose checkpoints it records.
//!
//! Nothing here asserts anything. It exists so the two test files below build
//! the same shapes — a real second log with a real key, signing real C2SP
//! notes — rather than each hand-rolling note bytes, which would make the
//! suite a conversation between two copies of this crate's own idea of a
//! checkpoint.
//!
//! The child is a plain `Log`, not an `Anchor`, deliberately: a witness must
//! work against any log that publishes checkpoints, and building the child out
//! of the type under test would hide any assumption the witness makes about
//! its counterparty.

use std::path::Path;

use lys_core::Ed25519Identity;
use lys_core::checkpoint::{CheckpointBody, sign_note};
use lys_log_store::{FileLeafStore, Log};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::admission::AcceptAll;
use crate::anchor::Anchor;
use crate::keys::FileSigner;

/// The origin the witness anchor's own store is created under.
pub const WITNESS_ORIGIN: &str = "example.com/lys/anchor-witness-test";

/// The origin of the log the witness observes.
pub const CHILD_ORIGIN: &str = "example.com/lys/child-log-test";

/// A second child, so "this origin" can be told from "any origin".
pub const OTHER_CHILD_ORIGIN: &str = "example.com/lys/other-child-log-test";

/// Genesis bytes for every witness anchor built here.
pub const WITNESS_GENESIS: &[u8] = b"lys-anchor increment 7 witness genesis";

/// `lys-core`'s conformance fixture seed, so key material in tests is
/// deterministic and never generated.
const WITNESS_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// A distinct seed for the child, so a note signed by the child can never
/// verify under the witness's key by accident.
const CHILD_SEED: &[u8; 32] = b"lys-anchor-increment-7-child-key";

/// Writes `seed` into `dir` under `name` and loads a signer over it.
fn signer_at(dir: &Path, name: &str, seed: &[u8; 32]) -> FileSigner {
    let path = dir.join(name);
    std::fs::write(&path, seed).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a witness anchor at `dir`, admitting everything.
///
/// `AcceptAll` is the right policy for these gates: the witness path must add
/// no admission rule of its own, so an anchor that refuses nothing is the one
/// that exposes any rule this crate smuggled in.
pub fn witness_anchor(dir: &Path) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    let store = FileLeafStore::create(dir, WITNESS_ORIGIN).unwrap();
    Anchor::create(
        store,
        WITNESS_GENESIS,
        signer_at(dir, "witness.key", WITNESS_SEED),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// A log the witness observes: real leaves, a real key, real signed notes.
pub struct Child {
    log: Log<FileLeafStore>,
    identity: Ed25519Identity,
    origin: String,
    /// The child's own storage directory. Held so it outlives the log, and
    /// readable through [`Child::dir`] so a test can reopen the child's store
    /// through a handle that never saw an append.
    dir: TempDir,
}

impl Child {
    /// Builds a child log under `origin` with one leaf already in it, so its
    /// first checkpoint is over a non-trivial tree.
    pub fn new(origin: &str) -> Self {
        Self::with_seed(origin, CHILD_SEED)
    }

    /// As [`Child::new`], under a named key seed.
    pub fn with_seed(origin: &str, seed: &[u8; 32]) -> Self {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("child.key");
        std::fs::write(&key_path, seed).unwrap();
        let identity = Ed25519Identity::load(&key_path).unwrap();
        let store = FileLeafStore::create(dir.path(), origin).unwrap();
        let mut log = Log::open(store).unwrap();
        log.append(b"child genesis").unwrap();
        Self {
            log,
            identity,
            origin: origin.to_string(),
            dir,
        }
    }

    /// The child's origin.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The directory the child's store lives in, so a test can reopen it.
    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    /// The child's public key, as a third party would be given it.
    pub fn public_key(&self) -> [u8; 32] {
        self.identity.public_key_bytes()
    }

    /// The child's current tree size.
    pub fn tree_size(&self) -> u64 {
        self.log.tree().len()
    }

    /// The child's current root hash.
    pub fn root(&self) -> [u8; 32] {
        self.log.tree().root().to_parts().0
    }

    /// Appends `count` leaves to the child's log.
    pub fn grow(&mut self, count: usize) {
        for n in 0..count {
            self.log
                .append(format!("child entry {n}").as_bytes())
                .unwrap();
        }
    }

    /// The child's signed checkpoint over its current tree.
    pub fn checkpoint(&self) -> Vec<u8> {
        let body = CheckpointBody::from_root(&self.origin, &self.log.tree().root()).unwrap();
        self.sign(&body)
    }

    /// A signed checkpoint stating `(tree_size, root)` — whatever the child's
    /// tree actually holds.
    ///
    /// This is how equivocation and rollback are staged: the note is genuinely
    /// signed by the child's key, which is precisely the case a witness cannot
    /// dismiss as a forgery.
    pub fn checkpoint_stating(&self, tree_size: u64, root: [u8; 32]) -> Vec<u8> {
        let body = CheckpointBody::new(&self.origin, tree_size, root).unwrap();
        self.sign(&body)
    }

    /// The RFC 6962 consistency proof bytes from `old_size` to the child's
    /// current size.
    pub fn consistency_from(&self, old_size: u64) -> Vec<u8> {
        self.log
            .tree()
            .prove_consistency(old_size, self.log.tree().len())
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    fn sign(&self, body: &CheckpointBody) -> Vec<u8> {
        sign_note(&body.encode(), &self.origin, &self.identity)
            .unwrap()
            .into_bytes()
    }
}

/// Flips one byte of a 32-byte root, so a "different root" is different by
/// construction rather than by luck.
pub fn flip_root(root: [u8; 32]) -> [u8; 32] {
    let mut flipped = root;
    flipped[0] ^= 0xff;
    flipped
}
