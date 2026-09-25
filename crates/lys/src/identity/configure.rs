//! `lys identity configure`: reconcile the two OIDC clients and their themes
//! on Rauthy. Each reconciliation has a stable operation identifier, is
//! idempotent (a second run changes nothing and says so), and after an
//! uncertain transport outcome reads the resource back instead of writing
//! it again.

use super::config::{ClientConfig, DeployConfig};
use super::credentials::Credentials;
use super::error::{IdentityError, IdentityResult};
use super::rauthy::{Client, NewClient, Rauthy, Theme};
use super::themes::ThemeMap;

/// What one reconciliation found and did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The resource did not exist and was created.
    Created,
    /// The resource existed with other settings and was updated.
    Updated,
    /// The resource already had these settings.
    Unchanged,
}

impl Change {
    /// The word the report prints.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
        }
    }
}

/// One reconciliation, by its stable operation identifier.
#[derive(Debug, Clone)]
pub struct Reconciled {
    /// `<client id>.client` or `<client id>.theme`; the same on every run.
    pub operation: String,
    /// What was found and done.
    pub change: Change,
}

/// Which client a configuration row is, for the settings only one of them has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The platform's own confidential client: S256 PKCE.
    Platform,
    /// Cambium's client: RS256 for its verifier.
    Cambium,
}

/// Reconcile both clients and both themes, in a fixed order.
pub fn run(config: &DeployConfig) -> IdentityResult<Vec<Reconciled>> {
    let key = Credentials::api_key_header_from(&config.venue)?;
    let rauthy = Rauthy::new(&config.rauthy_origin(), Some(key));
    let themes = ThemeMap::load(&config.themes.file)?;
    let mut done = Vec::with_capacity(4);
    for (role, client, theme_name) in [
        (Role::Platform, &config.clients.platform, "identity"),
        (Role::Cambium, &config.clients.cambium, "cambium"),
    ] {
        done.push(reconcile_client(&rauthy, role, client)?);
        let theme = themes.theme_for(theme_name, &client.id)?;
        done.push(reconcile_theme(&rauthy, &theme)?);
    }
    Ok(done)
}

/// Make the client exist with the settings this row owns; leave every other
/// setting as Rauthy holds it.
pub fn reconcile_client(
    rauthy: &Rauthy,
    role: Role,
    wanted: &ClientConfig,
) -> IdentityResult<Reconciled> {
    let operation = format!("{}.client", wanted.id);
    let mut created = false;
    let held = if let Some(held) = rauthy.client(&wanted.id)? {
        held
    } else {
        let new = NewClient {
            id: wanted.id.clone(),
            name: Some(wanted.id.clone()),
            confidential: true,
            redirect_uris: wanted.redirect_uris.clone(),
        };
        sent(rauthy.create_client(&new))?;
        created = true;
        rauthy
            .client(&wanted.id)?
            .ok_or_else(|| IdentityError::Invalid {
                operation: "create the client",
                resource: wanted.id.clone(),
                detail: "rauthy holds no client after the create was sent".to_string(),
            })?
    };
    if conforms(&held, role, wanted) {
        return Ok(Reconciled {
            operation,
            change: if created {
                Change::Created
            } else {
                Change::Unchanged
            },
        });
    }
    let desired = settle(&held, role, wanted);
    sent(rauthy.update_client(&desired))?;
    let after = rauthy
        .client(&wanted.id)?
        .ok_or_else(|| IdentityError::Invalid {
            operation: "update the client",
            resource: wanted.id.clone(),
            detail: "rauthy holds no client after the update was sent".to_string(),
        })?;
    if !conforms(&after, role, wanted) {
        return Err(IdentityError::Invalid {
            operation: "update the client",
            resource: wanted.id.clone(),
            detail: "the client read back does not hold the settings sent".to_string(),
        });
    }
    Ok(Reconciled {
        operation,
        change: if created {
            Change::Created
        } else {
            Change::Updated
        },
    })
}

/// A write whose transport failed after it was sent is not a failure yet:
/// the caller reads the resource back and judges from what Rauthy holds.
fn sent(outcome: IdentityResult<()>) -> IdentityResult<()> {
    match outcome {
        Err(IdentityError::Transport { .. }) => Ok(()),
        other => other,
    }
}

/// Whether the client holds every setting this row owns. Settings the row
/// does not own, and the order Rauthy keeps lists in, are not compared.
fn conforms(held: &Client, role: Role, wanted: &ClientConfig) -> bool {
    let has = |list: &[String], word: &str| list.iter().any(|item| item == word);
    held.enabled
        && held.confidential
        && held.redirect_uris == wanted.redirect_uris
        && has(&held.flows_enabled, "authorization_code")
        && has(&held.flows_enabled, "refresh_token")
        && ["openid", "email", "profile"]
            .iter()
            .all(|scope| has(&held.scopes, scope))
        && has(&held.default_scopes, "openid")
        && match role {
            Role::Platform => held
                .challenges
                .as_deref()
                .is_some_and(|challenges| has(challenges, "S256")),
            Role::Cambium => held.access_token_alg == "RS256" && held.id_token_alg == "RS256",
        }
}

/// The settings this row owns, applied over what Rauthy holds.
fn settle(held: &Client, role: Role, wanted: &ClientConfig) -> Client {
    let mut desired = held.clone();
    desired.enabled = true;
    desired.confidential = true;
    desired.redirect_uris.clone_from(&wanted.redirect_uris);
    desired.flows_enabled = vec![
        "authorization_code".to_string(),
        "refresh_token".to_string(),
    ];
    match role {
        Role::Platform => {
            desired.challenges = Some(vec!["S256".to_string()]);
        }
        Role::Cambium => {
            desired.access_token_alg = "RS256".to_string();
            desired.id_token_alg = "RS256".to_string();
        }
    }
    for scope in ["openid", "email", "profile"] {
        if !desired.scopes.iter().any(|held| held == scope) {
            desired.scopes.push(scope.to_string());
        }
    }
    if !desired.default_scopes.iter().any(|held| held == "openid") {
        desired.default_scopes.push("openid".to_string());
    }
    desired
}

/// Make the client's theme equal `wanted`.
pub fn reconcile_theme(rauthy: &Rauthy, wanted: &Theme) -> IdentityResult<Reconciled> {
    let operation = format!("{}.theme", wanted.client_id);
    let held = rauthy.theme(&wanted.client_id)?;
    if held.same_look(wanted) {
        return Ok(Reconciled {
            operation,
            change: Change::Unchanged,
        });
    }
    sent(rauthy.write_theme(wanted))?;
    let after = rauthy.theme(&wanted.client_id)?;
    if !after.same_look(wanted) {
        return Err(IdentityError::Invalid {
            operation: "write the theme",
            resource: wanted.client_id.clone(),
            detail: "the theme read back differs from the mapping sent".to_string(),
        });
    }
    Ok(Reconciled {
        operation,
        change: Change::Updated,
    })
}
