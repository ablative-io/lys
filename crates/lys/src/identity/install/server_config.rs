//! The directory service's configuration, written from the layout and the
//! deployment configuration. Every path is under the data root; every
//! credential is a file the service reads, never a value written here.
//!
//! The one choice it carries that neither of those holds, the Rauthy user
//! who signs in as the administrator, is read back from the configuration
//! already written, so an upgrade renders the new build's configuration
//! without asking Rauthy.

use std::path::Path;

use serde_json::{Value, json};

use super::super::config::DeploymentConfig;
use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::private_files;
use super::layout::Layout;
use super::ports::Ports;

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

/// The operator token's file in the state folder: a request carrying it acts
/// as the administrator.
pub const OPERATOR_TOKEN_FILE: &str = "operator-token";

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
    /// The listeners an earlier install recorded.
    pub ports: Ports,
    /// The administrator an earlier configuration named.
    pub administrator: Option<Value>,
    /// The products an earlier configuration registered as clients of Lys.
    pub products: Option<Value>,
    /// The message service bridge an earlier configuration declared.
    pub message_service: Option<Value>,
    /// Explicit front-proxy trust, absent unless an operator configured it.
    pub trusted_proxies: Option<Value>,
    /// The issuer an earlier configuration named.
    pub issuer: Option<String>,
    /// The issuer move an earlier configuration handed over.
    pub issuer_moved_from: Option<String>,
    /// The profile an earlier configuration named; one from before profiles
    /// existed is read as what it has been: development when it names an
    /// operator token that stands, else service.
    pub profile: Option<super::Profile>,
}

/// The configuration as the service reads it, with the screens served when
/// `surface` is present.
///
/// Lys's password policy is written as the deployment configuration states
/// it, the same values the install writes to the sign-in service, so the
/// setup and account screens show the policy that is enforced.
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
    let policy = &config.password_policy;
    let dir = |name: &str| Value::String(data.join(name).display().to_string());
    let mut rendered = json!({
        "listen": format!("127.0.0.1:{}", carried.ports.service),
        "log_dir": dir("directory-log"),
        "log_origin": LOG_ORIGIN,
        "event_key_file": layout.service_key().display().to_string(),
        "profile": carried.profile.unwrap_or_default().word(),
        "operator_token_file": match carried.profile.unwrap_or_default() {
            super::Profile::Development => Value::String(state.join(OPERATOR_TOKEN_FILE).display().to_string()),
            super::Profile::Service => Value::Null,
        },
        "operator_upgrade_file": layout.upgrade_intent().display().to_string(),
        "import_credential_file": super::super::import::credential_path(layout).display().to_string(),
        "issuer": issuer,
        "client_id": config.clients.platform.id,
        "client_secret_file": state
            .join(format!("{}-client-secret", config.clients.platform.id))
            .display()
            .to_string(),
        "redirect_url": format!("{}/api/callback", carried.ports.service_url()),
        "sign_in_api": format!("{}/auth/v1", config.issuer.admin_url.trim_end_matches('/')),
        "setup": {
            "code_file": state.join(super::setup_code::CODE_FILE).display().to_string(),
            "administrator_file": layout.administrator_file().display().to_string(),
            "email": config.deployment.admin_email,
        },
        "password_policy": {
            "length_min": policy.length_min,
            "length_max": policy.length_max,
            "lower_case": policy.lower_case,
            "upper_case": policy.upper_case,
            "digits": policy.digits,
            "special": policy.special,
            "not_recently_used": policy.not_recently_used,
        },
        "link_audit_source": {"issuer": issuer, "subject": LINK_AUDIT_SUBJECT},
        "session_seconds": SESSION_SECONDS,
        "sessions_file": state.join("sessions.json").display().to_string(),
        "secure_cookie": false,
        "grant_log_dir": dir("grant-log"),
        "grant_log_origin": GRANT_LOG_ORIGIN,
        "grant_model_file": layout.grant_model().display().to_string(),
        "spicedb": {
            "endpoint": format!("127.0.0.1:{}", config.spicedb.http_port),
            "key_file": state.join("spicedb-preshared-key").display().to_string(),
        },
        "secrets": {
            "broker": format!("http://127.0.0.1:{}", carried.ports.broker),
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
        "model_proxy": carried.ports.proxy_url(),
    });
    // When the issuer an earlier configuration named is not this one, the
    // sign-in service moved: the service is handed the earlier issuer and
    // records the move in the directory once, and the administrator's login
    // moves with every other. A move handed over before is handed over
    // again, which the directory finds already recorded.
    let moved_from = carried
        .issuer
        .clone()
        .filter(|earlier| *earlier != issuer)
        .or_else(|| carried.issuer_moved_from.clone())
        .filter(|earlier| *earlier != issuer);
    if let Some(administrator) = &carried.administrator {
        let mut administrator = administrator.clone();
        if moved_from.is_some() && administrator["issuer"].as_str() == moved_from.as_deref() {
            administrator["issuer"] = Value::String(issuer);
        }
        rendered["administrator"] = administrator;
    }
    if let Some(from) = moved_from {
        rendered["issuer_moved_from"] = Value::String(from);
    }
    if let Some(bridge) = &carried.message_service {
        rendered[MESSAGE_SERVICE] = bridge.clone();
    }
    if let Some(proxies) = &carried.trusted_proxies {
        rendered["trusted_proxies"] = proxies.clone();
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
        ports: Ports::from_value(&earlier)?,
        administrator: named(earlier.get("administrator")),
        products: named(earlier.pointer("/provider/clients")),
        message_service: message_service(&earlier, &path)?,
        trusted_proxies: named(earlier.get("trusted_proxies")),
        issuer: named(earlier.get("issuer")).and_then(|issuer| issuer.as_str().map(str::to_owned)),
        issuer_moved_from: named(earlier.get("issuer_moved_from"))
            .and_then(|issuer| issuer.as_str().map(str::to_owned)),
        profile: Some(if let Some(word) = named(earlier.get("profile")) {
            word.as_str()
                .and_then(super::Profile::from_word)
                .ok_or_else(|| {
                    IdentityError::new(
                        ErrorKind::ConfigInvalid,
                        "read",
                        "identity.json",
                        format!("profile {word} is neither service nor development"),
                    )
                    .at(&path)
                })?
        } else if named(earlier.get("operator_token_file"))
            .and_then(|file| file.as_str().map(|file| Path::new(file).is_file()))
            .unwrap_or(false)
        {
            super::Profile::Development
        } else {
            super::Profile::Service
        }),
    }))
}

/// The member the message service bridge is written under.
pub const MESSAGE_SERVICE: &str = "message_service";

/// The message service bridge `earlier` declares. An earlier build wrote it
/// as `<service>_messages` holding only `url` and `bindings`, and forwarded
/// the service's `<service>_session` cookie without naming it in the
/// configuration; that member is carried as `message_service` with that
/// cookie named, so the new build reads the same bridge.
fn message_service(earlier: &Value, path: &std::path::Path) -> IdentityResult<Option<Value>> {
    if let Some(current) = earlier
        .get(MESSAGE_SERVICE)
        .filter(|value| !value.is_null())
    {
        return Ok(Some(current.clone()));
    }
    let Some(members) = earlier.as_object() else {
        return Ok(None);
    };
    let mut older = members.iter().filter_map(|(key, value)| {
        let service = key.strip_suffix("_messages")?;
        let bridge = value.as_object()?;
        let shaped =
            bridge.len() == 2 && bridge.contains_key("url") && bridge.contains_key("bindings");
        shaped.then_some((service, bridge))
    });
    let Some((service, bridge)) = older.next() else {
        return Ok(None);
    };
    if older.next().is_some() {
        return Err(IdentityError::new(
            ErrorKind::ConfigInvalid,
            "read",
            "identity.json",
            "it declares more than one earlier message service bridge",
        )
        .at(path));
    }
    let mut carried = bridge.clone();
    carried.insert(
        "cookie".to_owned(),
        Value::String(format!("{service}_session")),
    );
    Ok(Some(Value::Object(carried)))
}

/// The message service connection in the JSON file at `path`, checked as
/// the service checks it when it starts, so a wrong one is refused before
/// anything is stopped.
pub fn messages_from(path: &Path) -> IdentityResult<Value> {
    let refused = |reason: String| {
        IdentityError::new(ErrorKind::ConfigInvalid, "read", "message_service", reason).at(path)
    };
    let bytes = std::fs::read(path).map_err(|error| refused(error.to_string()))?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|error| refused(error.to_string()))?;
    lys_identity::message_service::Settings::from_value(value.clone()).map_err(|reason| {
        refused(format!(
            "must be the connection itself, an object of url, cookie and bindings: {reason}"
        ))
    })?;
    Ok(value)
}
