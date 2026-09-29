//! The Cambium message connection an administrator declares: Cambium's base
//! URL and the explicit binding of each Cambium registry id to the Lys
//! identity it represents. Cambium alone judges message visibility; Lys
//! additionally requires the binding. The same settings and the same check
//! serve the service when it starts and the install and upgrade that write
//! them, so a wrong connection is refused before anything is stopped.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use serde::Deserialize;
use url::Url;

use crate::{AgentId, IdentityId, PersonId};

/// Administrator-declared identity bridge; no secret or service credential is stored here.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Cambium's trusted absolute base URL; HTTPS, or HTTP on loopback.
    pub url: String,
    /// Explicit bindings, never guessed from matching display names.
    pub bindings: Vec<Binding>,
}

/// One Cambium registry identity and the Lys directory identity it represents.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Cambium participant registry id.
    pub participant: String,
    /// Enduring Lys person or agent id.
    pub identity: String,
}

/// Settings that passed [`Settings::check`].
#[derive(Clone, Debug)]
pub struct Checked {
    /// The base URL, ending in `/`.
    pub url: Url,
    /// Each Cambium registry id and the Lys identity bound to it.
    pub bindings: BTreeMap<String, String>,
}

/// The person or agent `value` names.
///
/// # Errors
/// Refuses a value that is neither a person id nor an agent id.
pub fn parse_identity(value: &str) -> Result<IdentityId, crate::IdentityError> {
    if value.starts_with("person-") {
        PersonId::from_str(value).map(IdentityId::Person)
    } else {
        AgentId::from_str(value).map(IdentityId::Agent)
    }
}

impl Settings {
    /// The settings in `value`, checked.
    ///
    /// # Errors
    /// Refuses, in words, a value of another shape or one [`Settings::check`] refuses.
    pub fn from_value(value: serde_json::Value) -> Result<(Self, Checked), String> {
        let settings: Self = serde_json::from_value(value).map_err(|error| error.to_string())?;
        let checked = settings.check()?;
        Ok((settings, checked))
    }

    /// Validate the trusted endpoint and require a one-to-one identity mapping.
    ///
    /// # Errors
    /// Refuses, in words, an invalid endpoint, malformed identity or ambiguous binding.
    pub fn check(&self) -> Result<Checked, String> {
        let mut url = Url::parse(&self.url).map_err(|error| error.to_string())?;
        let loopback = url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(
                "use HTTPS or loopback HTTP, without credentials, query or fragment".to_owned(),
            );
        }
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        let mut bindings = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for binding in &self.bindings {
            parse_identity(&binding.identity)
                .map_err(|error| format!("identity {}: {error}", binding.identity))?;
            if binding.participant.is_empty()
                || !binding
                    .participant
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Err("participant must be a Cambium registry id".to_owned());
            }
            if bindings
                .insert(binding.participant.clone(), binding.identity.clone())
                .is_some()
                || !identities.insert(binding.identity.clone())
            {
                return Err(format!("ambiguous binding for {}", binding.participant));
            }
        }
        Ok(Checked { url, bindings })
    }
}
