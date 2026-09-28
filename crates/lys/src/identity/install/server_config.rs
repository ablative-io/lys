//! The directory service's configuration, written from the layout and the
//! deployment configuration. Every path is under the data root; every
//! credential is a file the service reads, never a value written here.

use serde_json::{Value, json};

use super::super::config::DeploymentConfig;
use super::layout::{BROKER_PORT, Layout, SERVICE_PORT};

/// The origin the directory service names its own log by.
pub const LOG_ORIGIN: &str = "lys-identity";

/// The origin the grant log is named by.
pub const GRANT_LOG_ORIGIN: &str = "lys-identity-grants";

/// The name the secrets broker knows the service by.
pub const SERVICE_NAME: &str = "lys-identity";

/// The subject the link-audit source signs in as; a name reserved for the
/// auditing tool, never a person.
pub const LINK_AUDIT_SUBJECT: &str = "lys-link-audit";

/// The file the sign-in providers API key is written to, under state.
pub const PROVIDERS_KEY_FILE: &str = "sign-in-providers-api-key";

/// How long a sign-in lasts, in seconds: one working day.
pub const SESSION_SECONDS: u64 = 28_800;

/// Rauthy's issuer for the deployment's public origin.
pub fn issuer(config: &DeploymentConfig) -> String {
    format!(
        "{}/auth/v1",
        config.issuer.public_origin.trim_end_matches('/')
    )
}

/// The configuration as the service reads it, with `administrator` the
/// Rauthy user id that signs in as the administrator and the screens served
/// when `surface` is present.
pub fn render(
    layout: &Layout,
    config: &DeploymentConfig,
    administrator: &str,
    surface: bool,
) -> Value {
    let issuer = issuer(config);
    let state = config.state_dir();
    let data = layout.data_dir();
    let dir = |name: &str| Value::String(data.join(name).display().to_string());
    let mut rendered = json!({
        "listen": format!("127.0.0.1:{SERVICE_PORT}"),
        "log_dir": dir("directory-log"),
        "log_origin": LOG_ORIGIN,
        "event_key_file": layout.service_key().display().to_string(),
        "issuer": issuer,
        "client_id": config.clients.platform.id,
        "client_secret_file": state
            .join(format!("{}-client-secret", config.clients.platform.id))
            .display()
            .to_string(),
        "redirect_url": format!("{}/api/callback", Layout::service_url()),
        "administrator": {"issuer": issuer, "subject": administrator},
        "link_audit_source": {"issuer": issuer, "subject": LINK_AUDIT_SUBJECT},
        "session_seconds": SESSION_SECONDS,
        "secure_cookie": false,
        "grant_log_dir": dir("grant-log"),
        "grant_log_origin": GRANT_LOG_ORIGIN,
        "grant_model_file": layout.grant_model().display().to_string(),
        "spicedb": {
            "endpoint": format!("http://127.0.0.1:{}", config.spicedb.http_port),
            "key_file": state.join("spicedb-preshared-key").display().to_string(),
        },
        "secrets": {
            "broker": format!("http://127.0.0.1:{BROKER_PORT}"),
            "service": SERVICE_NAME,
            "service_key_file": layout.service_key().display().to_string(),
        },
        "sign_in_providers": {
            "api": format!("{}/auth/v1", config.issuer.admin_url.trim_end_matches('/')),
            "api_key_file": state.join(PROVIDERS_KEY_FILE).display().to_string(),
        },
        "requests_dir": dir("requests"),
        "certificates_dir": dir("certificates"),
        "network_file": dir("network.json"),
        "roles_file": dir("roles.json"),
        "provisioning_file": dir("provisioning.json"),
        "homes_dir": dir("homes"),
        "runtime_dir": dir("runtime"),
        "service_accounts_dir": dir("service-accounts"),
        "teams_dir": dir("teams"),
        "stops_dir": dir("stops"),
        "reviews_dir": dir("reviews"),
    });
    if surface {
        rendered["surface_dir"] = Value::String(layout.surface_dir().display().to_string());
    }
    rendered
}
