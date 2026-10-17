use super::super::Table;
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use lys_core::Ed25519Identity;
use lys_identity::{Actor, AuthMethod, OperationId, Provenance};
use serde_json::{Value, json};
use std::error::Error;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub(super) fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

pub(super) fn acts(agent: &str) -> TestResult<Vec<(String, Value)>> {
    Ok(vec![
        (
            "/agents".to_owned(),
            json!({"operation": operation()?, "display_name": "New agent"}),
        ),
        (
            format!("/identities/{agent}/transitions"),
            json!({"operation": operation()?, "transition": "activate"}),
        ),
        (
            format!("/agents/{agent}/certificates"),
            json!({"operation": operation()?, "request": "invalid"}),
        ),
    ])
}

pub(super) fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

impl Table {
    pub(super) async fn app_fixture() -> TestResult<Self> {
        let mut table = Self::new().await?;
        let state = Arc::get_mut(&mut table.state).ok_or("fixture state shared")?;
        let key = Arc::new(Ed25519Identity::load(&state.grant_setup.key_file)?);
        state.certificates = Some(Mutex::new(
            crate::certificates_store::CertificateStore::open(
                &table.dir.path().join("certificates"),
                Arc::clone(&key),
            )?,
        ));
        state.policies = Some(Mutex::new(crate::agent_policy_store::PolicyStore::open(
            &table.dir.path().join("policies"),
            key,
        )?));
        for app in ["cambium", "fixture_product"] {
            table.register_app(app, true)?;
        }
        crate::apps_api::refresh(&table.state)?;
        Ok(table)
    }

    pub(super) fn register_app(&self, app: &str, approved: bool) -> TestResult {
        use crate::apps_state::{Approved, By, Client, Line, Registered};
        crate::apps_api::with_apps(&self.state, |apps, _| {
            apps.keep(Line::Registered(Registered {
                operation: format!("register-{app}"),
                app: app.to_owned(),
                name: app.to_owned(),
                redirects: vec!["https://app.test/callback".to_owned()],
                schema: json!({"kinds": {format!("{app}.agent"): {"actions": ["read"], "relations": {"viewer": ["read"]}}}}),
                service_account: None,
                by: By::Start,
                at: 1,
            }))?;
            if approved {
                apps.keep(Line::Approved(Approved {
                    operation: format!("approve-{app}"),
                    app: app.to_owned(),
                    client: Client {
                        client_id: app.to_owned(),
                        secret_sha256: crate::apps_binding::sha256_hex("fixture"),
                    },
                    binding: None,
                    by: By::Start,
                    at: 2,
                }))?;
            }
            Ok(())
        })?;
        Ok(())
    }

    pub(super) fn administrator(&mut self) -> TestResult {
        let binding = self
            .state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?;
        let actor = Actor::new(
            binding,
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        self.person = crate::routes::with_directory(&self.state, |directory| {
            crate::read_api::own_person(directory.projection()?, &actor)
        })?;
        let cookie = self.state.sessions.begin(actor)?;
        self.session = self.state.sessions.current(cookie.split(';').next())?;
        Ok(())
    }

    pub(super) fn cookie(&self) -> TestResult<String> {
        let actor = self
            .state
            .sessions
            .live(|_| true)?
            .into_iter()
            .find(|entry| entry.id == self.session)
            .ok_or("session absent")?
            .actor;
        Ok(self
            .state
            .sessions
            .begin(actor)?
            .split(';')
            .next()
            .ok_or("cookie absent")?
            .to_owned())
    }

    pub(super) fn other_person(&self) -> TestResult<String> {
        Ok(crate::routes::with_directory(&self.state, |directory| {
            let id = crate::routes::identity_id(&self.other)?;
            directory
                .projection()?
                .record(id)
                .and_then(lys_identity::projection::Record::responsible)
                .map(|person| person.to_string())
                .ok_or(crate::error::ServerError::NoPerson)
        })?)
    }

    pub(super) fn state_of(&self, agent: &str) -> TestResult<String> {
        Ok(crate::routes::with_directory(&self.state, |directory| {
            directory
                .projection()?
                .record(crate::routes::identity_id(agent)?)
                .map(|record| record.state().to_string())
                .ok_or(crate::error::ServerError::AgentNotVisible)
        })?)
    }

    pub(super) fn revoke(&self, pass: &str) -> TestResult {
        use sha2::{Digest, Sha256};
        let provider = self.state.provider.as_ref().ok_or("provider absent")?;
        super::super::held(&provider.tokens)?
            .revoke(&crate::routes::hex(&Sha256::digest(pass.as_bytes())))?;
        Ok(())
    }

    fn admitted(&self) -> TestResult<axum::Router> {
        let mut dispatch = self.dispatch.lock().map_err(|error| error.to_string())?;
        if let Some(router) = dispatch.as_ref() {
            return Ok(router.as_ref().clone());
        }
        let router = crate::signed_first::guarded(
            crate::routes_table::router(Arc::clone(&self.state))
                .merge(crate::certificates_api::routes().with_state(Arc::clone(&self.state))),
            Arc::clone(&self.state),
        );
        let router = Arc::new(
            crate::session_admission::guarded(router, Arc::clone(&self.state)).layer(
                axum::middleware::from_fn_with_state(
                    Arc::clone(&self.state),
                    crate::operator::guard,
                ),
            ),
        );
        self.state.draft_execution.attach(Arc::downgrade(&router))?;
        let answer = router.as_ref().clone();
        *dispatch = Some(router);
        Ok(answer)
    }

    pub(super) async fn send(
        &self,
        method: &str,
        path: &str,
        pass: &str,
        body: &Value,
        extra: &[(&str, &str)],
    ) -> TestResult<(u16, Value)> {
        let router = self.admitted()?;
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::AUTHORIZATION, format!("Bearer {pass}"))
            .header(header::CONTENT_TYPE, "application/json");
        for (name, value) in extra {
            request = request.header(*name, *value);
        }
        let response = router
            .oneshot(request.body(Body::from(serde_json::to_vec(body)?))?)
            .await?;
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), usize::MAX).await?;
        let answer = serde_json::from_slice(&bytes)
            .map_err(|error| format!("fixture response at {path}, status {status}: {error}"))?;
        Ok((status, answer))
    }
}
