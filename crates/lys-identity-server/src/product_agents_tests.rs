use super::{Access, OpenIdProvider, ProviderSettings, held};
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use lys_identity::PersonId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::Arc;
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn Error>>;

struct Table {
    _dir: tempfile::TempDir,
    state: Arc<crate::routes::AppState>,
    session: String,
    person: PersonId,
    own: String,
    other: String,
}

impl Table {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let (dir, mut state, actor, person, own, other) =
            crate::grant_settlement_tests::person_fixture().await?;
        let cookie = state.sessions.begin(actor)?;
        let session = state.sessions.current(cookie.split(';').next())?;
        state.provider = Some(OpenIdProvider::open(
            &ProviderSettings {
                key_file: state.grant_setup.key_file.clone(),
                code_seconds: 60,
                pass_seconds: 600,
                rights_bytes: None,
            },
            "https://fixture.test".to_owned(),
        )?);
        Ok(Self {
            _dir: dir,
            state: Arc::new(state),
            session,
            person,
            own: own.to_string(),
            other: other.to_string(),
        })
    }

    fn pass(&self, app: &str, expired: bool, kept: bool) -> Result<String, Box<dyn Error>> {
        self.pass_for(app, &self.person.to_string(), expired, kept)
    }

    fn pass_for(
        &self,
        app: &str,
        subject: &str,
        expired: bool,
        kept: bool,
    ) -> Result<String, Box<dyn Error>> {
        let at = crate::session::now();
        let expires_at = if expired { at - 1 } else { at + 60 };
        let provider = self.state.provider.as_ref().ok_or("provider absent")?;
        let pass = provider.signed(&json!({
            "iss": provider.issuer(), "aud": app, "sub": subject,
            "iat": at - 2, "exp": expires_at,
            "holder": { "id": subject, "kind": if subject.starts_with("person-") { "person" } else { "agent" }, "responsible": self.person.to_string() },
            "rights": []
        }))?;
        if kept {
            held(&provider.tokens)?.insert(
                crate::routes::hex(&Sha256::digest(pass.as_bytes())),
                Access {
                    session_id: self.session.clone(),
                    subject: subject.to_owned(),
                    app: app.to_owned(),
                    profile: false,
                    expires_at,
                    kind: None,
                },
                at,
            )?;
        }
        Ok(pass)
    }

    async fn get(&self, path: &str, pass: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let router = crate::signed_first::guarded(
            crate::routes_table::router(Arc::clone(&self.state)),
            Arc::clone(&self.state),
        );
        let response = crate::session_admission::guarded(router, Arc::clone(&self.state))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&self.state),
                crate::operator::guard,
            ))
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header(header::AUTHORIZATION, format!("Bearer {pass}"))
                    .body(Body::empty())?,
            )
            .await?;
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), usize::MAX).await?;
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)?
        };
        Ok((status, value))
    }
}

#[tokio::test]
async fn a_product_pass_lists_exactly_its_persons_agents() -> TestResult {
    let table = Table::new().await?;
    let (status, answer) = table
        .get(
            "/apps/me/agents",
            &table.pass("fixture_product", false, true)?,
        )
        .await?;
    assert_eq!(status, 200);
    assert_eq!(
        answer["agents"],
        json!([{ "id": table.own, "display_name": "Holder", "handle": null, "state": "active" }])
    );
    assert_eq!(answer["lys_url"], "https://fixture.test/api");
    Ok(())
}

#[tokio::test]
async fn a_product_pass_never_lists_another_persons_agents() -> TestResult {
    let table = Table::new().await?;
    let pass = table.pass("fixture_product", false, true)?;
    let (status, answer) = table.get("/apps/me/agents", &pass).await?;
    assert_eq!(status, 200);
    let agents = answer["agents"].as_array().ok_or("agents absent")?;
    assert_eq!(agents.len(), 1);
    assert!(agents.iter().all(|agent| agent["id"] != table.other));
    let (status, refusal) = table
        .get(&format!("/apps/me/agents?person={}", table.other), &pass)
        .await?;
    assert_eq!(status, 400);
    assert_eq!(refusal["refusal"], "RequestMalformed");
    for path in ["/me", "/people", "/dashboard"] {
        let (status, _) = table.get(path, &pass).await?;
        assert_ne!(status, 200, "a product pass reached {path}");
    }
    Ok(())
}

#[tokio::test]
async fn a_lys_audience_pass_lists_only_the_same_persons_agents() -> TestResult {
    let table = Table::new().await?;
    let (status, answer) = table
        .get("/apps/me/agents", &table.pass("lys", false, true)?)
        .await?;
    assert_eq!(status, 200);
    assert_eq!(answer["agents"][0]["id"], table.own);
    assert_eq!(answer["agents"].as_array().ok_or("agents absent")?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn an_unknown_access_token_is_refused_by_name() -> TestResult {
    let table = Table::new().await?;
    for pass in [
        "unknown".to_owned(),
        table.pass("fixture_product", false, false)?,
    ] {
        let (status, refusal) = table.get("/apps/me/agents", &pass).await?;
        assert_eq!(status, 401);
        assert_eq!(refusal["refusal"], "TokenUnknown");
    }
    Ok(())
}

#[tokio::test]
async fn an_expired_access_token_is_refused_by_name() -> TestResult {
    let table = Table::new().await?;
    let (status, refusal) = table
        .get(
            "/apps/me/agents",
            &table.pass("fixture_product", true, false)?,
        )
        .await?;
    assert_eq!(status, 401);
    assert_eq!(refusal["refusal"], "PassRefused");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("expired"))
    );
    Ok(())
}

#[tokio::test]
async fn an_agents_pass_cannot_read_a_persons_agents() -> TestResult {
    let table = Table::new().await?;
    let pass = table.pass_for("fixture_product", &table.own, false, false)?;
    let (status, refusal) = table.get("/apps/me/agents", &pass).await?;
    assert_eq!(status, 403);
    assert_eq!(refusal["refusal"], "ProductPassPersonRequired");
    Ok(())
}

#[path = "app_person_tests.rs"]
mod app_person_tests;
