//! Administrator-only email enrichment, after directory paging. No issuer
//! network call crosses a directory or foundings hold.
use std::str::FromStr;

use axum::Json;
use lys_identity::{OperationId, PersonId};

use crate::error::ServerError;
use crate::read_views::{PeopleView, PersonEmails};
use crate::routes::{AppState, with_directory};

pub(super) async fn attach(
    state: &AppState,
    mut answer: Json<PeopleView>,
) -> Result<Json<PeopleView>, ServerError> {
    let mailed = match state.mailer.as_ref() {
        Some(mailer) => mailer.added_addresses().await?,
        None => Vec::new(),
    };
    let (mut answer, subjects) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let foundings = crate::organisations_found::foundings(state)?;
        let mut subjects = Vec::with_capacity(answer.0.people.len());
        for person in &mut answer.0.people {
            let id = PersonId::from_str(&person.id)?;
            let record = super::person_record(projection, id)?;
            subjects.push(
                record
                    .bindings()
                    .iter()
                    .find(|binding| binding.issuer() == state.oidc.issuer())
                    .map(|binding| binding.subject().to_owned()),
            );
            let first = record.events().first().copied();
            let admitted_email = mailed
                .iter()
                .find_map(|(operation, email)| {
                    let operation = OperationId::from_str(operation).ok()?;
                    let register = OperationId::from_str(&crate::people_admit::derived(
                        &operation, "register",
                    ))
                    .ok()?;
                    (first.is_some() && projection.operation(register) == first)
                        .then(|| email.clone())
                })
                .or_else(|| foundings.original_email_for(&person.id).map(str::to_owned));
            person.emails = Some(PersonEmails {
                sign_in_email: None,
                admitted_email,
                email_recorded_at_sign_in: foundings.recorded_at_sign_in(&person.id),
            });
        }
        Ok((answer, subjects))
    })?;
    if let Some(api) = state.sign_in_providers.as_ref() {
        for (person, subject) in answer.0.people.iter_mut().zip(subjects) {
            if let (Some(emails), Some(subject)) = (&mut person.emails, subject) {
                emails.sign_in_email = match crate::accounts::email_of(api, &subject).await {
                    Ok(email) => Some(email),
                    Err(ServerError::SignInProvidersRefused { status: 404, .. }) => None,
                    Err(error) => return Err(error),
                };
            }
        }
    }
    Ok(answer)
}
