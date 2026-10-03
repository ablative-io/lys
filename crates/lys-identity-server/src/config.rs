//! The service's typed configuration and its validation.
//!
//! The configuration is a JSON file. The OIDC client secret is never in it:
//! the file names the path of a file holding the secret, which is read once at
//! start and held by openidconnect's `ClientSecret`, whose debug form is
//! redacted. No diagnostic prints a secret value.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use lys_identity::LoginBinding;
use lys_identity::grants::{Action, Model, Relation};
use serde::Deserialize;

use crate::error::ServerError;
use crate::spicedb::SpiceDbSettings;

/// An issuer and subject pair as the configuration names it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfiguredLogin {
    /// The issuer URL.
    pub issuer: String,
    /// The subject.
    pub subject: String,
}

/// What kind of install this is. A service install keeps no standing
/// credential on disk: nothing under the owning account's login can act as
/// the administrator without signing in. A development install keeps the
/// operator token (Tom, 3 Oct 2026: "not just like a dev install, but like
/// a particular profile"), and every act made with it is recorded as made
/// with the operator token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// The install people depend on: no operator token.
    #[default]
    Service,
    /// An install being developed on: the operator token is kept and read.
    Development,
}

/// Everything the service is started with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Where the service listens.
    pub listen: SocketAddr,
    /// Which kind of install this is; a service install when unsaid.
    #[serde(default)]
    pub profile: Profile,
    /// The directory the event log is kept in.
    pub log_dir: PathBuf,
    /// The log's origin, used when the log is created.
    pub log_origin: String,
    /// The file holding the service's event signing key seed.
    pub event_key_file: PathBuf,
    /// An installation-owned loader credential to provision once as an
    /// ordinary person-owned service account. No runtime bypass is granted.
    #[serde(default)]
    pub import_credential_file: Option<PathBuf>,
    /// The OIDC issuer URL.
    pub issuer: String,
    /// The service's OIDC client id.
    pub client_id: String,
    /// The file holding the service's OIDC client secret.
    pub client_secret_file: PathBuf,
    /// The URL the issuer redirects back to after sign-in.
    pub redirect_url: String,
    /// Where the issuer's own sign-in steps answer, over loopback, when that
    /// is not the issuer's address itself: Lys's sign-in page is carried
    /// there server-side.
    #[serde(default)]
    pub sign_in_api: Option<String>,
    /// Front proxies allowed to supply X-Forwarded-For; empty means trust
    /// socket peers alone. These are distinct from the issuer's proxies.
    #[serde(default)]
    pub trusted_proxies: Vec<std::net::IpAddr>,
    /// The file holding the install's operator token, read only under the
    /// development profile; a service install that names one is refused at
    /// start. A request carrying it in the `lys-operator` header acts as the
    /// administrator, recorded as authenticated by the operator token.
    #[serde(default)]
    pub operator_token_file: Option<PathBuf>,
    /// An installer upgrade intent blocks operator authority until rollback is
    /// no longer possible. Managed installs always supply this path.
    #[serde(default)]
    pub operator_upgrade_file: Option<PathBuf>,
    /// The step-1 administrator, by issuer and subject (P9), when the
    /// install named one before the service started. Without it the
    /// administrator is the one first-run setup records (`setup`).
    #[serde(default)]
    pub administrator: Option<ConfiguredLogin>,
    /// The authenticated link-audit source, by issuer and subject (R4).
    pub link_audit_source: ConfiguredLogin,
    /// The issuer this install's logins were bound under before an upgrade
    /// moved the sign-in service to `issuer`, as the upgrade hands it over.
    /// The directory records the move once, at start; the sign-in check
    /// compares against `issuer` alone.
    #[serde(default)]
    pub issuer_moved_from: Option<String>,
    /// How long a session lives, in seconds.
    pub session_seconds: u64,
    /// The file the signed-in sessions are kept in, so a restart leaves
    /// everyone signed in; without it they are kept in memory alone.
    #[serde(default)]
    pub sessions_file: Option<PathBuf>,
    /// Whether the session cookie is marked Secure.
    pub secure_cookie: bool,
    /// The directory the grant log is kept in.
    pub grant_log_dir: PathBuf,
    /// The grant log's origin, used when the log is created.
    pub grant_log_origin: String,
    /// The file holding the permission model grants are judged against.
    pub grant_model_file: PathBuf,
    /// The permission engine the grants are mirrored into. Without it the
    /// relationships are held in memory.
    #[serde(default)]
    pub spicedb: Option<SpiceDbSettings>,
    /// The secrets broker the secrets screens ask. Without it those routes
    /// answer `SecretsUnavailable`.
    #[serde(default)]
    pub secrets: Option<crate::secrets_api::SecretsSettings>,
    /// The directory the access requests are kept in, created when it does
    /// not exist. Without it the request routes answer `RequestsUnavailable`.
    #[serde(default)]
    pub requests_dir: Option<PathBuf>,
    /// The directory the issued certificates are kept in, created when it
    /// does not exist. Without it the certificate routes answer
    /// `CertificatesUnavailable`.
    #[serde(default)]
    pub certificates_dir: Option<PathBuf>,
    /// The file the machines are kept in, created at the first machine
    /// named. Without it the network routes answer `NetworkUnavailable`.
    #[serde(default)]
    pub network_file: Option<PathBuf>,
    /// The file the roles are kept in, created at the first role made.
    /// Without it the role routes answer `RolesUnavailable`.
    #[serde(default)]
    pub roles_file: Option<PathBuf>,
    /// The file the provisioning profiles are kept in, created at the first
    /// profile set. Without it the provisioning routes answer
    /// `ProvisioningUnavailable`.
    #[serde(default)]
    pub provisioning_file: Option<PathBuf>,
    /// The directory the homes of the agents are kept in, one home for each
    /// agent under the agent's id. It is read and never written. Without it
    /// the memory route answers `MemoryUnavailable`.
    #[serde(default)]
    pub homes_dir: Option<PathBuf>,
    /// The directory the runtime reports are kept in, created when it does
    /// not exist. Without it the runtime routes answer `RuntimeUnavailable`.
    #[serde(default)]
    pub runtime_dir: Option<PathBuf>,
    /// The directory the service accounts are kept in, created when it does
    /// not exist. Without it the service account routes answer
    /// `ServiceAccountsUnavailable` and `GET /me` lists none.
    #[serde(default)]
    pub service_accounts_dir: Option<PathBuf>,
    /// The directory the budgets are kept in, created when it does not
    /// exist. Without it the budget routes answer `BudgetsUnavailable`.
    #[serde(default)]
    pub budgets_dir: Option<PathBuf>,
    /// The directory the agents' tool-boundary policies are kept in, created
    /// when it does not exist. Without it the policy routes answer
    /// `PolicyUnavailable`.
    #[serde(default)]
    pub policies_dir: Option<PathBuf>,
    /// The directory the teams are kept in, created when it does not exist.
    /// Without it the team routes answer `TeamsUnavailable`.
    #[serde(default)]
    pub teams_dir: Option<PathBuf>,
    /// The directory the emergency stops are kept in, created when it does
    /// not exist. Without it the stop route answers `StopsUnavailable`.
    #[serde(default)]
    pub stops_dir: Option<PathBuf>,
    /// The directory the goals, expectations and deliverables are kept in,
    /// created when it does not exist. Without it the goal routes answer
    /// `goals_unavailable`.
    #[serde(default)]
    pub goals_dir: Option<PathBuf>,
    /// The directory the review decisions are kept in, created when it does
    /// not exist. Without it keeping a grant answers `ReviewsUnavailable`.
    #[serde(default)]
    pub reviews_dir: Option<PathBuf>,
    /// The issuer's administration API the sign-in providers are set
    /// through. Without it the sign-in provider routes answer
    /// `SignInProvidersUnavailable`.
    #[serde(default)]
    pub sign_in_providers: Option<crate::sign_in_providers::SignInProvidersSettings>,
    /// Where the sign-in providers answer, when not at their public
    /// origins: stand-ins, for a development or test service.
    #[serde(default)]
    pub provider_origins: Option<crate::sign_in_providers::ProviderOrigins>,
    /// Lys as the `OpenID` provider products are registered with. Without it
    /// the provider's routes answer `ProviderUnavailable`.
    #[serde(default)]
    pub provider: Option<crate::provider::ProviderSettings>,
    /// First-run setup: where the one-time setup code's digest is read and
    /// where the administrator it makes is recorded. Without it the setup
    /// page's routes answer `SetupUnavailable`.
    #[serde(default)]
    pub setup: Option<crate::setup::SetupSettings>,
    /// Lys's password policy, the one the install wrote to the issuer: the
    /// setup and account screens show it and it is checked before a password
    /// is sent. Without it the screens show no policy and the issuer alone
    /// checks one.
    #[serde(default)]
    pub password_policy: Option<crate::accounts::PasswordPolicy>,
    /// Lys's model proxy for Anthropic calls, the base URL a Claude Code run
    /// Lys starts sends its model calls to, so each call is recorded under
    /// its session. Without it a run reaches its provider as the machine's
    /// own setup says.
    #[serde(default)]
    pub model_proxy: Option<String>,
    /// The haematite stores the door reaches: each name and the absolute
    /// path of the Unix socket its `haem serve` listens on. Without any, the
    /// door lists no store and reaches none.
    #[serde(default)]
    pub haem_stores: std::collections::BTreeMap<String, PathBuf>,
    /// The compiled screens the service serves at `/`, its own routes then
    /// answering under `/api`. Without it the routes answer at the root and
    /// no screen is served.
    #[serde(default)]
    pub surface_dir: Option<PathBuf>,
    /// The Unix socket of this install's own runner, which a machine whose
    /// record names Lys's runner is driven through. Without it such a
    /// machine's sessions are refused `runner_unreachable`, by name.
    #[serde(default)]
    pub runner_socket: Option<PathBuf>,
    /// Message reads from the message service use the caller's session cookie and explicit identity bindings.
    #[serde(default)]
    pub message_service: Option<crate::message_edges::Settings>,
}

/// The permission model as its file writes it: a version, and each relation
/// with the actions it carries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelFile {
    version: u64,
    relations: std::collections::BTreeMap<String, Vec<String>>,
}

fn invalid(reason: impl Into<String>) -> ServerError {
    ServerError::ConfigInvalid {
        reason: reason.into(),
    }
}

impl Config {
    /// Read and validate the configuration at `path`.
    pub fn load(path: &Path) -> Result<Self, ServerError> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| invalid(format!("{} could not be read: {error}", path.display())))?;
        let config: Self = serde_json::from_str(&text).map_err(|error| {
            invalid(format!(
                "{} is not a configuration: {error}",
                path.display()
            ))
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Refuse a configuration the service cannot run under.
    pub fn validate(&self) -> Result<(), ServerError> {
        if let Some(settings) = &self.message_service {
            crate::message_edges::validate(settings)?;
        }
        self.configured_administrator()?;
        if self.administrator.is_none() && self.setup.is_none() {
            return Err(invalid(
                "neither an administrator nor first-run setup is configured, so nobody could ever administer the directory",
            ));
        }
        self.link_audit_binding()?;
        crate::haem_door::validate(&self.haem_stores).map_err(invalid)?;
        if let Some(policy) = &self.password_policy {
            policy
                .validate()
                .map_err(|reason| invalid(format!("password_policy: {reason}")))?;
        }
        if self.session_seconds == 0 {
            return Err(invalid("session_seconds is zero"));
        }
        if self.client_id.is_empty() {
            return Err(invalid("client_id is empty"));
        }
        Ok(())
    }

    /// The configured administrator's login, when one is configured.
    pub fn configured_administrator(&self) -> Result<Option<LoginBinding>, ServerError> {
        self.administrator
            .as_ref()
            .map(|login| {
                LoginBinding::new(&login.issuer, &login.subject)
                    .map_err(|error| invalid(format!("administrator: {error}")))
            })
            .transpose()
    }

    /// The configured administrator's login, refused by name when the
    /// configuration names none.
    pub fn administrator_binding(&self) -> Result<LoginBinding, ServerError> {
        self.configured_administrator()?
            .ok_or_else(|| invalid("no administrator is configured"))
    }

    /// Where the issuer's own sign-in steps answer, the ones a password is
    /// carried to from this service: `sign_in_api` when it is configured, the
    /// issuer's address otherwise.
    pub fn sign_in_api(&self) -> String {
        self.sign_in_api
            .as_deref()
            .unwrap_or(&self.issuer)
            .trim_end_matches('/')
            .to_owned()
    }

    /// The configured link-audit source's login.
    pub fn link_audit_binding(&self) -> Result<LoginBinding, ServerError> {
        LoginBinding::new(
            &self.link_audit_source.issuer,
            &self.link_audit_source.subject,
        )
        .map_err(|error| invalid(format!("link_audit_source: {error}")))
    }

    /// The directory the apps are kept in: `apps`, beside the grant log.
    /// The apps log is always kept, since it holds Lys's own model as the
    /// schema of the app `lys`.
    pub fn apps_dir(&self) -> PathBuf {
        self.grant_log_dir.with_file_name("apps")
    }

    /// The permission model, read from its file and checked. It is read only
    /// when the apps log does not yet hold the app `lys`, to record it there
    /// once; afterwards the log is the only source and the file is not read.
    pub fn grant_model(&self) -> Result<Model, ServerError> {
        let path = &self.grant_model_file;
        let text = std::fs::read_to_string(path)
            .map_err(|error| invalid(format!("{} could not be read: {error}", path.display())))?;
        let file: ModelFile = serde_json::from_str(&text)
            .map_err(|error| invalid(format!("{} is not a model: {error}", path.display())))?;
        let mut relations = Vec::with_capacity(file.relations.len());
        for (relation, actions) in file.relations {
            let actions = actions
                .iter()
                .map(|action| Action::new(action))
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|error| invalid(format!("grant model: {error}")))?;
            let relation = Relation::new(&relation)
                .map_err(|error| invalid(format!("grant model: {error}")))?;
            relations.push((relation, actions));
        }
        Model::new(file.version, relations)
            .map_err(|error| invalid(format!("grant model: {error}")))
    }

    /// The client secret, read from its file. The value is never part of an error.
    pub fn client_secret(&self) -> Result<openidconnect::ClientSecret, ServerError> {
        let text = std::fs::read_to_string(&self.client_secret_file).map_err(|error| {
            invalid(format!(
                "the client secret file {} could not be read: {}",
                self.client_secret_file.display(),
                error.kind()
            ))
        })?;
        let secret = text.trim_end_matches(['\n', '\r']);
        if secret.is_empty() {
            return Err(invalid("the client secret file is empty"));
        }
        Ok(openidconnect::ClientSecret::new(secret.to_owned()))
    }
}
