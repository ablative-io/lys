//! A temporary name a store could not remove after its leaf was linked.
//!
//! The link is the commit point, so a removal that fails after it fails no
//! append. It is kept here by name instead, for whoever tends the store: the
//! name costs space until it is removed.

use std::path::PathBuf;

/// One temporary name left in the leaves directory, and why.
#[derive(Debug)]
pub struct LeftBehind {
    /// The leaf that was written under the temporary name.
    pub index: u64,
    /// The temporary name that is still there.
    pub path: PathBuf,
    /// What the removal answered.
    pub source: std::io::Error,
}
