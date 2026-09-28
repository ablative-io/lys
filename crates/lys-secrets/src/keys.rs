//! The broker's two keys: the store key every entry is sealed to, and the
//! audit key that signs every audit line. Each lives in its own key file
//! outside the store and log directories.

use std::fmt;
use std::path::Path;

use lys_core::Ed25519Identity;

use crate::encoding::{hex, sha256};
use crate::error::SecretsError;
use crate::fsutil::{ensure_outside, ensure_owner_only};

/// The SHA-256 fingerprint of a key's public half, in hex. Never a key byte.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KeyId(String);

impl KeyId {
    /// The fingerprint of `public`.
    pub fn of_public(public: &[u8; 32]) -> Self {
        Self(hex(&sha256(public)))
    }

    /// The fingerprint as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyId({})", self.0)
    }
}

/// A key held by the broker: an Ed25519 seed read from an owner-only key
/// file. The store key uses its X25519 form for sealing; the audit key signs.
pub struct StoreKey {
    identity: Ed25519Identity,
    id: KeyId,
}

impl fmt::Debug for StoreKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StoreKey({})", self.id)
    }
}

impl StoreKey {
    /// Generates a new key into `path`, which must not exist yet and must lie
    /// outside every one of `outside`.
    ///
    /// # Errors
    ///
    /// `KeyFileExists` when the file is already there, `KeyFileMisplaced`
    /// when it lies inside a guarded directory, and `Trust` when the key
    /// cannot be written.
    pub fn generate(path: &Path, outside: &[&Path]) -> Result<Self, SecretsError> {
        if path.exists() {
            return Err(SecretsError::KeyFileExists {
                path: path.to_path_buf(),
            });
        }
        ensure_outside(path, outside)?;
        let identity = Ed25519Identity::load_or_generate(path)?;
        ensure_owner_only(path)?;
        Ok(Self::from_identity(identity))
    }

    /// Reads the key in `path`, which must exist, be owner-only and lie
    /// outside every one of `outside`.
    ///
    /// # Errors
    ///
    /// `StoreKeyMissing` when there is no file, `KeyFileMisplaced`,
    /// `KeyFilePermissions`, and `Trust` when the file is not a key.
    pub fn load(path: &Path, outside: &[&Path]) -> Result<Self, SecretsError> {
        if !path.exists() {
            return Err(SecretsError::StoreKeyMissing {
                path: Some(path.to_path_buf()),
            });
        }
        ensure_outside(path, outside)?;
        ensure_owner_only(path)?;
        let identity = Ed25519Identity::load(path)?;
        Ok(Self::from_identity(identity))
    }

    fn from_identity(identity: Ed25519Identity) -> Self {
        let id = KeyId::of_public(&identity.public_key_bytes());
        Self { identity, id }
    }

    /// The key's fingerprint.
    pub fn id(&self) -> &KeyId {
        &self.id
    }

    /// The Ed25519 public half, which verifies what this key signs.
    pub fn verifying_key(&self) -> [u8; 32] {
        self.identity.public_key_bytes()
    }

    /// The X25519 public half entries are sealed to.
    pub fn sealing_public(&self) -> [u8; 32] {
        self.identity.x25519_public_key()
    }

    pub(crate) fn identity(&self) -> &Ed25519Identity {
        &self.identity
    }
}
