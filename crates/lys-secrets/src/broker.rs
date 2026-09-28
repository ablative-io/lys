//! The broker: issues handles under a person's grant, swaps a presented
//! handle for its credential only inside the forwarding closure, and writes
//! one signed audit line for every issue, use, refusal and drop. Use counts,
//! drops and retry outcomes are what the audit log says, so a broker killed
//! at any point reopens to it. A start reads them from the log's signed
//! snapshot and the lines after it, never from the whole log (see `restart`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::audit::{AuditKind, AuditLine, AuditLog};
use crate::encoding::hex;
use crate::error::SecretsError;
use crate::fsutil::{io, write_atomic};
use crate::handle::{HandleId, HandleToken, Holder, IssuedHandle};
use crate::keys::StoreKey;
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::{EntryClass, SecretStore};

mod accounts;
mod admit;
mod folded;
mod inflight;
mod lineage;
mod oauth_grants;
mod owner;
pub use owner::OwnerChanged;
mod records;
mod restart;
pub use restart::{SNAPSHOT_EVERY, SnapshotReport};
mod revocation;
pub use revocation::{RevocationState, UpstreamRevocation};
mod rotation;
mod scope;
pub use scope::SecretSettings;
mod spawn;
mod using;

pub use using::{Admitted, Ticket};

const ROTATING: &str = "rotating ";
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
    /// Each operation id this handle was used under: its first outcome and
    /// the mark of the request it was for.
    #[serde(skip)]
    operations: BTreeMap<String, (String, String)>,
    /// The most the lease may spend, when it is capped.
    #[serde(default)]
    spend_cap: Option<u64>,
    /// Spend settled so far.
    #[serde(skip)]
    settled: u64,
    /// Reservations admitted and not yet settled, by operation id.
    #[serde(skip)]
    open: BTreeMap<String, u64>,
    /// The handle this one was derived from.
    #[serde(default)]
    parent: Option<String>,
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
    /// Every (identity, record) pair the log shows read, so a refusal after
    /// a relation's removal is named `RelationRemoved`.
    readers: BTreeSet<(String, String)>,
    /// Each secret's owner changes the log shows applied.
    owners: owner::Owners,
    /// The fold of the log's lines at the last snapshot.
    sealed: restart::Sealed,
    /// How many lines the log grows by between snapshots.
    every: NonZeroU64,
    /// Why the last snapshot could not be written, while no later one was.
    snapshot_failure: Option<String>,
    /// Who is told when a snapshot cannot be written.
    snapshot_report: Option<SnapshotReport>,
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
            readers: BTreeSet::new(),
            owners: owner::Owners::new(),
            sealed: restart::Sealed::default(),
            every: SNAPSHOT_EVERY,
            snapshot_failure: None,
            snapshot_report: None,
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
    /// As [`Broker::issue_capped`].
    pub fn issue(
        &mut self,
        holder: &Holder,
        secret: &str,
        max_uses: u64,
        not_after_ms: i64,
    ) -> Result<IssuedHandle, SecretsError> {
        self.issue_capped(holder, secret, max_uses, not_after_ms, None)
    }

    /// Issues a handle as [`Broker::issue`] does, with a hard cap on what
    /// its calls may spend when `spend_cap` is given: each call reserves
    /// before it is forwarded and settles after.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, `NoPersonRoot`, `PermissionDenied`, `InvalidLifetime`.
    pub fn issue_capped(
        &mut self,
        holder: &Holder,
        secret: &str,
        max_uses: u64,
        not_after_ms: i64,
        spend_cap: Option<u64>,
    ) -> Result<IssuedHandle, SecretsError> {
        self.discoverable(&holder.identity, secret)?;
        match self.store.entry(secret).map(|entry| entry.class) {
            None => {
                return Err(SecretsError::SecretUnknown {
                    name: secret.to_owned(),
                });
            }
            Some(EntryClass::Memory) => {
                return Err(SecretsError::MemoryNotUsable {
                    record: secret.to_owned(),
                });
            }
            Some(EntryClass::Credential | EntryClass::Key | EntryClass::OAuth) => {}
        }
        let now = (self.clock)();
        if not_after_ms <= now || max_uses == 0 {
            let asked = u128::try_from(not_after_ms.saturating_sub(now)).unwrap_or(0);
            return Err(SecretsError::InvalidLifetime {
                asked_ms: asked,
                max_ms: u128::from(u32::MAX),
            });
        }
        self.recipient_admitted(&holder.identity, secret)?;
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
            spend_cap,
            settled: 0,
            open: BTreeMap::new(),
            parent: None,
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
        subject: Subject<'_>,
        call: Option<(&str, &str)>,
        uses: Option<u64>,
        outcome: &str,
    ) -> Result<u64, SecretsError> {
        let line = self.line(kind, subject, call, uses, outcome);
        self.append(&line)
    }

    fn append(&mut self, line: &AuditLine) -> Result<u64, SecretsError> {
        let before = self.audit.len();
        let index = self.audit.append(line, self.audit_key.identity())?;
        self.snapshot_if_due(before);
        Ok(index)
    }

    fn line(
        &self,
        kind: AuditKind,
        (handle, identity, secret): Subject<'_>,
        call: Option<(&str, &str)>,
        uses: Option<u64>,
        outcome: &str,
    ) -> AuditLine {
        AuditLine {
            kind,
            at_ms: (self.clock)(),
            handle: handle.map(str::to_owned),
            identity: identity.map(str::to_owned),
            secret: secret.map(str::to_owned),
            operation: call.map(|(operation, _mark)| operation.to_owned()),
            request: call.map(|(_operation, mark)| mark.to_owned()),
            uses,
            spend: None,
            outcome: outcome.to_owned(),
        }
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
