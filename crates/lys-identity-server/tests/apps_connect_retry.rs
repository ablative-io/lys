#![cfg(test)]
//! Retries preserve app credentials and a failed replacement preserves the old one.

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use identity_contract::apps::{Auth, NOTES, get, login, ok, op, post, seeded, workspace_schema};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn body(operation: &str) -> Value {
    json!({"operation": operation,
        "manifest": {"id":NOTES,"name":"Notes","redirects":["https://notes.test/callback"],"schema":workspace_schema(NOTES)},
        "return":"https://notes.test/connected","state":"connection"})
}

fn live(app: &Value) -> TestResult<Vec<String>> {
    app["client_credentials"]
        .as_array()
        .ok_or("credential references absent")?
        .iter()
        .filter(|entry| entry["live"] == true)
        .map(|entry| {
            entry["credential_id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "credential id absent".into())
        })
        .collect()
}

#[tokio::test]
async fn connect_retry_returns_the_first_outcome_and_one_live_credential() -> TestResult
{
    let (mut service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let body = body(&op()?);
    let first = ok(post(&service, "/apps/connect", Auth::Cookie(&admin), &body).await?)?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    let before = live(&app)?;
    let again = ok(post(&service, "/apps/connect", Auth::Cookie(&admin), &body).await?)?;
    assert!(
        first == again,
        "the same connect operation answered a different outcome"
    );
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(
        live(&app)?,
        before,
        "the retry replaced the credential reference"
    );
    assert_eq!(before.len(), 1);
    let mut changed = body.clone();
    changed["state"] = json!("another-request");
    let reused = post(&service, "/apps/connect", Auth::Cookie(&admin), &changed).await?;
    assert_eq!(
        reused.0, 409,
        "one operation admitted different connection words"
    );
    assert_eq!(reused.1["refusal"], "app_operation_reused");
    service.restart().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let restarted = ok(post(&service, "/apps/connect", Auth::Cookie(&admin), &body).await?)?;
    assert!(
        restarted == first,
        "restart forgot the first connect outcome"
    );
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_eq!(live(&app)?, before);
    let redirect = first["redirect"]
        .as_str()
        .ok_or("connection redirect absent")?;
    let code = reqwest::Url::parse(redirect)?
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, code)| code.into_owned())
        .ok_or("connection code absent")?;
    let exchanged = ok(post(
        &service,
        "/apps/connect/exchange",
        Auth::Nobody,
        &json!({"code":code}),
    )
    .await?)?;
    assert_eq!(exchanged["app"], NOTES);
    let credential = exchanged["client_secret"]
        .as_str()
        .ok_or("client credential absent")?;
    for entry in std::fs::read_dir(service.dir.path().join("app-connections"))? {
        let bytes = std::fs::read(entry?.path())?;
        assert!(
            !bytes
                .windows(credential.len())
                .any(|window| window == credential.as_bytes()),
            "checkpoint exposed a credential"
        );
        assert!(
            !bytes
                .windows(code.len())
                .any(|window| window == code.as_bytes()),
            "checkpoint exposed an exchange code"
        );
    }
    service.restart().await?;
    let spent = post(
        &service,
        "/apps/connect/exchange",
        Auth::Nobody,
        &json!({"code":code}),
    )
    .await?;
    assert_eq!(spent.0, 400, "restart admitted a spent exchange code");
    assert_eq!(spent.1["refusal"], "connect_code_used");
    Ok(())
}

#[tokio::test]
async fn connect_initializes_checkpoints_for_an_existing_app_install() -> TestResult {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    identity_contract::apps::registered(&service, &admin, NOTES).await?;
    let issued = ok(post(
        &service,
        &format!("/apps/{NOTES}/credentials/issue"),
        Auth::Cookie(&admin),
        &json!({"operation":op()?}),
    )
    .await?)?;
    let old = issued["credential_id"]
        .as_str()
        .ok_or("old credential reference absent")?;
    assert!(!service.dir.path().join("app-connections").exists());
    ok(post(
        &service,
        "/apps/connect",
        Auth::Cookie(&admin),
        &body(&op()?),
    )
    .await?)?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    let current = live(&app)?;
    assert_eq!(current.len(), 1);
    assert_ne!(current[0], old);
    Ok(())
}

#[derive(Clone)]
struct Proxy {
    upstream: String,
    fail_bearer: Arc<AtomicBool>,
    client_issued: Arc<AtomicBool>,
}

async fn proxy(State(state): State<Proxy>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    if parts.uri.path() == "/_lys/apps/bearer/issue"
        && state.fail_bearer.swap(false, Ordering::SeqCst)
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "injected failure after client issuance",
        )
            .into_response();
    }
    let bytes = match to_bytes(body, 1_048_576).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let response = reqwest::Client::new()
        .request(parts.method, format!("{}{}", state.upstream, parts.uri))
        .headers(parts.headers)
        .body(bytes)
        .send()
        .await;
    match response {
        Ok(response) => {
            let status = response.status();
            if parts.uri.path() == "/_lys/apps/client/issue" && status.is_success() {
                state.client_issued.store(true, Ordering::SeqCst);
            }
            match response.bytes().await {
                Ok(bytes) => (status, bytes).into_response(),
                Err(_) => StatusCode::BAD_GATEWAY.into_response(),
            }
        }
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

#[tokio::test]
async fn connect_failure_between_issue_and_revoke_keeps_the_old_credential_live()
-> TestResult {
    let custody = identity_contract::app_custody::serve().await?;
    let state = Proxy {
        upstream: custody.base,
        fail_bearer: Arc::default(),
        client_issued: Arc::default(),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let broker = format!("http://{}", listener.local_addr()?);
    let router = axum::Router::new()
        .fallback(proxy)
        .with_state(state.clone());
    let worker = tokio::spawn(async move { axum::serve(listener, router).await });
    let (service, _) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.secrets = Some(lys_identity_server::secrets_api::SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
        },
        |config| {
            Ok(lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "other"],
            )?)
        },
    )
    .await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    ok(post(
        &service,
        "/apps/connect",
        Auth::Cookie(&admin),
        &body(&op()?),
    )
    .await?)?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    let before = live(&app)?;
    state.client_issued.store(false, Ordering::SeqCst);
    state.fail_bearer.store(true, Ordering::SeqCst);
    let replacement = body(&op()?);
    let failed = post(
        &service,
        "/apps/connect",
        Auth::Cookie(&admin),
        &replacement,
    )
    .await?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    assert_ne!(failed.0, 200, "the injected failure was not reported");
    assert!(
        state.client_issued.load(Ordering::SeqCst),
        "failure did not follow a durable client issue"
    );
    assert!(
        live(&app)?.contains(&before[0]),
        "replacement failure revoked the previous credential"
    );
    let partial = live(&app)?;
    assert_eq!(partial.len(), 2);
    let replacement_id = partial
        .iter()
        .find(|id| !before.contains(id))
        .ok_or("replacement reference absent")?;
    state.client_issued.store(false, Ordering::SeqCst);
    ok(post(
        &service,
        "/apps/connect",
        Auth::Cookie(&admin),
        &replacement,
    )
    .await?)?;
    let app = ok(get(&service, &format!("/apps/{NOTES}"), Auth::Cookie(&admin)).await?)?;
    worker.abort();
    assert_eq!(live(&app)?, [replacement_id.clone()]);
    assert!(
        !state.client_issued.load(Ordering::SeqCst),
        "retry issued another client credential"
    );
    Ok(())
}
