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

/// Everything the service is started with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Where the service listens.
    pub listen: SocketAddr,
    /// The directory the event log is kept in.
    pub log_dir: PathBuf,
    /// The log's origin, used when the log is created.
    pub log_origin: String,
    /// The file holding the service's event signing key seed.
    pub event_key_file: PathBuf,
    /// The OIDC issuer URL.
    pub issuer: String,
    /// The service's OIDC client id.
    pub client_id: String,
    /// The file holding the service's OIDC client secret.
    pub client_secret_file: PathBuf,
    /// The URL the issuer redirects back to after sign-in.
    pub redirect_url: String,
    /// The step-1 administrator, by issuer and subject (P9).
    pub administrator: ConfiguredLogin,
    /// The authenticated link-audit source, by issuer and subject (R4).
    pub link_audit_source: ConfiguredLogin,
    /// How long a session lives, in seconds.
    pub session_seconds: u64,
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
    /// The directory the runtime reports are kept in, created when it does
    /// not exist. Without it the runtime routes answer `RuntimeUnavailable`.
    #[serde(default)]
    pub runtime_dir: Option<PathBuf>,
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
        self.administrator_binding()?;
        self.link_audit_binding()?;
        if self.session_seconds == 0 {
            return Err(invalid("session_seconds is zero"));
        }
        if self.client_id.is_empty() {
            return Err(invalid("client_id is empty"));
        }
        Ok(())
    }

    /// The configured administrator's login.
    pub fn administrator_binding(&self) -> Result<LoginBinding, ServerError> {
        LoginBinding::new(&self.administrator.issuer, &self.administrator.subject)
            .map_err(|error| invalid(format!("administrator: {error}")))
    }

    /// The configured link-audit source's login.
    pub fn link_audit_binding(&self) -> Result<LoginBinding, ServerError> {
        LoginBinding::new(
            &self.link_audit_source.issuer,
            &self.link_audit_source.subject,
        )
        .map_err(|error| invalid(format!("link_audit_source: {error}")))
    }

    /// The permission model, read from its file and checked.
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
