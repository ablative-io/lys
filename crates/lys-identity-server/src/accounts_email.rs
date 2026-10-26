//! Email ownership and held-address writes shared by own and administrator changes.
use lys_identity::{Actor, IdentityId, LoginBinding};
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::sign_in_providers::SignInProviders;

pub(super) fn owner(state: &AppState, actor: &Actor) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        directory
            .projection()?
            .person_for(actor.binding())
            .map(|person| person.to_string())
            .ok_or(ServerError::EmailChangeCodeRefused)
    })
}

pub(super) fn held(state: &AppState, person: &str) -> Result<String, ServerError> {
    with_directory(state, |_directory| {
        crate::organisations_found::foundings(state)?
            .email_for(person)
            .map(str::to_owned)
            .ok_or_else(|| {
                super::refused(
                    "Lys has no held address for you. Ask an administrator to change your email.",
                )
            })
    })
}

/// Reuse DIRECTORY-121's bound-account check, and also protect a retained
/// held address after an issuer's account has historically diverged from it.
pub(super) async fn available(
    state: &std::sync::Arc<AppState>,
    api: &SignInProviders,
    person: &str,
    email: &str,
) -> Result<(), ServerError> {
    if let Some((other, _subject, display_name)) =
        crate::people_admit::admitted_already(state, api, state.oidc.issuer(), email).await?
        && other != person
    {
        return Err(ServerError::EmailHeldByAnother {
            person: other,
            display_name,
            email: email.to_owned(),
        });
    }
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let foundings = crate::organisations_found::foundings(state)?;
        for (id, record) in projection.records() {
            if let IdentityId::Person(other) = id {
                let other = other.to_string();
                if other != person
                    && foundings.email_for(&other).is_some_and(|held| {
                        held.trim().to_lowercase() == email.trim().to_lowercase()
                    })
                {
                    return Err(ServerError::EmailHeldByAnother {
                        person: other,
                        display_name: record.profile().display_name().to_owned(),
                        email: email.to_owned(),
                    });
                }
            }
        }
        Ok(())
    })
}

/// The caller holds the issuer account lock across its update and the held
/// address's durable write. Directory always precedes foundings. A failed
/// durable write refuses the request; provider admission still fails closed.
pub(super) async fn write_email(
    state: &AppState,
    api: &SignInProviders,
    id: &str,
    person: &str,
    email: &str,
) -> Result<(), ServerError> {
    let binding = LoginBinding::new(state.oidc.issuer(), id)?;
    with_directory(state, |directory| {
        if directory
            .projection()?
            .person_for(&binding)
            .map(|owner| owner.to_string())
            .as_deref()
            != Some(person)
        {
            return Err(ServerError::EmailChangeCodeRefused);
        }
        Ok(())
    })?;
    let mut update = super::update_of(&super::read(api, id).await?)?;
    update["email"] = Value::String(email.to_owned());
    update["email_verified"] = Value::Bool(true);
    api.call(reqwest::Method::PUT, &format!("/users/{id}"), Some(&update))
        .await
        .map_err(super::account_refusal)?;
    with_directory(state, |_directory| {
        crate::organisations_found::foundings(state)?.hold_email(person, email, false)
    })
}
