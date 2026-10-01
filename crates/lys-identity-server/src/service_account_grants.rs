//! Grant principals come from the settled service-account log, not from a
//! credential's label. The owner remains the responsible person, and a
//! retired account or inactive owner cannot exercise a grant.

use std::str::FromStr;
use std::sync::PoisonError;

use axum::http::{HeaderMap, header};
use lys_identity::projection::Projection;
use lys_identity::{IdentityId, LifecycleState, LoginBinding, PersonId, Profile, ServiceAccountId};

use crate::apps_binding::{Acting, acting};
use crate::error::ServerError;
use crate::grants::Judged;
use crate::routes::AppState;

/// Record a machine caller explicitly beside its responsible person. The
/// owner's binding is provenance, never an administrator admission.
pub(crate) fn actor(
    judged: &Judged<'_>,
    caller: IdentityId,
) -> Result<(lys_identity::Actor, PersonId), ServerError> {
    let IdentityId::ServiceAccount(id) = caller else {
        return Err(ServerError::NotAdmitted {
            reason: "expected a service-account caller",
        });
    };
    let owner = judged
        .directory
        .record(caller)
        .and_then(lys_identity::projection::Record::responsible)
        .ok_or(ServerError::ServiceAccountUnknown)?;
    let binding = judged
        .directory
        .record(IdentityId::Person(owner))
        .and_then(|record| record.bindings().first())
        .cloned()
        .ok_or(ServerError::NotAdmitted {
            reason: "the service account's owner has no recorded login",
        })?;
    Ok((
        lys_identity::Actor::new(
            binding,
            lys_identity::Provenance::new(
                lys_identity::AuthMethod::ServiceAccountBearer(id),
                crate::session::now(),
            ),
        ),
        owner,
    ))
}

/// Check an ordinary edit grant on one directory collection. Authentication
/// alone admits nothing, and this read-only check creates no duplicate use
/// events on an idempotent import.
pub(crate) fn admit(
    judged: &mut Judged<'_>,
    caller: IdentityId,
    collection: &str,
) -> Result<(), ServerError> {
    use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
    let request = ExerciseRequest {
        caller,
        route: Route::Api,
        resource: Resource::new("directory", collection)?,
        action: Action::new("edit")?,
    };
    crate::grants::decide(
        judged,
        &request,
        crate::session::now(),
        None,
        crate::grants::Decision::Explain,
    )?;
    Ok(())
}

/// A request projection augmented from the accounts' current committed state.
pub(crate) fn projection(
    state: &AppState,
    directory: &Projection,
) -> Result<Projection, ServerError> {
    let Some(accounts) = &state.service_accounts else {
        return expanded(directory, &crate::service_accounts_state::Held::default());
    };
    let mut accounts = accounts.lock().unwrap_or_else(PoisonError::into_inner);
    accounts.settle()?;
    expanded(directory, accounts.held())
}

fn expanded(
    directory: &Projection,
    accounts: &crate::service_accounts_state::Held,
) -> Result<Projection, ServerError> {
    let mut projection = directory.clone();
    #[cfg(test)]
    tests::copied(directory, &projection);
    for account in &accounts.accounts {
        #[cfg(test)]
        tests::visited();
        let created = &account.created;
        projection.service_account(
            ServiceAccountId::from_str(&created.id)?,
            PersonId::from_str(&created.owner)?,
            Profile::new(&created.name)?,
            account.is_retired(),
            LoginBinding::new(&created.by.provider, &created.by.subject)?,
        )?;
    }
    Ok(projection)
}

#[cfg(test)]
#[path = "service_account_grants_tests.rs"]
mod tests;

/// Authenticate a grant caller. Bearer authentication never falls back to a
/// cookie; it names the account itself and confers no grant or root powers.
pub(crate) fn caller(
    state: &AppState,
    headers: &HeaderMap,
    judged: &Judged<'_>,
) -> Result<IdentityId, ServerError> {
    if !headers.contains_key(header::AUTHORIZATION) {
        return crate::grants::caller(state, headers, judged.directory);
    }
    let (Acting::Registrar {
        service_account: account,
    }
    | Acting::App {
        service_account: Some(account),
        ..
    }) = acting(state, judged.apps.held(), headers, judged.directory)?
    else {
        return Err(ServerError::NotAdmitted {
            reason: "the credential names no service account",
        });
    };
    let identity = IdentityId::ServiceAccount(ServiceAccountId::from_str(&account)?);
    let record = judged
        .directory
        .record(identity)
        .ok_or(ServerError::ServiceAccountUnknown)?;
    if record.state() != LifecycleState::Active {
        return Err(ServerError::NotAdmitted {
            reason: "the service account or its owner is not active",
        });
    }
    Ok(identity)
}
