//! A directory on a temporary log, a service key over a fixed seed, and a
//! store that can be told to fail an append at a chosen step.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity::Directory;
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreError, StoreResult};

/// Where the next append fails, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Nothing fails.
    None,
    /// The leaf write fails before any byte is stored.
    BeforeLeaf,
    /// The leaf is stored and the pin write fails.
    AfterLeaf,
    /// The leaf is stored, the pin write fails, and the store cannot be opened again until cleared.
    AfterLeafUnreadable,
}

/// The fault plan every store opened by one harness shares.
pub type Plan = Arc<Mutex<Fault>>;

fn fault(plan: &Plan) -> Fault {
    *plan.lock().unwrap_or_else(PoisonError::into_inner)
}

fn set(plan: &Plan, next: Fault) {
    *plan.lock().unwrap_or_else(PoisonError::into_inner) = next;
}

fn injected(context: &str) -> StoreError {
    StoreError::Io {
        context: context.to_owned(),
        source: std::io::Error::other("injected"),
    }
}

/// A file store that fails where its plan says.
pub struct FaultStore {
    inner: FileLeafStore,
    plan: Plan,
}

impl LeafStore for FaultStore {
    fn origin(&self) -> &str {
        self.inner.origin()
    }
    fn extent(&self) -> u64 {
        self.inner.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        if fault(&self.plan) == Fault::BeforeLeaf {
            set(&self.plan, Fault::None);
            return Err(injected("leaf write"));
        }
        self.inner.put_leaf(index, bytes)
    }
    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        match fault(&self.plan) {
            Fault::AfterLeaf => {
                set(&self.plan, Fault::None);
                Err(injected("pin write"))
            }
            Fault::AfterLeafUnreadable => Err(injected("pin write")),
            Fault::None | Fault::BeforeLeaf => self.inner.pin(pin),
        }
    }
}

/// One directory's temporary log and key.
pub struct Harness {
    /// The directory holding the log and the key.
    pub dir: tempfile::TempDir,
    /// The fault plan.
    pub plan: Plan,
}

/// The origin every harness log is created with.
pub const ORIGIN: &str = "example.test/lys/directory";

impl Harness {
    /// A fresh log and a service key over `seed`.
    pub fn new(seed: u8) -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        FileLeafStore::create(&dir.path().join("log"), ORIGIN)?;
        let key = dir.path().join("service.key");
        std::fs::write(&key, [seed; 32])?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            dir,
            plan: Arc::new(Mutex::new(Fault::None)),
        })
    }

    /// Where the log lives.
    pub fn log_path(&self) -> PathBuf {
        self.dir.path().join("log")
    }

    /// Fail the next append as `next` says.
    pub fn fail(&self, next: Fault) {
        set(&self.plan, next);
    }

    /// Open the directory over the log, as a restarted service would.
    pub fn open(&self) -> Result<Directory<FaultStore>, Box<dyn Error>> {
        let path = self.log_path();
        let plan = Arc::clone(&self.plan);
        let reopen = Box::new(move || {
            if fault(&plan) == Fault::AfterLeafUnreadable {
                return Err(injected("reopen"));
            }
            Ok(FaultStore {
                inner: FileLeafStore::open(Path::new(&path))?,
                plan: Arc::clone(&plan),
            })
        });
        let key = Ed25519Identity::load(&self.dir.path().join("service.key"))?;
        Ok(Directory::open(reopen, key)?)
    }
}
