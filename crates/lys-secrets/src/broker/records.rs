//! Sealed records: memory and keys kept encrypted in the store. A memory
//! record is read by name, in the smallest piece asked for, when the asking
//! identity holds the `read` relation on it; a key is used through the
//! proxy and never read. Every read and every refusal is one audit line.

use std::ops::Range;

use crate::audit::AuditKind;
use crate::error::SecretsError;
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::EntryClass;

use super::Broker;

/// The outcome of an admitted read.
pub(super) const READ: &str = "read";

impl<P: PermissionCheck> Broker<P> {
    /// Seals a memory or key record into the store, owned by `owner`.
    ///
    /// # Errors
    ///
    /// Every refusal of [`crate::SecretStore::add`], and the audit log's.
    pub fn seal_record(
        &mut self,
        name: &str,
        class: EntryClass,
        owner: &str,
        value: &Secret,
    ) -> Result<(), SecretsError> {
        self.store.add(&self.store_key, name, class, owner, value)?;
        self.record(
            AuditKind::Seal,
            (None, Some(owner), Some(name)),
            None,
            None,
            "sealed",
        )?;
        Ok(())
    }

    /// Reads `piece` of the memory record `name` for `identity`; the whole
    /// record when no piece is named. A piece past the record's end is cut
    /// at the end.
    ///
    /// # Errors
    ///
    /// `NotFound`, `NoRelation`, `RelationRemoved`, `KeyNotReadable`, the
    /// store's opening refusals, and the audit log's.
    pub fn read_record(
        &mut self,
        identity: &str,
        name: &str,
        piece: Option<Range<usize>>,
    ) -> Result<Secret, SecretsError> {
        match self.admit_read(identity, name) {
            Ok(class) => {
                let whole = self.store.open_for_use(&self.store_key, name, class)?;
                let bytes = whole.expose();
                let range = piece.unwrap_or(0..bytes.len());
                let end = range.end.min(bytes.len());
                let start = range.start.min(end);
                let read = Secret::from_slice(bytes.get(start..end).ok_or_else(|| {
                    SecretsError::StoreCorrupt {
                        reason: format!("{name} did not cut at {start}..{end}"),
                    }
                })?);
                self.record(
                    AuditKind::SealedRead,
                    (None, Some(identity), Some(name)),
                    None,
                    None,
                    READ,
                )?;
                self.readers.insert((identity.to_owned(), name.to_owned()));
                Ok(read)
            }
            Err(refusal) => {
                self.record(
                    AuditKind::SealedRead,
                    (None, Some(identity), Some(name)),
                    None,
                    None,
                    refusal.name(),
                )?;
                Err(refusal)
            }
        }
    }

    fn admit_read(&self, identity: &str, name: &str) -> Result<EntryClass, SecretsError> {
        self.within_scope(identity, name)
            .map_err(|_reason| SecretsError::NotFound)?;
        let class = self
            .store
            .entry(name)
            .map(|entry| entry.class)
            .ok_or(SecretsError::NotFound)?;
        if self.permissions.may_read(identity, name).is_err() {
            let pair = (identity.to_owned(), name.to_owned());
            return Err(if self.readers.contains(&pair) {
                SecretsError::RelationRemoved {
                    identity: identity.to_owned(),
                    record: name.to_owned(),
                }
            } else {
                SecretsError::NoRelation {
                    identity: identity.to_owned(),
                    record: name.to_owned(),
                }
            });
        }
        if class != EntryClass::Memory {
            return Err(SecretsError::KeyNotReadable {
                record: name.to_owned(),
            });
        }
        Ok(class)
    }
}
