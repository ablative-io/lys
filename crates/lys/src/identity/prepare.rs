//! `lys identity prepare`: validate the deployment config and materialise
//! the declared private artifacts.
//!
//! # Invariants
//!
//! - Nothing is written until the config has validated, so a refused config
//!   (`invalid_issuer`, `invalid_redirect_uri`, `invalid_database_host`, ...)
//!   leaves no file behind.
//! - The declared private set is exactly [`declared_files`]: every
//!   credential in [`DECLARED`] under `secrets/`, plus `identity.env`, each
//!   `0600` in `0700` directories.
//! - A deployment is prepared once `identity.env` exists. Before that an
//!   absent credential is generated and a present one reused; after it an
//!   absent credential is `secret_missing` and nothing is generated.
//! - `identity.env` is rewritten on every run from the config and the stable
//!   credentials, so a config change reaches compose while every credential
//!   stays what it was.
//! - Output names each file and whether it was generated or reused. It
//!   never prints a value.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use super::config::{LoadedConfig, RAUTHY_DB_USER, SPICEDB_DB_USER};
use super::credentials::{Credentials, DECLARED, ENC_KEY, Provenance};
use super::error::IdentityError;
use super::private_files;
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;

/// The environment file deploy/identity/compose.yaml is run with.
pub const ENV_FILE: &str = "identity.env";

/// The private subdirectory holding one file per credential.
pub const SECRETS_DIR: &str = "secrets";

/// The name of the API key Rauthy bootstraps for configure.
pub const API_KEY_NAME: &str = "lys_identity";

/// What one prepare run did.
#[derive(Debug)]
pub struct PrepareReport {
    /// The rendered environment file.
    pub env_file: PathBuf,
    /// Each credential file and whether this run generated or reused it.
    pub credentials: Vec<(PathBuf, Provenance)>,
}

/// `lys identity prepare --config <file>`.
pub fn run(config: &Path, json: bool) -> CliResult<()> {
    let loaded = LoadedConfig::load(config)?;
    let report = prepare(&loaded)?;
    let mut emit = Emitter::new(json);
    emit.field("config", "config", loaded.path.display().to_string());
    emit.field("node", "node", loaded.config.deployment.node.as_str());
    emit.field("issuer", "issuer", loaded.issuer());
    emit.field(
        "database",
        "database",
        format!(
            "{}:{}/{}",
            loaded.config.database.host, loaded.config.database.port, loaded.config.database.name
        ),
    );
    emit.field(
        "private directory",
        "private_dir",
        loaded.private_dir.display().to_string(),
    );
    emit.field(
        "environment file",
        "env_file",
        report.env_file.display().to_string(),
    );
    let mut files = Vec::with_capacity(report.credentials.len());
    for (path, provenance) in &report.credentials {
        emit.note(&format!("{}: {}", provenance.as_str(), path.display()));
        files.push(json!({"path": path.display().to_string(), "outcome": provenance.as_str()}));
    }
    if emit.is_json() {
        emit.field("credentials", "credentials", Value::Array(files));
    }
    emit.finish();
    Ok(())
}

/// Every private file prepare declares, under `private_dir`.
pub fn declared_files(private_dir: &Path) -> Vec<PathBuf> {
    let secrets = private_dir.join(SECRETS_DIR);
    DECLARED
        .iter()
        .map(|spec| secrets.join(spec.name))
        .chain([private_dir.join(ENV_FILE)])
        .collect()
}

/// Materialises the private artifacts for an already validated config.
pub fn prepare(loaded: &LoadedConfig) -> Result<PrepareReport, IdentityError> {
    let private = &loaded.private_dir;
    private_files::ensure_private_dir(private)?;
    let secrets = private.join(SECRETS_DIR);
    private_files::ensure_private_dir(&secrets)?;
    let env_file = private.join(ENV_FILE);
    let prepared = env_file.try_exists().map_err(|source| IdentityError::Io {
        operation: "check for the environment file",
        path: env_file.clone(),
        source,
    })?;
    let credentials = Credentials::load_or_generate(&secrets, !prepared)?;
    let env = render_env(loaded, &credentials, &secrets)?;
    private_files::write_private(&env_file, env.as_bytes())?;
    // Reconcile the outcome against the declared set, not against what this
    // run happened to write.
    for path in declared_files(private) {
        let mode = private_files::mode_of(&path)?;
        if mode != private_files::FILE_MODE {
            return Err(IdentityError::PrivateModeUnrestricted {
                path,
                mode,
                expected: private_files::FILE_MODE,
            });
        }
    }
    Ok(PrepareReport {
        env_file,
        credentials: credentials
            .iter()
            .map(|(spec, _, provenance)| (secrets.join(spec.name), provenance))
            .collect(),
    })
}

/// The environment file compose interpolates: every configured value and
/// every credential, one `NAME=value` per line.
fn render_env(
    loaded: &LoadedConfig,
    credentials: &Credentials,
    secrets: &Path,
) -> Result<Zeroizing<String>, IdentityError> {
    let config = &loaded.config;
    let database = &config.database;
    let rauthy = &config.rauthy;
    let public = &loaded.public;
    let api_key = json!({
        "name": API_KEY_NAME,
        "access": [{"group": "Clients", "access_rights": ["read", "create", "update"]}],
    });
    let key_id =
        credentials
            .get(ENC_KEY, secrets)?
            .key_id()
            .ok_or(IdentityError::SecretInvalid {
                name: ENC_KEY,
                path: secrets.join(ENC_KEY),
                reason: "not <id>/<base64 key>",
            })?;
    let scheme = if public.https { "https" } else { "http" };
    let values: [(&str, String); 21] = [
        ("LYS_IDENTITY_NODE", config.deployment.node.clone()),
        ("LYS_IDENTITY_PG_HOST", database.host.clone()),
        ("LYS_IDENTITY_PG_PORT", database.port.to_string()),
        ("LYS_IDENTITY_PG_DATABASE", database.name.clone()),
        ("LYS_IDENTITY_PG_ADMIN_USER", database.admin_user.clone()),
        ("LYS_IDENTITY_PG_SSLMODE", database.sslmode.clone()),
        ("LYS_IDENTITY_PG_PUBLISH", database.publish.clone()),
        ("LYS_IDENTITY_RAUTHY_DB_USER", RAUTHY_DB_USER.to_string()),
        ("LYS_IDENTITY_SPICEDB_DB_USER", SPICEDB_DB_USER.to_string()),
        ("LYS_IDENTITY_RAUTHY_PUB_URL", public.authority()),
        ("LYS_IDENTITY_RAUTHY_PROXY_MODE", public.https.to_string()),
        (
            "LYS_IDENTITY_RAUTHY_TRUSTED_PROXIES",
            rauthy.trusted_proxy.clone(),
        ),
        ("LYS_IDENTITY_RAUTHY_RP_ID", public.host.clone()),
        (
            "LYS_IDENTITY_RAUTHY_RP_ORIGIN",
            format!("{scheme}://{}:{}", public.host, public.effective_port()),
        ),
        (
            "LYS_IDENTITY_RAUTHY_ADMIN_EMAIL",
            rauthy.admin_email.clone(),
        ),
        ("LYS_IDENTITY_RAUTHY_PUBLISH", rauthy.publish.clone()),
        (
            "LYS_IDENTITY_RAUTHY_API_KEY",
            STANDARD.encode(api_key.to_string()),
        ),
        ("LYS_IDENTITY_RAUTHY_ENC_KEY_ID", key_id.to_string()),
        (
            "LYS_IDENTITY_SPICEDB_HTTP_PUBLISH",
            config.spicedb.http_publish.clone(),
        ),
        (
            "LYS_IDENTITY_SPICEDB_GRPC_PUBLISH",
            config.spicedb.grpc_publish.clone(),
        ),
        ("LYS_IDENTITY_ISSUER", loaded.issuer()),
    ];
    let mut env = Zeroizing::new(String::with_capacity(8192));
    env.push_str("# Rendered by `lys identity prepare`. Owner-only; never commit or share.\n");
    for (name, value) in &values {
        push_line(&mut env, name, value);
    }
    for (spec, secret, _) in credentials.iter() {
        push_line(&mut env, spec.env, secret.expose());
    }
    Ok(env)
}

fn push_line(env: &mut String, name: &str, value: &str) {
    env.push_str(name);
    env.push('=');
    env.push_str(value);
    env.push('\n');
}

#[cfg(test)]
#[path = "prepare_tests.rs"]
mod tests;
