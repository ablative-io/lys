//! Admitting a person in one act: register them, make their sign-in account
//! at the issuer by email, bind that login, activate them, issue their first
//! named root grant when one is asked for, and write to them, in that order.
//! A fresh install has nothing to grant yet, so the grant is optional and a
//! person admitted without one holds no access (PERMISSIONS-STORY 9 Oct
//! 2026: a root grant to a person not yet active is refused
//! `IdentityNotActive`).
//!
//! Each step runs as the existing route does, under an operation derived from
//! the act's one operation and the step's name, so asking again replays the
//! steps already done and finishes what is left; nothing is made twice. The
//! issuer account is found by email before one is made. A failed step is
//! answered with its own refusal and a receipt naming it and the steps that
//! completed before it. The receipt names the log leaf each written step was
//! recorded at, the directory's three and the grant's, so it is the logs'
//! own record: asking again with the same operation answers the same leaves.
//!
//! The letter (DIRECTORY-110): a person admitted now receives one added
//! letter, never two. When the account step made their account, the sign-in
//! service's own letter on a new account is the added letter, and Lys records
//! it as sent by the issuer; when the account was already held, Lys writes
//! the added letter itself. An install with no mail settings sends no
//! letter: the letter step hands a sign-in link in the answer instead, for
//! the admitter to give the person (DIRECTORY-112 R3, `people_admit_handed`).
//! A letter
//! that does not go leaves every step before it recorded and is answered as
//! `mail_not_sent`; asking again with the same operation sends it.
//! Nothing here calls a product or holds a lock across the issuer's answer.

use axum::Json;
use axum::body::to_bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use lys_identity::{OperationId, OrganisationId, PersonId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::sync::Arc;

use crate::error::ServerError;
use crate::mail::{LetterView, Sender};
use crate::mail_letters::Letter;
use crate::routes::{Shared, signed_in};
use crate::sign_in_providers::{Provider, SignInProviders};

/// The steps of the act, in the order they run.
pub const STEPS: [&str; 6] = ["register", "account", "bind", "activate", "grant", "letter"];

/// What the admitting caller asks for.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleAdmitBody)]
pub(crate) struct AdmitBody {
    /// The act's operation; each step's is derived from it.
    operation: String,
    /// The person's display name.
    display_name: String,
    /// Their first name, kept at the issuer beside `family_name`.
    #[serde(default)]
    given_name: Option<String>,
    /// Their last name, kept at the issuer beside `given_name`.
    #[serde(default)]
    family_name: Option<String>,
    /// The email their issuer account is found or made by.
    email: String,
    /// The organisation the person is admitted into, when the asker names
    /// it: the caller's own and no other (ACCESS-009 R1).
    #[serde(default)]
    organisation: Option<String>,
    /// Their first root grant, if any. An install with nothing to grant yet
    /// admits a person with no access; the grant step is then not run.
    #[serde(default)]
    #[schema(inline)]
    grant: Option<FirstGrant>,
}

impl AdmitBody {
    /// The kind of the first grant asked for, if one is.
    pub(crate) fn first_grant_kind(&self) -> Option<&str> {
        self.grant
            .as_ref()
            .map(|grant| grant.resource.kind.as_str())
    }
}

/// The first root grant: use only, from now, with no end.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleAdmitGrant)]
pub(crate) struct FirstGrant {
    /// The route the grant is exercised on.
    route: String,
    /// The object it is on.
    #[schema(inline)]
    resource: GrantResource,
    /// The relation it is issued as.
    relation: String,
}

/// A resource by kind and id.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleAdmitResource)]
pub(crate) struct GrantResource {
    kind: String,
    id: String,
}

/// What the act did: the steps completed, in order, and the one that failed.
#[derive(Clone, Debug, Default, Serialize, utoipa::ToSchema)]
pub struct AdmitReceipt {
    /// The act's operation.
    pub operation: String,
    /// The registered person, once the register step has answered.
    pub person: Option<String>,
    /// The steps completed, in order.
    pub completed: Vec<String>,
    /// The step that failed, if one did.
    pub failed: Option<String>,
    /// The log leaf each completed step was recorded at, in order. The
    /// issuer account is written at the issuer, and the letter is sent, so
    /// neither has one.
    pub logged: Vec<Logged>,
    /// Whose act it was, as the register leaf was signed: the person, and
    /// the app it came through when a product asked (DIRECTORY-113
    /// Amendment 2). Absent until that leaf is written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<crate::directory_views::ReceiptActorView>,
}

/// Where one step was recorded.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[schema(as = PeopleAdmitLogged)]
pub struct Logged {
    /// The step.
    pub step: String,
    /// The log: `directory` or `grants`.
    pub log: String,
    /// The leaf's index in that log.
    pub index: u64,
}

impl AdmitReceipt {
    /// Record `step` as completed, at `index` of `log` if it wrote one.
    fn done(&mut self, step: &str, leaf: Option<(&str, u64)>) {
        self.completed.push(step.to_owned());
        if let Some((log, index)) = leaf {
            self.logged.push(Logged {
                step: step.to_owned(),
                log: log.to_owned(),
                index,
            });
        }
    }
}

/// The person admitted.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Admitted {
    /// The person.
    pub person: String,
    /// Their subject at the issuer.
    pub subject: String,
    /// Their first grant, when one was asked for.
    pub grant: Option<String>,
    /// The added letter: who wrote to them, and where.
    pub letter: LetterView,
    /// What the act did.
    #[schema(inline)]
    pub receipt: AdmitReceipt,
}

/// The operation of `step` within the act `operation`: the first sixteen
/// bytes of SHA-256 over the act's operation, a zero byte and the step name.
pub(crate) fn derived(operation: &OperationId, step: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(operation.to_string().as_bytes());
    digest.update([0]);
    digest.update(step.as_bytes());
    let mut id = [0_u8; 16];
    id.copy_from_slice(&digest.finalize()[..16]);
    OperationId::from_bytes(id).to_string()
}

fn malformed(reason: String) -> ServerError {
    ServerError::RequestMalformed { reason }
}

/// `value` as the body a route takes, refused by name if it is not one.
fn body<T: serde::de::DeserializeOwned>(value: Value) -> Result<Json<T>, ServerError> {
    serde_json::from_value(value)
        .map(Json)
        .map_err(|error| malformed(format!("the act built a body the route refused: {error}")))
}

/// `error` answered as the route answers it, carrying `receipt` beside it.
/// A founding's receipts ride the same way (DIRECTORY-111).
pub(crate) async fn refused<R: Serialize + Sync>(error: ServerError, receipt: &R) -> Response {
    let (mut parts, carried) = error.into_response().into_parts();
    // The answer is this service's own refusal object, made just above from
    // a ServerError, so it is read whole; no size is invented for it.
    let Ok(bytes) = to_bytes(carried, usize::MAX).await else {
        return parts_only(parts);
    };
    let Ok(Value::Object(mut answer)) = serde_json::from_slice::<Value>(&bytes) else {
        return Response::from_parts(parts, axum::body::Body::from(bytes));
    };
    answer.insert("receipt".to_owned(), json!(receipt));
    parts.headers.remove(axum::http::header::CONTENT_LENGTH);
    Response::from_parts(
        parts,
        axum::body::Body::from(Value::Object(answer).to_string()),
    )
}

fn parts_only(parts: axum::http::response::Parts) -> Response {
    Response::from_parts(parts, axum::body::Body::empty())
}

/// The act's words checked before any step: the body is whole, the
/// operation and the email are well formed, the install can write to the
/// person (DIRECTORY-110 R4), and the caller is an active person holding
/// organisation.admit on their own organisation, the one the body names if
/// it names one (ACCESS-009 R1). The question is asked here and recorded
/// nothing; the register step exercises the grant. The issuer is the one
/// the caller signed in through. The caller is the session's, or the person
/// an approved app's pass speaks for (DIRECTORY-113 R1).
pub(crate) fn checked(
    state: &Shared,
    actor: &lys_identity::Actor,
    asked: Result<Json<AdmitBody>, JsonRejection>,
) -> Result<(AdmitBody, OperationId, String, String, OrganisationId), ServerError> {
    let Json(asked) = asked.map_err(|rejected| malformed(rejected.body_text()))?;
    let operation = OperationId::from_str(&asked.operation)?;
    let email = crate::accounts::check_email(&asked.email)
        .map_err(|_refused| malformed(format!("{} is not an email", asked.email)))?
        .to_owned();
    let organisation = crate::grants::with_grants(state, |mut judged| {
        crate::people_admission::organisation(
            &mut judged,
            actor,
            asked.organisation.as_deref(),
            crate::grants::Decision::Explain,
        )
    })?;
    let issuer = actor.binding().issuer().to_owned();
    Ok((asked, operation, email, issuer, organisation))
}

/// POST /people/admit: the six steps, in order, as one act; the fifth, the
/// first grant, only when one was asked for.
pub(crate) async fn admit(
    State(state): State<Shared>,
    headers: HeaderMap,
    asked: Result<Json<AdmitBody>, JsonRejection>,
) -> Response {
    let mut receipt = AdmitReceipt::default();
    // An approved app's agent pass admits as the agent, narrowed as a
    // person's app pass is (AGENTS-019 R2).
    match crate::provider::app_agent(&state, &headers) {
        Ok(Some((actor, app))) => {
            return crate::people_admit_app::admit(&state, &headers, &actor, &app, asked).await;
        }
        Ok(None) => {}
        Err(error) => return refused(error, &receipt).await,
    }
    let app = match crate::provider::app_person(&state, &headers) {
        Ok(app) => app,
        Err(error) => return refused(error, &receipt).await,
    };
    if let Some((actor, app)) = app {
        return crate::people_admit_app::admit(&state, &headers, &actor, &app, asked).await;
    }
    let checked = signed_in(&state, &headers).and_then(|actor| checked(&state, &actor, asked));
    let (asked, operation, email, issuer, organisation) = match checked {
        Ok(checked) => checked,
        Err(error) => return refused(error, &receipt).await,
    };
    receipt.operation = operation.to_string();
    let act = Act {
        asked: &asked,
        operation: &operation,
        email: &email,
        issuer: &issuer,
        organisation,
        continue_to: None,
    };
    let answered = steps(&state, &headers, &act, &mut receipt).await;
    answer(answered, receipt).await
}

/// The act's answer: the person admitted, or the failed step's refusal
/// beside the receipt.
pub(crate) async fn answer(
    answered: Result<Done, (&'static str, ServerError)>,
    mut receipt: AdmitReceipt,
) -> Response {
    match answered {
        Ok(done) => Json(Admitted {
            person: done.person,
            subject: done.subject,
            grant: done.grant,
            letter: done.letter,
            receipt,
        })
        .into_response(),
        Err((step, error)) => {
            receipt.failed = Some(step.to_owned());
            refused(error, &receipt).await
        }
    }
}

/// The act's checked words. The login is bound at the issuer the caller
/// signed in through, the one whose accounts Lys administers, as first-run
/// setup binds its own.
pub(crate) struct Act<'a> {
    pub(crate) asked: &'a AdmitBody,
    pub(crate) operation: &'a OperationId,
    pub(crate) email: &'a str,
    pub(crate) issuer: &'a str,
    pub(crate) organisation: OrganisationId,
    /// Where a handed link goes on to, when a product asked (DIRECTORY-113 R2).
    pub(crate) continue_to: Option<String>,
}

/// What the steps answered.
pub(crate) struct Done {
    person: String,
    subject: String,
    grant: Option<String>,
    letter: LetterView,
}

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
    if let Some((person, subject)) = if replay {
        None
    } else {
        admitted_already(state, api, issuer, email)
            .await
            .map_err(at(register))?
    } {
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

    let bound = crate::routes::bind_login(
        State(Arc::clone(state)),
        headers.clone(),
        Path(person.clone()),
        body(json!({
            "operation": derived(operation, bind),
            "issuer": issuer,
            "subject": subject,
        }))
        .map_err(at(bind))?,
    )
    .await
    .map_err(at(bind))?;
    receipt.done(bind, Some(("directory", bound.0.receipt.log.index)));

    let activated = crate::routes::transition(
        State(Arc::clone(state)),
        headers.clone(),
        Path(person.clone()),
        body(json!({
            "operation": derived(operation, activate),
            "transition": "activate",
            "reason": "admitted in one act",
        }))
        .map_err(at(activate))?,
    )
    .await
    .map_err(at(activate))?;
    receipt.done(activate, Some(("directory", activated.0.receipt.log.index)));
    granted_and_written(state, headers, act, receipt, person, subject, made).await
}

/// The person `email` is already admitted as, with their issuer subject:
/// an issuer account under the email whose login at `issuer` is bound to a
/// person. An account with no bound person is not an admission.
async fn admitted_already(
    state: &Shared,
    api: &crate::sign_in_providers::SignInProviders,
    issuer: &str,
    email: &str,
) -> Result<Option<(String, String)>, ServerError> {
    let Some(account) = crate::accounts::find_by_email(api, email).await? else {
        return Ok(None);
    };
    let Some(subject) = account.get("id").and_then(Value::as_str) else {
        return Ok(None);
    };
    let binding = lys_identity::LoginBinding::new(issuer, subject)?;
    let person = crate::routes::with_directory(state, |directory| {
        Ok(directory.projection()?.person_for(&binding))
    })?;
    Ok(person.map(|person| (person.to_string(), subject.to_owned())))
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
        let person_id = PersonId::from_str(&person).map_err(|error| (letter, error.into()))?;
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
            grant: granted,
            letter: wrote,
        });
    };
    let added = Letter::Added {
        name: asked.display_name.clone(),
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
        grant: granted,
        letter: wrote,
    })
}

/// Google's name at the issuer when people may sign in with it, so the
/// added letter tells the person to choose it.
pub(crate) async fn google_offered(api: &SignInProviders) -> Result<Option<String>, ServerError> {
    Ok(api
        .listed()
        .await?
        .into_iter()
        .find(|listed| {
            listed.enabled && Provider::from_name(&listed.name) == Some(Provider::Google)
        })
        .map(|listed| listed.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_has_its_own_operation_and_the_same_one_every_time() {
        let operation = OperationId::from_bytes([7; 16]);
        let ops: Vec<String> = STEPS.iter().map(|step| derived(&operation, step)).collect();
        let distinct: std::collections::BTreeSet<&String> = ops.iter().collect();
        assert_eq!(distinct.len(), STEPS.len());
        assert_eq!(ops[0], derived(&operation, "register"));
        assert!(ops.iter().all(|op| OperationId::from_str(op).is_ok()));
        assert_ne!(
            derived(&operation, "register"),
            derived(&OperationId::from_bytes([8; 16]), "register")
        );
    }

    #[test]
    fn the_letter_is_the_last_step_and_the_grant_before_it() {
        assert_eq!(STEPS[4..], ["grant", "letter"]);
    }

    #[test]
    fn a_person_is_admitted_with_no_first_grant_when_none_is_asked() {
        let asked: AdmitBody = serde_json::from_value(json!({
            "operation": "op-00000000000000000000000000000000",
            "display_name": "Care Helper",
            "email": "helper@example.test",
        }))
        .expect("a body without a grant is whole");
        assert!(asked.grant.is_none());
        let nulled: AdmitBody = serde_json::from_value(json!({
            "operation": "op-00000000000000000000000000000000",
            "display_name": "Care Helper",
            "email": "helper@example.test",
            "grant": null,
        }))
        .expect("a null grant is no grant");
        assert!(nulled.grant.is_none());
    }

    #[test]
    fn a_first_grant_when_asked_is_still_read_whole() {
        let asked: AdmitBody = serde_json::from_value(json!({
            "operation": "op-00000000000000000000000000000000",
            "display_name": "Care Helper",
            "email": "helper@example.test",
            "grant": {"route": "browser", "resource": {"kind": "workspace", "id": "ward-7"}, "relation": "editor"},
        }))
        .expect("a body with a grant is whole");
        let grant = asked.grant.expect("the grant is kept");
        assert_eq!(
            (grant.resource.kind.as_str(), grant.relation.as_str()),
            ("workspace", "editor")
        );
        let wrong = serde_json::from_value::<AdmitBody>(json!({
            "operation": "op-00000000000000000000000000000000",
            "display_name": "Care Helper",
            "email": "helper@example.test",
            "grant": {"route": "browser", "resource": {"kind": "workspace", "id": "ward-7"}},
        }));
        assert!(
            wrong.is_err(),
            "a grant without its relation is still refused"
        );
    }
}
