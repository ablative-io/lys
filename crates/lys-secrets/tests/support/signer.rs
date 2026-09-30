//! A holder with an Ed25519 key: the identity a handle is issued to, and
//! the signer of its presentations.

use std::path::Path;

use lys_core::Ed25519Identity;
use lys_secrets::{HandleId, Holder, Presentation, SecretsError, new_operation_id};

use super::world::NOW_MS;

/// The digest of the one request every test presentation is for.
const CALL: [u8; 32] = [7; 32];

pub struct Signer {
    key: Ed25519Identity,
    pub holder: Holder,
}

impl Signer {
    /// A holder named `identity`, whose key is kept under `keys`.
    pub fn new(keys: &Path, identity: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let key = Ed25519Identity::load_or_generate(&keys.join(format!("{identity}.key")))?;
        let holder = Holder {
            identity: identity.to_owned(),
            key: key.public_key_bytes(),
        };
        Ok(Self { key, holder })
    }

    /// A presentation of `id` for a fresh operation, signed at `NOW_MS` for
    /// the test request.
    pub fn present(&self, id: &HandleId) -> Result<Presentation, SecretsError> {
        Presentation::sign(id, &new_operation_id()?, NOW_MS, CALL, &self.key)
    }
}
