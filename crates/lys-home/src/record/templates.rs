//! The home's template store (HOME-002 R3): each launch template a home has
//! rendered is kept once as an object named by the SHA-256 of its bytes,
//! `templates/<hh>/<hash>`, with the block store's write discipline (a
//! temporary file, fsynced, renamed, the directory fsynced). A template is
//! never rewritten or deleted. The directory appears when the first template
//! is stored, so opening a home creates nothing here.

use std::path::{Path, PathBuf};

use crate::error::HomeError;
use crate::record::blocks::{BlockStore, Hash, Put};

/// The templates of one home.
#[derive(Clone, Debug)]
pub struct TemplateStore {
    root: PathBuf,
}

impl TemplateStore {
    /// The store under `root`; nothing is created until a template is put.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Where the store lives, whether or not it exists yet.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The path a template of this hash is kept at.
    #[must_use]
    pub fn path_of(&self, hash: &Hash) -> PathBuf {
        self.root.join(&hash.as_str()[..2]).join(hash.as_str())
    }

    /// Keep a template's bytes under their hash: the hash and whether this
    /// put wrote it. Bytes already held are not written again.
    pub fn put(&self, bytes: &[u8]) -> Result<Put, HomeError> {
        BlockStore::open(&self.root)?.put(bytes)
    }

    /// Whether a template of this hash is held.
    #[must_use]
    pub fn contains(&self, hash: &Hash) -> bool {
        self.path_of(hash).is_file()
    }

    /// Read a template by hash.
    pub fn get(&self, hash: &Hash) -> Result<Vec<u8>, HomeError> {
        let path = self.path_of(hash);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(HomeError::NoBlock {
                hash: hash.to_string(),
            }),
            Err(e) => Err(HomeError::io("reading a template", &path, e)),
        }
    }
}
