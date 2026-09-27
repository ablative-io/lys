//! Store key rotation: every entry is resealed to a new key beside its
//! current sealing, one audit line records the old and the new key id, and
//! only then do the new sealings become current and the old key file go.

use std::fs;
use std::path::Path;

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::fsutil::io;
use crate::keys::{KeyId, StoreKey};
use crate::permission::PermissionCheck;

use super::Broker;

impl<P: PermissionCheck> Broker<P> {
    /// Rotates the store key to a new key generated at `new_key`, which must
    /// lie outside the store and log directories. Answers the old and the
    /// new key id.
    ///
    /// # Errors
    ///
    /// `KeyFileExists`, `KeyFileMisplaced`, every refusal of opening an
    /// entry under the current key, and the audit log's refusals.
    pub fn rotate_store_key(&mut self, new_key: &Path) -> Result<(KeyId, KeyId), SecretsError> {
        let guarded = [self.paths.store_dir.as_path(), self.paths.log_dir.as_path()];
        let new = StoreKey::generate(new_key, &guarded)?;
        let staged = self.store.prepare_rotation(&self.store_key, &new)?;
        let old_id = self.store_key.id().clone();
        let outcome = format!("rotated {old_id} to {}", new.id());
        self.record(
            AuditKind::Rotation,
            (None, None, None),
            None,
            None,
            &outcome,
        )?;
        self.store.commit_rotation(&new, staged)?;
        let old_path = std::mem::replace(&mut self.paths.store_key, new_key.to_path_buf());
        let new_id = new.id().clone();
        self.store_key = new;
        fs::remove_file(&old_path).map_err(io(format!(
            "removing the retired key {}",
            old_path.display()
        )))?;
        Ok((old_id, new_id))
    }
}
