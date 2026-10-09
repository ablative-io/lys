//! Admitting a person in one act: register them, make their sign-in account
//! at the issuer by email, bind that login, activate them and issue their
//! first named root grant, in that order (PERMISSIONS-STORY 9 Oct 2026: a
//! root grant to a person not yet active is refused `IdentityNotActive`).
//!
//! Each step runs as the existing route does, under an operation derived from
//! the act's one operation and the step's name, so asking again replays the
//! steps already done and finishes what is left; nothing is made twice. The
//! issuer account is found by email before one is made. A failed step is
//! answered with its own refusal and a receipt naming it and the steps that
//! completed before it. The receipt names the log leaf each written step was
//! recorded at, the directory's three and the grant's, so it is the logs'
//! own record: asking again with the same operation answers the same leaves.
//! Nothing here calls a product or holds a lock across the issuer's answer.

use axum::Json;
use axum::body::to_bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::error::ServerError;
use crate::routes::{Shared, signed_in};

/// The steps of the act, in the order they run.
pub const STEPS: [&str; 5] = ["register", "account", "bind", "activate", "grant"];

/// What the administrator asks for.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleAdmitBody)]
pub(crate) struct AdmitBody {
    /// The act's operation; each step's is derived from it.
    operation: String,
    /// The person's display name.
    display_name: String,
    /// The email their issuer account is found or made by.
    email: String,
    /// Their first root grant.
    #[schema(inline)]
    grant: FirstGrant,
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
    /// issuer account is written at the issuer, and has none.
    pub logged: Vec<Logged>,
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
    /// Their first grant.
    pub grant: String,
    /// What the act did.
    #[schema(inline)]
    pub receipt: AdmitReceipt,
}

/// The operation of `step` within the act `operation`: the first sixteen
/// bytes of SHA-256 over the act's operation, a zero byte and the step name.
fn derived(operation: &OperationId, step: &str) -> String {
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
async fn refused(error: ServerError, receipt: &AdmitReceipt) -> Response {
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
    Response::from_parts(parts, axum::body::Body::from(Value::Object(answer).to_string()))
}

fn parts_only(parts: axum::http::response::Parts) -> Response {
    Response::from_parts(parts, axum::body::Body::empty())
}

/// The act's words checked before any step: the caller is the
/// administrator, the body is whole, the operation and the email are well
/// formed. The issuer is the one the administrator signed in through.
fn checked(
    state: &Shared,
    headers: &HeaderMap,
    asked: Result<Json<AdmitBody>, JsonRejection>,
) -> Result<(AdmitBody, OperationId, String, String), ServerError> {
    let actor = signed_in(state, headers)?;
    crate::routes::administrator(state, &actor)?;
    let Json(asked) = asked.map_err(|rejected| malformed(rejected.body_text()))?;
    let operation = OperationId::from_str(&asked.operation)?;
    let email = crate::accounts::check_email(&asked.email)
        .map_err(|_refused| malformed(format!("{} is not an email", asked.email)))?
        .to_owned();
    let issuer = actor.binding().issuer().to_owned();
    Ok((asked, operation, email, issuer))
}

/// POST /people/admit: the five steps, in order, as one act.
pub(crate) async fn admit(
    State(state): State<Shared>,
    headers: HeaderMap,
    asked: Result<Json<AdmitBody>, JsonRejection>,
) -> Response {
    let mut receipt = AdmitReceipt::default();
    let (asked, operation, email, issuer) = match checked(&state, &headers, asked) {
        Ok(checked) => checked,
        Err(error) => return refused(error, &receipt).await,
    };
    receipt.operation = operation.to_string();
    let act = Act {
        asked: &asked,
        operation: &operation,
        email: &email,
        issuer: &issuer,
    };
    match steps(&state, &headers, &act, &mut receipt).await {
        Ok((person, subject, grant)) => Json(Admitted {
            person,
            subject,
            grant,
            receipt,
        })
        .into_response(),
        Err((step, error)) => {
            receipt.failed = Some(step.to_owned());
            refused(error, &receipt).await
        }
    }
}

/// The act's checked words. The login is bound at the issuer the
/// administrator signed in through, the one whose accounts Lys administers,
/// as first-run setup binds its own.
struct Act<'a> {
    asked: &'a AdmitBody,
    operation: &'a OperationId,
    email: &'a str,
    issuer: &'a str,
}

/// Run every step, recording each as it completes; the failing step's name
/// beside its refusal.
async fn steps(
    state: &Shared,
    headers: &HeaderMap,
    act: &Act<'_>,
    receipt: &mut AdmitReceipt,
) -> Result<(String, String, String), (&'static str, ServerError)> {
    let Act {
        asked,
        operation,
        email,
        issuer,
    } = *act;
    let [register, account, bind, activate, grant] = STEPS;
    let at = |step: &'static str| move |error: ServerError| (step, error);
    let registered = crate::routes::register_person(
        State(state.clone()),
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
    receipt.done(register, Some(("directory", registered.0.receipt.log.index)));

    let api = crate::sign_in_providers::api(state).map_err(at(account))?;
    let (subject, _made) = crate::accounts::make(api, email, &asked.display_name)
        .await
        .map_err(at(account))?;
    receipt.done(account, None);

    let bound = crate::routes::bind_login(
        State(state.clone()),
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
        State(state.clone()),
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

    let issued = crate::grants::issue_root(
        State(state.clone()),
        headers.clone(),
        body(json!({
            "operation": derived(operation, grant),
            "route": asked.grant.route,
            "holder": person,
            "resource": asked.grant.resource,
            "relation": asked.grant.relation,
            "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": null},
        }))
        .map_err(at(grant))?,
    )
    .await
    .map_err(at(grant))?;
    receipt.done(grant, Some(("grants", issued.0.index)));
    Ok((person, subject, issued.0.grant.clone()))
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
}
