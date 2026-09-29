//! Approved-app authority snapshots and unambiguous client authentication.

use std::collections::BTreeSet;

use axum::http::{HeaderMap, header};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::apps_state::{App, Standing};
use crate::provider::ProductClient;

use super::{Refusal, exact, same};

/// Explicit authority source. Equal client-id text never joins these registries.
pub enum ClientSource<'a> {
    /// The existing configured client; refused for this new delegated class.
    Configured(&'a ProductClient),
    /// A freshly settled app record, checked below for approval and revision.
    ApprovedApp(&'a App),
}

/// Pure authority coordinates, not a signed record or permission to issue tokens.
/// Credential and redirect fingerprints stay private to the server.
#[derive(Clone, PartialEq, Eq)]
pub struct Binding {
    app: String,
    registration: String,
    approval: String,
    version: u64,
    version_operation: String,
    client: String,
    credential: String,
    redirects: [u8; 32],
}

impl std::fmt::Debug for Binding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Binding")
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

impl Binding {
    /// Capture the stable registration, approval and current version authority.
    /// No configured-client registry is consulted or inferred.
    ///
    /// # Errors
    /// Refuses missing approval/version, retirement, invalid coordinates or
    /// duplicate/empty redirect addresses.
    pub fn current(app: &App) -> Result<Self, Refusal> {
        if app.standing() != Standing::Approved {
            return Err(Refusal::BindingInvalid);
        }
        let approved = app.approved.as_ref().ok_or(Refusal::BindingInvalid)?;
        let version = app.current().ok_or(Refusal::BindingInvalid)?;
        if approved.app != app.registered.app
            || approved.client.client_id != app.registered.app
            || version.version == 0
            || [
                &app.registered.app,
                &app.registered.operation,
                &approved.operation,
                &version.operation,
            ]
            .iter()
            .any(|value| !exact(value))
            || approved.client.secret_sha256.len() != 64
            || !approved
                .client
                .secret_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(Refusal::BindingInvalid);
        }
        let redirects: BTreeSet<_> = app.registered.redirects.iter().collect();
        if redirects.is_empty()
            || redirects.len() != app.registered.redirects.len()
            || redirects.iter().any(|value| !exact(value))
        {
            return Err(Refusal::BindingInvalid);
        }
        let mut digest = Sha256::new();
        digest.update(b"lys.delegation.redirects/v1\0");
        for redirect in redirects {
            let len = u64::try_from(redirect.len()).map_err(|_error| Refusal::BindingInvalid)?;
            digest.update(len.to_be_bytes());
            digest.update(redirect.as_bytes());
        }
        Ok(Self {
            app: app.registered.app.clone(),
            registration: app.registered.operation.clone(),
            approval: approved.operation.clone(),
            version: version.version,
            version_operation: version.operation.clone(),
            client: approved.client.client_id.clone(),
            credential: approved.client.secret_sha256.clone(),
            redirects: digest.finalize().into(),
        })
    }

    /// Compare with today's settled app; retirement or any changed binding refuses.
    ///
    /// # Errors
    /// Returns `BindingChanged` for a changed, replaced, unapproved or retired app.
    pub fn check_current(&self, app: &App) -> Result<(), Refusal> {
        if Self::current(app).as_ref() == Ok(self) {
            Ok(())
        } else {
            Err(Refusal::BindingChanged)
        }
    }
}

struct Credentials {
    client: String,
    secret: Zeroizing<String>,
}

// OAuth Basic encodes each component with form encoding before joining it with
// a colon and base64 encoding it (RFC 6749 section 2.3.1). Decode exactly once.
fn component(text: &str) -> Result<Zeroizing<String>, Refusal> {
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(text.len()));
    let mut input = text.bytes();
    while let Some(byte) = input.next() {
        bytes.push(match byte {
            b'+' => b' ',
            b'%' => {
                let high = input
                    .next()
                    .and_then(digit)
                    .ok_or(Refusal::ClientAuthentication)?;
                let low = input
                    .next()
                    .and_then(digit)
                    .ok_or(Refusal::ClientAuthentication)?;
                (high << 4) | low
            }
            other => other,
        });
    }
    let decoded = std::str::from_utf8(&bytes).map_err(|_error| Refusal::ClientAuthentication)?;
    Ok(Zeroizing::new(decoded.to_owned()))
}

fn credentials(headers: &HeaderMap, form: &[(&str, &str)]) -> Result<Credentials, Refusal> {
    let mut names = BTreeSet::new();
    if form.iter().any(|(key, _)| !names.insert(*key)) {
        return Err(Refusal::ClientAuthentication);
    }
    let mut authorization = headers.get_all(header::AUTHORIZATION).iter();
    let first = authorization.next();
    if authorization.next().is_some() {
        return Err(Refusal::ClientAuthentication);
    }
    let member = |name| {
        form.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
    };
    let id = member("client_id");
    let secret = member("client_secret");
    let answer = if let Some(value) = first {
        if id.is_some() || secret.is_some() {
            return Err(Refusal::ClientAuthentication);
        }
        let text = value
            .to_str()
            .map_err(|_error| Refusal::ClientAuthentication)?;
        let mut words = text.split_ascii_whitespace();
        let scheme = words.next().ok_or(Refusal::ClientAuthentication)?;
        let encoded = words.next().ok_or(Refusal::ClientAuthentication)?;
        if !scheme.eq_ignore_ascii_case("Basic") || words.next().is_some() {
            return Err(Refusal::ClientAuthentication);
        }
        let bytes = Zeroizing::new(
            STANDARD
                .decode(encoded)
                .map_err(|_error| Refusal::ClientAuthentication)?,
        );
        let decoded =
            std::str::from_utf8(&bytes).map_err(|_error| Refusal::ClientAuthentication)?;
        let (client, secret) = decoded
            .split_once(':')
            .ok_or(Refusal::ClientAuthentication)?;
        Credentials {
            client: component(client)?.to_string(),
            secret: component(secret)?,
        }
    } else {
        Credentials {
            client: id.ok_or(Refusal::ClientAuthentication)?.to_owned(),
            secret: Zeroizing::new(secret.ok_or(Refusal::ClientAuthentication)?.to_owned()),
        }
    };
    if !exact(&answer.client) || answer.secret.is_empty() {
        return Err(Refusal::ClientAuthentication);
    }
    Ok(answer)
}

/// Parse authentication before resolving authority. The form must retain every
/// decoded pair so duplicate fields cannot be hidden by a map/struct decoder.
/// The lookup callback is invoked only after syntax and ambiguity checks pass.
/// This does not authenticate a human, grant consent or issue anything.
///
/// # Errors
/// Refuses duplicate/mixed authentication, configured-client authority, wrong
/// credentials, an unapproved app or a redirect absent from its exact set.
pub fn authenticate<'a>(
    headers: &HeaderMap,
    form: &[(&str, &str)],
    lookup: impl FnOnce(&str) -> Result<ClientSource<'a>, Refusal>,
    redirect: &str,
) -> Result<Binding, Refusal> {
    let given = credentials(headers, form)?;
    let ClientSource::ApprovedApp(app) = lookup(&given.client)? else {
        return Err(Refusal::ClientAuthority);
    };
    let binding = Binding::current(app)?;
    let digest = format!("{:x}", Sha256::digest(given.secret.as_bytes()));
    if binding.client != given.client
        || !same(binding.credential.as_bytes(), digest.as_bytes())
        || !app
            .registered
            .redirects
            .iter()
            .any(|value| value == redirect)
    {
        return Err(Refusal::ClientRefused);
    }
    Ok(binding)
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;
