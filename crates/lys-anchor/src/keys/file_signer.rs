//! [`FileSigner`] — the anchor's key, at rest in a file.
//!
//! # Loading never mints
//!
//! [`FileSigner::load`] fails when the file is absent. It has no
//! generate-if-missing sibling, and that is the whole point of the type having
//! one constructor: an anchor's key *is* its published identity. A signer that
//! minted a fresh key on a missing file would, after a mislaid path or a
//! restored-without-secrets backup, carry on publishing checkpoints under a key
//! no verifier has ever been told about — while reporting success at every
//! step. The failure that surfaces instead names the path.
//!
//! Minting a key is an operator ceremony, and it belongs in a command an
//! operator runs on purpose rather than in a code path that happens to be
//! reached.
//!
//! # No constructor takes key material
//!
//! There is no `from_identity` and no `from_seed`. A file path is a custody
//! story an operator can inspect — permissions, backups, who can read it —
//! and a constructor that accepted an in-memory key would let key material
//! arrive from anywhere while the type's name went on claiming otherwise.

use std::fmt;
use std::path::Path;

use lys_core::Ed25519Identity;

use crate::error::{AnchorError, AnchorResult};
use crate::keys::signer::{InProcessSigner, Signer};

/// A [`Signer`] backed by an Ed25519 key file.
///
/// The file format is `lys-core`'s: a raw 32-byte Ed25519 seed. Obtain one with
/// [`FileSigner::load`].
pub struct FileSigner {
    identity: Ed25519Identity,
}

impl fmt::Debug for FileSigner {
    /// Delegates to `Ed25519Identity`'s own redacting `Debug` rather than
    /// deriving one.
    ///
    /// A derived `Debug` on a struct holding key material is a private key in
    /// every log line that ever formats the enclosing value, so the impl is
    /// written out even though it does nothing but forward — a derive is one
    /// keystroke away, and the reason not to reach for it belongs next to the
    /// decision.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSigner")
            .field("identity", &self.identity)
            .finish()
    }
}

impl FileSigner {
    /// Loads the anchor's signing key from `path`.
    ///
    /// The file must exist and must hold a raw 32-byte Ed25519 seed. A missing
    /// file is an error and never a trigger to generate one — see the module
    /// docs.
    ///
    /// # Errors
    ///
    /// [`AnchorError::SignerKey`] if the file is absent, unreadable, or not
    /// exactly 32 bytes long, carrying the path and `lys-core`'s own reason.
    pub fn load(path: &Path) -> AnchorResult<Self> {
        let identity = Ed25519Identity::load(path).map_err(|source| AnchorError::SignerKey {
            path: path.display().to_string(),
            source,
        })?;
        Ok(Self { identity })
    }
}

impl Signer for FileSigner {
    fn public_key(&self) -> [u8; 32] {
        self.identity.public_key_bytes()
    }

    fn sign(&self, message: &[u8]) -> AnchorResult<[u8; 64]> {
        // Infallible for a local key: Ed25519 signing in dalek 2 cannot fail.
        // The `Result` is the trait's, shaped for the remote custody this type
        // is the local case of.
        Ok(self.identity.sign(message))
    }
}

impl InProcessSigner for FileSigner {
    fn identity(&self) -> &Ed25519Identity {
        &self.identity
    }
}

#[cfg(test)]
#[path = "file_signer_tests.rs"]
mod tests;
