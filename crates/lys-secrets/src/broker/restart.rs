//! Starting the broker without reading its whole audit log, and the
//! snapshots that make that possible.
//!
//! The broker's leases, readers, unfinished rotation and owner changes are
//! a fold of the audit log. A snapshot holds that fold at one tree size,
//! signed by the audit key and bound to the log's root at that size. A
//! start loads it and folds only the lines after it, each with its
//! signature checked.
//!
//! A snapshot that is missing, does not verify, is of another kind or
//! another log, or is not at the log's root is refused by name in
//! [`Broker::start`]. So is one whose state does not read. Nothing of a
//! refused snapshot is used: every line is folded, and a new snapshot is
//! written before the broker answers anything.
//!
//! A snapshot is written when the log crosses a multiple of a count of
//! lines. What it seals is folded from the log's own lines since the last
//! snapshot, never copied from what the broker holds in memory, so it cannot
//! run ahead of or behind the lines it is bound to. One that cannot be
//! written does not undo the line it follows, which is already durable: the
//! failure is reported by name and kept until a later snapshot succeeds.

use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroU64;
use std::path::Path;

use lys_log_store::Start;

use crate::audit::AuditLog;
use crate::error::SecretsError;
use crate::fsutil::io;
use crate::keys::StoreKey;
use crate::permission::PermissionCheck;
use crate::store::SecretStore;

use super::folded::{Folded, Handles};
use super::{Broker, BrokerPaths, Clock, HANDLES, HandleRecord};

/// How many lines the audit log grows by between snapshots, unless the
/// broker is opened with another count.
pub const SNAPSHOT_EVERY: NonZeroU64 = NonZeroU64::MIN.saturating_add(1023);

/// Told, by name, why a snapshot could not be written.
pub type SnapshotReport = Box<dyn Fn(&str) + Send + Sync>;

/// The fold of the audit log's first `size` lines.
#[derive(Debug, Default)]
pub(super) struct Sealed {
    size: u64,
    state: Folded,
}

fn read_handles(path: &Path) -> Result<Handles, SecretsError> {
    let bytes = fs::read(path).map_err(io(format!("reading {}", path.display())))?;
    let records: Vec<HandleRecord> =
        serde_json::from_slice(&bytes).map_err(|error| SecretsError::StoreCorrupt {
            reason: format!("{HANDLES} does not read: {error}"),
        })?;
    Ok(records
        .into_iter()
        .map(|record| (record.id.clone(), record))
        .collect::<BTreeMap<_, _>>())
}

impl<P: PermissionCheck> Broker<P> {
    /// Opens an existing broker from its audit log's snapshot and the lines
    /// after it, writing a snapshot every [`SNAPSHOT_EVERY`] lines.
    ///
    /// # Errors
    ///
    /// As [`Broker::open_every`].
    pub fn open(paths: &BrokerPaths, permissions: P, clock: Clock) -> Result<Self, SecretsError> {
        Self::open_every(paths, permissions, clock, SNAPSHOT_EVERY)
    }

    /// Opens an existing broker from its audit log's snapshot and the lines
    /// after it, writing a snapshot every `every` lines. When the snapshot
    /// is refused every line is folded, and a new snapshot is written.
    ///
    /// # Errors
    ///
    /// `StoreKeyMissing`, `KeyFileMisplaced`, `StoreKeyMismatch`,
    /// `StoreLocked`, and every audit refusal of [`AuditLog::open`].
    pub fn open_every(
        paths: &BrokerPaths,
        permissions: P,
        clock: Clock,
        every: NonZeroU64,
    ) -> Result<Self, SecretsError> {
        let guarded = paths.guarded();
        let store_key = StoreKey::load(&paths.store_key, &guarded)?;
        let audit_key = StoreKey::load(&paths.audit_key, &guarded)?;
        let store = SecretStore::open(&paths.store_dir, &store_key)?;
        let (audit, opened) = AuditLog::open(&paths.log_dir, &paths.anchor, &guarded, &audit_key)?;
        let (audit, state, lines) = match opened.state.as_deref().map(Folded::decode) {
            None => (audit, Folded::default(), opened.lines),
            Some(Ok(state)) => (audit, state, opened.lines),
            Some(Err(unreadable)) => {
                let (audit, opened) = audit.rebuild(unreadable.to_string())?;
                (audit, Folded::default(), opened.lines)
            }
        };
        let handles_path = paths.store_dir.join(HANDLES);
        let mut handles = read_handles(&handles_path)?;
        state.lay_over(&mut handles);
        let (mut readers, mut rotating, mut owners) = (state.readers, state.rotating, state.owners);
        let folded = u64::try_from(lines.len()).unwrap_or(u64::MAX);
        let from = audit.len().saturating_sub(folded);
        for recorded in lines {
            Folded::apply(
                &mut handles,
                &mut readers,
                &mut rotating,
                &mut owners,
                recorded.line,
            );
        }
        let sealed = Sealed {
            size: audit.len(),
            state: Folded::of(&handles, &readers, rotating.as_deref(), &owners),
        };
        let rebuilt = audit.start().refusal().is_some();
        let mut broker = Self {
            store,
            store_key,
            audit_key,
            audit,
            handles,
            permissions,
            clock,
            handles_path,
            paths: paths.clone(),
            readers,
            owners,
            sealed,
            every,
            snapshot_failure: None,
            snapshot_report: None,
        };
        if rebuilt {
            broker.write_snapshot()?;
        } else {
            broker.snapshot_if_due(from);
        }
        broker.settle_rotation(rotating.as_deref())?;
        broker.settle_unknown_outcomes()?;
        Ok(broker)
    }

    /// How the broker was started: from its snapshot and how many lines it
    /// read after it, or from every line and the refusal that made it so.
    pub fn start(&self) -> &Start {
        self.audit.start()
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// Names who is told when a snapshot cannot be written.
    pub fn report_snapshots_to(&mut self, report: SnapshotReport) {
        self.snapshot_report = Some(report);
    }

    /// What the broker holds of its audit log now, each lease's counts,
    /// every reader and each secret's owner changes, in the encoding a
    /// snapshot carries. It holds no secret, token, token digest or key.
    ///
    /// # Errors
    ///
    /// `Encoding`.
    pub fn folded(&self) -> Result<Vec<u8>, SecretsError> {
        Folded::of(&self.handles, &self.readers, None, &self.owners).encode()
    }

    /// Writes a snapshot when the log has crossed a multiple of the count
    /// since it held `before` lines. A failure is reported and kept by name.
    pub(super) fn snapshot_if_due(&mut self, before: u64) {
        let every = self.every.get();
        if self.audit.len() / every <= before / every {
            return;
        }
        if let Err(failure) = self.write_snapshot() {
            let reason = format!("SnapshotNotWritten: {failure}");
            if let Some(report) = &self.snapshot_report {
                report(&reason);
            }
            self.snapshot_failure = Some(reason);
        }
    }

    /// Folds the lines written since the last snapshot into its state, and
    /// seals the result at the log's size.
    fn write_snapshot(&mut self) -> Result<(), SecretsError> {
        let lines = self.audit.lines_from(self.sealed.size)?;
        let mut handles = self.handles.clone();
        self.sealed.state.lay_over(&mut handles);
        let mut readers = self.sealed.state.readers.clone();
        let mut rotating = self.sealed.state.rotating.clone();
        let mut owners = self.sealed.state.owners.clone();
        for recorded in lines {
            Folded::apply(
                &mut handles,
                &mut readers,
                &mut rotating,
                &mut owners,
                recorded.line,
            );
        }
        let state = Folded::of(&handles, &readers, rotating.as_deref(), &owners);
        let size = self
            .audit
            .write_snapshot(&state.encode()?, self.audit_key.identity())?;
        self.sealed = Sealed { size, state };
        self.snapshot_failure = None;
        Ok(())
    }
}
