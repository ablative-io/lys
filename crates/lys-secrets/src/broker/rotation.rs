//! Store key rotation: every entry is resealed to a new key beside its
//! current sealing, an audit line records that the rotation is starting, one
//! index write makes the new sealings current, a second line records that it
//! took effect, and only then does the old key file go. A broker stopped
//! between the two lines settles the rotation from the store when it opens:
//! the store's key says whether the index write happened.

use std::fs;
use std::path::Path;

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::fsutil::io;
use crate::keys::{KeyId, StoreKey};
use crate::permission::PermissionCheck;

use super::{Broker, ROTATING};

const ROTATED: &str = "rotated ";
const ABANDONED: &str = "abandoned ";

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
        let change = format!("{old_id} to {}", new.id());
        self.record_rotation(ROTATING, &change)?;
        self.store.commit_rotation(&new, staged)?;
        self.record_rotation(ROTATED, &change)?;
        let old_path = std::mem::replace(&mut self.paths.store_key, new_key.to_path_buf());
        let new_id = new.id().clone();
        self.store_key = new;
        fs::remove_file(&old_path).map_err(io(format!(
            "removing the retired key {}",
            old_path.display()
        )))?;
        Ok((old_id, new_id))
    }

    /// Closes a rotation the log shows starting and never finishing: it took
    /// effect if the store is now under its new key, and was abandoned if
    /// not, when opening swept its new sealings.
    pub(super) fn settle_rotation(&mut self, rotating: Option<&str>) -> Result<(), SecretsError> {
        let Some(change) = rotating else {
            return Ok(());
        };
        let took_effect = change
            .rsplit_once(" to ")
            .is_some_and(|(_, new)| new == self.store_key.id().as_str());
        let settled = if took_effect { ROTATED } else { ABANDONED };
        self.record_rotation(settled, change)
    }

    fn record_rotation(&mut self, state: &str, change: &str) -> Result<(), SecretsError> {
        self.record(
            AuditKind::Rotation,
            (None, None, None),
            None,
            None,
            &format!("{state}{change}"),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ABANDONED, ROTATED, ROTATING};
    use crate::audit::AuditKind;
    use crate::broker::{Broker, BrokerPaths};
    use crate::local_grants::LocalGrants;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn paths(root: &std::path::Path) -> BrokerPaths {
        let keys = root.join("keys");
        BrokerPaths {
            store_dir: root.join("store"),
            log_dir: root.join("log"),
            store_key: keys.join("store.key"),
            audit_key: keys.join("audit.key"),
            anchor: keys.join("audit.anchor"),
        }
    }

    fn last_rotation(broker: &Broker<LocalGrants>) -> Result<Option<String>, crate::SecretsError> {
        Ok(broker
            .audit()
            .replay()?
            .into_iter()
            .rev()
            .find(|recorded| recorded.line.kind == AuditKind::Rotation)
            .map(|recorded| recorded.line.outcome))
    }

    fn settles(target: impl Fn(&str) -> String, expected: &str) -> TestResult {
        let root = tempfile::tempdir()?;
        std::fs::create_dir_all(root.path().join("keys"))?;
        let paths = paths(root.path());
        let mut broker = Broker::create(&paths, LocalGrants::new(), Box::new(|| 1))?;
        let current = broker.store_key.id().as_str().to_owned();
        let change = format!("old to {}", target(&current));
        broker.record_rotation(ROTATING, &change)?;
        drop(broker);
        let reopened = Broker::open(&paths, LocalGrants::new(), Box::new(|| 2))?;
        assert_eq!(
            last_rotation(&reopened)?,
            Some(format!("{expected}{change}"))
        );
        Ok(())
    }

    #[test]
    fn a_rotation_whose_index_write_happened_settles_as_rotated() -> TestResult {
        settles(str::to_owned, ROTATED)
    }

    #[test]
    fn a_rotation_whose_index_write_never_happened_settles_as_abandoned() -> TestResult {
        settles(|_| "another-key".to_owned(), ABANDONED)
    }
}
