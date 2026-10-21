//! POST /people/admit asked with an approved app's person pass
//! (DIRECTORY-113 R1): the act is judged as the person the pass speaks for,
//! exactly as from their own Lys session, and narrowly. A product admits only
//! with the person's first grant, on a kind the asking app owns; both are
//! checked before any step, so a refusal records nothing.
//!
//! The steps call the routes they always call. For the act's duration the
//! person's actor is held here, `routes::signed_in` answers it first, and the
//! routes are called with the request's headers less its Authorization, so
//! no step reads the app's pass as a credential of its own, and every leaf
//! the act writes names the app beside the person. Nothing else sets the
//! held actor: every other route still refuses an app's pass the person's
//! administrator authority.
//!
//! An admission a product asks for goes on to that product (DIRECTORY-113
//! R2): its handed link records the origin of the app's first return address
//! in Lys's own record of the app, never a request's word.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, header};
use axum::response::Response;
use lys_identity::Actor;

use crate::error::ServerError;
use crate::people_admit::{Act, AdmitBody, AdmitReceipt, answer, checked, refused, steps};
use crate::routes::Shared;

tokio::task_local! {
    static ADMITTER: Actor;
}

/// The person an app's pass admits as, while that act's steps run.
pub(crate) fn actor() -> Option<Actor> {
    ADMITTER.try_with(Clone::clone).ok()
}

/// POST /people/admit for `actor`, the person `app`'s pass speaks for.
pub(crate) async fn admit(
    state: &Shared,
    headers: &HeaderMap,
    actor: &Actor,
    app: &str,
    asked: Result<Json<AdmitBody>, JsonRejection>,
) -> Response {
    let mut receipt = AdmitReceipt::default();
    // What the product may ask is judged first, then the person's authority.
    let judged = asked.map_err(|rejected| ServerError::RequestMalformed {
        reason: rejected.body_text(),
    });
    let judged = judged.and_then(|Json(body)| {
        let continue_to = owned_kind(state, app, &body)?;
        Ok((checked(state, actor, Ok(Json(body)))?, continue_to))
    });
    let ((asked, operation, email, issuer, organisation), continue_to) = match judged {
        Ok(judged) => judged,
        Err(error) => return refused(error, &receipt).await,
    };
    receipt.operation = operation.to_string();
    let act = Act {
        asked: &asked,
        operation: &operation,
        email: &email,
        issuer: &issuer,
        organisation,
        continue_to,
    };
    let mut inner = headers.clone();
    inner.remove(header::AUTHORIZATION);
    // Every leaf of the act is signed as the person, naming the app beside
    // them (DIRECTORY-113 Amendment 2).
    let answered = ADMITTER
        .scope(
            actor.clone().with_through_app(app),
            steps(state, &inner, &act, &mut receipt),
        )
        .await;
    answer(answered, receipt).await
}

/// The first grant is asked for and on a kind `app` owns; answers where the
/// admission goes on to.
fn owned_kind(state: &Shared, app: &str, asked: &AdmitBody) -> Result<Option<String>, ServerError> {
    let kind = asked
        .first_grant_kind()
        .ok_or(ServerError::AppAdmitGrantRequired)?;
    crate::apps_api::with_apps(state, |apps, _| {
        apps.admit_kind(Some(app), kind)?;
        Ok(apps.app(app).and_then(|held| {
            held.sign_in
                .as_ref()
                .map(|set| set.redirects.as_slice())
                .filter(|redirects| !redirects.is_empty())
                .unwrap_or(held.registered.redirects.as_slice())
                .first()
                .and_then(|address| origin(address))
        }))
    })
}

/// `address`'s scheme, host and port, the port only when it is not the
/// scheme's default.
fn origin(address: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(address).ok()?;
    let origin = parsed.origin();
    origin.is_tuple().then(|| origin.ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::origin;

    #[test]
    fn an_origin_keeps_the_scheme_host_and_a_port_only_when_it_is_not_the_default() {
        assert_eq!(
            origin("https://app.example.test/identity/oidc/callback").as_deref(),
            Some("https://app.example.test")
        );
        assert_eq!(
            origin("https://app.example.test:443/x").as_deref(),
            Some("https://app.example.test")
        );
        assert_eq!(
            origin("http://127.0.0.1:8611/callback").as_deref(),
            Some("http://127.0.0.1:8611")
        );
        assert_eq!(origin("not an address"), None);
    }
}
