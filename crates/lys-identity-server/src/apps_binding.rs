//! Who acts on the apps and for which app.
//!
//! A person acts through their signed-in session. An app acts through the
//! bearer credential its approval created, `Authorization: Bearer
//! lys-app.{app}.{secret}`, and so acts for that app alone, as the service
//! account its registration named: the app binding record written on
//! approval says which. A registrar service account acts through the
//! credential an administrator made it, `lys-registrar.{account}.{secret}`,
//! and registers apps only. Lys keeps the SHA-256 of each secret and never
//! the secret, compares digests in time independent of where they differ,
//! and never names a credential in a refusal.

use axum::http::{HeaderMap, header};
use lys_identity::{Actor, AuthMethod};
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::apps_error::AppError;
use crate::apps_state::{App, Approved, By, Held, Standing};
use crate::error::ServerError;
use crate::read_api::login;
use crate::routes::{AppState, hex};

/// The scheme word of an app's bearer credential.
pub const APP_CREDENTIAL: &str = "lys-app";

/// The scheme word of a registrar's bearer credential.
pub const REGISTRAR_CREDENTIAL: &str = "lys-registrar";

/// A service account bound to an app: the app binding record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// The service account.
    pub service_account: String,
    /// The app it acts for.
    pub app: String,
    /// Who bound it, by approving the registration that names it.
    pub bound_by: By,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A service account made a registrar, by the SHA-256 of its credential's secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registrar {
    /// The operation id it was made with.
    pub operation: String,
    /// The service account.
    pub service_account: String,
    /// The SHA-256 of the credential's secret, in hex.
    pub secret_sha256: String,
    /// Who made it.
    pub by: By,
    /// When.
    pub at: u64,
}

/// Who is acting.
#[derive(Debug, Clone)]
pub enum Acting {
    /// The configured administrator, through a session or the operator token.
    Administrator(Actor),
    /// A signed-in person who is not the administrator.
    Person(Actor),
    /// An app, through its credential, as the service account bound to it.
    App {
        /// The app.
        app: String,
        /// The service account bound to it, when its registration named one.
        service_account: Option<String>,
    },
    /// A registrar service account, through its credential.
    Registrar {
        /// The service account.
        service_account: String,
    },
}

impl Acting {
    /// The app the caller acts for, when it acts for one.
    pub fn app(&self) -> Option<&str> {
        match self {
            Self::App { app, .. } => Some(app),
            Self::Administrator(_) | Self::Person(_) | Self::Registrar { .. } => None,
        }
    }

    /// Whether the caller is the administrator.
    pub fn is_administrator(&self) -> bool {
        matches!(self, Self::Administrator(_))
    }

    /// How the apps' log records the caller.
    pub fn by(&self) -> By {
        match self {
            Self::Administrator(actor) | Self::Person(actor) => {
                let login = login(actor.binding());
                if actor.provenance().method() == AuthMethod::Operator {
                    By::Operator { login }
                } else {
                    By::Person { login }
                }
            }
            Self::App {
                app,
                service_account,
            } => By::ServiceAccount {
                id: service_account.clone().unwrap_or_else(|| app.clone()),
            },
            Self::Registrar { service_account } => By::ServiceAccount {
                id: service_account.clone(),
            },
        }
    }

    /// The administrator's actor, or `NotAdmitted`.
    pub fn administrator(&self) -> Result<&Actor, ServerError> {
        match self {
            Self::Administrator(actor) => Ok(actor),
            _ => Err(ServerError::NotAdmitted {
                reason: "only the administrator does this, on a Lys screen",
            }),
        }
    }
}

/// A new secret, 32 bytes from the secure random source as hex, and its SHA-256.
pub fn new_secret() -> Result<(String, String), ServerError> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| AppError::AppsUnavailable {
            reason: format!("the secure random source failed: {error}"),
        })?;
    let secret = hex(&bytes);
    let digest = sha256_hex(&secret);
    Ok((secret, digest))
}

/// The SHA-256 of `text`, in hex.
pub fn sha256_hex(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

/// Whether two digests are the same, compared in time independent of where they differ.
fn same(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0u8, |differ, (a, b)| differ | (a ^ b))
            == 0
}

/// The bearer credential the request carries, as its scheme word, its
/// holder and its secret; none when it carries no bearer credential.
fn bearer(headers: &HeaderMap) -> Result<Option<(String, String, String)>, AppError> {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return Ok(None);
    };
    let refused = AppError::CredentialRefused {
        reason: "the Authorization header is not `Bearer` and a Lys app or registrar credential",
    };
    let text = value.to_str().map_err(|_unreadable| refused.clone())?;
    let credential = text
        .strip_prefix("Bearer ")
        .ok_or_else(|| refused.clone())?;
    let mut words = credential.splitn(3, '.');
    match (words.next(), words.next(), words.next()) {
        (Some(scheme), Some(holder), Some(secret)) if !secret.is_empty() => Ok(Some((
            scheme.to_owned(),
            holder.to_owned(),
            secret.to_owned(),
        ))),
        _ => Err(refused),
    }
}

/// Who is acting on `headers`: the app or registrar whose credential the
/// request carries, or else the signed-in person. A credential that is
/// carried and does not verify is refused by name, never passed over for
/// the session.
pub fn acting(
    state: &AppState,
    held: &Held,
    headers: &HeaderMap,
    projection: &lys_identity::projection::Projection,
) -> Result<Acting, ServerError> {
    let Some((scheme, holder, secret)) = bearer(headers)? else {
        let actor = crate::routes::signed_in(state, headers)?;
        return Ok(if state.admission.is_administrator(projection, &actor)? {
            Acting::Administrator(actor)
        } else {
            Acting::Person(actor)
        });
    };
    let digest = sha256_hex(&secret);
    match scheme.as_str() {
        APP_CREDENTIAL => app_acting(state, held, &holder, &digest),
        REGISTRAR_CREDENTIAL => {
            let made = held
                .registrars
                .iter()
                .rev()
                .find(|registrar| registrar.service_account == holder)
                .filter(|registrar| same(&registrar.secret_sha256, &digest));
            match made {
                Some(_) => {
                    active_account(state, &holder)?;
                    Ok(Acting::Registrar {
                        service_account: holder,
                    })
                }
                None => Err(AppError::CredentialRefused {
                    reason: "no registrar holds that credential",
                }
                .into()),
            }
        }
        _ => Err(AppError::CredentialRefused {
            reason: "the credential's scheme is neither lys-app nor lys-registrar",
        }
        .into()),
    }
}

/// The app acting through the credential whose secret has `digest`.
pub(crate) fn app_acting(
    state: &AppState,
    held: &Held,
    app: &str,
    digest: &str,
) -> Result<Acting, ServerError> {
    let refused = || AppError::CredentialRefused {
        reason: "no approved app holds that credential",
    };
    let found = held.app(app).ok_or_else(refused)?;
    let approved = found.approved.as_ref().ok_or_else(refused)?;
    if !same(&approved.client.secret_sha256, digest) {
        return Err(refused().into());
    }
    if found.standing() == Standing::Retired {
        return Err(AppError::AppRetired {
            app: app.to_owned(),
        }
        .into());
    }
    if let Some(binding) = &approved.binding {
        active_account(state, &binding.service_account)?;
    }
    Ok(Acting::App {
        app: app.to_owned(),
        service_account: approved
            .binding
            .as_ref()
            .map(|binding| binding.service_account.clone()),
    })
}

fn active_account(state: &AppState, id: &str) -> Result<(), ServerError> {
    let unavailable = |reason: String| ServerError::ServiceAccountsUnavailable { reason };
    let store = state
        .service_accounts
        .as_ref()
        .ok_or_else(|| unavailable("the service account store is not configured".to_owned()))?;
    let mut store = store.lock().map_err(|error| {
        unavailable(format!("the service account store is unavailable: {error}"))
    })?;
    store.settle()?;
    match store.account(id) {
        Some(account) if !account.is_retired() => Ok(()),
        _ => Err(AppError::CredentialRefused {
            reason: "the credential's service account is not active",
        }
        .into()),
    }
}

/// The app whose sign-in client is `client_id`, when that client may complete
/// a sign-in to `redirect` with `secret`: the lookup Lys's provider makes at
/// the token exchange, judged from the apps' record at the request. A client
/// exists only once its app is approved, so a pending or declined app's is
/// refused `app_not_approved`, a retired app's `app_retired`, an id no app
/// holds or a secret that does not verify `credential_refused`, and an
/// address the registration does not list `redirect_invalid`. The secret is
/// compared by its SHA-256 and named in no refusal.
pub fn sign_in_client<'a>(
    held: &'a Held,
    client_id: &str,
    secret: &str,
    redirect: &str,
) -> Result<&'a App, AppError> {
    let (app, approved) = approved_client(held, client_id)?;
    if !same(&approved.client.secret_sha256, &sha256_hex(secret)) {
        return Err(AppError::CredentialRefused {
            reason: "no client by that id holds that secret",
        });
    }
    listed(app, redirect)?;
    Ok(app)
}

/// The app whose sign-in client is `client_id`, when `redirect` is an address
/// its registration lists: the lookup Lys's provider makes at authorize, where
/// a product presents no secret. The refusals are `sign_in_client`'s, without
/// the secret's.
pub fn sign_in_redirect<'a>(
    held: &'a Held,
    client_id: &str,
    redirect: &str,
) -> Result<&'a App, AppError> {
    let (app, _approved) = approved_client(held, client_id)?;
    listed(app, redirect)?;
    Ok(app)
}

/// The approved app `client_id` names, with its approval; a pending or
/// declined app is `app_not_approved`, a retired one `app_retired`, and an
/// id no approved app's client holds `credential_refused`.
fn approved_client<'a>(
    held: &'a Held,
    client_id: &str,
) -> Result<(&'a App, &'a Approved), AppError> {
    let refused = || AppError::CredentialRefused {
        reason: "no approved app holds that client id",
    };
    let app = held.app(client_id).ok_or_else(refused)?;
    match app.standing() {
        Standing::Pending | Standing::Declined => {
            return Err(AppError::AppNotApproved {
                app: client_id.to_owned(),
            });
        }
        Standing::Retired => {
            return Err(AppError::AppRetired {
                app: client_id.to_owned(),
            });
        }
        Standing::Approved => {}
    }
    let approved = app.approved.as_ref().ok_or_else(refused)?;
    if approved.client.client_id != client_id {
        return Err(refused());
    }
    Ok((app, approved))
}

/// Refuse `redirect` unless the app's registration lists it exactly.
fn listed(app: &App, redirect: &str) -> Result<(), AppError> {
    if app
        .registered
        .redirects
        .iter()
        .any(|listed| listed == redirect)
    {
        return Ok(());
    }
    Err(AppError::RedirectInvalid {
        address: redirect.to_owned(),
        reason: "is not an address the app's registration lists",
    })
}

#[cfg(test)]
#[path = "apps_binding_tests.rs"]
mod tests;
