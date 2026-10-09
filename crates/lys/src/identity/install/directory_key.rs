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

use base64::Engine;
use serde_json::{Value, json};

use super::server_config;
use crate::identity::config::DeploymentConfig;
use crate::identity::credentials::Credential;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use crate::identity::prepare::{API_KEY_NAME, API_KEY_SECRET, bootstrap_api_key, read_secret};
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

fn invalid_rights() -> IdentityError {
    IdentityError::new(ErrorKind::RauthyUnexpected, "verify live API key rights", "api_keys",
        "the issuer answered malformed or duplicate API key rights")
}

fn rights(access: &Value) -> IdentityResult<Rights> {
    let mut rights = Rights::new();
    for entry in access.as_array().ok_or_else(invalid_rights)? {
        let group = entry.get("group").and_then(Value::as_str)
            .filter(|group| !group.is_empty()).ok_or_else(invalid_rights)?;
        let mut granted = BTreeSet::new();
        for right in entry.get("access_rights").and_then(Value::as_array).ok_or_else(invalid_rights)? {
            let right = right.as_str().filter(|right| !right.is_empty()).ok_or_else(invalid_rights)?;
            if !granted.insert(right.to_owned()) { return Err(invalid_rights()); }
        }
        if rights.insert(group.to_owned(), granted).is_some() { return Err(invalid_rights()); }
    }
    Ok(rights)
}

fn forbidden(error: IdentityError, right: &str) -> IdentityError {
    if error.kind() != ErrorKind::RauthyForbidden { return error; }
    IdentityError::new(ErrorKind::RauthyForbidden, "verify live API key rights", API_KEY_NAME,
        format!("the live key cannot exercise the required API key rights: {right}; in the issuer administrator screen, edit the {API_KEY_NAME} API key and grant {right}, then rerun the install; bootstrap declarations do not update a live key"))
}

fn keys(api: &RauthyApi) -> IdentityResult<Vec<Value>> {
    api.list_api_keys().map_err(|error| forbidden(error, "ApiKeys/read"))
}

fn named(keys: &[Value], name: &str) -> IdentityResult<Option<Rights>> {
    let mut matches = keys.iter().filter(|key| key.get("name").and_then(Value::as_str) == Some(name));
    let found = matches.next().map(|key| rights(&key["access"])).transpose()?;
    if matches.next().is_some() { return Err(invalid_rights()); }
    Ok(found)
}

fn differences(current: &Rights, wanted: &Rights) -> (Vec<String>, Vec<String>) {
    let missing = wanted.iter().flat_map(|(group, rights)| rights.iter()
        .filter(|right| !current.get(group).is_some_and(|held| held.contains(*right)))
        .map(|right| format!("{group}/{right}"))).collect();
    let extra = current.iter().flat_map(|(group, rights)| rights.iter()
        .filter(|right| !wanted.get(group).is_some_and(|held| held.contains(*right)))
        .map(|right| format!("{group}/{right}"))).collect();
    (missing, extra)
}

fn exact(name: &str, current: Option<&Rights>, wanted: &Rights) -> IdentityResult<()> {
    let empty = Rights::new();
    let (missing, extra) = differences(current.unwrap_or(&empty), wanted);
    if current == Some(wanted) { return Ok(()); }
    Err(IdentityError::new(ErrorKind::ReadBackMismatch, "verify live API key rights", name,
        format!("missing rights: {}; undeclared rights: {}; in the issuer administrator screen, edit the {name} API key to grant the missing rights and remove the undeclared rights, then rerun the install; bootstrap declarations do not update a live key", missing.join(", "), extra.join(", "))))
}

fn configure_rights() -> IdentityResult<Rights> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(bootstrap_api_key())
        .map_err(|error| IdentityError::new(ErrorKind::RauthyUnexpected, "decode declared API key rights", API_KEY_NAME, error.to_string()))?;
    let declaration: Value = serde_json::from_slice(&bytes).map_err(|error|
        IdentityError::new(ErrorKind::RauthyUnexpected, "parse declared API key rights", API_KEY_NAME, error.to_string()))?;
    rights(&declaration["access"])
}

fn verify_configure(api: &RauthyApi) -> IdentityResult<Vec<Value>> {
    let listed = keys(api)?;
    exact(API_KEY_NAME, named(&listed, API_KEY_NAME)?.as_ref(), &configure_rights()?)?;
    Ok(listed)
}

/// The directory key's rights as the sign-in service lists them, `None`
/// when it holds no such key. A configure key that may not list keys is
/// refused by name, with the step that grants the right.
fn listed(api: &RauthyApi) -> IdentityResult<Option<Rights>> {
    named(&keys(api)?, DIRECTORY_KEY_NAME)
}

/// Makes the key. A create whose answer was lost is resolved by listing:
/// a key that was made has its secret renewed, since the lost answer held
/// the only copy of it.
fn create(api: &RauthyApi) -> IdentityResult<Credential> {
    match api.create_api_key(DIRECTORY_KEY_NAME, &request()) {
        Err(error) if error.kind() == ErrorKind::RauthyUncertain => {
            if listed(api)?.is_some() {
                api.renew_api_key_secret(DIRECTORY_KEY_NAME).map_err(|error| forbidden(error, "ApiKeys/update"))
            } else {
                Err(error)
            }
        }
        other => other.map_err(|error| forbidden(error, "ApiKeys/create")),
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
    let wanted = rights(&directory_key_access())?;
    let token = match listed(api)? {
        None => Some(create(api)?),
        Some(current) => {
            if current != wanted
                && let Err(error) = api.update_api_key(DIRECTORY_KEY_NAME, &request())
                && error.kind() != ErrorKind::RauthyUncertain
            {
                return Err(forbidden(error, "ApiKeys/update"));
            }
            if held.is_some_and(holds_directory_token) {
                None
            } else {
                Some(api.renew_api_key_secret(DIRECTORY_KEY_NAME).map_err(|error| forbidden(error, "ApiKeys/update"))?)
            }
        }
    };
    exact(DIRECTORY_KEY_NAME, listed(api)?.as_ref(), &wanted)?;
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
    verify_configure(&api)?;
    match reconcile(&api, held.as_deref().map(Vec::as_slice))? {
        Some(token) => private_files::write(&path, token.expose().as_bytes()),
        None => Ok(Outcome::Unchanged),
    }
}

/// Reads the live authority before an upgrade changes any installed unit.
pub fn verify(config: &DeploymentConfig) -> IdentityResult<()> {
    let state = config.state_dir();
    let held = private_files::read(&state.join(server_config::PROVIDERS_KEY_FILE))?;
    let api = RauthyApi::new(&config.issuer.admin_url, Some(read_secret(&state, API_KEY_SECRET)?))?;
    let listed = verify_configure(&api)?;
    exact(DIRECTORY_KEY_NAME, named(&listed, DIRECTORY_KEY_NAME)?.as_ref(), &rights(&directory_key_access())?)?;
    if !held.as_deref().is_some_and(|token| holds_directory_token(token)) {
        return Err(IdentityError::new(ErrorKind::ReadBackMismatch, "verify the directory service's key", DIRECTORY_KEY_NAME,
            "the service key file does not hold lys_directory; after an issuer administrator grants the declared lys_configure rights, rerun lys identity install to provision the directory key before upgrading"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "directory_key_tests.rs"]
mod tests;
