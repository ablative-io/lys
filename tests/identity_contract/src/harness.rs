//! A directory on a temporary log, a service key over a fixed seed, a store
//! that can be told to fail an append at a chosen step, and the directory
//! service started on a local port behind the fake issuer.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity::Directory;
use lys_identity_server::config::ConfiguredLogin;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::spicedb::SpiceDbSettings;
use lys_identity_server::{Config, service};
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreError, StoreResult};

use crate::fake_issuer::{CLIENT_ID, CLIENT_SECRET, FakeIssuer, Login};

/// Where the next append fails, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Nothing fails.
    None,
    /// The leaf write fails before any byte is stored.
    BeforeLeaf,
    /// The leaf is stored and the store still answers the write failed, as a
    /// store does when the leaf is named but its durability is uncertain.
    LeafStoredWriteFailed,
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
        match fault(&self.plan) {
            Fault::BeforeLeaf => {
                set(&self.plan, Fault::None);
                Err(injected("leaf write"))
            }
            Fault::LeafStoredWriteFailed => {
                set(&self.plan, Fault::None);
                self.inner.put_leaf(index, bytes)?;
                Err(injected("leaf durability"))
            }
            Fault::None | Fault::AfterLeaf | Fault::AfterLeafUnreadable => {
                self.inner.put_leaf(index, bytes)
            }
        }
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
            Fault::None | Fault::BeforeLeaf | Fault::LeafStoredWriteFailed => self.inner.pin(pin),
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

/// The grant log's origin in every started service.
pub const GRANT_ORIGIN: &str = "example.test/lys/grants";
/// The permission model every started service judges grants against. Its
/// relation names say nothing of their actions: `alpha` carries read and
/// write, `beta` carries read alone.
pub const GRANT_MODEL: &str =
    r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"]}}"#;

/// The administrator's subject at the fake issuer.
pub const ADMINISTRATOR: &str = "administrator-subject";
/// The link-audit source's subject at the fake issuer.
pub const LINK_AUDIT_SOURCE: &str = "link-audit-source-subject";

/// A started directory service on a local port, signing in through a fake issuer.
pub struct Service {
    /// The service's base URL.
    pub base: String,
    /// The issuer people sign in through.
    pub issuer: FakeIssuer,
    client: reqwest::Client,
    /// Holds the log, keys and secret file for the service's life.
    pub dir: tempfile::TempDir,
}

fn secret_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, bytes)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
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
    let status = response.status().as_u16();
    let text = response.text().await?;
    Ok((status, serde_json::from_str(&text)?))
}

impl Service {
    /// Start the service over a fresh log, with the fake issuer's administrator
    /// and link-audit source configured.
    pub async fn start() -> Result<Self, Box<dyn Error>> {
        Ok(Self::start_with(|_| Ok(())).await?.0)
    }

    /// Start the service as [`Service::start`] does, after `prepare` has
    /// written to the directory its configuration names, answering what
    /// `prepare` answered.
    pub async fn start_with<T: Send>(
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_judging(GRANT_MODEL, None, prepare).await
    }

    /// Start the service as [`Service::start_with`] does, judging grants by
    /// `model` and keeping their relationships in the permission database
    /// `spicedb` names, or in the process when it names none.
    pub async fn start_judging<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_asking(model, spicedb, None, prepare).await
    }

    /// Start the service as [`Service::start_judging`] does, asking the
    /// secrets broker `secrets` names for the secrets screens, when it names
    /// one.
    pub async fn start_asking<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        secret_file(&dir.path().join("issuer.key"), &[3; 32])?;
        secret_file(&dir.path().join("service.key"), &[9; 32])?;
        secret_file(&dir.path().join("client.secret"), CLIENT_SECRET.as_bytes())?;
        let issuer = FakeIssuer::start(&dir.path().join("issuer.key")).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        let base = format!("http://{listen}");
        let configured = |subject: &str| ConfiguredLogin {
            issuer: issuer.issuer().to_owned(),
            subject: subject.to_owned(),
        };
        let config = Config {
            listen,
            log_dir: dir.path().join("log"),
            log_origin: ORIGIN.to_owned(),
            event_key_file: dir.path().join("service.key"),
            issuer: issuer.issuer().to_owned(),
            client_id: CLIENT_ID.to_owned(),
            client_secret_file: dir.path().join("client.secret"),
            redirect_url: format!("{base}/callback"),
            administrator: configured(ADMINISTRATOR),
            link_audit_source: configured(LINK_AUDIT_SOURCE),
            session_seconds: 600,
            secure_cookie: false,
            grant_log_dir: dir.path().join("grant-log"),
            grant_log_origin: GRANT_ORIGIN.to_owned(),
            grant_model_file: dir.path().join("grant-model.json"),
            spicedb,
            secrets,
        };
        std::fs::write(&config.grant_model_file, model)?;
        config.validate()?;
        let prepared = prepare(&config)?;
        let app = service(&config).await?;
        tokio::spawn(async move { axum::serve(listener, app).await });
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok((
            Self {
                base,
                issuer,
                client,
                dir,
            },
            prepared,
        ))
    }

    /// Begin a sign-in and let the issuer answer it as `login`, answering the
    /// service path the issuer sends the browser back to.
    pub async fn issuer_answer(&self, login: Login) -> Result<String, Box<dyn Error>> {
        self.issuer.sign_in_as(login);
        let to_issuer = self
            .client
            .get(format!("{}/login", self.base))
            .send()
            .await?;
        let authorize = location(&to_issuer)?;
        let back = self.client.get(authorize).send().await?;
        let url = location(&back)?;
        Ok(url
            .strip_prefix(&self.base)
            .ok_or_else(|| format!("the issuer sent the browser to {url}"))?
            .to_owned())
    }

    /// Sign in at the issuer as `login`, answering the session cookie.
    pub async fn sign_in(&self, login: Login) -> Result<String, Box<dyn Error>> {
        let back = self.issuer_answer(login).await?;
        let signed_in = self
            .client
            .get(format!("{}{back}", self.base))
            .send()
            .await?;
        let cookie = signed_in
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .map(|value| value.to_str().map(str::to_owned));
        let (status, body) = answer(signed_in).await?;
        let Some(cookie) = cookie else {
            return Err(format!("sign-in answered {status} without a session: {body}").into());
        };
        Ok(cookie?
            .split(';')
            .next()
            .ok_or("the session cookie is empty")?
            .to_owned())
    }

    /// GET `path`, with the session `cookie` when one is given.
    pub async fn get(&self, path: &str, cookie: Option<&str>) -> Result<Answer, Box<dyn Error>> {
        let mut request = self.client.get(format!("{}{path}", self.base));
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        answer(request.send().await?).await
    }

    /// POST `body` as JSON to `path`, with the session `cookie` when one is given.
    pub async fn post(
        &self,
        path: &str,
        cookie: Option<&str>,
        body: &serde_json::Value,
    ) -> Result<Answer, Box<dyn Error>> {
        let mut request = self
            .client
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string());
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        answer(request.send().await?).await
    }
}
