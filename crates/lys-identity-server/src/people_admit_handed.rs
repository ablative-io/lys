//! The admission's letter step on an install with no mail settings
//! (DIRECTORY-112 R3): in place of the added letter, a proving token is
//! minted and its digest recorded in the foundings store as an admission,
//! naming the person and the organisation they were admitted into, and the
//! link `<public origin>/#/added/<token>` is handed in the answer for the
//! admitter to give them. The link signs the person in once, by
//! `HandedLink`, through POST /organisations/found/prove. An unspent
//! admission link for the same person is replaced in the same write, so one
//! link at a time signs them in.

use std::str::FromStr;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::{
    Actor, Change, IdentityEvent, IdentityId, OperationId, OrganisationId, PersonId,
};
use serde::Serialize;

use crate::error::ServerError;
use crate::foundings_store::{Founding, Kind, digest, new_token};
use crate::mail::LetterView;
use crate::organisations_found::foundings;
use crate::routes::{AppState, Shared, signed_in, with_directory};

/// Mint and record `person`'s admission link and answer the letter view
/// that hands it; `continue_to` is where an admission asked by a product
/// goes on to (DIRECTORY-113 R2).
///
/// # Errors
/// Names the operating system's refusal to give random bytes, or a
/// foundings write that did not reach the disk.
pub(crate) fn hand(
    state: &AppState,
    person: PersonId,
    organisation: OrganisationId,
    email: &str,
    operation: &str,
    continue_to: Option<String>,
) -> Result<LetterView, ServerError> {
    let display_name = person_name(state, person)?;
    let token = new_token()?;
    let admission = Founding {
        organisation: organisation.to_string(),
        founder: person.to_string(),
        email: email.to_owned(),
        operation: operation.to_owned(),
        spent: false,
        withdrawn: false,
        withdrawal_event: None,
        handed: true,
        kind: Kind::Admission,
        continue_to,
    };
    let mut held = foundings(state)?;
    let replaced = held
        .unspent_admission_for(&admission.founder)
        .map(str::to_owned);
    held.record(digest(&token), admission, replaced.as_deref())?;
    drop(held);
    let link = format!("{}/#/added/{token}", state.oidc.public_origin());
    let mut letter = LetterView::handed("added", email.to_owned(), link);
    letter.display_name = Some(display_name);
    Ok(letter)
}

/// The display name of the record an admission opens, read from the directory.
pub(crate) fn person_name(state: &AppState, person: PersonId) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        Ok(directory
            .projection()?
            .record(IdentityId::Person(person))
            .ok_or_else(|| lys_identity::IdentityError::IdentityUnknown {
                identity: person.to_string(),
            })?
            .profile()
            .display_name()
            .to_owned())
    })
}

/// An administrator's bounded read of a person's waiting link.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AdmissionView {
    /// The person whose link is waiting, if any.
    person: String,
    /// Whether an admission remains unspent and unwithdrawn.
    waiting: bool,
}

/// The withdrawn record, named by digest without disclosing the token.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct Withdrawn {
    /// The person whose link was withdrawn.
    person: String,
    /// The retained admission's token digest.
    digest: String,
}

/// Require an active person holding organisation.administer in the target's
/// organisation. Authority over a different organisation admits nothing.
fn administrator(state: &AppState, actor: &Actor, person: PersonId) -> Result<(), ServerError> {
    use lys_identity::grants::{Action, ExerciseRequest, GrantError, Resource, Route};
    crate::grants::with_grants(state, |mut judged| {
        let (caller, organisation) =
            crate::caller_admission::caller_within(judged.directory, actor)?;
        let refused = || ServerError::NotAdmitted {
            reason: "only an administrator of the person's organisation may manage their admission link",
        };
        if !matches!(caller, IdentityId::Person(_))
            || judged.directory.organisation_of(IdentityId::Person(person)) != Some(organisation)
        {
            return Err(refused());
        }
        let request = ExerciseRequest {
            caller,
            route: Route::Api,
            resource: Resource::new("organisation", &organisation.to_string())?,
            action: Action::new("organisation.administer")?,
        };
        match crate::grants::decide(
            &mut judged,
            &request,
            crate::session::now(),
            None,
            crate::grants::Decision::Explain,
        ) {
            Ok(_) => Ok(()),
            Err(
                GrantError::NotHeld { .. }
                | GrantError::Revoked { .. }
                | GrantError::Expired { .. },
            ) => Err(refused()),
            Err(error) => Err(error.into()),
        }
    })
}

/// Finish the directory append from the withdrawal's durable copy. The
/// directory is always held before the foundings; neither crosses an await.
pub(crate) fn record_withdrawals(
    directory: &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
    held: &crate::foundings_store::Foundings,
    person: &str,
) -> Result<(), ServerError> {
    let events = held
        .withdrawals_for(person)
        .map(lys_identity::encoding::decode_body)
        .collect::<Result<Vec<_>, _>>()?;
    directory.commit_batch(&events)?;
    Ok(())
}

pub(crate) async fn waiting(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AdmissionView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let person = PersonId::from_str(&id)?;
    administrator(&state, &actor, person)?;
    with_directory(&state, |directory| {
        let held = foundings(&state)?;
        record_withdrawals(directory, &held, &id)?;
        Ok(Json(AdmissionView {
            waiting: held.unspent_admission_for(&id).is_some(),
            person: id,
        }))
    })
}

pub(crate) async fn withdraw(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Withdrawn>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let person = PersonId::from_str(&id)?;
    administrator(&state, &actor, person)?;
    with_directory(&state, |directory| {
        let mut held = foundings(&state)?;
        record_withdrawals(directory, &held, &id)?;
        let digest = held
            .unspent_admission_for(&id)
            .ok_or(ServerError::NoUnspentAdmission)?
            .to_owned();
        let event = IdentityEvent::new(
            OperationId::generate()?,
            actor,
            IdentityId::Person(person),
            crate::session::now(),
            Change::AdmissionWithdrawn {
                digest: digest.clone(),
            },
        )?;
        held.withdraw(&digest, lys_identity::encoding::encode_body(&event))?;
        record_withdrawals(directory, &held, &id)?;
        Ok(Json(Withdrawn { person: id, digest }))
    })
}

/// Begin and spend under the directory and foundings holds after checking
/// the issuer. A withdrawal or another proof that wins those holds refuses
/// this proof before any session is made.
pub(crate) async fn begin(
    state: &AppState,
    digest: &str,
    actor: Actor,
    receipt: &mut crate::organisations_found::FoundingReceipt,
) -> Result<String, (&'static str, ServerError)> {
    let [session, spend] = crate::organisations_prove::ADMISSION_STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let binding = actor.binding();
    let guard = if let Some(api) = state.sign_in_providers.as_ref() {
        let guard = api
            .lock_account(binding.subject())
            .await
            .map_err(at(session))?;
        let account = crate::accounts::read(api, binding.subject())
            .await
            .map_err(at(session))?;
        match account.get("enabled").and_then(serde_json::Value::as_bool) {
            Some(true) => {}
            Some(false) => return Err((session, ServerError::SignInRefused)),
            None => {
                return Err((
                    session,
                    ServerError::SignInProvidersUnavailable {
                        reason: "the sign-in service's account answer has no boolean enabled field"
                            .to_owned(),
                    },
                ));
            }
        }
        Some(guard)
    } else {
        None
    };
    let mut failed = session;
    let answer = with_directory(state, |directory| {
        let caller = crate::caller_admission::active_caller(directory.projection()?, &actor)?;
        let mut held = foundings(state)?;
        held.by_digest(digest)
            .filter(|entry| {
                !entry.spent
                    && !entry.withdrawn
                    && entry.kind == Kind::Admission
                    && entry.founder == caller.to_string()
            })
            .ok_or(ServerError::ProofRefused)?;
        let cookie = state.sessions.begin(actor)?;
        receipt.done(session);
        failed = spend;
        if let Err(error) = held.spend(digest) {
            let session = state.sessions.current(Some(&cookie))?;
            state.sessions.end(&session, |_| true)?;
            return Err(error);
        }
        Ok(cookie)
    });
    drop(guard);
    answer.map_err(at(failed))
}
