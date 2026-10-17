//! The consent door receives socket peers without requiring host aliases.
#![cfg(test)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use serde_json::{Value, json};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
const REDIRECT: &str = "http://127.0.0.1:23456/lys/sign-in";

pub(super) struct Door {
    router: Router,
    cookie: String,
    dir: tempfile::TempDir,
}

impl Door {
    pub(super) async fn new() -> TestResult<Self> {
        let (dir, state, actor, _, _, _) = crate::grant_settlement_tests::person_fixture().await?;
        let cookie = state
            .sessions
            .begin(actor)?
            .split(';')
            .next()
            .ok_or("session cookie absent")?
            .to_owned();
        let state = Arc::new(state);
        let apps = super::super::Apps::open(
            Arc::clone(&state),
            &dir.path().join("directory"),
            "http://fixture.test",
            "/mcp",
            &[],
        )?;
        let router = crate::session_admission::guarded(
            crate::signed_first::guarded(super::super::routes(apps), Arc::clone(&state)),
            Arc::clone(&state),
        )
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::operator::guard,
        ));
        Ok(Self {
            router,
            cookie,
            dir,
        })
    }

    async fn answer(
        &self,
        peer: SocketAddr,
        mut request: Request<Body>,
    ) -> TestResult<(StatusCode, HeaderMap, String)> {
        request.extensions_mut().insert(ConnectInfo(peer));
        request.headers_mut().insert("cookie", self.cookie.parse()?);
        let response = self.router.clone().oneshot(request).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = String::from_utf8(to_bytes(response.into_body(), usize::MAX).await?.to_vec())?;
        Ok((status, headers, body))
    }

    pub(super) async fn register(&self, peer: SocketAddr) -> TestResult<String> {
        let request = Request::post("/oauth/mcp/register")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"client_name":"Claude Code","redirect_uris":[REDIRECT]}).to_string(),
            ))?;
        let (status, _, body) = self.answer(peer, request).await?;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let app: Value = serde_json::from_str(&body)?;
        Ok(app["client_id"].as_str().ok_or("client absent")?.to_owned())
    }

    pub(super) async fn authorize(&self, peer: SocketAddr, client: &str) -> TestResult<String> {
        let address = reqwest::Url::parse_with_params(
            "http://fixture.test/oauth/mcp/authorize",
            &[
                ("response_type", "code"),
                ("client_id", client),
                ("redirect_uri", REDIRECT),
                ("code_challenge", &"a".repeat(43)),
                ("code_challenge_method", "S256"),
                ("state", "kept"),
            ],
        )?;
        let request = Request::get(address.as_str()).body(Body::empty())?;
        let (status, _, body) = self.answer(peer, request).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        Ok(body
            .split("name=\"asking\" value=\"")
            .nth(1)
            .and_then(|text| text.split('"').next())
            .ok_or("asking absent")?
            .to_owned())
    }

    pub(super) async fn decide(
        &self,
        peer: SocketAddr,
        asking: &str,
    ) -> TestResult<(StatusCode, HeaderMap, String)> {
        let request = Request::post("/oauth/mcp/consent")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("asking={asking}&decision=approve")))?;
        self.answer(peer, request).await
    }

    pub(super) fn finish(self) -> TestResult {
        drop(self.router);
        self.dir.close()?;
        Ok(())
    }
}
