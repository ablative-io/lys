#![cfg(test)]
//! The signed broker route seals both values, never returns them, and refuses tampering.
use crate::{
    files::{FileGrants, Layout, now_ms},
    serve::Shared,
    spice::Grants,
};
use axum::{
    body::Body,
    extract::{Request, State},
};
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, Holder, OnBehalf, Presentation, Relation, ServiceKey, ServiceWindow, new_operation_id,
    request_digest, to_hex,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

type Outcome = Result<(), Box<dyn std::error::Error>>;
const PATH: &str = "/_lys/apps/save";
fn signed(key: &Ed25519Identity, body: &[u8]) -> Result<Request, Box<dyn std::error::Error>> {
    signed_path(key, body, PATH)
}
fn signed_path(
    key: &Ed25519Identity,
    body: &[u8],
    path: &str,
) -> Result<Request, Box<dyn std::error::Error>> {
    let proof = OnBehalf::sign(
        "identity",
        "person-fixture",
        &new_operation_id()?,
        now_ms(),
        request_digest("POST", path, body)?,
        key,
    )?;
    let [service, person, op, at, signature] = proof.to_wire();
    Ok(Request::builder()
        .method("POST")
        .uri(path)
        .header("lys-service", service)
        .header("lys-on-behalf-of", person)
        .header("lys-operation", op)
        .header("lys-signed-at", at)
        .header("lys-service-signature", signature)
        .body(Body::from(body.to_vec()))?)
}
#[tokio::test]
async fn signed_save_is_private_repeatable_and_bound_to_body() -> Outcome {
    let dir = tempfile::tempdir()?;
    let layout = Layout::new(&dir.path().join("broker"), &dir.path().join("keys"));
    layout.prepare()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
    layout.trust_service(ServiceKey {
        name: "identity".to_owned(),
        public_key: to_hex(&key.public_key_bytes()),
    })?;
    let grants = Grants::File(FileGrants::new(layout.grants()));
    let broker = Broker::create(&layout.paths(), grants.clone(), Box::new(now_ms))?;
    let shared = Arc::new(Shared {
        broker: Mutex::new(broker),
        layout,
        client: reqwest::Client::new(),
        window: Mutex::new(ServiceWindow::new()),
        permissions: Arc::new(grants),
    });
    let secret = "ab".repeat(32);
    let expected = format!("Bearer lys-app.fixture_app.{secret}");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let upstream = format!("http://{}/api", listener.local_addr()?);
    let target = axum::Router::new().route(
        "/api/apps/me",
        axum::routing::get(move |headers: axum::http::HeaderMap| async move {
            if headers.get("authorization").and_then(|h| h.to_str().ok()) == Some(expected.as_str())
            {
                "authorised fixture"
            } else {
                "wrong credential"
            }
        }),
    );
    let serving = tokio::spawn(async move { axum::serve(listener, target).await });
    let body = serde_json::to_vec(
        &json!({"app":"fixture_app","client_secret":secret,"upstream":upstream}),
    )?;
    let answer = crate::save_app::save(State(Arc::clone(&shared)), signed(&key, &body)?)
        .await
        .map_err(|e| format!("{} {}", e.0, e.1))?
        .0;
    assert!(!answer.to_string().contains(&secret));
    let again = crate::save_app::save(State(Arc::clone(&shared)), signed(&key, &body)?)
        .await
        .map_err(|e| format!("{} {}", e.0, e.1))?
        .0;
    assert_eq!(answer, again);
    {
        let broker = shared.broker.lock().expect("test state lock poisoned");
        assert_eq!(broker.store().entries().count(), 2);
        for entry in broker.store().entries() {
            assert_eq!(entry.owner, "person-fixture");
            assert_eq!(entry.sequence, 1);
        }
    }
    let mut forged = signed(&key, &body)?;
    *forged.body_mut() = Body::from("{}");
    assert!(
        crate::save_app::save(State(Arc::clone(&shared)), forged)
            .await
            .is_err()
    );
    let remote = serde_json::to_vec(
        &json!({"app":"fixture_app","client_secret":secret,"upstream":"https://outside.example"}),
    )?;
    assert!(
        crate::save_app::save(State(Arc::clone(&shared)), signed(&key, &remote)?)
            .await
            .is_err()
    );
    let routes = shared.layout.routes()?;
    assert_eq!(
        routes
            .get(answer["api_credential_ref"].as_str().ok_or("no ref")?)
            .ok_or("no route")?
            .upstream,
        upstream
    );
    let reference = answer["api_credential_ref"]
        .as_str()
        .ok_or("no credential reference")?;
    let holder_key = Ed25519Identity::load_or_generate(&dir.path().join("holder.key"))?;
    let holder = Holder {
        identity: "agent-fixture".to_owned(),
        key: holder_key.public_key_bytes(),
    };
    // Saving grants no use authority. The holder must receive an explicit grant.
    assert!(
        shared
            .broker
            .lock()
            .expect("test state lock poisoned")
            .issue(&holder, reference, 2, now_ms() + 60_000)
            .is_err()
    );
    FileGrants::new(shared.layout.grants()).set(
        Relation::Use,
        "agent-fixture",
        reference,
        Some("person-fixture"),
    )?;
    let issued = shared
        .broker
        .lock()
        .expect("test state lock poisoned")
        .issue(&holder, reference, 2, now_ms() + 60_000)?;
    let path = format!("/{reference}/apps/me");
    let presentation = Presentation::sign(
        &issued.id,
        &new_operation_id()?,
        now_ms(),
        request_digest("GET", &path, b"")?,
        &holder_key,
    )?;
    let [id, op, at, signature] = presentation.to_wire();
    let request = Request::builder()
        .uri(path)
        .header("lys-handle", to_hex(issued.token.expose()))
        .header("lys-handle-id", id)
        .header("lys-operation", op)
        .header("lys-signed-at", at)
        .header("lys-presentation", signature)
        .body(Body::empty())?;
    let response = crate::serve::forward(&shared, request)
        .await
        .map_err(|e| format!("{} {}", e.0, e.1))?;
    let received = axum::body::to_bytes(response.into_body(), 1024).await?;
    assert_eq!(received.as_ref(), b"authorised fixture");
    serving.abort();
    Ok(())
}

#[tokio::test]
async fn prepare_reconciles_after_reopen_and_never_returns_plaintext() -> Outcome {
    let dir = tempfile::tempdir()?;
    let layout = Layout::new(&dir.path().join("broker"), &dir.path().join("keys"));
    layout.prepare()?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
    layout.trust_service(ServiceKey {
        name: "identity".to_owned(),
        public_key: to_hex(&key.public_key_bytes()),
    })?;
    let grants = Grants::File(FileGrants::new(layout.grants()));
    let broker = Broker::create(&layout.paths(), grants.clone(), Box::new(now_ms))?;
    let shared = Arc::new(Shared {
        broker: Mutex::new(broker),
        layout: layout.clone(),
        client: reqwest::Client::new(),
        window: Mutex::new(ServiceWindow::new()),
        permissions: Arc::new(grants.clone()),
    });
    let path = "/_lys/apps/prepare";
    let body =
        serde_json::to_vec(&json!({"app":"prepared_app","upstream":"http://127.0.0.1:8491/api"}))?;
    let first = crate::save_app::save(State(Arc::clone(&shared)), signed_path(&key, &body, path)?)
        .await
        .map_err(|e| format!("{} {}", e.0, e.1))?
        .0;
    assert_eq!(first.as_object().ok_or("not object")?.len(), 4);
    assert!(first.get("client_secret").is_none());
    assert!(first.get("credential").is_none());
    assert_eq!(
        first["client_secret_sha256"]
            .as_str()
            .ok_or("no digest")?
            .len(),
        64
    );
    drop(shared);
    let broker = Broker::open(&layout.paths(), grants.clone(), Box::new(now_ms))?;
    let shared = Arc::new(Shared {
        broker: Mutex::new(broker),
        layout,
        client: reqwest::Client::new(),
        window: Mutex::new(ServiceWindow::new()),
        permissions: Arc::new(grants),
    });
    let again = crate::save_app::save(State(Arc::clone(&shared)), signed_path(&key, &body, path)?)
        .await
        .map_err(|e| format!("{} {}", e.0, e.1))?
        .0;
    assert_eq!(first, again);
    assert_eq!(
        shared
            .broker
            .lock()
            .expect("test state lock poisoned")
            .store()
            .entries()
            .count(),
        2
    );
    assert!(
        shared
            .layout
            .routes()?
            .contains_key(again["api_credential_ref"].as_str().ok_or("no api ref")?)
    );
    let forged = serde_json::to_vec(
        &json!({"app":"prepared_app","upstream":"http://127.0.0.1:8491/api","client_secret":"ab".repeat(32)}),
    )?;
    assert!(
        crate::save_app::save(State(shared), signed_path(&key, &forged, path)?)
            .await
            .is_err()
    );
    Ok(())
}
