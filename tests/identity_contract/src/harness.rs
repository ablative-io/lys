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
use std::sync::{Arc, Mutex, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity::Directory;
use lys_identity_server::config::ConfiguredLogin;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::sign_in_providers::{ProviderOrigins, SignInProvidersSettings};
use lys_identity_server::spicedb::SpiceDbSettings;
use lys_identity_server::{Config, Say, service, service_saying};
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

    /// A way to open the log's leaf store that fails where the plan says.
    pub fn leaves(&self) -> Box<dyn Fn() -> StoreResult<FaultStore> + Send> {
        let path = self.log_path();
        let plan = Arc::clone(&self.plan);
        Box::new(move || {
            if fault(&plan) == Fault::AfterLeafUnreadable {
                return Err(injected("reopen"));
            }
            Ok(FaultStore {
                inner: FileLeafStore::open(Path::new(&path))?,
                plan: Arc::clone(&plan),
            })
        })
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
    config: Config,
    server: tokio::task::JoinHandle<std::io::Result<()>>,
}

fn secret_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, bytes)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// The password the harness types for a login the issuer was told to sign.
pub const HARNESS_PASSWORD: &str = "Harness-Password-2026";

/// The session cookie an answer began, or its refusal as an error.
pub async fn session_cookie(signed_in: reqwest::Response) -> Result<String, Box<dyn Error>> {
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

fn location(answer: &reqwest::Response) -> Result<String, Box<dyn Error>> {
    Ok(answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or_else(|| format!("{} answered no redirect", answer.url()))?
        .to_str()?
        .to_owned())
}

/// The service `config` describes, answering on `listener`, and a client
/// that follows no redirect.
async fn serve(
    listener: tokio::net::TcpListener,
    config: &Config,
    say: Option<Say>,
) -> Result<
    (
        tokio::task::JoinHandle<std::io::Result<()>>,
        reqwest::Client,
    ),
    Box<dyn Error>,
> {
    let documented = crate::refusals::listed();
    let app = match say {
        Some(say) => service_saying(config, say).await?,
        None => service(config).await?,
    };
    let app = app.layer(axum::middleware::from_fn(move |request, next| {
        crate::refusals::listed_only(Arc::clone(&documented), request, next)
    }));
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
    });
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    Ok((server, client))
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
        Self::start_setting(model, spicedb, secrets, None, prepare).await
    }

    /// Start the service as [`Service::start_asking`] does, setting the
    /// sign-in providers through the issuer API `sign_in_providers` names,
    /// when it names one.
    pub async fn start_setting<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_adjusted(model, spicedb, secrets, sign_in_providers, |_| {}, prepare).await
    }

    /// Start the service as [`Service::start_setting`] does, after `adjust`
    /// has changed the configuration the harness wrote, so a test can
    /// configure an administrator the fake issuer never signs in.
    pub async fn start_adjusted<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        adjust: impl FnOnce(&mut Config) + Send,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_saying(
            model,
            spicedb,
            secrets,
            sign_in_providers,
            adjust,
            None,
            prepare,
        )
        .await
    }

    /// Start the service as [`Service::start_adjusted`] does, saying each
    /// line of its log to `say` when one is given, so a test reads every
    /// line the service said.
    pub async fn start_saying<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        adjust: impl FnOnce(&mut Config) + Send,
        say: Option<Say>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        secret_file(&dir.path().join("issuer.key"), &[3; 32])?;
        secret_file(&dir.path().join("service.key"), &[9; 32])?;
        secret_file(&dir.path().join("client.secret"), CLIENT_SECRET.as_bytes())?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        let base = format!("http://{listen}");
        let issuer = FakeIssuer::start_behind(&base, &dir.path().join("issuer.key")).await?;
        issuer.set_public_callback(&format!("{base}/auth/v1/providers/callback"));
        let configured = |subject: &str| ConfiguredLogin {
            issuer: issuer.issuer().to_owned(),
            subject: subject.to_owned(),
        };
        let mut config = Config {
            listen,
            log_dir: dir.path().join("log"),
            log_origin: ORIGIN.to_owned(),
            event_key_file: dir.path().join("service.key"),
            import_credential_file: None,
            issuer: issuer.issuer().to_owned(),
            client_id: CLIENT_ID.to_owned(),
            client_secret_file: dir.path().join("client.secret"),
            redirect_url: format!("{base}/callback"),
            sign_in_api: Some(issuer.api().to_owned()),
            administrator: Some(configured(ADMINISTRATOR)),
            link_audit_source: configured(LINK_AUDIT_SOURCE),
            session_seconds: 600,
            secure_cookie: false,
            grant_log_dir: dir.path().join("grant-log"),
            grant_log_origin: GRANT_ORIGIN.to_owned(),
            grant_model_file: dir.path().join("grant-model.json"),
            spicedb,
            secrets,
            requests_dir: Some(dir.path().join("requests")),
            certificates_dir: Some(dir.path().join("certificates")),
            network_file: Some(dir.path().join("network.json")),
            roles_file: Some(dir.path().join("roles.json")),
            provisioning_file: Some(dir.path().join("provisioning.json")),
            homes_dir: Some(dir.path().join("homes")),
            runtime_dir: Some(dir.path().join("runtime")),
            service_accounts_dir: Some(dir.path().join("service-accounts")),
            teams_dir: Some(dir.path().join("teams")),
            stops_dir: Some(dir.path().join("stops")),
            reviews_dir: Some(dir.path().join("reviews")),
            sign_in_providers,
            provider_origins: Some(ProviderOrigins {
                google: issuer.provider_base().to_owned(),
                microsoft: issuer.provider_base().to_owned(),
                github: issuer.provider_base().to_owned(),
            }),
            provider: None,
            setup: None,
            surface_dir: None,
            runner_socket: None,
        };
        adjust(&mut config);
        std::fs::write(&config.grant_model_file, model)?;
        config.validate()?;
        let prepared = prepare(&config)?;
        let (server, client) = serve(listener, &config, say).await?;
        Ok((
            Self {
                base,
                issuer,
                client,
                dir,
                config,
                server,
            },
            prepared,
        ))
    }

    /// Stop the service and start it again over the same directory, on a new
    /// address with a new client, so every store is opened from disk and no
    /// session or connection of the stopped service carries over.
    pub async fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        self.server.abort();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        self.base = format!("http://{listen}");
        self.config.listen = listen;
        self.config.redirect_url = format!("{}/callback", self.base);
        let (server, client) = serve(listener, &self.config, None).await?;
        self.server = server;
        self.client = client;
        Ok(())
    }

    /// Restart an existing installation with the configuration an upgrade
    /// writes, retaining every durable log and credential.
    pub async fn restart_adjusted(&mut self, adjust: impl FnOnce(&mut Config)) -> Result<(), Box<dyn Error>> {
        adjust(&mut self.config);
        self.restart().await
    }

    /// Begin a sign-in through a sign-in provider and let it answer as
    /// `login`, answering the service path the browser is sent back to.
    pub async fn issuer_answer(&self, login: Login) -> Result<String, Box<dyn Error>> {
        self.issuer.sign_in_as(login);
        let to_issuer = self
            .client
            .get(format!("{}/sign-in/providers/harness", self.base))
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

    /// Sign in on Lys's own sign-in route as `login`, which the issuer
    /// signs whatever password is typed, answering the session cookie.
    pub async fn sign_in(&self, login: Login) -> Result<String, Box<dyn Error>> {
        let email = login.email.clone();
        self.issuer.sign_in_as(login);
        self.sign_in_with(&email, HARNESS_PASSWORD).await
    }

    /// Sign in on Lys's own sign-in route with `email` and `password`,
    /// answering the session cookie, or the refusal as an error.
    pub async fn sign_in_with(
        &self,
        email: &str,
        password: &str,
    ) -> Result<String, Box<dyn Error>> {
        let body = serde_json::json!({ "email": email, "password": password });
        let signed_in = self
            .client
            .post(format!("{}/sign-in", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await?;
        session_cookie(signed_in).await
    }

    /// GET `path`, with the session `cookie` when one is given.
    pub async fn get(&self, path: &str, cookie: Option<&str>) -> Result<Answer, Box<dyn Error>> {
        let mut request = self.client.get(format!("{}{path}", self.base));
        if let Some(cookie) = cookie {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        answer(request.send().await?).await
    }

    /// GET `path` as a browser following a link does, asking for a page: the
    /// status, the redirect it names, and whether it set a session cookie.
    pub async fn get_page(
        &self,
        path: &str,
    ) -> Result<(u16, Option<String>, bool), Box<dyn Error>> {
        let response = self
            .client
            .get(format!("{}{path}", self.base))
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .send()
            .await?;
        let status = response.status().as_u16();
        let to = location(&response).ok();
        let cookie = response.headers().contains_key(reqwest::header::SET_COOKIE);
        Ok((status, to, cookie))
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

    /// The directory log's size as the public receipt route shows it: the
    /// first index it holds no leaf at, so a test can prove a refused call
    /// logged nothing without a session.
    pub async fn log_size(&self) -> Result<u64, Box<dyn Error>> {
        let mut size = 0;
        loop {
            let (status, _) = self.get(&format!("/receipts/{size}"), None).await?;
            if status != 200 {
                return Ok(size);
            }
            size += 1;
        }
    }

    /// POST the JSON `body` bytes to `path` carrying the header `name: value`
    /// and no session cookie, so a request signed over those exact bytes
    /// arrives as signed.
    pub async fn post_signed(
        &self,
        path: &str,
        (name, value): (&str, &str),
        body: Vec<u8>,
    ) -> Result<Answer, Box<dyn Error>> {
        let request = self
            .client
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(name, value)
            .body(body);
        answer(request.send().await?).await
    }

    /// POST the JSON `body` bytes to `path` carrying every header of
    /// `headers`, so a request may carry a signature over those exact bytes
    /// and a session cookie at once.
    pub async fn post_carrying(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Result<Answer, Box<dyn Error>> {
        let mut request = self
            .client
            .post(format!("{}{path}", self.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        answer(request.send().await?).await
    }
}
