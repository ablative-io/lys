//! A computer's owner can read retained runner replies and acknowledge receipt explicitly.

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/network/machines/{id}/completions", get(list))
        .route("/network/machines/{id}/completions/{ticket}", get(read))
        .route(
            "/network/machines/{id}/completions/{ticket}/acknowledge",
            post(acknowledge),
        )
}

fn admitted(state: &AppState, headers: &HeaderMap, machine: &str) -> Result<(), ServerError> {
    let actor = signed_in(state, headers)?;
    crate::network_api::computer_owner(state, &actor, machine).map(drop)
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    admitted(&state, &headers, &id)?;
    let tickets = state.runners.hub().completion_tickets(&id)?;
    Ok(Json(json!({"machine":id,"tickets":tickets})))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, ticket)): Path<(String, String)>,
) -> Result<Json<Value>, ServerError> {
    admitted(&state, &headers, &id)?;
    let reply = state
        .runners
        .hub()
        .completion(&id, &ticket)?
        .ok_or_else(|| ServerError::Runner {
            refusal: "runner_completion_unknown".to_owned(),
            words: "this computer holds no retained completion for that ticket".to_owned(),
        })?;
    Ok(Json(json!({"machine":id,"ticket":ticket,"reply":reply})))
}

async fn acknowledge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, ticket)): Path<(String, String)>,
) -> Result<Json<Value>, ServerError> {
    admitted(&state, &headers, &id)?;
    state.runners.hub().acknowledge_completion(&id, &ticket)?;
    Ok(Json(
        json!({"machine":id,"ticket":ticket,"acknowledged":true}),
    ))
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, header};
    use lys_identity::{Actor, AuthMethod, Provenance};
    use serde_json::Value;
    use std::error::Error;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn an_administrator_reads_and_acknowledges_a_late_completion_over_http()
    -> Result<(), Box<dyn Error>> {
        let (_dir, state, runner, _) =
            super::super::machine_folder_admission_tests::fixture().await?;
        let actor = Actor::new(
            state
                .admission
                .administrator_login()?
                .ok_or("administrator absent")?,
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        let cookie = state.sessions.begin(actor)?;
        let cookie = cookie.split(';').next().ok_or("cookie absent")?;
        let error = state
            .runners
            .hub()
            .reply("own", "fixture-ticket", Ok("finished".to_owned()))
            .err()
            .ok_or("ticket was not delivered")?;
        assert_eq!(error.name(), "runner_reply_unknown");
        let router = super::routes().with_state(Arc::clone(&state));
        let stranger = Actor::new(
            lys_identity::LoginBinding::new(state.oidc.issuer(), "unbound-reader")?,
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        let stranger_cookie = state.sessions.begin(stranger)?;
        let stranger_cookie = stranger_cookie.split(';').next().ok_or("cookie absent")?;
        let denied = router
            .clone()
            .oneshot(
                Request::get("/network/machines/own/completions/fixture-ticket")
                    .header(header::COOKIE, stranger_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(denied.status(), 403);
        let denied: Value =
            serde_json::from_slice(&to_bytes(denied.into_body(), usize::MAX).await?)?;
        assert_eq!(denied["refusal"], "NoPerson");
        for (method, path) in [
            ("GET", "/network/machines/own/completions/fixture-ticket"),
            (
                "POST",
                "/network/machines/own/completions/fixture-ticket/acknowledge",
            ),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header(header::COOKIE, cookie)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), 200);
            let answer: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?;
            if method == "GET" {
                assert_eq!(answer["reply"], "finished");
            } else {
                assert_eq!(answer["acknowledged"], true);
            }
        }
        assert!(
            state
                .runners
                .hub()
                .completion("own", "fixture-ticket")?
                .is_none()
        );
        runner.stop()?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn completion_routes_declare_machine_scopes() {
    for (method, path) in [
        ("GET", "/network/machines/own/completions"),
        ("GET", "/network/machines/own/completions/ticket"),
        (
            "POST",
            "/network/machines/own/completions/ticket/acknowledge",
        ),
    ] {
        assert!(
            crate::openapi_table::token_scope(method, path).is_ok(),
            "undeclared completion route: {method} {path}"
        );
    }
}
