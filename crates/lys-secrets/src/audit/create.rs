//! A new audit log is anchored before it can be returned.
use super::{
    AtomicU64, AuditLog, FileLeafStore, FrontierLog, Path, SecretsError, SnapshotRefusal, Start,
    StoreKey, ensure_outside, fs, io,
};
impl AuditLog {
    /// Makes a new log in `dir`, anchored at `anchor`.
    ///
    /// # Errors
    ///
    /// `KeyFileMisplaced` when the anchor lies in a guarded directory, and
    /// `Log` when the log cannot be made.
    pub fn create(
        dir: &Path,
        origin: &str,
        anchor: &Path,
        guarded: &[&Path],
        key: &StoreKey,
    ) -> Result<Self, SecretsError> {
        fs::create_dir_all(dir).map_err(io(format!("creating {}", dir.display())))?;
        ensure_outside(anchor, guarded)?;
        let (log, _tail) = FrontierLog::open(FileLeafStore::create(dir, origin)?)?;
        let mut audit = Self {
            log,
            dir: dir.to_path_buf(),
            anchor: anchor.to_path_buf(),
            verifying_key: key.verifying_key(),
            head: [0u8; 32],
            start: Start::Rebuilt {
                refusal: SnapshotRefusal::Missing,
                replayed: 0,
            },
            anchor_writes: 0,
            lines_read: AtomicU64::new(0),
            every_line_audits: AtomicU64::new(0),
        };
        audit.write_anchor(key.identity())?;
        Ok(audit)
    }
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLog")
            .field("origin", &self.log.origin())
            .field("len", &self.log.len())
            .finish_non_exhaustive()
    }
}
