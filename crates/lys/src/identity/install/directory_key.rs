//! The API key the directory service holds: made by the install, and
//! narrower than the install's own key.
//!
//! The install's configure key carries rights the running service must
//! never hold. One is Secrets update, which writes Lys's password policy and
//! in the sign-in service also rotates its signing keys and regenerates any
//! client's secret. The others are the API key rights this key is made
//! with. The service is handed this key alone, [`DIRECTORY_KEY_NAME`],
//! whose rights are exactly [`directory_key_access`]: read, create and
//! update on clients, users and sign-in providers, and read on secrets.
//!
//! Every install sets the key's rights to exactly those and reads them
//! back, so a key widened by hand is narrowed again and a key holding other
//! rights is refused by name. The key's token is renewed only when the
//! service's key file does not already hold this key's token, so running
//! the install again leaves the running service's key working.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::server_config;
use crate::identity::config::DeploymentConfig;
use crate::identity::credentials::Credential;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use crate::identity::prepare::{API_KEY_SECRET, read_secret};
use crate::identity::private_files::{self, Outcome};
use crate::identity::rauthy::RauthyApi;

/// The name of the API key the directory service holds.
pub const DIRECTORY_KEY_NAME: &str = "lys_directory";

/// The directory service's rights: read, create and update on clients,
/// users and sign-in providers, and read on secrets. Never Secrets update
/// and never a right over API keys.
pub fn directory_key_access() -> Value {
    json!([
        {"group": "Clients", "access_rights": ["read", "create", "update"]},
        {"group": "Secrets", "access_rights": ["read"]},
        {"group": "Users", "access_rights": ["read", "create", "update"]},
        {"group": "AuthProviders", "access_rights": ["read", "create", "update"]},
    ])
}

fn request() -> Value {
    json!({
        "name": DIRECTORY_KEY_NAME,
        "exp": null,
        "access": directory_key_access(),
    })
}

/// Rights by group, each group's rights as a set, so the order the
/// sign-in service answers them in does not matter.
type Rights = BTreeMap<String, BTreeSet<String>>;

fn rights(access: &Value) -> Rights {
    access
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let group = entry.get("group")?.as_str()?.to_owned();
            let granted = entry
                .get("access_rights")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            Some((group, granted))
        })
        .collect()
}

/// The directory key's rights as the sign-in service lists them, `None`
/// when it holds no such key. A configure key that may not list keys is
/// refused by name, with the step that grants the right.
fn listed(api: &RauthyApi) -> IdentityResult<Option<Rights>> {
    let keys = api.list_api_keys().map_err(|error| {
        if error.kind() == ErrorKind::RauthyForbidden {
            IdentityError::new(
                ErrorKind::RauthyForbidden,
                "make the directory service's key",
                DIRECTORY_KEY_NAME,
                "the install's configure key lacks the API key rights the directory service's own key is made with; upgrading the install grants them to the key",
            )
        } else {
            error
        }
    })?;
    Ok(keys
        .iter()
        .find(|key| key.get("name").and_then(Value::as_str) == Some(DIRECTORY_KEY_NAME))
        .map(|key| rights(&key["access"])))
}

/// Makes the key. A create whose answer was lost is resolved by listing:
/// a key that was made has its secret renewed, since the lost answer held
/// the only copy of it.
fn create(api: &RauthyApi) -> IdentityResult<Credential> {
    match api.create_api_key(DIRECTORY_KEY_NAME, &request()) {
        Err(error) if error.kind() == ErrorKind::RauthyUncertain => {
            if listed(api)?.is_some() {
                api.renew_api_key_secret(DIRECTORY_KEY_NAME)
            } else {
                Err(error)
            }
        }
        other => other,
    }
}

/// Whether `held` is a token of the directory key.
fn holds_directory_token(held: &[u8]) -> bool {
    held.strip_prefix(DIRECTORY_KEY_NAME.as_bytes())
        .is_some_and(|rest| rest.len() > 1 && rest.starts_with(b"$"))
}

/// Makes or narrows the directory key through `api`, reads its rights back,
/// and answers a new token for the service when `held`, the token the
/// service's key file holds, is not already this key's.
pub fn reconcile(api: &RauthyApi, held: Option<&[u8]>) -> IdentityResult<Option<Credential>> {
    let wanted = rights(&directory_key_access());
    let token = match listed(api)? {
        None => Some(create(api)?),
        Some(current) => {
            if current != wanted
                && let Err(error) = api.update_api_key(DIRECTORY_KEY_NAME, &request())
                && error.kind() != ErrorKind::RauthyUncertain
            {
                return Err(error);
            }
            if held.is_some_and(holds_directory_token) {
                None
            } else {
                Some(api.renew_api_key_secret(DIRECTORY_KEY_NAME)?)
            }
        }
    };
    if listed(api)? != Some(wanted) {
        return Err(IdentityError::new(
            ErrorKind::ReadBackMismatch,
            "make the directory service's key",
            DIRECTORY_KEY_NAME,
            "the key holds other rights than the directory service is given",
        ));
    }
    Ok(token)
}

/// Makes the directory service's key with the install's configure key and
/// writes its token where the service reads it, answering whether the file
/// changed. The token is written as the sign-in service reads it: the
/// key's name, a dollar sign and its secret.
pub fn provide(config: &DeploymentConfig) -> IdentityResult<Outcome> {
    let state = config.state_dir();
    let path = state.join(server_config::PROVIDERS_KEY_FILE);
    let held = private_files::read(&path)?;
    let credential = read_secret(&state, API_KEY_SECRET)?;
    let api = RauthyApi::new(&config.issuer.admin_url, Some(credential))?;
    match reconcile(&api, held.as_deref().map(Vec::as_slice))? {
        Some(token) => private_files::write(&path, token.expose().as_bytes()),
        None => Ok(Outcome::Unchanged),
    }
}

#[cfg(test)]
#[path = "directory_key_tests.rs"]
mod tests;
