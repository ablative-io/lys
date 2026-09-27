//! `lys identity prepare`: validate the deployment configuration and
//! materialise the declared private artifacts.
//!
//! Invariants:
//!
//! - The declared set is exact: [`declared_private_files`] names every file
//!   prepare writes, the private env file and one file per credential, and
//!   prepare writes nothing else. Each is mode `0600` in a `0700` directory.
//! - The state directory may not lie inside a Git work tree, so a generated
//!   credential can never be staged by accident.
//! - Credentials are generated once and reused; the env file is re-rendered
//!   from the current configuration each run and reported `unchanged` when
//!   the result is byte-identical.
//! - Output names paths and outcomes only, never a value from the env file.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::identity::config::{DatabaseTls, DeploymentConfig};
use crate::identity::credentials::{
    API_KEY_NAME, CREDENTIALS_DIR, CredentialKind, Credentials, ENC_KEY_ID, Secret,
};
use crate::identity::error::{IdentityError, IdentityResult};
use crate::identity::private_files::{self, WriteOutcome};

/// The private env file compose interpolates, under the state directory.
pub const ENV_FILE: &str = "identity.env";

/// The compose profile that runs the local `PostgreSQL` service.
pub const LOCAL_DATABASE_PROFILE: &str = "local-database";

/// What prepare wrote.
#[derive(Debug)]
pub struct PrepareReport {
    /// The resolved state directory.
    pub state_dir: PathBuf,
    /// Every declared file with what happened to it.
    pub files: Vec<(PathBuf, WriteOutcome)>,
}

/// Every private file prepare declares, relative to the state directory.
pub fn declared_private_files() -> Vec<String> {
    let mut files = vec![ENV_FILE.to_string()];
    files.extend(
        CredentialKind::ALL
            .iter()
            .map(|kind| format!("{CREDENTIALS_DIR}/{}", kind.name())),
    );
    files
}

/// Run `lys identity prepare --config <config>`.
pub fn run(config_path: &Path, json: bool) -> CliResult<()> {
    let config = DeploymentConfig::load(config_path)?;
    let report = prepare(&config)?;
    let env_file = report.state_dir.join(ENV_FILE);
    let mut out = Emitter::new(json);
    out.field(
        "state directory",
        "state_dir",
        report.state_dir.display().to_string(),
    );
    if out.is_json() {
        let files: Vec<Value> = report
            .files
            .iter()
            .map(|(path, outcome)| json!({"path": path.display().to_string(), "outcome": outcome.as_str()}))
            .collect();
        out.field("files", "files", Value::Array(files));
    } else {
        for (path, outcome) in &report.files {
            out.note(&format!("{:<9} {}", outcome.as_str(), path.display()));
        }
    }
    out.field("issuer", "issuer", config.issuer());
    out.field("database", "database", config.database_probe());
    out.field(
        "database service",
        "database_service",
        if config.uses_local_database() {
            "local (compose profile local-database)"
        } else {
            "external"
        },
    );
    out.note(&format!(
        "next: docker compose --env-file {} -f deploy/identity/compose.yaml up -d",
        env_file.display()
    ));
    out.finish();
    Ok(())
}

/// Validate, generate or reuse every credential, and render the env file.
pub fn prepare(config: &DeploymentConfig) -> IdentityResult<PrepareReport> {
    let state_dir = config.state_dir();
    refuse_inside_git(&state_dir)?;
    private_files::ensure_private_dir(&state_dir, "state_dir")?;
    let (credentials, mut files) = Credentials::load_or_generate(&state_dir)?;
    let env = render_env(config, &credentials)?;
    let env_path = state_dir.join(ENV_FILE);
    let outcome = private_files::write_private(&env_path, env.as_bytes(), ENV_FILE)?;
    files.insert(0, (env_path, outcome));
    // Reconcile the outcome against the declared set: every declared file
    // exists now and grants no group or other access.
    for relative in declared_private_files() {
        let path = state_dir.join(&relative);
        let mode = private_files::mode_of(&path, &relative)?;
        if mode & 0o077 != 0 {
            return Err(IdentityError::PrivateFileTooOpen {
                resource: relative,
                path,
                mode,
            });
        }
    }
    Ok(PrepareReport { state_dir, files })
}

/// The bootstrap API key's definition, base64 as Rauthy reads it: read,
/// create and update on clients, and read on secrets. No delete right exists,
/// so the key cannot remove the built-in client or any other.
pub fn bootstrap_api_key() -> String {
    let definition = json!({
        "name": API_KEY_NAME,
        "access": [
            {"group": "Clients", "access_rights": ["read", "create", "update"]},
            {"group": "Secrets", "access_rights": ["read"]}
        ]
    });
    STANDARD.encode(definition.to_string())
}

fn render_env(
    config: &DeploymentConfig,
    credentials: &Credentials,
) -> IdentityResult<Zeroizing<String>> {
    let secret = |kind| credentials.get(kind).map(Secret::expose);
    let tls = match config.database.tls {
        DatabaseTls::Disable => "disable",
        DatabaseTls::Require => "require",
    };
    let profiles = if config.uses_local_database() {
        LOCAL_DATABASE_PROFILE
    } else {
        ""
    };
    let proxy_mode = if config.proxy_mode()? { "true" } else { "false" };
    let port = config.database.port.to_string();
    let pub_url = config.pub_url()?;
    let rp_id = config.rp_id()?;
    let api_key = bootstrap_api_key();
    let trusted = format!("\"{}\"", config.rauthy.trusted_proxies.join("\\n"));

    let mut enc_keys = Zeroizing::new(String::from(ENC_KEY_ID));
    enc_keys.push('/');
    enc_keys.push_str(secret(CredentialKind::RauthyEncryptionKey)?);

    let mut env = Zeroizing::new(String::from(
        "# Generated by `lys identity prepare`. Private (mode 0600): never commit, copy or log it.\n",
    ));
    let entries: [(&str, &str); 28] = [
        ("COMPOSE_PROFILES", profiles),
        ("IDENTITY_COMPOSE_PROJECT", &config.deployment.compose_project),
        ("IDENTITY_DB_HOST", &config.database.host),
        ("IDENTITY_DB_PORT", &port),
        ("IDENTITY_DB_NAME", &config.database.name),
        ("IDENTITY_DB_SSLMODE", tls),
        ("IDENTITY_DB_RAUTHY_TLS", tls),
        ("IDENTITY_PG_PUBLISH", &config.database.local_publish),
        ("IDENTITY_POSTGRES_PASSWORD", secret(CredentialKind::PostgresSuperuser)?),
        ("IDENTITY_RAUTHY_DB_PASSWORD", secret(CredentialKind::RauthyDatabase)?),
        ("IDENTITY_SPICEDB_DB_PASSWORD", secret(CredentialKind::SpicedbDatabase)?),
        ("IDENTITY_SPICEDB_PRESHARED_KEY", secret(CredentialKind::SpicedbPresharedKey)?),
        ("IDENTITY_SPICEDB_HTTP_PUBLISH", &config.spicedb.http_publish),
        ("IDENTITY_SPICEDB_GRPC_PUBLISH", &config.spicedb.grpc_publish),
        ("IDENTITY_RAUTHY_HQL_SECRET_RAFT", secret(CredentialKind::HiqliteRaft)?),
        ("IDENTITY_RAUTHY_HQL_SECRET_API", secret(CredentialKind::HiqliteApi)?),
        ("IDENTITY_RAUTHY_ENC_KEYS", &enc_keys),
        ("IDENTITY_RAUTHY_ENC_KEY_ACTIVE", ENC_KEY_ID),
        ("IDENTITY_RAUTHY_PUB_URL", &pub_url),
        ("IDENTITY_RAUTHY_PROXY_MODE", proxy_mode),
        ("IDENTITY_RAUTHY_TRUSTED_PROXIES", &trusted),
        ("IDENTITY_RAUTHY_RP_ID", &rp_id),
        ("IDENTITY_RAUTHY_RP_ORIGIN", &config.rauthy.public_origin),
        ("IDENTITY_RAUTHY_ADMIN_EMAIL", &config.rauthy.admin_email),
        ("IDENTITY_RAUTHY_ADMIN_PASSWORD", secret(CredentialKind::RauthyAdminPassword)?),
        ("IDENTITY_RAUTHY_BOOTSTRAP_API_KEY", &api_key),
        ("IDENTITY_RAUTHY_BOOTSTRAP_API_KEY_SECRET", secret(CredentialKind::RauthyApiKey)?),
        ("IDENTITY_RAUTHY_PUBLISH", &config.rauthy.publish),
    ];
    for (key, value) in entries {
        env.push_str(key);
        env.push('=');
        env.push_str(value);
        env.push('\n');
    }
    Ok(env)
}

/// Refuse a state directory inside a Git work tree.
fn refuse_inside_git(state_dir: &Path) -> IdentityResult<()> {
    let mut existing = state_dir.to_path_buf();
    while !existing.exists() {
        match existing.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => existing = parent.to_path_buf(),
            _ => {
                existing = PathBuf::from(".");
                break;
            }
        }
    }
    let resolved = std::fs::canonicalize(&existing).map_err(|source| IdentityError::Io {
        operation: "resolve",
        resource: "state_dir".to_string(),
        path: existing.clone(),
        source,
    })?;
    for ancestor in resolved.ancestors() {
        if ancestor.join(".git").exists() {
            return Err(IdentityError::StateDirInsideGit {
                state_dir: state_dir.to_path_buf(),
                worktree: ancestor.to_path_buf(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "prepare_tests.rs"]
mod tests;
