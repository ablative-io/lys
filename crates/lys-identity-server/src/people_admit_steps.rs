//! The admission steps, including explicit reuse and the letter.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lys_identity::{OperationId, PersonId};
use serde_json::{Value, json};

use super::{Act, AdmitReceipt, Done, Existing, STEPS, body, derived, google_offered};
use crate::error::ServerError;
use crate::mail::Sender;
use crate::mail_letters::Letter;
use crate::routes::Shared;

/// Run every step, recording each as it completes; the failing step's name
/// beside its refusal.
pub(crate) async fn steps(
    state: &Shared,
    headers: &HeaderMap,
    act: &Act<'_>,
    receipt: &mut AdmitReceipt,
) -> Result<Done, (&'static str, ServerError)> {
    let Act {
        asked,
        operation,
        email,
        issuer,
        organisation,
        ..
    } = *act;
    let [register, account, bind, activate, _, _] = STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let api = crate::sign_in_providers::api(state).map_err(at(account))?;
    // An email already admitted, an issuer account whose login is bound to
    // a person, is not registered again (DIRECTORY-116 R3): the person
    // stands, the four steps that made them are already done, and the act
    // goes on to the grant asked for, which is added to what they hold.
    // The same act asked again replays its own leaves, so only an operation
    // the directory has not answered takes this way.
    let replay = crate::routes::with_directory(state, |directory| {
        let first = OperationId::from_str(&derived(operation, register))?;
        Ok(directory.projection()?.operation(first).is_some())
    })
    .map_err(at(register))?;
    if let Some((person, subject, display_name)) = if replay {
        None
    } else {
        admitted_already(state, api, issuer, email)
            .await
            .map_err(at(register))?
    } {
        if asked.display_name.trim() != display_name.trim()
            && asked.person.as_deref() != Some(person.as_str())
        {
            return Err((
                register,
                ServerError::EmailHeldByAnother {
                    person,
                    display_name,
                    email: email.to_owned(),
                },
            ));
        }
        receipt.existing = Some(Existing {
            person: person.clone(),
            display_name,
        });
        receipt.person = Some(person.clone());
        for step in [register, account, bind, activate] {
            receipt.done(step, None);
        }
        return granted_and_written(state, headers, act, receipt, person, subject, false).await;
    }
    let registered = crate::routes::register_person(
        State(Arc::clone(state)),
        headers.clone(),
        body(json!({
            "operation": derived(operation, register),
            "display_name": asked.display_name,
        }))
        .map_err(at(register))?,
    )
    .await
    .map_err(at(register))?;
    let person = registered.0.person.clone();
    receipt.person = Some(person.clone());
    receipt.actor = Some(registered.0.receipt.actor.clone());
    receipt.done(
        register,
        Some(("directory", registered.0.receipt.log.index)),
    );

    let name = crate::accounts::AccountName {
        display: &asked.display_name,
        given: asked.given_name.as_deref(),
        family: asked.family_name.as_deref(),
    };
    let (subject, made) = crate::accounts::make(api, email, name)
        .await
        .map_err(at(account))?;
    receipt.done(account, None);

    let within = Within {
        state,
        headers,
        organisation,
        person: &person,
    };
    let bound = within
        .change(&derived(operation, bind), Some((issuer, &subject)))
        .map_err(at(bind))?;
    receipt.done(bind, Some(("directory", bound)));

    let activated = within
        .change(&derived(operation, activate), None)
        .map_err(at(activate))?;
    receipt.done(activate, Some(("directory", activated)));
    granted_and_written(state, headers, act, receipt, person, subject, made).await
}

/// The bind and activate steps of one admission. The caller was admitted
/// to the act by organisation.admit on their own organisation, which the
/// register step exercised, and these steps change only the person that
/// step registered there, still Registered; so they are not judged again
/// as the install's administrator, and an organisation's own administrator
/// admits their people. A retried act replays its own leaves.
struct Within<'a> {
    state: &'a Shared,
    headers: &'a HeaderMap,
    organisation: lys_identity::OrganisationId,
    person: &'a str,
}

impl Within<'_> {
    /// Bind the login at the issuer and subject `login` names, or activate
    /// when it names none, answering the directory index of the leaf.
    fn change(&self, operation: &str, login: Option<(&str, &str)>) -> Result<u64, ServerError> {
        let actor = crate::routes::signed_in(self.state, self.headers)?;
        let operation = OperationId::from_str(operation)?;
        let person = PersonId::from_str(self.person)?;
        let binding = login
            .map(|(issuer, subject)| lys_identity::LoginBinding::new(issuer, subject))
            .transpose()?;
        if let Some(binding) = &binding
            && self.state.admission.administrator_login()?.as_ref() == Some(binding)
        {
            return Err(ServerError::NotAdmitted {
                reason: "an admission never binds the administrator's own sign-in",
            });
        }
        crate::routes::with_directory(self.state, |directory| {
            let projection = directory.projection()?;
            if projection.operation(operation).is_none() {
                let registered = projection
                    .record(lys_identity::IdentityId::Person(person))
                    .is_some_and(|record| {
                        record.state() == lys_identity::LifecycleState::Registered
                    });
                if !registered
                    || projection.organisation_of(lys_identity::IdentityId::Person(person))
                        != Some(self.organisation)
                {
                    return Err(ServerError::NotAdmitted {
                        reason: "an admission binds and activates only the person it registered in the caller's own organisation",
                    });
                }
            }
            let now = crate::session::now();
            let receipt = match binding {
                Some(binding) => directory.bind_login(actor, operation, person, binding, now)?,
                None => directory.transition(
                    actor,
                    operation,
                    lys_identity::IdentityId::Person(person),
                    lys_identity::Transition::Activate,
                    "admitted in one act",
                    now,
                )?,
            };
            Ok(crate::directory_views::receipt_view(&receipt).log.index)
        })
    }
}

/// The person `email` is already admitted as, with their issuer subject:
/// an issuer account under the email whose login at `issuer` is bound to a
/// person. An account with no bound person is not an admission.
pub(crate) async fn admitted_already(
    state: &Shared,
    api: &crate::sign_in_providers::SignInProviders,
    issuer: &str,
    email: &str,
) -> Result<Option<(String, String, String)>, ServerError> {
    let Some(account) = crate::accounts::find_by_email(api, email).await? else {
        return Ok(None);
    };
    let Some(subject) = account.get("id").and_then(Value::as_str) else {
        return Ok(None);
    };
    let binding = lys_identity::LoginBinding::new(issuer, subject)?;
    crate::routes::with_directory(state, |directory| {
        let projection = directory.projection()?;
        Ok(projection.person_for(&binding).and_then(|person| {
            projection
                .record(lys_identity::IdentityId::Person(person))
                .map(|record| {
                    (
                        person.to_string(),
                        subject.to_owned(),
                        record.profile().display_name().to_owned(),
                    )
                })
        }))
    })
}

/// The grant and the letter, the act's last two steps, for `person` bound
/// to the issuer `subject`; `made` says the account was made in this act.
async fn granted_and_written(
    state: &Shared,
    headers: &HeaderMap,
    act: &Act<'_>,
    receipt: &mut AdmitReceipt,
    person: String,
    subject: String,
    made: bool,
) -> Result<Done, (&'static str, ServerError)> {
    let Act {
        asked,
        operation,
        email,
        organisation,
        ref continue_to,
        ..
    } = *act;
    let [_, _, _, _, grant, letter] = STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let api = crate::sign_in_providers::api(state).map_err(at(letter))?;
    let person_id = PersonId::from_str(&person).map_err(|error| (letter, error.into()))?;
    let display_name =
        crate::people_admit_handed::person_name(state, person_id).map_err(at(letter))?;

    let mut granted = None;
    if let Some(first) = asked.grant.as_ref() {
        let Json(root) = body(json!({
            "operation": derived(operation, grant),
            "route": first.route,
            "holder": person,
            "resource": first.resource,
            "relation": first.relation,
            "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": null},
        }))
        .map_err(at(grant))?;
        // The grant is recorded here and its own letter is not sent: the
        // added letter below is the one letter this act writes.
        let (recorded, _recipient) =
            crate::grants::issue_root_recorded(state, headers, &root).map_err(at(grant))?;
        receipt.done(grant, Some(("grants", recorded.index)));
        granted = Some(recorded.grant);
    }

    let Some(mailer) = state.mailer.as_ref() else {
        let wrote = crate::people_admit_handed::hand(
            state,
            person_id,
            organisation,
            email,
            &operation.to_string(),
            continue_to.clone(),
        )
        .map_err(at(letter))?;
        receipt.done(letter, None);
        return Ok(Done {
            person,
            subject,
            display_name,
            grant: granted,
            letter: wrote,
        });
    };
    crate::routes::with_directory(state, |_directory| {
        let mut held = crate::organisations_found::foundings(state)?;
        if held.email_for(&person).is_none() {
            held.hold_email(&person, email, false)?;
        }
        Ok(())
    })
    .map_err(at(letter))?;
    let added = Letter::Added {
        name: display_name.clone(),
        email: email.to_owned(),
        address: state.oidc.public_origin().to_owned(),
        google: google_offered(api).await.map_err(at(letter))?,
    };
    let wrote = if made {
        mailer
            .note_sent_by_issuer(&operation.to_string(), &added)
            .await
    } else {
        mailer.send_once(&operation.to_string(), &added).await
    }
    .map_err(at(letter))?;
    debug_assert!(wrote.by == Sender::Issuer || !made);
    receipt.done(letter, None);
    Ok(Done {
        person,
        subject,
        display_name,
        grant: granted,
        letter: wrote,
    })
}
