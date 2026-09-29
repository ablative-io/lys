//! Reconcile a save against the encrypted value, without replacing any entry.
use crate::{Broker, EntryClass, PermissionCheck, Secret, SecretsError};

impl<P: PermissionCheck> Broker<P> {
    /// Prepare both app credentials durably, returning only the client digest.
    /// Reconciliation reuses the sealed client after a partial or lost answer.
    ///
    /// # Errors
    /// Refuses conflicting entries and propagates store/audit failures.
    pub fn prepare_app(&mut self, app: &str, owner: &str) -> Result<String, SecretsError> {
        let prefix = format!("lys-app-{owner}-{app}");
        let client = format!("{prefix}-client");
        let value = if let Some(entry) = self.store.entry(&client) {
            if entry.owner != owner || entry.class != EntryClass::Key {
                return Err(SecretsError::SecretExists { name: client });
            }
            self.store
                .open_for_use(&self.store_key, &client, EntryClass::Key)?
        } else {
            Secret::from_slice(
                crate::encoding::hex(&crate::encoding::random_bytes::<32>()?).as_bytes(),
            )
        };
        self.seal_once(&client, EntryClass::Key, owner, &value)?;
        let mut credential = zeroize::Zeroizing::new(format!("lys-app.{app}.").into_bytes());
        credential.extend_from_slice(value.expose());
        self.seal_once(
            &format!("{prefix}-api"),
            EntryClass::Credential,
            owner,
            &Secret::from_slice(&credential),
        )?;
        Ok(crate::encoding::hex(&crate::encoding::sha256(
            value.expose(),
        )))
    }

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
    #[test]
    fn second_credential_index_failure_retry_survives_reopen()
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
        let client = Secret::from_slice(b"fixture client only");
        let api = Secret::from_slice(b"fixture api only");
        broker.seal_once("app-client", EntryClass::Key, "person-a", &client)?;
        let index = paths.store_dir.join("index.json");
        let saved_index = paths.store_dir.join("saved-index.json");
        std::fs::rename(&index, &saved_index)?;
        // The sealed file is writable, but rename onto this directory must fail.
        std::fs::create_dir(&index)?;
        let failed = broker.seal_once("app-api", EntryClass::Credential, "person-a", &api);
        std::fs::remove_dir(&index)?;
        std::fs::rename(&saved_index, &index)?;
        assert!(
            matches!(failed, Err(SecretsError::Io { .. })),
            "index rename fault did not fire"
        );
        broker.seal_once("app-client", EntryClass::Key, "person-a", &client)?;
        broker.seal_once("app-api", EntryClass::Credential, "person-a", &api)?;
        drop(broker);
        let broker = Broker::open(&paths, LocalGrants::new(), Box::new(|| 1000))?;
        for (name, class, expected) in [
            ("app-client", EntryClass::Key, &client),
            ("app-api", EntryClass::Credential, &api),
        ] {
            assert!(
                broker.store().entry(name).is_some(),
                "successful retry lost {name} on reopen"
            );
            let stored = broker.store.open_for_use(&broker.store_key, name, class)?;
            assert_eq!(stored.expose(), expected.expose());
        }
        Ok(())
    }
}
