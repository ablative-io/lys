#![cfg(test)]
//! Profile review checks current account ownership and provider routing.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::secrets_api::SecretsSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

struct BrokerTask(tokio::task::JoinHandle<std::io::Result<()>>);

impl Drop for BrokerTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn inventory(State(mode): State<Arc<AtomicU8>>, request: Request) -> Response {
    if request.uri().path() == "/_lys/account/hold" {
        assert_eq!(request.headers()["lys-service"], "identity");
        assert!(request.headers().contains_key("lys-service-signature"));
        let bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
        let asked: Value = match serde_json::from_slice(&bytes) {
            Ok(asked) => asked,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
        if asked["proxy_key"] == "invalid" {
            return (
                StatusCode::FORBIDDEN,
                "account_proxy_key_invalid: fixture key is invalid",
            )
                .into_response();
        }
        return Json(json!({"handle":"first","holder":asked["agent"],"secret":asked["secret"]}))
            .into_response();
    }
    let mode = mode.load(Ordering::SeqCst);
    let answer = match request.uri().path() {
        "/_lys/handles" => {
            let handles = if mode == 0 {
                json!([])
            } else {
                json!([{"id":"first","secret":"login","dropped":mode == 3}])
            };
            json!({"handles":handles})
        }
        "/_lys/secrets" => {
            let upstream = if mode == 1 {
                "https://api.openai.com"
            } else {
                "https://api.anthropic.com"
            };
            json!({"secrets":[{"name":"login","class":"credential","upstream":upstream,"header":"authorization"}]})
        }
        path => json!({"unexpected_path":path}),
    };
    Json(answer).into_response()
}

#[tokio::test]
async fn account_review_refuses_unheld_dropped_and_misrouted_handles_before_recording() -> TestResult
{
    let mode = Arc::new(AtomicU8::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let broker = format!("http://{}", listener.local_addr()?);
    let router = Router::new()
        .fallback(inventory)
        .with_state(Arc::clone(&mode));
    let broker_task = BrokerTask(tokio::spawn(
        async move { axum::serve(listener, router).await },
    ));
    let keys = tempfile::tempdir()?;
    let service_key_file = keys.path().join("service.key");
    Ed25519Identity::load_or_generate(&service_key_file)?;
    let settings = SecretsSettings {
        broker,
        service: "identity".to_owned(),
        service_key_file,
    };
    let (service, seeded) = Service::start_asking(GRANT_MODEL, None, Some(settings), |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    hold_route(
        &service,
        &cookie,
        &seeded.people[0].agents[0].id.to_string(),
    )
    .await?;
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
    let profile = json!({
        "operation":OperationId::generate()?.to_string(), "from_version":0,
        "model_access":["haiku"], "tools":[], "skills":[], "mcp_servers":[],
        "instructions":"", "note":"",
        "harness":{"name":"fixture","program":"/bin/sh","package":"fixture","description":catalogue["description"]},
        "session":{"stall_after":300,"accounts":{"accounts":["first"],"variable":"LYS_HANDLE_ACCOUNT","limit":{"signal":"plan_window"}}}
    });
    let path = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    let (status, body) = service.post(&path, Some(&cookie), &profile).await?;
    assert_eq!(status, 200, "{body}");
    for (case, refusal) in [
        (0, "account_handle_unheld"),
        (1, "account_provider_mismatch"),
        (3, "account_handle_unheld"),
    ] {
        mode.store(case, Ordering::SeqCst);
        let review = json!({"operation":OperationId::generate()?.to_string()});
        let (status, body) = service
            .post(&format!("{path}/1/review"), Some(&cookie), &review)
            .await?;
        assert_eq!(status, 403, "{body}");
        assert_eq!(body["refusal"], refusal, "{body}");
        assert!(
            body["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("first")),
            "{body}"
        );
        let (status, current) = service.get(&path, Some(&cookie)).await?;
        assert_eq!(status, 200, "{current}");
        assert_eq!(current["profile"]["reviewed_by"], Value::Null);
    }
    mode.store(2, Ordering::SeqCst);
    let review = json!({"operation":OperationId::generate()?.to_string()});
    let (status, body) = service
        .post(&format!("{path}/1/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 200, "{body}");
    assert!(body["profile"]["reviewed_by"].is_string(), "{body}");
    drop(broker_task);
    Ok(())
}

async fn hold_route(service: &Service, cookie: &str, agent: &str) -> TestResult {
    let mut request = json!({
        "agent":agent, "secret":"login", "proxy_key":"invalid",
        "max_uses":10, "not_after_ms":2_100_000_000_000_i64,
    });
    let (status, refusal) = service
        .post("/secrets/accounts/hold", Some(cookie), &request)
        .await?;
    assert_eq!(status, 403, "{refusal}");
    assert_eq!(refusal["refusal"], "account_proxy_key_invalid", "{refusal}");
    request["proxy_key"] = Value::String("ab".repeat(32));
    let (status, held) = service
        .post("/secrets/accounts/hold", Some(cookie), &request)
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["handle"], "first");
    assert_eq!(held["holder"], agent);
    assert_eq!(held["secret"], "login");
    assert!(held.get("token").is_none());
    assert!(held.get("value").is_none());
    Ok(())
}
