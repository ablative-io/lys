//! Reconcile ambiguous index commits without discarding a renamed entry.
use super::{INDEX, SecretStore};
use crate::{SecretsError, fsutil};

impl SecretStore {
    fn index_bytes(&self) -> Result<Vec<u8>, SecretsError> {
        serde_json::to_vec_pretty(&self.index).map_err(|error| SecretsError::Encoding {
            context: "store index",
            reason: error.to_string(),
        })
    }

    pub(super) fn commit_add(&mut self, name: &str) -> Result<(), SecretsError> {
        let bytes = match self.index_bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                self.index.entries.remove(name);
                return Err(error);
            }
        };
        match fsutil::write_atomic_outcome(&self.dir.join(INDEX), &bytes) {
            Ok(()) => Ok(()),
            Err(fsutil::AtomicWriteError::BeforeRename(error)) => {
                self.index.entries.remove(name);
                Err(error)
            }
            Err(fsutil::AtomicWriteError::AfterRename(error)) => {
                // The cache describes the new file already at the destination.
                // Keep it even when syncing the directory did not finish.
                Err(SecretsError::IndexUnresolved {
                    reason: error.to_string(),
                })
            }
        }
    }

    /// Confirm the persisted index agrees with this broker's cache and finish
    /// its directory sync before an idempotent save claims durable custody.
    pub(crate) fn confirm_index(&self) -> Result<(), SecretsError> {
        let unresolved = |reason| SecretsError::IndexUnresolved { reason };
        let path = self.dir.join(INDEX);
        let bytes = std::fs::read(&path).map_err(|error| unresolved(error.to_string()))?;
        let expected = self
            .index_bytes()
            .map_err(|error| unresolved(error.to_string()))?;
        if bytes != expected {
            return Err(unresolved(
                "persisted index differs from the broker's held index".to_owned(),
            ));
        }
        fsutil::sync_parent(&path).map_err(|error| unresolved(error.to_string()))
    }
}
