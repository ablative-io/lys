//! A stranger founds an organisation (DIRECTORY-111 R1): POST
//! /organisations/found, unauthenticated, with the organisation's name, the
//! founder's name and their email. Three steps under operations derived from
//! the act's one: found, the organisation and its founder recorded as
//! Registered under a founding actor (the install's issuer, subject
//! `founding:<operation>`, provenance Founding: no credential, the address
//! unproven at this leaf); proof, a token drawn from the operating system
//! whose digest alone is stored with the founding; letter, Lys's founded
//! letter carrying the link `<public origin>/#/found/<token>`.
//!
//! One answer shape to everyone, the operation and the letter and no
//! identity id, so the route is never an oracle for addresses: the
//! difference is in the letter. An address that already signs in as an
//! active person gets the already-yours letter and nothing is founded. An
//! address with an unproven founding waiting gets a fresh link for that
//! founding, the old digest replaced in the same write, whatever name was
//! typed this time: one unproven founding per address. The same operation
//! asked again after its letter went answers the same and mints nothing, so
//! the link in the letter stays the one that proves. The proof is
//! `organisations_prove.rs`.
//!
//! An install with no mail settings (DIRECTORY-112 R1) runs the same steps
//! and hands the link in the answer's letter view in place of the letter,
//! the founding recorded as handed; an address that already signs in is
//! told so on the screen, since no letter can carry the difference. With no
//! letters-sent record to read, the same operation asked again hands a
//! fresh link for the same founding.

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, Profile, Provenance,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use std::sync::MutexGuard;

use crate::error::ServerError;
use crate::foundings_store::{Founding, Foundings, Kind, digest, new_token};
use crate::mail::LetterView;
use crate::mail_letters::Letter;
use crate::people_admit::{derived, refused};
use crate::routes::{AppState, Shared, with_directory};

/// The founding act's steps, in order.
pub const STEPS: [&str; 3] = ["found", "proof", "letter"];

/// The two founding routes, mounted beside /people/admit.
pub fn routes() -> axum::Router<Shared> {
    axum::Router::new()
        .route("/organisations/found", post(found))
        .route(
            "/organisations/found/prove",
            post(crate::organisations_prove::prove),
        )
}

/// What the stranger asks for.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = OrganisationsFoundBody)]
pub struct FoundBody {
    /// The act's operation; each step's is derived from it.
    operation: String,
    /// The organisation's name.
    organisation: String,
    /// The founder's name.
    display_name: String,
    /// The founder's email, where the proving letter goes.
    email: String,
}

/// What a founding or a proof did: the steps completed, in order, and the
/// one that failed. No identity is named.
#[derive(Clone, Debug, Default, Serialize, utoipa::ToSchema)]
#[schema(as = FoundingReceipt)]
pub struct FoundingReceipt {
    /// The act's operation.
    pub operation: String,
    /// The steps completed, in order.
    pub completed: Vec<String>,
    /// The step that failed, if one did.
    pub failed: Option<String>,
}

impl FoundingReceipt {
    /// Record `step` as completed.
    pub(crate) fn done(&mut self, step: &str) {
        self.completed.push(step.to_owned());
    }
}

/// The founding's answer: the same to everyone.
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = OrganisationFounded)]
pub struct Founded {
    /// The act's operation.
    pub operation: String,
    /// The letter that went, and where.
    pub letter: LetterView,
}

fn malformed(reason: String) -> ServerError {
    ServerError::RequestMalformed { reason }
}

/// The operation of `step` within the act `operation`.
///
/// # Errors
/// Never in practice: a derived operation is sixteen bytes rendered the way
/// the parser reads them.
pub(crate) fn step_operation(
    operation: &OperationId,
    step: &str,
) -> Result<OperationId, ServerError> {
    Ok(OperationId::from_str(&derived(operation, step))?)
}

/// The actor the founding leaves are recorded under: the install's issuer
/// with the subject `founding:<operation>`, authenticated by nothing.
///
/// # Errors
/// Refuses a binding the directory would not take.
pub(crate) fn founding_actor(
    state: &AppState,
    operation: &OperationId,
    now: u64,
) -> Result<Actor, ServerError> {
    let binding = LoginBinding::new(state.oidc.issuer(), &format!("founding:{operation}"))?;
    Ok(Actor::new(
        binding,
        Provenance::new(AuthMethod::Founding, now),
    ))
}

/// The foundings, one caller at a time; never held across an await.
///
/// # Errors
/// Names a poisoned lock.
pub(crate) fn foundings(state: &AppState) -> Result<MutexGuard<'_, Foundings>, ServerError> {
    state
        .foundings
        .lock()
        .map_err(|error| ServerError::DirectoryUnavailable {
            reason: format!("the foundings lock is poisoned: {error}"),
        })
}

/// The act's checked words.
struct Asked {
    operation: OperationId,
    organisation: Profile,
    founder: Profile,
    email: String,
}

/// The words checked before any step: the body is whole, the operation
/// and the email are well formed and the names are ones the directory
/// takes.
fn checked(asked: Result<Json<FoundBody>, JsonRejection>) -> Result<Asked, ServerError> {
    let Json(asked) = asked.map_err(|rejected| malformed(rejected.body_text()))?;
    let operation = OperationId::from_str(&asked.operation)?;
    let email = crate::accounts::check_email(&asked.email)
        .map_err(|_refused| malformed(format!("{} is not an email", asked.email)))?
        .to_owned();
    let organisation = Profile::new(&asked.organisation)
        .map_err(|error| malformed(format!("the organisation's name is not one: {error}")))?;
    let founder = Profile::new(&asked.display_name)
        .map_err(|error| malformed(format!("the founder's name is not one: {error}")))?;
    Ok(Asked {
        operation,
        organisation,
        founder,
        email,
    })
}

/// POST /organisations/found: the three steps, in order, as one act.
pub(crate) async fn found(
    State(state): State<Shared>,
    asked: Result<Json<FoundBody>, JsonRejection>,
) -> Response {
    let mut receipt = FoundingReceipt::default();
    let asked = match checked(asked) {
        Ok(asked) => asked,
        Err(error) => return refused(error, &receipt).await,
    };
    receipt.operation = asked.operation.to_string();
    match steps(&state, &asked, &mut receipt).await {
        Ok(letter) => Json(Founded {
            operation: receipt.operation,
            letter,
        })
        .into_response(),
        Err((step, error)) => {
            receipt.failed = Some(step.to_owned());
            refused(error, &receipt).await
        }
    }
}

/// Whether `email` already signs in to Lys as an active person.
async fn already_theirs(state: &AppState, email: &str) -> Result<bool, ServerError> {
    let api = crate::sign_in_providers::api(state)?;
    let Some(held) = crate::accounts::find_by_email(api, email).await? else {
        return Ok(false);
    };
    let subject = held
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in service's account answer names no id".to_owned(),
        })?
        .to_owned();
    let binding = LoginBinding::new(state.oidc.issuer(), &subject)?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        Ok(projection
            .person_for(&binding)
            .and_then(|person| projection.record(IdentityId::Person(person)))
            .is_some_and(|record| record.state() == LifecycleState::Active))
    })
}

/// Run every step, recording each as it completes; the failing step's name
/// beside its refusal.
async fn steps(
    state: &Shared,
    asked: &Asked,
    receipt: &mut FoundingReceipt,
) -> Result<LetterView, (&'static str, ServerError)> {
    let [found, proof, letter] = STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let mailer = state.mailer.as_ref();
    let operation = asked.operation.to_string();
    let address = state.oidc.public_origin().to_owned();

    // The address already signs in: its person is told, nothing is founded.
    if already_theirs(state, &asked.email)
        .await
        .map_err(at(found))?
    {
        let yours = Letter::AlreadyYours {
            email: asked.email.clone(),
            organisation: asked.organisation.display_name().to_owned(),
            address,
        };
        let wrote = match mailer {
            Some(mailer) => mailer
                .send_once(&operation, &yours)
                .await
                .map_err(at(letter))?,
            None => LetterView::unsent(
                yours.name(),
                Some(asked.email.clone()),
                "This address already signs in to Lys",
            ),
        };
        receipt.done(letter);
        return Ok(wrote);
    }

    // Asked again after its letter went: the same answer and nothing
    // minted, so the link already in the letter stays the one that proves.
    let recorded = match mailer {
        Some(mailer) => mailer
            .recorded_by(&operation, "founded")
            .await
            .map_err(at(found))?,
        None => None,
    };
    if let Some(by) = recorded {
        for step in STEPS {
            receipt.done(step);
        }
        return Ok(LetterView {
            letter: "founded".to_owned(),
            to: Some(asked.email.clone()),
            sent: true,
            by,
            reason: None,
            handed: None,
        });
    }

    let now = crate::session::now();
    let waiting = foundings(state)
        .map_err(at(found))?
        .unspent_for_email(&asked.email)
        .map(|(old, waiting)| (old.to_owned(), waiting.clone()));
    let (organisation, founder, founding_operation, replaced) =
        if let Some((old, waiting)) = waiting {
            (
                waiting.organisation,
                waiting.founder,
                waiting.operation,
                Some(old),
            )
        } else {
            let actor = founding_actor(state, &asked.operation, now).map_err(at(found))?;
            let step = step_operation(&asked.operation, found).map_err(at(found))?;
            let founded = with_directory(state, |directory| {
                Ok(directory.found_organisation(
                    actor,
                    step,
                    asked.organisation.clone(),
                    asked.founder.clone(),
                    now,
                )?)
            })
            .map_err(at(found))?;
            (
                founded.organisation.to_string(),
                founded.founder.to_string(),
                operation.clone(),
                None,
            )
        };
    receipt.done(found);

    let token = new_token().map_err(at(proof))?;
    let founding = Founding {
        organisation,
        founder,
        email: asked.email.clone(),
        operation: founding_operation,
        spent: false,
        handed: mailer.is_none(),
        kind: Kind::Founding,
    };
    foundings(state)
        .map_err(at(proof))?
        .record(digest(&token), founding, replaced.as_deref())
        .map_err(at(proof))?;
    receipt.done(proof);

    let link = format!("{address}/#/found/{token}");
    let wrote = match mailer {
        Some(mailer) => {
            let proving = Letter::Founded {
                name: asked.founder.display_name().to_owned(),
                email: asked.email.clone(),
                link,
                organisation: asked.organisation.display_name().to_owned(),
            };
            mailer
                .send_once(&operation, &proving)
                .await
                .map_err(at(letter))?
        }
        None => LetterView::handed("founded", asked.email.clone(), link),
    };
    receipt.done(letter);
    Ok(wrote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_has_its_own_operation_and_the_actor_names_the_act() {
        let operation = OperationId::from_bytes([3; 16]);
        let ops: Vec<String> = STEPS.iter().map(|step| derived(&operation, step)).collect();
        let distinct: std::collections::BTreeSet<&String> = ops.iter().collect();
        assert_eq!(distinct.len(), STEPS.len());
        assert!(ops.iter().all(|op| OperationId::from_str(op).is_ok()));
        assert_eq!(
            step_operation(&operation, "found")
                .ok()
                .map(|op| op.to_string()),
            Some(ops[0].clone())
        );
    }

    #[test]
    fn the_body_takes_the_four_members_and_nothing_else() {
        let whole: Result<FoundBody, _> = serde_json::from_value(serde_json::json!({
            "operation": "op-00000000000000000000000000000000",
            "organisation": "Acme",
            "display_name": "Ada Lovelace",
            "email": "ada@example.test",
        }));
        assert!(whole.is_ok());
        let extra: Result<FoundBody, _> = serde_json::from_value(serde_json::json!({
            "operation": "op-00000000000000000000000000000000",
            "organisation": "Acme",
            "display_name": "Ada Lovelace",
            "email": "ada@example.test",
            "person": "person-1",
        }));
        assert!(extra.is_err(), "an unknown member is refused");
    }
}
