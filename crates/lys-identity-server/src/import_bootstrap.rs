//! Installation provisioning is a recorded root grant and delegation, not
//! a special runtime permission. Stable operation ids resume a partial
//! setup and never restore a revoked grant or retired account. The service
//! records itself as creating the credential; it fabricates no OIDC session.

use std::fs;
use std::str::FromStr;

use lys_identity::grants::{
    Action, DelegateRequest, PassOn, RecipientKind, Relation, Resource, RootRequest, Route, Window,
};
use lys_identity::{IdentityId, OperationId, ServiceAccountId};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::apps_binding::{Registrar, sha256_hex};
use crate::apps_state::{By, Line};
use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::service_accounts_state::Created;
use crate::session::now;

fn unavailable() -> ServerError {
    ServerError::ConfigInvalid {
        reason:
            "the loader credential is absent, unreadable or not an owner-only registrar credential"
                .to_owned(),
    }
}

fn operation(account: &str, label: &str) -> OperationId {
    let hash = Sha256::digest(format!("lys/import-bootstrap/v1/{account}/{label}"));
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    OperationId::from_bytes(id)
}

#[cfg(unix)]
pub(crate) fn read(path: &std::path::Path) -> Result<Zeroizing<String>, ServerError> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let metadata = fs::symlink_metadata(path).map_err(|_error| unavailable())?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(unavailable());
    }
    let mut file = fs::File::open(path).map_err(|_error| unavailable())?;
    let opened = file.metadata().map_err(|_error| unavailable())?;
    if !opened.is_file()
        || opened.permissions().mode() & 0o077 != 0
        || opened.dev() != metadata.dev()
        || opened.ino() != metadata.ino()
    {
        return Err(unavailable());
    }
    let mut bytes = Zeroizing::new(String::new());
    file.read_to_string(&mut bytes)
        .map_err(|_error| unavailable())?;
    Ok(bytes)
}

#[cfg(not(unix))]
pub(crate) fn read(path: &std::path::Path) -> Result<Zeroizing<String>, ServerError> {
    Err(ServerError::ConfigInvalid {
        reason: format!(
            "owner-only loader credential {} requires Unix",
            path.display()
        ),
    })
}

/// Bring an older installation forward once its administrator has a person
/// record. Before first-run setup there is nobody to own an account, so no
/// account or authority is invented. Setup calls this again after recording
/// the administrator; upgrade reaches it when the new service starts.
pub(crate) fn ensure(state: &AppState) -> Result<(), ServerError> {
    let Some(path) = &state.import_credential_file else {
        return Ok(());
    };
    let Some(administrator) = state.admission.administrator_login()? else {
        return Ok(());
    };
    let bytes = read(path)?;
    let mut parts = bytes.trim().split('.');
    let (Some("lys-registrar"), Some(account), Some(secret), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(unavailable());
    };
    let id = ServiceAccountId::from_str(account).map_err(|_error| unavailable())?;
    if secret.len() != 64 || !secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(unavailable());
    }
    let owner = with_directory(state, |directory| {
        let Some(owner) = directory.projection()?.person_for(&administrator) else {
            return Ok(None);
        };
        let accounts = state.service_accounts.as_ref().ok_or_else(unavailable)?;
        let mut accounts =
            accounts
                .lock()
                .map_err(|error| ServerError::ServiceAccountsUnavailable {
                    reason: format!("the service accounts lock is poisoned: {error}"),
                })?;
        accounts.create(Created {
            id: account.to_owned(),
            owner: owner.to_string(),
            name: "Lys directory loader".to_owned(),
            description: "Imports directory entries through explicitly delegated grants".to_owned(),
            by: crate::read_api::login(&administrator),
            at: now(),
        })?;
        drop(accounts);
        let mut apps =
            state
                .apps
                .lock()
                .map_err(|error| crate::apps_error::AppError::AppsUnavailable {
                    reason: format!("the apps lock is poisoned: {error}"),
                })?;
        apps.keep(Line::Registrar(Registrar {
            operation: operation(account, "credential").to_string(),
            service_account: account.to_owned(),
            secret_sha256: sha256_hex(secret),
            by: By::Start,
            at: now(),
        }))?;
        Ok(Some(owner))
    })?;
    let Some(owner) = owner else { return Ok(()) };
    crate::grants::with_grants(state, |judged| {
        for collection in ["apps", "agents"] {
            let resource = Resource::new("directory", collection)?;
            let root = judged
                .grants
                .issue_root(
                    judged.directory,
                    &RootRequest {
                        operation: operation(account, &format!("{collection}/root")),
                        caller: IdentityId::Person(owner),
                        route: Route::Api,
                        holder: owner,
                        resource: resource.clone(),
                        relation: Relation::new("editor")?,
                        pass_on: PassOn::to(
                            [Action::new("view")?, Action::new("edit")?].into(),
                            [RecipientKind::ServiceAccount].into(),
                        )?,
                        window: Window::new(0, None)?,
                    },
                    now(),
                )?
                .event
                .grant();
            judged.grants.delegate(
                judged.directory,
                &DelegateRequest {
                    operation: operation(account, &format!("{collection}/delegated")),
                    caller: IdentityId::Person(owner),
                    route: Route::Api,
                    source: root,
                    recipient: IdentityId::ServiceAccount(id),
                    responsible: owner,
                    resource,
                    relation: Relation::new("editor")?,
                    pass_on: PassOn::UseOnly,
                    window: Window::new(0, None)?,
                },
                now(),
            )?;
        }
        Ok(())
    })
}
