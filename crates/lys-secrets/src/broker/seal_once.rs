//! Reconcile a save against the encrypted value, without replacing any entry.
use crate::{Broker, EntryClass, PermissionCheck, Secret, SecretsError};

impl<P: PermissionCheck> Broker<P> {
    /// Seal a new value, or confirm that this exact owner, class and value
    /// are already sealed. A conflicting entry is never replaced.
    ///
    /// # Errors
    /// `SecretExists` for a conflicting name, plus store and audit failures.
    pub fn seal_once(
        &mut self,
        name: &str,
        class: EntryClass,
        owner: &str,
        value: &Secret,
    ) -> Result<bool, SecretsError> {
        if let Some(entry) = self.store.entry(name) {
            if entry.owner == owner && entry.class == class {
                let existing = self.store.open_for_use(&self.store_key, name, class)?;
                let left = existing.expose();
                let right = value.expose();
                let same = left.len() == right.len()
                    && left.iter().zip(right).fold(0_u8, |d, (a, b)| d | (a ^ b)) == 0;
                if same {
                    // A previous store write may have succeeded before its audit
                    // append failed. Confirm the save durably before answering.
                    self.record(
                        crate::audit::AuditKind::Seal,
                        (None, Some(owner), Some(name)),
                        None,
                        None,
                        "confirmed",
                    )?;
                    return Ok(true);
                }
            }
            return Err(SecretsError::SecretExists {
                name: name.to_owned(),
            });
        }
        self.seal_record(name, class, owner, value)?;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BrokerPaths, LocalGrants};

    #[test]
    fn same_save_after_restart_preserves_entry_and_conflicts_never_replace_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let paths = BrokerPaths {
            store_dir: dir.path().join("store"),
            log_dir: dir.path().join("log"),
            store_key: dir.path().join("keys/store"),
            audit_key: dir.path().join("keys/audit"),
            anchor: dir.path().join("keys/anchor"),
        };
        std::fs::create_dir_all(dir.path().join("keys"))?;
        let mut broker = Broker::create(&paths, LocalGrants::new(), Box::new(|| 1000))?;
        let secret = Secret::from_slice(b"fixture only");
        assert!(!broker.seal_once("app-key", EntryClass::Key, "person-a", &secret)?);
        let before = broker.store().entry("app-key").cloned();
        drop(broker);
        let mut broker = Broker::open(&paths, LocalGrants::new(), Box::new(|| 1000))?;
        assert!(broker.seal_once("app-key", EntryClass::Key, "person-a", &secret)?);
        assert!(
            broker
                .seal_once("app-key", EntryClass::Key, "person-b", &secret)
                .is_err()
        );
        assert!(
            broker
                .seal_once("app-key", EntryClass::Credential, "person-a", &secret)
                .is_err()
        );
        assert!(
            broker
                .seal_once(
                    "app-key",
                    EntryClass::Key,
                    "person-a",
                    &Secret::from_slice(b"different")
                )
                .is_err()
        );
        assert_eq!(broker.store().entry("app-key"), before.as_ref());
        Ok(())
    }
}
