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

/// How long a sign-in lasts, in seconds: thirty days, the same window as
/// the identity server's `ENDED_REFRESH_KEPT_SECONDS`. A daily sign-in on
/// every device is the work that makes people go around Lys, and sign-out,
/// suspension and retirement already end a session early, so a shorter
/// length buys no safety those acts don't (DIRECTORY-099 R4).
pub const SESSION_SECONDS: u64 = 2_592_000;

/// The sign-in length every install was written with before
/// [`SESSION_SECONDS`] became thirty days: one working day. An upgrade moves
/// an install still at it rather than carrying it as an operator's choice.
pub const FORMER_SESSION_SECONDS: u64 = 28_800;

/// Rauthy's issuer for the deployment's public origin: the origin, `/auth/v1`
/// and a trailing slash, exactly as its discovery document states it.
#[must_use]
pub fn issuer(config: &DeploymentConfig) -> String {
    format!(
        "{}/auth/v1/",
        config.issuer.public_origin.trim_end_matches('/')
    )
}

/// The address the issuer sends a person back to after the platform's
/// sign-in, which must be one the platform's client registers
/// (`clients.platform.redirect_uris`, from the deployment configuration).
/// An origin a browser reaches through a fronting proxy is the address the
/// registered callback names: an `https` public origin, which the
/// configuration accepts for every host that is not loopback (plain `http`
/// is refused for any other), gives `<public origin>/api/callback`. A
/// loopback `http` origin is the service's own listener, as before.
#[must_use]
pub fn redirect_url(config: &DeploymentConfig, ports: Ports) -> String {
    if config.public_tls() {
        format!(
            "{}/api/callback",
            config.issuer.public_origin.trim_end_matches('/')
        )
    } else {
        format!("{}/api/callback", ports.service_url())
    }
}

/// What an earlier run's service configuration named that a run again keeps:
/// the administrator, and the products registered as clients of Lys.
#[derive(Debug, Default)]
pub struct Carried {
    /// The listeners an earlier install recorded.
    pub ports: Ports,
    /// The administrator an earlier configuration named.
    pub administrator: Option<Value>,
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
    /// The operator's channel membership page settings an earlier
    /// configuration named (ACCESS-006 R3, R5). The install writes none of
    /// its own: no number for them is decided, so a service with none
    /// refuses every membership page by name until the operator names them.
    pub membership: Option<Value>,
    /// The written review cadence an upgrade preserves.
    pub reviews: Option<Value>,
    /// The ceiling on a pass's rights an earlier configuration named
    /// (`provider.rights_bytes`, ACCESS-002 R1). The install writes none of its
    /// own; one the operator named survives every install and upgrade run.
    pub rights_bytes: Option<Value>,
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
pub use rendering::render;
mod rendering;

/// The settings a service with no membership page bounds refuses every
/// page for, until the operator names them (ACCESS-006 R3, R5).
pub const MEMBERSHIP_SETTINGS: [&str; 4] = [
    "membership.page_rows_max",
    "membership.page_bytes_max",
    "membership.cursor_seconds",
    "membership.cursors_max",
];

/// The readback line an install or upgrade says for the `membership`
/// settings of the configuration it wrote: the values it names, each setting
/// it leaves out said as unset, or, when it names none, that every page is
/// refused until the operator names them. No value is chosen here.
#[must_use]
pub fn membership_readback(membership: Option<&Value>) -> String {
    let Some(named) = membership.filter(|named| !named.is_null()) else {
        return format!(
            "membership pages: refused membership_pages_unconfigured until the operator names {} in identity.json; no value is chosen for them",
            MEMBERSHIP_SETTINGS.join(", ")
        );
    };
    let values: Vec<String> = MEMBERSHIP_SETTINGS
        .iter()
        .map(|setting| {
            let key = setting.trim_start_matches("membership.");
            named.get(key).filter(|value| !value.is_null()).map_or_else(
                || format!("{setting} unset"),
                |value| format!("{setting} {value}"),
            )
        })
        .collect();
    format!(
        "membership pages: identity.json names {}",
        values.join(", ")
    )
}

/// The words the service's largest-rights line opens with after its name
/// (`lys-identity-server`'s `provider/rights_claim.rs`, `LARGEST_SEEN`).
const LARGEST_SEEN: &str = "provider: the largest pass rights document so far is";

/// The readback line for a pass's rights ceiling: the `provider.rights_bytes`
/// the configuration names, or unset, and the largest rights document the
/// running service has measured, the last such line in `log`, so the ceiling
/// is raised on evidence. A log not yet written says no pass is measured.
///
/// # Errors
/// Refuses unreadable provider answers or configuration that cannot be carried into the install.
pub fn provider_readback(rights_bytes: Option<&Value>, log: &Path) -> IdentityResult<String> {
    let cap = rights_bytes
        .filter(|cap| !cap.is_null())
        .map_or_else(|| "unset".to_owned(), Value::to_string);
    let text = match std::fs::read_to_string(log) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(IdentityError::new(
                ErrorKind::ConfigUnreadable,
                "read",
                "identity.log",
                error.to_string(),
            )
            .at(log));
        }
    };
    let seen = text
        .lines()
        .rev()
        .find_map(|line| line.split_once(LARGEST_SEEN).map(|(_, rest)| rest.trim()));
    Ok(match seen {
        Some(seen) => format!(
            "pass rights: provider.rights_bytes is {cap}; the largest rights document the service has measured is {seen}"
        ),
        None => format!(
            "pass rights: provider.rights_bytes is {cap}; the service has measured no pass's rights since its log began"
        ),
    })
}

/// What the configuration written under `layout` names that a run again
/// keeps, or `None` when no configuration has been written.
///
/// # Errors
/// Refuses unreadable provider answers or configuration that cannot be carried into the install.
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
        message_service: message_service(&earlier, &path)?,
        trusted_proxies: named(earlier.get("trusted_proxies")),
        membership: named(earlier.get("membership")),
        reviews: earlier.get("reviews").cloned(),
        rights_bytes: named(
            earlier
                .get("provider")
                .and_then(|provider| provider.get("rights_bytes")),
        ),
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
///
/// # Errors
/// Refuses unreadable provider answers or configuration that cannot be carried into the install.
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

#[cfg(test)]
#[path = "server_config_membership_tests.rs"]
mod membership_tests;

#[cfg(test)]
mod repair_tests {
    #[test]
    fn service_configuration_keeps_all_required_sections() -> Result<(), Box<dyn std::error::Error>>
    {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("deployment.toml");
        std::fs::write(&path, include_str!("../config/deployment-2026-09-29.toml"))?;
        let config = super::DeploymentConfig::load_install(&path)?;
        let layout = super::Layout::at(folder.path().to_path_buf());
        let value = super::render(&layout, &config, &super::Carried::default(), true);
        for section in [
            "setup",
            "password_policy",
            "spicedb",
            "secrets",
            "provider",
            "sign_in_providers",
            "link_audit_source",
        ] {
            assert!(value[section].is_object(), "missing {section}");
        }
        assert_eq!(value["session_seconds"], super::SESSION_SECONDS);
        assert_eq!(
            value["password_policy"]["length_min"],
            config.password_policy.length_min
        );
        assert_eq!(
            value["surface_dir"],
            layout.surface_dir().display().to_string()
        );
        Ok(())
    }

    /// DIRECTORY-099 R4: a new install's sign-in lasts thirty days.
    #[test]
    fn a_fresh_render_signs_people_in_for_thirty_days() -> Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("deployment.toml");
        std::fs::write(&path, include_str!("../config/deployment-2026-09-29.toml"))?;
        let config = super::DeploymentConfig::load_install(&path)?;
        let layout = super::Layout::at(folder.path().to_path_buf());
        let value = super::render(&layout, &config, &super::Carried::default(), true);
        assert_eq!(value["session_seconds"], 2_592_000);
        Ok(())
    }
}
