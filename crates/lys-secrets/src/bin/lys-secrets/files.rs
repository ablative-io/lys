//! Where the command line keeps things, the upstream each secret is bound
//! to, and the grants file every check reads afresh.
//!
//! The routes and the trusted screen services are read once and held (see
//! `watched`); the grants file is read afresh by every check, except inside
//! one request's [`FileGrants::pinned`] span, where it is read once.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use lys_secrets::{
    BrokerPaths, Denied, PermissionCheck, Permitted, Relation, SecretsError, ServiceKey,
};
use serde::{Deserialize, Serialize};

use crate::watched::Watched;

/// The broker's clock.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

fn io_error(context: String) -> impl FnOnce(std::io::Error) -> SecretsError {
    move |source| SecretsError::Io { context, source }
}

fn json_error(context: &'static str) -> impl FnOnce(serde_json::Error) -> SecretsError {
    move |error| SecretsError::Encoding {
        context,
        reason: error.to_string(),
    }
}

/// Where a secret may be sent, and how.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// The only origin the credential goes to.
    pub upstream: String,
    /// The header that carries it.
    pub header: String,
    /// Text before the credential in that header.
    pub prefix: String,
    /// The upstream answer header that reports what a call spent, for a
    /// lease with a spend cap. With none, a call settles at its reservation.
    #[serde(default)]
    pub spend_header: Option<String>,
}

/// Every route, by secret, each held once and shared.
pub type Routes = BTreeMap<String, Arc<Route>>;

/// The broker's folders, and the routes and services files as last read.
#[derive(Debug, Clone)]
pub struct Layout {
    root: PathBuf,
    keys: PathBuf,
    /// `routes.json`, read once and again only when it changes.
    pub(crate) routes: Arc<Watched<Routes>>,
    /// `services.json`, read once and again only when it changes.
    pub(crate) services: Arc<Watched<Vec<ServiceKey>>>,
}

impl Layout {
    pub fn new(root: &Path, keys: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            keys: keys.to_path_buf(),
            routes: Arc::new(Watched::new(root.join(ROUTES))),
            services: Arc::new(Watched::new(root.join(SERVICES))),
        }
    }

    pub fn prepare(&self) -> Result<(), SecretsError> {
        for dir in [&self.root, &self.keys] {
            fs::create_dir_all(dir).map_err(io_error(format!("creating {}", dir.display())))?;
        }
        Ok(())
    }

    pub fn paths(&self) -> BrokerPaths {
        BrokerPaths {
            store_dir: self.root.join("store"),
            log_dir: self.root.join("audit-log"),
            store_key: self.keys.join("store.key"),
            audit_key: self.keys.join("audit.key"),
            anchor: self.keys.join("audit.anchor"),
        }
    }

    pub fn grants(&self) -> PathBuf {
        Self::grants_in(&self.root)
    }

    pub fn grants_in(root: &Path) -> PathBuf {
        root.join("grants.json")
    }

    /// Every route, as `routes.json` holds it now. The file is read the
    /// first time and again only when it has changed since.
    pub fn routes(&self) -> Result<Arc<Routes>, SecretsError> {
        self.routes.get(|bytes| {
            let routes: BTreeMap<String, Route> =
                serde_json::from_slice(bytes).map_err(json_error("routes file"))?;
            Ok(routes
                .into_iter()
                .map(|(secret, route)| (secret, Arc::new(route)))
                .collect())
        })
    }

    /// The screen services the broker trusts. None are trusted until one
    /// is added. The file is read the first time and again only when it has
    /// changed since.
    pub fn services(&self) -> Result<Vec<ServiceKey>, SecretsError> {
        let held = self
            .services
            .get(|bytes| serde_json::from_slice(bytes).map_err(json_error("services file")))?;
        Ok(Vec::clone(&held))
    }

    /// Trusts the screen service `service`, replacing its key if it was
    /// trusted already.
    pub fn trust_service(&self, service: ServiceKey) -> Result<(), SecretsError> {
        let mut services = self.services()?;
        services.retain(|known| known.name != service.name);
        services.push(service);
        let bytes = serde_json::to_vec_pretty(&services).map_err(json_error("services file"))?;
        self.services.write(&bytes)
    }

    pub fn add_route(&self, secret: &str, route: Route) -> Result<(), SecretsError> {
        let mut routes: BTreeMap<String, Route> = self
            .routes()?
            .iter()
            .map(|(name, held)| (name.clone(), Route::clone(held)))
            .collect();
        routes.insert(secret.to_owned(), route);
        let bytes = serde_json::to_vec_pretty(&routes).map_err(json_error("routes file"))?;
        self.routes.write(&bytes)
    }
}

const ROUTES: &str = "routes.json";
const SERVICES: &str = "services.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GrantRow {
    identity: String,
    secret: String,
    granted_by: Option<String>,
    #[serde(default = "use_relation")]
    relation: String,
    /// When the grant's window ends, in milliseconds since the epoch, when
    /// it has an end.
    #[serde(default)]
    ends_at_ms: Option<i64>,
}

fn use_relation() -> String {
    Relation::Use.label().to_owned()
}

/// A grant as (identity, secret, relation, granted by).
pub type GrantView = (String, String, String, Option<String>);

/// Grants kept in a file and read afresh on every check, so a revocation
/// takes effect at the next use with no restart. Inside one request's
/// [`FileGrants::pinned`] span the file is read once, by the first check.
#[derive(Debug)]
pub struct FileGrants {
    path: PathBuf,
    /// The rows read inside the pinned span in hand, when one is.
    pin: Mutex<Pin>,
    /// How many checks were asked.
    pub(crate) checks: AtomicU64,
    /// How many times the file was read and parsed.
    pub(crate) parses: AtomicU64,
}

/// Whether a pinned span is open, and the rows read in it.
#[derive(Debug, Default)]
struct Pin {
    open: bool,
    rows: Option<Arc<[GrantRow]>>,
}

struct Span<'a> {
    grants: &'a FileGrants,
    open: bool,
}

impl Drop for Span<'_> {
    fn drop(&mut self) {
        if self.open
            && let Err(error) = self.grants.pin_open(false)
        {
            eprintln!("lys-secrets: closing grants span: {error}");
        }
    }
}

impl Clone for FileGrants {
    /// The same file, with no span pinned and nothing counted.
    fn clone(&self) -> Self {
        Self::new(self.path.clone())
    }
}

impl FileGrants {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            pin: Mutex::new(Pin::default()),
            checks: AtomicU64::new(0),
            parses: AtomicU64::new(0),
        }
    }

    /// Runs `work` with the grants file read at most once, by the first
    /// check `work` asks: one request's checks all see one reading of it.
    pub fn pinned<T>(&self, work: impl FnOnce() -> T) -> Result<T, SecretsError> {
        self.pin_open(true)?;
        let mut span = Span {
            grants: self,
            open: true,
        };
        let done = work();
        self.pin_open(false)?;
        span.open = false;
        Ok(done)
    }

    /// Opens or closes the pinned span, and lets go of any rows read in it.
    fn pin_open(&self, open: bool) -> Result<(), SecretsError> {
        let mut pin = self
            .pin
            .lock()
            .map_err(|error| SecretsError::StatePoisoned {
                reason: error.to_string(),
            })?;
        *pin = Pin { open, rows: None };
        Ok(())
    }

    fn rows(&self) -> Result<Arc<[GrantRow]>, SecretsError> {
        let mut pin = self
            .pin
            .lock()
            .map_err(|error| SecretsError::StatePoisoned {
                reason: error.to_string(),
            })?;
        if let Some(rows) = &pin.rows {
            return Ok(Arc::clone(rows));
        }
        let rows: Arc<[GrantRow]> = Arc::from(self.read()?);
        if pin.open {
            pin.rows = Some(Arc::clone(&rows));
        }
        Ok(rows)
    }

    fn read(&self) -> Result<Vec<GrantRow>, SecretsError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        self.parses.fetch_add(1, Ordering::Relaxed);
        let bytes =
            fs::read(&self.path).map_err(io_error(format!("reading {}", self.path.display())))?;
        serde_json::from_slice(&bytes).map_err(json_error("grants file"))
    }

    fn write(&self, rows: &[GrantRow]) -> Result<(), SecretsError> {
        let bytes = serde_json::to_vec_pretty(rows).map_err(json_error("grants file"))?;
        let mut pin = self
            .pin
            .lock()
            .map_err(|error| SecretsError::StatePoisoned {
                reason: error.to_string(),
            })?;
        pin.rows = None;
        drop(pin);
        fs::write(&self.path, bytes).map_err(io_error(format!("writing {}", self.path.display())))
    }

    pub fn set(
        &self,
        relation: Relation,
        identity: &str,
        secret: &str,
        by: Option<&str>,
    ) -> Result<(), SecretsError> {
        let mut rows = self.rows()?.to_vec();
        rows.retain(|row| !row.is(relation, identity, secret));
        rows.push(GrantRow {
            identity: identity.to_owned(),
            secret: secret.to_owned(),
            granted_by: by.map(str::to_owned),
            relation: relation.label().to_owned(),
            ends_at_ms: None,
        });
        self.write(&rows)
    }

    /// Every grant as (identity, secret, relation, granted by).
    pub fn list(&self) -> Result<Vec<GrantView>, SecretsError> {
        Ok(self
            .rows()?
            .iter()
            .map(|row| {
                (
                    row.identity.clone(),
                    row.secret.clone(),
                    row.relation.clone(),
                    row.granted_by.clone(),
                )
            })
            .collect())
    }

    pub fn remove(
        &self,
        relation: Relation,
        identity: &str,
        secret: &str,
    ) -> Result<(), SecretsError> {
        let mut rows = self.rows()?.to_vec();
        rows.retain(|row| !row.is(relation, identity, secret));
        self.write(&rows)
    }

    fn check(&self, relation: Relation, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        self.checks.fetch_add(1, Ordering::Relaxed);
        let rows = self.rows().map_err(|error| Denied {
            reason: format!("the grants file does not read: {error}"),
            no_person_root: false,
        })?;
        match rows.iter().find(|row| row.is(relation, identity, secret)) {
            Some(GrantRow {
                granted_by: Some(person),
                ends_at_ms,
                ..
            }) => Ok(Permitted {
                person: person.clone(),
                ends_at_ms: *ends_at_ms,
            }),
            Some(GrantRow {
                granted_by: None, ..
            }) => Err(Denied {
                reason: "the grant traces to no person".to_owned(),
                no_person_root: true,
            }),
            None => Err(Denied {
                reason: format!("no {} relation", relation.label()),
                no_person_root: false,
            }),
        }
    }
}

impl GrantRow {
    fn is(&self, relation: Relation, identity: &str, secret: &str) -> bool {
        self.relation == relation.label() && self.identity == identity && self.secret == secret
    }
}

impl PermissionCheck for FileGrants {
    fn may_use(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Use, identity, secret)
    }

    fn may_read(&self, identity: &str, record: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Read, identity, record)
    }

    fn may_lend(&self, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Lend, identity, secret)
    }

    fn member_of(&self, identity: &str, target: &str) -> Result<Permitted, Denied> {
        self.check(Relation::Member, identity, target)
    }
}

#[cfg(test)]
mod poison_tests {
    use super::*;

    #[test]
    fn poisoned_file_grants_refuse_checks_and_changes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("grants");
        let grants = FileGrants::new(path.clone());
        grants.set(Relation::Use, "agent", "secret", Some("person"))?;
        let durable = fs::read(&path)?;
        let interrupted = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let mut pin = grants.pin.lock().expect("healthy initial grants cache");
                    pin.rows = Some(Arc::from(Vec::<GrantRow>::new()));
                    panic!("interrupted grants cache mutation");
                })
                .join()
        });
        assert!(interrupted.is_err());
        assert!(matches!(
            grants.list(),
            Err(SecretsError::StatePoisoned { .. })
        ));
        assert!(matches!(
            grants.set(Relation::Use, "other", "secret", Some("person")),
            Err(SecretsError::StatePoisoned { .. })
        ));
        let refused = grants
            .check(Relation::Use, "agent", "secret")
            .expect_err("poisoned cache cannot authorise");
        assert!(refused.reason.contains("StatePoisoned:"));
        assert_eq!(fs::read(path)?, durable);
        Ok(())
    }

    #[test]
    fn interrupted_pinned_span_discards_the_previous_reading()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("grants");
        let grants = FileGrants::new(path.clone());
        grants.set(Relation::Use, "agent", "secret", Some("person"))?;
        let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _answer = grants.pinned(|| {
                assert_eq!(grants.list().expect("initial grant rows").len(), 1);
                panic!("interrupted grants span");
            });
        }));
        assert!(interrupted.is_err());
        fs::write(path, b"[]")?;
        assert!(grants.list()?.is_empty());
        Ok(())
    }
}
