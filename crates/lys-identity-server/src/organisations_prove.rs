//! The proving link activates everything and signs the founder in
//! (DIRECTORY-111 R2): POST /organisations/found/prove, unauthenticated,
//! with the proof alone. An unknown or spent digest is refused
//! `proof_refused` with one text, so a link's standing is never read off
//! the refusal. Otherwise eight steps run in order under operations
//! derived from the founding operation: account (the issuer's account by
//! email, found before one is made; `already_admitted` when its subject is
//! bound to an active person who is not the founder), bind, activate,
//! organisation, grant (the administrator grant on the organisation, as
//! first-run setup issues it), letter (the issuer's own new-account letter
//! recorded as this act's added letter when the account was made here;
//! Lys sends none), session (begun for the founder's new binding with
//! provenance `AddressProof`) and spend, the digest marked spent, durably,
//! last. A failed step is answered with its refusal and a receipt naming
//! the steps completed; the digest stays unspent so the same proof asked
//! again finishes what is left. The found and bind leaves are the founding
//! actor's; from the activate on they are the founder's own.
//!
//! On an install with no mail settings (DIRECTORY-112 R2, R3) the letter
//! step records nothing, and a link that was handed begins its session as
//! `HandedLink`, never `AddressProof`. A handed admission link runs two
//! steps only, session and spend: the admitted person must be Active and
//! bound at this install's issuer, else the proof is refused as any
//! unknown proof is and nothing is recorded.

use axum::Json;
use axum::extract::{Path, State};
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, OrganisationId,
    PersonId, Provenance, Transition,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::admission::AUTHORITY;
use crate::error::ServerError;
use crate::foundings_store::{Founding, Kind, digest};
use crate::mail_letters::Letter;
use crate::organisations_found::{FoundingReceipt, founding_actor, foundings, step_operation};
use crate::people_admit::refused;
use crate::routes::{AppState, Shared, with_directory};
use crate::sessions_api::SessionLogin;

/// A handed admission link's steps, in order (DIRECTORY-112 R3).
pub const ADMISSION_STEPS: [&str; 2] = ["session", "spend"];

/// The proving act's steps, in order.
pub const STEPS: [&str; 8] = [
    "account",
    "bind",
    "activate",
    "organisation",
    "grant",
    "letter",
    "session",
    "spend",
];

/// What the link's opener posts.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = OrganisationsProveBody)]
pub struct ProveBody {
    /// The token from the letter's link.
    proof: String,
}

/// What POST /sign-in answers, with the organisation and person added.
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = OrganisationProved)]
pub struct Proved {
    /// The login the session was begun for.
    pub signed_in: SessionLogin,
    /// The authority this service speaks for.
    pub authority: String,
    /// The organisation, now Active.
    pub organisation: String,
    /// The founder, now Active and signed in.
    pub person: String,
    /// Where an admission asked by a product goes on to, the origin Lys
    /// recorded for it (DIRECTORY-113 R2); absent for every other link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_to: Option<String>,
}

/// The founding the proof names, its digest and the ids as the directory
/// reads them.
struct Proven {
    digest: String,
    founding: Founding,
    operation: OperationId,
    organisation: OrganisationId,
    founder: PersonId,
}

fn malformed(reason: String) -> ServerError {
    ServerError::RequestMalformed { reason }
}

/// The proof checked before any step: the body is whole, the digest names
/// an unspent founding or admission.
fn checked(
    state: &AppState,
    asked: Result<Json<ProveBody>, JsonRejection>,
) -> Result<Proven, ServerError> {
    let Json(asked) = asked.map_err(|rejected| malformed(rejected.body_text()))?;
    let digest = digest(&asked.proof);
    let founding = foundings(state)?
        .by_digest(&digest)
        .filter(|founding| !founding.spent && !founding.withdrawn)
        .cloned()
        .ok_or(ServerError::ProofRefused)?;
    let operation = OperationId::from_str(&founding.operation)?;
    let organisation = OrganisationId::from_str(&founding.organisation)?;
    let founder = PersonId::from_str(&founding.founder)?;
    Ok(Proven {
        digest,
        founding,
        operation,
        organisation,
        founder,
    })
}

/// POST /organisations/found/prove: the eight steps, in order, as one act.
pub(crate) async fn prove(
    State(state): State<Shared>,
    headers: HeaderMap,
    asked: Result<Json<ProveBody>, JsonRejection>,
) -> Response {
    let mut receipt = FoundingReceipt::default();
    let proven = match checked(&state, asked) {
        Ok(proven) => proven,
        Err(error) => return refused(error, &receipt).await,
    };
    if let Err(error) = same_person(&state, &headers, proven.founder) {
        return refused(error, &receipt).await;
    }
    receipt.operation = proven.operation.to_string();
    let answered = match proven.founding.kind {
        Kind::Founding => steps(&state, &proven, &mut receipt).await,
        Kind::Admission => admission_steps(&state, &proven, &mut receipt).await,
    };
    match answered {
        Ok((cookie, answer)) => ([(header::SET_COOKIE, cookie)], Json(answer)).into_response(),
        Err((step, error)) => {
            receipt.failed = Some(step.to_owned());
            refused(error, &receipt).await
        }
    }
}

/// Whether `binding` already names an active person other than `founder`.
fn bound_to_another(
    state: &AppState,
    binding: &LoginBinding,
    founder: PersonId,
) -> Result<bool, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        Ok(projection
            .person_for(binding)
            .filter(|person| *person != founder)
            .and_then(|person| projection.record(IdentityId::Person(person)))
            .is_some_and(|record| record.state() == LifecycleState::Active))
    })
}

/// The founder's display name, as the directory holds it.
fn founder_name(state: &AppState, founder: PersonId) -> Result<String, ServerError> {
    with_directory(state, |directory| {
        Ok(directory
            .projection()?
            .record(IdentityId::Person(founder))
            .ok_or(ServerError::ProofRefused)?
            .profile()
            .display_name()
            .to_owned())
    })
}

/// Activate `identity` under `actor`, once; a repeat under the same
/// operation records nothing, and an identity already Active by another
/// act is left as it is.
fn activate(
    state: &AppState,
    actor: Actor,
    operation: OperationId,
    identity: IdentityId,
    reason: &str,
    now: u64,
) -> Result<(), ServerError> {
    with_directory(state, |directory| {
        let state_now = directory
            .projection()?
            .record(identity)
            .map(lys_identity::projection::Record::state);
        if state_now == Some(LifecycleState::Active) {
            return Ok(());
        }
        directory.transition(
            actor,
            operation,
            identity,
            Transition::Activate,
            reason,
            now,
        )?;
        Ok(())
    })
}

/// Run every step, recording each as it completes; the failing step's name
/// beside its refusal.
async fn steps(
    state: &Shared,
    proven: &Proven,
    receipt: &mut FoundingReceipt,
) -> Result<(String, Proved), (&'static str, ServerError)> {
    let [
        account,
        bind,
        activate_step,
        organisation,
        grant,
        letter,
        session,
        spend,
    ] = STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let now = crate::session::now();
    let operation = &proven.operation;
    let founder = proven.founder;

    let api = crate::sign_in_providers::api(state).map_err(at(account))?;
    let name = founder_name(state, founder).map_err(at(account))?;
    let (subject, made) = crate::accounts::make(
        api,
        &proven.founding.email,
        crate::accounts::AccountName {
            display: &name,
            given: None,
            family: None,
        },
    )
    .await
    .map_err(at(account))?;
    let binding = LoginBinding::new(state.oidc.issuer(), &subject)
        .map_err(ServerError::from)
        .map_err(at(account))?;
    if bound_to_another(state, &binding, founder).map_err(at(account))? {
        return Err((account, ServerError::AlreadyAdmitted));
    }
    receipt.done(account);

    let as_founding = founding_actor(state, operation, now).map_err(at(bind))?;
    let step = step_operation(operation, bind).map_err(at(bind))?;
    with_directory(state, |directory| {
        let bound = directory
            .projection()?
            .record(IdentityId::Person(founder))
            .is_some_and(|record| record.bindings().contains(&binding));
        if !bound {
            directory.bind_login(as_founding, step, founder, binding.clone(), now)?;
        }
        Ok(())
    })
    .map_err(at(bind))?;
    receipt.done(bind);

    let own = Actor::new(
        binding.clone(),
        Provenance::new(signed_in_by(&proven.founding), now),
    );
    let step = step_operation(operation, activate_step).map_err(at(activate_step))?;
    activate(
        state,
        own.clone(),
        step,
        IdentityId::Person(founder),
        "address proven by the founding letter's link",
        now,
    )
    .map_err(at(activate_step))?;
    receipt.done(activate_step);

    let step = step_operation(operation, organisation).map_err(at(organisation))?;
    activate(
        state,
        own.clone(),
        step,
        IdentityId::Organisation(proven.organisation),
        "founded; the founder's address proven",
        now,
    )
    .map_err(at(organisation))?;
    receipt.done(organisation);

    crate::organisation_boot::founder_grant(state, founder).map_err(at(grant))?;
    receipt.done(grant);

    if let (true, Some(mailer)) = (made, state.mailer.as_ref()) {
        let added = Letter::Added {
            name,
            email: proven.founding.email.clone(),
            address: state.oidc.public_origin().to_owned(),
            google: crate::people_admit::google_offered(api)
                .await
                .map_err(at(letter))?,
        };
        mailer
            .note_sent_by_issuer(&operation.to_string(), &added)
            .await
            .map_err(at(letter))?;
    }
    receipt.done(letter);

    let cookie = crate::session_admission::begin(state, own)
        .await
        .map_err(at(session))?;
    receipt.done(session);

    foundings(state)
        .map_err(at(spend))?
        .spend(&proven.digest)
        .map_err(at(spend))?;
    receipt.done(spend);

    Ok((
        cookie,
        Proved {
            signed_in: SessionLogin {
                issuer: binding.issuer().to_owned(),
                subject: binding.subject().to_owned(),
            },
            authority: AUTHORITY.to_owned(),
            organisation: proven.organisation.to_string(),
            person: founder.to_string(),
            continue_to: None,
        },
    ))
}

/// How the proof signs the person in: by a handed link when the link was
/// handed on the screen, by address proof when a letter carried it.
fn signed_in_by(founding: &Founding) -> AuthMethod {
    if founding.handed {
        AuthMethod::HandedLink
    } else {
        AuthMethod::AddressProof
    }
}

/// The admitted person's binding at this install's issuer, when they are
/// Active; anything else is refused as an unknown proof is.
fn admitted_binding(state: &AppState, person: PersonId) -> Result<LoginBinding, ServerError> {
    let issuer = state.oidc.issuer().to_owned();
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let record = projection
            .record(IdentityId::Person(person))
            .filter(|record| record.state() == LifecycleState::Active)
            .ok_or(ServerError::ProofRefused)?;
        record
            .bindings()
            .iter()
            .find(|binding| binding.issuer() == issuer)
            .cloned()
            .ok_or(ServerError::ProofRefused)
    })
}

/// A handed admission link's two steps: the session begun for the admitted
/// person's binding by `HandedLink`, then the digest spent, last.
async fn admission_steps(
    state: &Shared,
    proven: &Proven,
    receipt: &mut FoundingReceipt,
) -> Result<(String, Proved), (&'static str, ServerError)> {
    let [session, spend] = ADMISSION_STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let person = proven.founder;
    let binding = admitted_binding(state, person).map_err(at(session))?;
    let own = Actor::new(
        binding.clone(),
        Provenance::new(AuthMethod::HandedLink, crate::session::now()),
    );
    let cookie = crate::people_admit_handed::begin(state, &proven.digest, own, receipt).await?;
    receipt.done(spend);
    Ok((
        cookie,
        Proved {
            signed_in: SessionLogin {
                issuer: binding.issuer().to_owned(),
                subject: binding.subject().to_owned(),
            },
            authority: AUTHORITY.to_owned(),
            organisation: proven.organisation.to_string(),
            person: person.to_string(),
            continue_to: proven.founding.continue_to.clone(),
        },
    ))
}

/// A link read has no effects; spent and withdrawn proofs have one refusal.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct Whom {
    /// The record this link opens.
    display_name: String,
    /// One letter and the domain, never the full address.
    masked_email: String,
}

pub(crate) async fn whom(
    State(state): State<Shared>,
    Path(proof): Path<String>,
) -> Result<Json<Whom>, ServerError> {
    let proven = checked(&state, Ok(Json(ProveBody { proof })))?;
    let name = founder_name(&state, proven.founder)?;
    Ok(Json(Whom {
        display_name: name,
        masked_email: crate::people_admit_handed::masked_email(&proven.founding.email),
    }))
}

fn same_person(
    state: &AppState,
    headers: &HeaderMap,
    intended: PersonId,
) -> Result<(), ServerError> {
    let actor = match crate::routes::signed_in(state, headers) {
        Ok(actor) => actor,
        Err(ServerError::NotSignedIn) => return Ok(()),
        Err(error) => return Err(error),
    };
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let Some(current) = projection.person_for(actor.binding()) else {
            return Ok(());
        };
        if current == intended {
            return Ok(());
        }
        let name = |person| {
            projection
                .record(IdentityId::Person(person))
                .map(|record| record.profile().display_name().to_owned())
                .ok_or(ServerError::ProofRefused)
        };
        Err(ServerError::SignedInAsAnother {
            signed_in: name(current)?,
            intended: name(intended)?,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spend_is_last_and_the_session_before_it() {
        assert_eq!(STEPS[6..], ["session", "spend"]);
        assert_eq!(STEPS[0], "account");
    }

    #[test]
    fn the_body_takes_the_proof_alone() {
        let whole: Result<ProveBody, _> =
            serde_json::from_value(serde_json::json!({"proof": "abc"}));
        assert!(whole.is_ok());
        let extra: Result<ProveBody, _> =
            serde_json::from_value(serde_json::json!({"proof": "abc", "email": "x@y"}));
        assert!(extra.is_err(), "an unknown member is refused");
    }
}
