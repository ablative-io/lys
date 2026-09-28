//! The directory service's configuration, written from the layout and the
//! deployment configuration. Every path is under the data root; every
//! credential is a file the service reads, never a value written here.
//!
//! The one choice it carries that neither of those holds, the Rauthy user
//! who signs in as the administrator, is read back from the configuration
//! already written, so an upgrade renders the new build's configuration
//! without asking Rauthy.

use serde_json::{Value, json};

use super::super::config::DeploymentConfig;
use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::private_files;
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

/// The file under state holding the seed Lys signs products' ID tokens with,
/// written owner-only by the install.
pub const PROVIDER_KEY_FILE: &str = "provider-signing.key";

/// How long a sign-in lasts, in seconds: one working day.
pub const SESSION_SECONDS: u64 = 28_800;

/// Rauthy's issuer for the deployment's public origin: the origin, `/auth/v1`
/// and a trailing slash, exactly as its discovery document states it.
pub fn issuer(config: &DeploymentConfig) -> String {
    format!(
        "{}/auth/v1/",
        config.issuer.public_origin.trim_end_matches('/')
    )
}

/// What an earlier run's service configuration named that a run again keeps:
/// the administrator, and the products registered as clients of Lys.
#[derive(Debug, Default)]
pub struct Carried {
    /// The administrator an earlier configuration named.
    pub administrator: Option<Value>,
    /// The products an earlier configuration registered as clients of Lys.
    pub products: Option<Value>,
}

/// The configuration as the service reads it, with the screens served when
/// `surface` is present.
///
/// No administrator is written for a new install: the person makes the
/// administrator on the setup page, which the `setup` settings configure.
/// What `carried` names, an earlier configuration's administrator and
/// registered products, is written again, so an install run again never
/// loses either.
pub fn render(
    layout: &Layout,
    config: &DeploymentConfig,
    carried: &Carried,
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
        "sign_in_api": format!("{}/auth/v1", config.issuer.admin_url.trim_end_matches('/')),
        "setup": {
            "code_file": state.join(super::setup_code::CODE_FILE).display().to_string(),
            "administrator_file": layout.administrator_file().display().to_string(),
            "email": config.deployment.admin_email,
        },
        "link_audit_source": {"issuer": issuer, "subject": LINK_AUDIT_SUBJECT},
        "session_seconds": SESSION_SECONDS,
        "secure_cookie": false,
        "grant_log_dir": dir("grant-log"),
        "grant_log_origin": GRANT_LOG_ORIGIN,
        "grant_model_file": layout.grant_model().display().to_string(),
        "spicedb": {
            "endpoint": format!("127.0.0.1:{}", config.spicedb.http_port),
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
        "provider": {
            "key_file": state.join(PROVIDER_KEY_FILE).display().to_string(),
            "clients": carried.products.clone().unwrap_or_else(|| json!([])),
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
        "budgets_dir": dir("budgets"),
        "policies_dir": dir("policies"),
        "stops_dir": dir("stops"),
        "goals_dir": dir("goals"),
        "reviews_dir": dir("reviews"),
        "runner_socket": layout.runner_socket().display().to_string(),
    });
    if let Some(administrator) = &carried.administrator {
        rendered["administrator"] = administrator.clone();
    }
    if surface {
        rendered["surface_dir"] = Value::String(layout.surface_dir().display().to_string());
    }
    rendered
}

/// What the configuration written under `layout` names that a run again
/// keeps, or `None` when no configuration has been written.
pub fn carried(layout: &Layout) -> IdentityResult<Option<Carried>> {
    let path = layout.service_config();
    let Some(bytes) = private_files::read(&path)? else {
        return Ok(None);
    };
    let earlier: Value = serde_json::from_slice(&bytes).map_err(|error| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "read",
            "identity.json",
            error.to_string(),
        )
        .at(&path)
    })?;
    let named = |value: Option<&Value>| value.filter(|value| !value.is_null()).cloned();
    Ok(Some(Carried {
        administrator: named(earlier.get("administrator")),
        products: named(earlier.pointer("/provider/clients")),
    }))
}
