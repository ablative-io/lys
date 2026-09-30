//! Where the command line keeps things, the upstream each secret is bound
//! to, and the grants file every check reads afresh.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lys_secrets::{
    BrokerPaths, Denied, PermissionCheck, Permitted, Relation, SecretsError, ServiceKey,
};
use serde::{Deserialize, Serialize};

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

/// The broker's folders.
#[derive(Debug, Clone)]
pub struct Layout {
    root: PathBuf,
    keys: PathBuf,
}

impl Layout {
    pub fn new(root: &Path, keys: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            keys: keys.to_path_buf(),
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

    fn routes_path(&self) -> PathBuf {
        self.root.join("routes.json")
    }

    pub fn routes(&self) -> Result<BTreeMap<String, Route>, SecretsError> {
        let path = self.routes_path();
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let bytes = fs::read(&path).map_err(io_error(format!("reading {}", path.display())))?;
        serde_json::from_slice(&bytes).map_err(json_error("routes file"))
    }

    fn services_path(&self) -> PathBuf {
        self.root.join("services.json")
    }

    /// The screen services the broker trusts. None are trusted until one
    /// is added.
    pub fn services(&self) -> Result<Vec<ServiceKey>, SecretsError> {
        let path = self.services_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path).map_err(io_error(format!("reading {}", path.display())))?;
        serde_json::from_slice(&bytes).map_err(json_error("services file"))
    }

    /// Trusts the screen service `service`, replacing its key if it was
    /// trusted already.
    pub fn trust_service(&self, service: ServiceKey) -> Result<(), SecretsError> {
        let mut services = self.services()?;
        services.retain(|known| known.name != service.name);
        services.push(service);
        let bytes = serde_json::to_vec_pretty(&services).map_err(json_error("services file"))?;
        let path = self.services_path();
        fs::write(&path, bytes).map_err(io_error(format!("writing {}", path.display())))
    }

    pub fn add_route(&self, secret: &str, route: Route) -> Result<(), SecretsError> {
        let mut routes = self.routes()?;
        routes.insert(secret.to_owned(), route);
        let bytes = serde_json::to_vec_pretty(&routes).map_err(json_error("routes file"))?;
        let path = self.routes_path();
        fs::write(&path, bytes).map_err(io_error(format!("writing {}", path.display())))
    }
}

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
/// takes effect at the next use with no restart.
#[derive(Debug)]
pub struct FileGrants {
    path: PathBuf,
}

impl FileGrants {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn rows(&self) -> Result<Vec<GrantRow>, SecretsError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes =
            fs::read(&self.path).map_err(io_error(format!("reading {}", self.path.display())))?;
        serde_json::from_slice(&bytes).map_err(json_error("grants file"))
    }

    fn write(&self, rows: &[GrantRow]) -> Result<(), SecretsError> {
        let bytes = serde_json::to_vec_pretty(rows).map_err(json_error("grants file"))?;
        fs::write(&self.path, bytes).map_err(io_error(format!("writing {}", self.path.display())))
    }

    pub fn set(
        &self,
        relation: Relation,
        identity: &str,
        secret: &str,
        by: Option<&str>,
    ) -> Result<(), SecretsError> {
        let mut rows = self.rows()?;
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
            .into_iter()
            .map(|row| (row.identity, row.secret, row.relation, row.granted_by))
            .collect())
    }

    pub fn remove(
        &self,
        relation: Relation,
        identity: &str,
        secret: &str,
    ) -> Result<(), SecretsError> {
        let mut rows = self.rows()?;
        rows.retain(|row| !row.is(relation, identity, secret));
        self.write(&rows)
    }

    fn check(&self, relation: Relation, identity: &str, secret: &str) -> Result<Permitted, Denied> {
        let rows = self.rows().map_err(|error| Denied {
            reason: format!("the grants file does not read: {error}"),
            no_person_root: false,
        })?;
        match rows
            .into_iter()
            .find(|row| row.is(relation, identity, secret))
        {
            Some(GrantRow {
                granted_by: Some(person),
                ends_at_ms,
                ..
            }) => Ok(Permitted { person, ends_at_ms }),
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
