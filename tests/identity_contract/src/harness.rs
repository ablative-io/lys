//! A directory on a temporary log, a service key over a fixed seed, a store
//! that can be told to fail an append at a chosen step, and the directory
//! service started on a local port behind the fake issuer.
//!
//! The issuer is set up as the install sets it up: it names itself on the
//! service's own origin, under `/auth/v1`, and answers only on a loopback
//! port of its own, which the service reaches through `sign_in_api`. So a
//! test that finds the issuer's loopback address or port in anything a
//! browser is given finds what production would give.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use lys_core::clock::{Clock, ClockError, ClockSource, UtcInstant};

use lys_core::Ed25519Identity;
use lys_identity::Directory;
use lys_identity_server::{Config, Say};
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreError, StoreResult};

use crate::fake_issuer::FakeIssuer;
use crate::harness_serve::DropMarker;
pub use crate::harness_serve::StageTimer;

/// A separately owned synchronous fixture clock; no wall-time scheduling.
#[derive(Debug)]
pub struct ManualClock {
    seconds: AtomicI64,
    refused: AtomicBool,
    reads: AtomicU64,
}

impl ManualClock {
    /// Construct one provider at an explicit whole Unix second.
    #[must_use]
    pub fn new(seconds: i64) -> Self {
        Self {
            seconds: AtomicI64::new(seconds),
            refused: AtomicBool::new(false),
            reads: AtomicU64::new(0),
        }
    }

    /// Assign the next reading synchronously.
    pub fn set(&self, seconds: i64) {
        self.seconds.store(seconds, Ordering::SeqCst);
    }

    /// Explicitly refuse or admit provider reads.
    pub fn refuse(&self, refused: bool) {
        self.refused.store(refused, Ordering::SeqCst);
    }

    /// Provider calls completed by this independently owned fixture.
    #[must_use]
    pub fn reads(&self) -> u64 {
        self.reads.load(Ordering::SeqCst)
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Result<UtcInstant, ClockError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.refused.load(Ordering::SeqCst) {
            return Err(ClockError::Unavailable {
                reason: "fixture read refused".to_owned(),
            });
        }
        UtcInstant::from_timestamp(self.seconds.load(Ordering::SeqCst), 0)
            .ok_or(ClockError::InstantOutOfRange { source: None })
    }
}

struct ClockServing {
    say: Option<Say>,
    clock: ClockSource,
}

/// Where the next append fails, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Nothing fails.
    None,
    /// The append fails before any byte is stored.
    BeforeLeaf,
    /// The act is stored whole, leaves and pin, and the store still answers
    /// the write failed, as a store does when its one flush returned but the
    /// answer was lost.
    LeafStoredWriteFailed,
    /// The act is stored whole and the answer names the pin as what failed;
    /// the store writes leaves and pin as one act, so this is the same lost
    /// answer under the other name, kept so every caller's handling of it is
    /// exercised. A leaf stored without its pin is not a state the store
    /// produces.
    AfterLeaf,
    /// The act is stored whole, the answer is lost, and the store cannot be
    /// read again until cleared.
    AfterLeafUnreadable,
}

/// The fault plan every store opened by one harness shares.
pub type Plan = Arc<Mutex<Fault>>;

fn fault(plan: &Plan) -> StoreResult<Fault> {
    plan.lock()
        .map(|held| *held)
        .map_err(|error| StoreError::Io {
            context: "fixture_lock_poisoned".to_owned(),
            source: std::io::Error::other(error.to_string()),
        })
}

fn set(plan: &Plan, next: Fault) -> StoreResult<()> {
    let mut held = plan.lock().map_err(|error| StoreError::Io {
        context: "fixture_lock_poisoned".to_owned(),
        source: std::io::Error::other(error.to_string()),
    })?;
    *held = next;
    Ok(())
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
    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        match fault(&self.plan)? {
            Fault::BeforeLeaf => {
                set(&self.plan, Fault::None)?;
                Err(injected("leaf write"))
            }
            Fault::LeafStoredWriteFailed => {
                set(&self.plan, Fault::None)?;
                self.inner.append(index, leaves, pin)?;
                Err(injected("leaf durability"))
            }
            Fault::AfterLeaf => {
                set(&self.plan, Fault::None)?;
                self.inner.append(index, leaves, pin)?;
                Err(injected("pin write"))
            }
            Fault::AfterLeafUnreadable => {
                self.inner.append(index, leaves, pin)?;
                Err(injected("pin write"))
            }
            Fault::None => self.inner.append(index, leaves, pin),
        }
    }
    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        match fault(&self.plan)? {
            Fault::AfterLeafUnreadable => Err(injected("pin write")),
            Fault::None | Fault::BeforeLeaf | Fault::LeafStoredWriteFailed | Fault::AfterLeaf => {
                self.inner.pin(pin)
            }
        }
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
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
    ///
    /// # Errors
    /// Propagates fixture directory, key, log or session setup failures.
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
    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.dir.path().join("log")
    }

    /// Fail the next append as `next` says.
    ///
    /// # Errors
    /// Returns the named fixture fault if a state lock is poisoned.
    pub fn fail(&self, next: Fault) -> StoreResult<()> {
        set(&self.plan, next)
    }

    /// A way to open the log's leaf store that fails where the plan says.
    #[must_use]
    pub fn leaves(&self) -> Box<dyn Fn() -> StoreResult<FaultStore> + Send> {
        let path = self.log_path();
        let plan = Arc::clone(&self.plan);
        Box::new(move || {
            if fault(&plan)? == Fault::AfterLeafUnreadable {
                return Err(injected("reopen"));
            }
            Ok(FaultStore {
                inner: FileLeafStore::open(Path::new(&path))?,
                plan: Arc::clone(&plan),
            })
        })
    }

    /// Open the directory over the log, as a restarted service would.
    ///
    /// # Errors
    /// Propagates fixture directory, key, log or session setup failures.
    pub fn open(&self) -> Result<Directory<FaultStore>, Box<dyn Error>> {
        let path = self.log_path();
        let plan = Arc::clone(&self.plan);
        let reopen = Box::new(move || {
            if fault(&plan)? == Fault::AfterLeafUnreadable {
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

/// The grant log's origin in every started service.
pub const GRANT_ORIGIN: &str = "example.test/lys/grants";

/// The membership page ceiling every started service is configured with:
/// test settings, chosen for these fixtures, never an install's number.
pub const MEMBERSHIP_PAGE_ROWS_MAX: u32 = 64;
/// The membership page byte ceiling every started service is configured with.
pub const MEMBERSHIP_PAGE_BYTES_MAX: u32 = 65_536;
/// How long every started service keeps a membership cursor, in seconds.
pub const MEMBERSHIP_CURSOR_SECONDS: u64 = 600;
/// How many membership cursors every started service keeps at once.
pub const MEMBERSHIP_CURSORS_MAX: u32 = 64;
/// The permission model every started service judges grants against. Its
/// relation names say nothing of their actions: `alpha` carries read and
/// write, `beta` carries read alone; `administrator` carries the four
/// organisation acts, so founding an organisation issues its founder's grant
/// here as the shipped model does (DIRECTORY-109 R3).
pub const GRANT_MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"],"administrator":["organisation.administer","organisation.admit","organisation.grant","organisation.read"]}}"#;

/// The administrator's subject at the fake issuer.
pub const ADMINISTRATOR: &str = "administrator-subject";
/// The link-audit source's subject at the fake issuer.
pub const LINK_AUDIT_SOURCE: &str = "link-audit-source-subject";

/// A started directory service on a local port, signing in through a fake issuer.
pub struct Service {
    drop_start: DropMarker,
    /// The service's base URL.
    pub base: String,
    /// Process-wide log-store flush attempts during startup; exact in an isolated test.
    pub startup_flushes: u64,
    /// The issuer people sign in through.
    pub issuer: FakeIssuer,
    drop_issuer: DropMarker,
    client: reqwest::Client,
    drop_client: DropMarker,
    /// Holds the log, keys and secret file for the service's life.
    pub dir: tempfile::TempDir,
    drop_dir: DropMarker,
    config: Config,
    clock: ClockSource,
    drop_config: DropMarker,
    server: crate::harness_serve::Serving,
    /// Writes the stores receipt when the service stops cleanly, as the
    /// service's own stop does (DIRECTORY-107 R2).
    at_stop: Option<lys_identity_server::stores_receipt::AtStop>,
    drop_server: DropMarker,
}

impl Drop for Service {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("identity_service_shutdown_failed: {error}");
        }
        self.drop_start.arm();
        self.drop_issuer.arm();
        self.drop_client.arm();
        self.drop_dir.arm();
        self.drop_config.arm();
        self.drop_server.arm();
    }
}

fn secret_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, bytes)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// The password the harness types for a login the issuer was told to sign.
pub const HARNESS_PASSWORD: &str = "Harness-Password-2026";

/// The session cookie an answer began, or its refusal as an error.
///
/// # Errors
/// Propagates fixture directory, key, log or session setup failures.
pub async fn session_cookie(signed_in: reqwest::Response) -> Result<String, Box<dyn Error>> {
    let timing = StageTimer::new("http.session_cookie");
    let cookie = signed_in
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .map(|value| value.to_str().map(str::to_owned));
    let (status, body) = answer(signed_in).await?;
    let Some(cookie) = cookie else {
        return Err(format!("sign-in answered {status} without a session: {body}").into());
    };
    let result = Ok(cookie?
        .split(';')
        .next()
        .ok_or("the session cookie is empty")?
        .to_owned());
    drop(timing);
    result
}

fn location(answer: &reqwest::Response) -> Result<String, Box<dyn Error>> {
    Ok(answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or_else(|| format!("{} answered no redirect", answer.url()))?
        .to_str()?
        .to_owned())
}

/// A status and a JSON body.
pub type Answer = (u16, serde_json::Value);

async fn answer(response: reqwest::Response) -> Result<Answer, Box<dyn Error>> {
    let timing = StageTimer::new("http.answer_body");
    let status = response.status().as_u16();
    let text = response.text().await?;
    let answer = Ok((status, serde_json::from_str(&text)?));
    drop(timing);
    answer
}

#[path = "harness_requests.rs"]
mod requests;
#[path = "harness_startup.rs"]
mod startup;
pub use startup::with_administrator;

impl Service {
    /// The active signing key path, so runner clients authenticate as this service.
    #[must_use]
    pub fn event_key_file(&self) -> &Path {
        &self.config.event_key_file
    }
}
