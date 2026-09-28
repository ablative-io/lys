//! Signed snapshots of a log owner's folded state: `lys/log-snapshot/v1`.
//!
//! A service that folds its log into state (a directory, a grant book, a
//! broker's leases) would otherwise rebuild that state at every start by
//! reading, verifying and applying every leaf the log has ever held. A
//! snapshot records the state at one tree size instead, with what the tree
//! needs to resume from that size, so a start reads only the leaves after it.
//!
//! # Format
//!
//! Every variable-length field is framed as an 8-byte big-endian length and
//! its bytes. The file is two fields, the body and the signature:
//!
//! - `format`: the literal `lys/log-snapshot/v1`
//! - `domain`: what the state is, named by its owner
//! - `origin`: the origin of the log the state was folded from
//! - `tree_size`: 8 bytes, big-endian
//! - `root`: the 32-byte RFC 6962 root at `tree_size`
//! - `frontier`: the subtree roots at `tree_size`, 32 bytes each, largest first
//! - `state`: the owner's encoding of its folded state
//!
//! The signature is Ed25519 over the whole body, by the key the owner already
//! signs its log's leaves with. Nothing after the signature is allowed.
//!
//! # What is trusted, and on what grounds
//!
//! A snapshot is local state, like the pin, and it is believed only as far as
//! it is checked: the signature must verify under the owner's key, the format,
//! domain and origin must be this owner's and this log's, and the frontier must
//! fold to the signed root. [`crate::start`] then requires the log to agree
//! with that root at that size before a single tail leaf is applied. A snapshot
//! failing any check is refused by its [`SnapshotRefusal`] name and the owner
//! rebuilds from the whole log; nothing in a refused snapshot is used.

use lys_core::Ed25519Identity;

use crate::frontier::Frontier;

/// The format tag every snapshot body opens with.
pub const SNAPSHOT_FORMAT: &str = "lys/log-snapshot/v1";

/// Ed25519 signature length.
const SIGNATURE_LEN: usize = 64;

/// Why a snapshot was not used. Each is announced by name, and the owner
/// rebuilds from the whole log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SnapshotRefusal {
    /// The store holds no snapshot.
    #[error("SnapshotMissing: the store holds no snapshot")]
    Missing,

    /// The bytes are not a snapshot in this format.
    #[error("SnapshotMalformed: {reason}")]
    Malformed {
        /// What did not parse.
        reason: String,
    },

    /// The snapshot carries no signature.
    #[error("SnapshotUnsigned: the snapshot carries no signature")]
    Unsigned,

    /// The signature does not verify under the owner's key.
    #[error("SnapshotSignatureInvalid: the signature does not verify under the owner's key")]
    SignatureInvalid,

    /// The snapshot is of another kind of state than the owner keeps.
    #[error("SnapshotWrongKind: the snapshot holds {found:?}, not {expected:?}")]
    WrongKind {
        /// The domain the owner keeps.
        expected: String,
        /// The domain the snapshot names.
        found: String,
    },

    /// The snapshot was folded from another log.
    #[error("SnapshotWrongLog: the snapshot is of the log {found:?}, not {expected:?}")]
    WrongLog {
        /// The origin of the log being opened.
        expected: String,
        /// The origin the snapshot names.
        found: String,
    },

    /// The snapshot's root is not the log's root at its size: either its
    /// frontier does not fold to the root it signs, or the log's leaves after
    /// it do not reach the log's pinned root from there.
    #[error("SnapshotWrongRoot: the snapshot's root at tree size {size} is not the log's")]
    WrongRoot {
        /// The tree size the snapshot claims.
        size: u64,
    },

    /// The snapshot claims more leaves than the log has pinned.
    #[error("SnapshotBeyondLog: the snapshot is at tree size {size}, past the log's {pinned}")]
    BeyondLog {
        /// The tree size the snapshot claims.
        size: u64,
        /// The tree size the log has pinned.
        pinned: u64,
    },

    /// The owner could not read the state the snapshot carries.
    #[error("SnapshotStateUnreadable: {reason}")]
    StateUnreadable {
        /// Why the owner refused it.
        reason: String,
    },
}

/// A snapshot whose signature, domain, origin and root were checked.
#[derive(Clone, PartialEq, Eq)]
pub struct Snapshot {
    frontier: Frontier,
    state: Vec<u8>,
}

impl std::fmt::Debug for Snapshot {
    /// Summarizes the snapshot without its state, which may be large.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snapshot")
            .field("frontier", &self.frontier)
            .field("state_len", &self.state.len())
            .finish()
    }
}

impl Snapshot {
    /// The tree at the snapshot's size.
    pub fn frontier(&self) -> &Frontier {
        &self.frontier
    }

    /// The owner's encoded state.
    pub fn state(&self) -> &[u8] {
        &self.state
    }

    /// The frontier and state, taken apart.
    pub fn into_parts(self) -> (Frontier, Vec<u8>) {
        (self.frontier, self.state)
    }
}

/// Encodes and signs a snapshot of `state`, folded from the log `origin` at
/// the tree `frontier` describes.
pub fn seal(
    domain: &str,
    origin: &str,
    frontier: &Frontier,
    state: &[u8],
    key: &Ed25519Identity,
) -> Vec<u8> {
    let mut body = Vec::new();
    field(&mut body, SNAPSHOT_FORMAT.as_bytes());
    field(&mut body, domain.as_bytes());
    field(&mut body, origin.as_bytes());
    body.extend_from_slice(&frontier.size().to_be_bytes());
    body.extend_from_slice(&frontier.root());
    field(&mut body, &frontier.nodes().concat());
    field(&mut body, state);
    let signature = key.sign(&body);
    let mut sealed = Vec::with_capacity(body.len() + SIGNATURE_LEN + 16);
    field(&mut sealed, &body);
    field(&mut sealed, &signature);
    sealed
}

/// Reads and checks a sealed snapshot: its framing, signature under
/// `public_key`, domain, origin, and that its frontier folds to its root.
///
/// # Errors
///
/// The [`SnapshotRefusal`] naming the first check that failed.
pub fn unseal(
    sealed: &[u8],
    domain: &str,
    origin: &str,
    public_key: &[u8; 32],
) -> Result<Snapshot, SnapshotRefusal> {
    let mut outer = Reader::new(sealed);
    let body = outer.field("body")?;
    if outer.is_done() {
        return Err(SnapshotRefusal::Unsigned);
    }
    let signature = outer.field("signature")?;
    outer.finish()?;
    if signature.is_empty() {
        return Err(SnapshotRefusal::Unsigned);
    }
    Ed25519Identity::verify(public_key, body, signature)
        .ok()
        .ok_or(SnapshotRefusal::SignatureInvalid)?;
    let mut reader = Reader::new(body);
    if reader.field("format")? != SNAPSHOT_FORMAT.as_bytes() {
        return Err(malformed("the body does not open with the snapshot format"));
    }
    let found_domain = reader.text("domain")?;
    if found_domain != domain {
        return Err(SnapshotRefusal::WrongKind {
            expected: domain.to_owned(),
            found: found_domain,
        });
    }
    let found_origin = reader.text("origin")?;
    if found_origin != origin {
        return Err(SnapshotRefusal::WrongLog {
            expected: origin.to_owned(),
            found: found_origin,
        });
    }
    let size = reader.number("tree size")?;
    let root = reader.hash("root")?;
    let nodes = reader
        .field("frontier")?
        .chunks(32)
        .map(|chunk| <[u8; 32]>::try_from(chunk).ok())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| malformed("the frontier is not a whole number of 32-byte roots"))?;
    let state = reader.field("state")?.to_vec();
    reader.finish()?;
    let frontier =
        Frontier::from_parts(size, nodes).map_err(|error| SnapshotRefusal::Malformed {
            reason: error.to_string(),
        })?;
    if frontier.root() != root {
        return Err(SnapshotRefusal::WrongRoot { size });
    }
    Ok(Snapshot { frontier, state })
}

fn malformed(reason: &str) -> SnapshotRefusal {
    SnapshotRefusal::Malformed {
        reason: reason.to_owned(),
    }
}

/// Appends `bytes` framed by their 8-byte big-endian length.
fn field(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}

/// A strict reader over framed fields.
struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    fn is_done(&self) -> bool {
        self.rest.is_empty()
    }

    fn take(&mut self, len: usize, what: &str) -> Result<&'a [u8], SnapshotRefusal> {
        if self.rest.len() < len {
            return Err(SnapshotRefusal::Malformed {
                reason: format!("the {what} runs past the end of the snapshot"),
            });
        }
        let (taken, rest) = self.rest.split_at(len);
        self.rest = rest;
        Ok(taken)
    }

    fn number(&mut self, what: &str) -> Result<u64, SnapshotRefusal> {
        let bytes = self.take(8, what)?;
        <[u8; 8]>::try_from(bytes)
            .map(u64::from_be_bytes)
            .ok()
            .ok_or_else(|| malformed("a number is not 8 bytes"))
    }

    fn hash(&mut self, what: &str) -> Result<[u8; 32], SnapshotRefusal> {
        let bytes = self.take(32, what)?;
        <[u8; 32]>::try_from(bytes).ok().ok_or_else(|| malformed("a hash is not 32 bytes"))
    }

    fn field(&mut self, what: &str) -> Result<&'a [u8], SnapshotRefusal> {
        let len = self.number(what)?;
        let len = usize::try_from(len)
            .ok()
            .ok_or_else(|| SnapshotRefusal::Malformed {
                reason: format!("the {what} is longer than this machine can address"),
            })?;
        self.take(len, what)
    }

    fn text(&mut self, what: &str) -> Result<String, SnapshotRefusal> {
        let bytes = self.field(what)?;
        String::from_utf8(bytes.to_vec())
            .ok()
            .ok_or_else(|| SnapshotRefusal::Malformed {
                reason: format!("the {what} is not UTF-8"),
            })
    }

    fn finish(&self) -> Result<(), SnapshotRefusal> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(malformed("bytes follow the last field"))
        }
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
