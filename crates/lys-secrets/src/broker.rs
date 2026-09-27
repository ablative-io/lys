//! The broker: issues handles under a person's grant, swaps a presented
//! handle for its credential only inside the forwarding closure, and writes
//! one signed audit line for every issue, use, refusal and drop. Use counts,
//! drops and retry outcomes are read back from the audit log at start, so a
//! broker killed at any point reopens to what the log says.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::audit::{AuditKind, AuditLine, AuditLog};
use crate::encoding::hex;
use crate::error::SecretsError;
use crate::fsutil::{io, write_atomic};
use crate::handle::{HandleId, HandleToken, Holder, IssuedHandle, Presentation};
use crate::keys::StoreKey;
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::{EntryClass, SecretStore};

use admit::Admission;

mod admit;
mod rotation;

const HANDLES: &str = "handles.json";
const LOG_ORIGIN: &str = "lys.local/secrets-audit";
/// How far a presentation's time may be from the broker's clock.
pub const PRESENTATION_SKEW_MS: i64 = 30_000;

/// The broker's clock, in milliseconds since the epoch.
pub type Clock = Box<dyn Fn() -> i64 + Send + Sync>;

/// Where the broker keeps things. Both key files and the anchor lie outside
/// both directories.
#[derive(Debug, Clone)]
pub struct BrokerPaths {
    /// The sealed store.
    pub store_dir: PathBuf,
    /// The audit log.
    pub log_dir: PathBuf,
    /// The store key file.
    pub store_key: PathBuf,
    /// The audit key file.
    pub audit_key: PathBuf,
    /// The audit log's anchor file.
    pub anchor: PathBuf,
}

impl BrokerPaths {
    fn guarded(&self) -> [&Path; 2] {
        [&self.store_dir, &self.log_dir]
    }
}

/// A use the broker admitted.
#[derive(Debug)]
pub enum Used<R> {
    /// The credential was handed to the forwarding closure, which answered.
    Forwarded {
        /// The closure's answer.
        answer: R,
        /// Uses left on the lease.
        uses_left: u64,
    },
    /// The same operation was presented before; its first outcome is
    /// returned and nothing is forwarded again.
    Retried {
        /// The recorded outcome.
        outcome: String,
    },
}

/// What a use can be refused with.
pub type UseError = SecretsError;

/// What dropping a handle found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeOutcome {
    /// The handle was live and is now dropped.
    Dropped,
    /// The handle was already dropped.
    AlreadyDropped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandleRecord {
    id: String,
    digest: String,
    identity: String,
    holder_key: String,
    secret: String,
    max_uses: u64,
    not_after_ms: i64,
    #[serde(skip)]
    used: u64,
    #[serde(skip)]
    dropped: bool,
    #[serde(skip)]
    operations: BTreeMap<String, String>,
}

/// The secrets broker.
pub struct Broker<P: PermissionCheck> {
    store: SecretStore,
    store_key: StoreKey,
    audit_key: StoreKey,
    audit: AuditLog,
    handles: BTreeMap<String, HandleRecord>,
    permissions: P,
    clock: Clock,
    handles_path: PathBuf,
    paths: BrokerPaths,
}

impl<P: PermissionCheck> std::fmt::Debug for Broker<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Broker")
            .field("store", &self.store.dir())
            .field("handles", &self.handles.len())
            .field("audit", &self.audit)
            .finish_non_exhaustive()
    }
}

impl<P: PermissionCheck> Broker<P> {
    /// Makes a new store, log and both keys, and starts the broker.
    ///
    /// # Errors
    ///
    /// Every refusal of [`StoreKey::generate`], [`SecretStore::create`] and
    /// [`AuditLog::create`].
    pub fn create(paths: &BrokerPaths, permissions: P, clock: Clock) -> Result<Self, SecretsError> {
        fs::create_dir_all(&paths.store_dir)
            .map_err(io(format!("creating {}", paths.store_dir.display())))?;
        fs::create_dir_all(&paths.log_dir)
            .map_err(io(format!("creating {}", paths.log_dir.display())))?;
        let guarded = paths.guarded();
        let store_key = StoreKey::generate(&paths.store_key, &guarded)?;
        let audit_key = StoreKey::generate(&paths.audit_key, &guarded)?;
        let store = SecretStore::create(&paths.store_dir, &store_key)?;
        let audit = AuditLog::create(
            &paths.log_dir,
            LOG_ORIGIN,
            &paths.anchor,
            &guarded,
            &audit_key,
        )?;
        let broker = Self {
            handles_path: paths.store_dir.join(HANDLES),
            paths: paths.clone(),
            store,
            store_key,
            audit_key,
            audit,
            handles: BTreeMap::new(),
            permissions,
            clock,
        };
        broker.write_handles()?;
        Ok(broker)
    }

    /// Opens an existing broker, rebuilding every lease from the audit log.
    ///
    /// # Errors
    ///
    /// `StoreKeyMissing`, `KeyFileMisplaced`, `StoreKeyMismatch`,
    /// `StoreLocked`, and every audit refusal of [`AuditLog::open`].
    pub fn open(paths: &BrokerPaths, permissions: P, clock: Clock) -> Result<Self, SecretsError> {
        let guarded = paths.guarded();
        let store_key = StoreKey::load(&paths.store_key, &guarded)?;
        let audit_key = StoreKey::load(&paths.audit_key, &guarded)?;
        let store = SecretStore::open(&paths.store_dir, &store_key)?;
        let audit = AuditLog::open(&paths.log_dir, &paths.anchor, &guarded, &audit_key)?;
        let handles_path = paths.store_dir.join(HANDLES);
        let bytes =
            fs::read(&handles_path).map_err(io(format!("reading {}", handles_path.display())))?;
        let records: Vec<HandleRecord> =
            serde_json::from_slice(&bytes).map_err(|error| SecretsError::StoreCorrupt {
                reason: format!("{HANDLES} does not read: {error}"),
            })?;
        let mut handles: BTreeMap<String, HandleRecord> = records
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect();
        for recorded in audit.replay()? {
            let line = recorded.line;
            let Some(record) = line.handle.as_ref().and_then(|id| handles.get_mut(id)) else {
                continue;
            };
            match line.kind {
                AuditKind::Drop => record.dropped = true,
                AuditKind::Use if line.outcome == "admitted" => {
                    record.used = record.used.saturating_add(1);
                    if let Some(operation) = line.operation {
                        record.operations.insert(operation, line.outcome);
                    }
                }
                _ => {}
            }
        }
        Ok(Self {
            store,
            store_key,
            audit_key,
            audit,
            handles,
            permissions,
            clock,
            handles_path,
            paths: paths.clone(),
        })
    }

    /// The sealed store, for listing entries. No secret byte is reachable.
    pub fn store(&self) -> &SecretStore {
        &self.store
    }

    /// The audit log.
    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    /// The permission source.
    pub fn permissions(&self) -> &P {
        &self.permissions
    }

    /// Seals a credential into the store and records it.
    ///
    /// # Errors
    ///
    /// Every refusal of [`SecretStore::add`].
    pub fn seal(&mut self, name: &str, owner: &str, value: &Secret) -> Result<(), SecretsError> {
        self.store
            .add(&self.store_key, name, EntryClass::Credential, owner, value)?;
        self.record(
            AuditKind::Seal,
            (None, Some(owner), Some(name)),
            None,
            None,
            "sealed",
        )?;
        Ok(())
    }

    /// Issues a handle on `secret` to `holder`, for `max_uses` uses until
    /// `not_after_ms`, under a grant that must trace to a person.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, `NoPersonRoot`, `PermissionDenied`, `InvalidLifetime`.
    pub fn issue(
        &mut self,
        holder: &Holder,
        secret: &str,
        max_uses: u64,
        not_after_ms: i64,
    ) -> Result<IssuedHandle, SecretsError> {
        if self.store.entry(secret).is_none() {
            return Err(SecretsError::SecretUnknown {
                name: secret.to_owned(),
            });
        }
        let now = (self.clock)();
        if not_after_ms <= now || max_uses == 0 {
            let asked = u128::try_from(not_after_ms.saturating_sub(now)).unwrap_or(0);
            return Err(SecretsError::InvalidLifetime {
                asked_ms: asked,
                max_ms: u128::from(u32::MAX),
            });
        }
        match self.permissions.may_use(&holder.identity, secret) {
            Ok(_permit) => {}
            Err(denied) if denied.no_person_root => {
                return Err(SecretsError::NoPersonRoot {
                    holder: holder.identity.clone(),
                });
            }
            Err(denied) => {
                return Err(SecretsError::PermissionDenied {
                    holder: holder.identity.clone(),
                    secret: secret.to_owned(),
                    reason: denied.reason,
                });
            }
        }
        let id = HandleId::generate()?;
        let token = HandleToken::generate()?;
        let record = HandleRecord {
            id: id.as_str().to_owned(),
            digest: hex(&token.digest()),
            identity: holder.identity.clone(),
            holder_key: hex(&holder.key),
            secret: secret.to_owned(),
            max_uses,
            not_after_ms,
            used: 0,
            dropped: false,
            operations: BTreeMap::new(),
        };
        self.record(
            AuditKind::Issue,
            (Some(id.as_str()), Some(&holder.identity), Some(secret)),
            None,
            Some(0),
            "issued",
        )?;
        self.handles.insert(record.id.clone(), record);
        self.write_handles()?;
        Ok(IssuedHandle { id, token })
    }

    /// Swaps `token` for its credential inside `forward`, after checking the
    /// handle, the presentation, the lease and the permission, in that order.
    /// The credential is never returned; only `forward`'s answer is.
    ///
    /// # Errors
    ///
    /// One named refusal per attempt, each recorded in the audit log.
    pub fn use_handle<R>(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        forward: impl FnOnce(&Secret) -> R,
    ) -> Result<Used<R>, UseError> {
        let operation = hex(&presentation.operation_id);
        match self.admit(token, presentation, &operation) {
            Ok(Admission::Retry(outcome)) => Ok(Used::Retried { outcome }),
            Ok(Admission::Fresh {
                id,
                identity,
                secret,
                uses_left,
                used,
            }) => {
                self.record(
                    AuditKind::Use,
                    (Some(&id), Some(&identity), Some(&secret)),
                    Some(&operation),
                    Some(used),
                    "admitted",
                )?;
                if let Some(record) = self.handles.get_mut(&id) {
                    record.used = used;
                    record.operations.insert(operation, "admitted".to_owned());
                }
                let credential =
                    self.store
                        .open_for_use(&self.store_key, &secret, EntryClass::Credential)?;
                let answer = forward(&credential);
                Ok(Used::Forwarded { answer, uses_left })
            }
            Err(refusal) => {
                let handle = self.find(token).map(|record| {
                    (
                        record.id.clone(),
                        record.identity.clone(),
                        record.secret.clone(),
                    )
                });
                let (id, identity, secret) = match &handle {
                    Some((id, identity, secret)) => (
                        Some(id.as_str()),
                        Some(identity.as_str()),
                        Some(secret.as_str()),
                    ),
                    None => (None, None, None),
                };
                self.record(
                    AuditKind::Use,
                    (id, identity, secret),
                    Some(&operation),
                    None,
                    refusal.name(),
                )?;
                Err(refusal)
            }
        }
    }

    /// Drops the handle `id`; its next presentation is refused.
    ///
    /// # Errors
    ///
    /// `HandleUnknown`, and the audit log's refusals.
    pub fn drop_handle(&mut self, id: &HandleId) -> Result<RevokeOutcome, SecretsError> {
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(SecretsError::HandleUnknown)?;
        if record.dropped {
            return Ok(RevokeOutcome::AlreadyDropped);
        }
        let (identity, secret) = (record.identity.clone(), record.secret.clone());
        self.record(
            AuditKind::Drop,
            (Some(id.as_str()), Some(&identity), Some(&secret)),
            None,
            None,
            "dropped",
        )?;
        if let Some(record) = self.handles.get_mut(id.as_str()) {
            record.dropped = true;
        }
        Ok(RevokeOutcome::Dropped)
    }

    fn record(
        &mut self,
        kind: AuditKind,
        (handle, identity, secret): Subject<'_>,
        operation: Option<&str>,
        uses: Option<u64>,
        outcome: &str,
    ) -> Result<u64, SecretsError> {
        let line = AuditLine {
            kind,
            at_ms: (self.clock)(),
            handle: handle.map(str::to_owned),
            identity: identity.map(str::to_owned),
            secret: secret.map(str::to_owned),
            operation: operation.map(str::to_owned),
            uses,
            outcome: outcome.to_owned(),
        };
        self.audit.append(&line, self.audit_key.identity())
    }

    fn write_handles(&self) -> Result<(), SecretsError> {
        let records: Vec<&HandleRecord> = self.handles.values().collect();
        let bytes =
            serde_json::to_vec_pretty(&records).map_err(|error| SecretsError::Encoding {
                context: "handle records",
                reason: error.to_string(),
            })?;
        write_atomic(&self.handles_path, &bytes)
    }
}

type Subject<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);
