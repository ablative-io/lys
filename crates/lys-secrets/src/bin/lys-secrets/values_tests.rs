#![cfg(test)]
//! A person adds a secret with its value and where it is used, replaces the
//! value and retires it, each through a trusted screen service under an
//! operation id. The value reaches the upstream it is routed to and nothing
//! else: no answer, no refusal, and no file the broker writes outside its
//! sealed entries carries it. A retired name is never used again.

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, Holder, IssuedHandle, OnBehalf, Presentation, Relation, ServiceKey, ServiceWindow,
    new_operation_id, request_digest, to_hex,
};
use serde_json::{Value, json};

use crate::files::{FileGrants, Layout, now_ms};
use crate::serve::Shared;
use crate::spice::Grants;

pub(crate) type Outcome<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const PERSON: &str = "person-fixture";
const OTHER: &str = "person-other";
const AGENT: &str = "agent-fixture";
const VALUE: &str = "sentinel-value-7d1e9a40c2b35f68";
const NEW_VALUE: &str = "sentinel-value-replaced-93ab0c71";
const OPERATION: &str = "op-0123456789abcdef0123456789abcdef";

/// A request the screen service `identity` signs for `person`.
pub(crate) fn vouched(
    key: &Ed25519Identity,
    (method, path): (&str, &str),
    person: &str,
    body: &[u8],
) -> Outcome<Request> {
    let proof = OnBehalf::sign(
        "identity",
        person,
        &new_operation_id()?,
        now_ms(),
        request_digest(method, path, body)?,
        key,
    )?;
    let [service, person, op, at, signature] = proof.to_wire();
    Ok(Request::builder()
        .method(method)
        .uri(path)
        .header("lys-service", service)
        .header("lys-on-behalf-of", person)
        .header("lys-operation", op)
        .header("lys-signed-at", at)
        .header("lys-service-signature", signature)
        .body(Body::from(body.to_vec()))?)
}

/// A broker in `dir` that trusts the screen service whose key it answers.
pub(crate) fn broker(dir: &Path) -> Outcome<(Arc<Shared>, Ed25519Identity)> {
    let layout = Layout::new(&dir.join("broker"), &dir.join("keys"));
    layout.prepare()?;
    let key = Ed25519Identity::load_or_generate(&dir.join("service.key"))?;
    layout.trust_service(ServiceKey {
        name: "identity".to_owned(),
        public_key: to_hex(&key.public_key_bytes()),
    })?;
    let grants = Grants::File(FileGrants::new(layout.grants()));
    let broker = Broker::create(&layout.paths(), grants.clone(), Box::new(now_ms))?;
    Ok((
        Arc::new(Shared {
            broker: Mutex::new(broker),
            layout,
            client: reqwest::Client::new(),
            window: Mutex::new(ServiceWindow::new()),
            permissions: Arc::new(grants),
        }),
        key,
    ))
}

/// A stand-in upstream that keeps every authorization header it is sent
/// and answers it back in its body. Answers its base address.
pub(crate) async fn upstream(seen: Arc<Mutex<Vec<String>>>) -> Outcome<String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}/api", listener.local_addr()?);
    let target = axum::Router::new().fallback(move |headers: HeaderMap| {
        let seen = Arc::clone(&seen);
        async move {
            let carried = headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_owned();
            seen.lock()
                .expect("test state lock poisoned")
                .push(carried.clone());
            format!("the upstream was sent {carried}")
        }
    });
    tokio::spawn(async move { axum::serve(listener, target).await });
    Ok(base)
}

/// Calls `secret` through the broker's proxy with `issued`, signed by
/// `key`; answers the status and the body.
pub(crate) async fn call(
    shared: &Arc<Shared>,
    (secret, issued): (&str, &IssuedHandle),
    key: &Ed25519Identity,
) -> Outcome<(StatusCode, String)> {
    let path = format!("/{secret}/v1/messages");
    let body = b"{\"model\":\"fixture\"}";
    let presentation = Presentation::sign(
        &issued.id,
        &new_operation_id()?,
        now_ms(),
        request_digest("POST", &path, body)?,
        key,
    )?;
    let [id, op, at, signature] = presentation.to_wire();
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("lys-handle", to_hex(issued.token.expose()))
        .header("lys-handle-id", id)
        .header("lys-operation", op)
        .header("lys-signed-at", at)
        .header("lys-presentation", signature)
        .body(Body::from(body.to_vec()))?;
    let response = match crate::serve::forward(shared, request).await {
        Ok(response) => response,
        Err((status, error)) => return Ok((status, error.to_string())),
    };
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

/// Every file under `dir` other than the sealed entries, none of which may
/// carry `value`.
pub(crate) fn never_written(dir: &Path, value: &str) -> Outcome {
    for entry in walk(dir)? {
        let bytes = std::fs::read(&entry)?;
        assert!(
            !bytes
                .windows(value.len())
                .any(|window| window == value.as_bytes()),
            "{} carries the value",
            entry.display()
        );
    }
    Ok(())
}

fn walk(dir: &Path) -> Outcome<Vec<std::path::PathBuf>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(walk(&path)?);
        } else {
            found.push(path);
        }
    }
    Ok(found)
}

type Answered = Result<axum::Json<Value>, (StatusCode, String)>;

fn answered(answer: Answered) -> Outcome<Value> {
    answer
        .map(|json| json.0)
        .map_err(|(status, words)| format!("{status} {words}").into())
}

#[tokio::test]
async fn a_secret_is_added_replaced_and_retired_and_its_value_goes_only_upstream() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (shared, key) = broker(dir.path())?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = upstream(Arc::clone(&seen)).await?;
    let add = |value: &str, upstream: &str| {
        serde_json::to_vec(&json!({
            "operation": OPERATION, "name": "fixture-token", "class": "credential",
            "upstream": upstream, "header": "authorization", "prefix": "Bearer ", "value": value,
        }))
    };
    let body = add(VALUE, &base)?;
    let first = answered(
        crate::values::add(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/add"), PERSON, &body)?,
        )
        .await,
    )?;
    assert!(!first.to_string().contains(VALUE));
    assert_eq!(first["owner"], PERSON);
    assert_eq!(first["sequence"], 1);
    assert_eq!(first["repeated"], false);
    // The same change sent again is answered, not applied twice.
    let again = answered(
        crate::values::add(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/add"), PERSON, &body)?,
        )
        .await,
    )?;
    assert_eq!(again["repeated"], true);
    // The same operation with another route is refused by name.
    let other = add(VALUE, "https://elsewhere.example")?;
    let reused = crate::values::add(
        State(Arc::clone(&shared)),
        vouched(&key, ("POST", "/_lys/add"), PERSON, &other)?,
    )
    .await
    .err()
    .ok_or("a reused operation id was applied")?;
    assert!(reused.1.starts_with("OperationReused"), "{}", reused.1);
    // A body that does not read is refused in fixed words, never quoting it.
    let mut quoted = serde_json::Map::new();
    quoted.insert(VALUE.to_owned(), Value::from(VALUE));
    let unread = serde_json::to_vec(&quoted)?;
    let refused = crate::values::add(
        State(Arc::clone(&shared)),
        vouched(&key, ("POST", "/_lys/add"), PERSON, &unread)?,
    )
    .await
    .err()
    .ok_or("an unreadable body was taken")?;
    assert_eq!(refused.0, StatusCode::BAD_REQUEST);
    assert!(!refused.1.contains(VALUE));

    FileGrants::new(shared.layout.grants()).set(
        Relation::Use,
        AGENT,
        "fixture-token",
        Some(PERSON),
    )?;
    let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let holder = Holder {
        identity: AGENT.to_owned(),
        key: agent.public_key_bytes(),
    };
    let issued = shared
        .broker
        .lock()
        .expect("test state lock poisoned")
        .issue(&holder, "fixture-token", 10, now_ms() + 600_000)?;
    let (status, text) = call(&shared, ("fixture-token", &issued), &agent).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !text.contains(VALUE),
        "the answer carried the value: {text}"
    );
    assert_eq!(
        seen.lock().expect("test state lock poisoned").last(),
        Some(&format!("Bearer {VALUE}"))
    );

    let replace = serde_json::to_vec(&json!({
        "operation": "op-replace-0123456789abcdef", "secret": "fixture-token", "value": NEW_VALUE,
    }))?;
    let stranger = crate::values::replace(
        State(Arc::clone(&shared)),
        vouched(&key, ("POST", "/_lys/replace"), OTHER, &replace)?,
    )
    .await
    .err()
    .ok_or("someone who does not own the secret replaced its value")?;
    assert_eq!(stranger.0, StatusCode::FORBIDDEN);
    let replaced = answered(
        crate::values::replace(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/replace"), PERSON, &replace)?,
        )
        .await,
    )?;
    assert_eq!(replaced["sequence"], 2);
    assert!(!replaced.to_string().contains(NEW_VALUE));
    let (_, text) = call(&shared, ("fixture-token", &issued), &agent).await?;
    assert!(!text.contains(NEW_VALUE));
    assert_eq!(
        seen.lock().expect("test state lock poisoned").last(),
        Some(&format!("Bearer {NEW_VALUE}"))
    );

    let retire = serde_json::to_vec(&json!({
        "operation": "op-retire-0123456789abcdef", "secret": "fixture-token",
    }))?;
    let gone = answered(
        crate::values::retire(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/retire"), PERSON, &retire)?,
        )
        .await,
    )?;
    assert!(gone["retired_at"].as_i64().is_some_and(|at| at > 0));
    assert!(!shared.layout.routes()?.contains_key("fixture-token"));
    let (status, _) = call(&shared, ("fixture-token", &issued), &agent).await?;
    assert_ne!(status, StatusCode::OK, "a retired secret's handle was used");
    // The name is never used again.
    let reborn = serde_json::to_vec(&json!({
        "operation": "op-reborn-0123456789abcdef", "name": "fixture-token", "class": "key",
        "upstream": base, "header": "authorization", "prefix": "", "value": VALUE,
    }))?;
    let refused = crate::values::add(
        State(Arc::clone(&shared)),
        vouched(&key, ("POST", "/_lys/add"), PERSON, &reborn)?,
    )
    .await
    .err()
    .ok_or("a retired name was used again")?;
    assert_eq!(refused.0, StatusCode::CONFLICT);
    assert!(refused.1.starts_with("SecretRetired"), "{}", refused.1);

    // No file the broker wrote outside its sealed entries carries either
    // value; the sealed entries themselves are ciphertext.
    never_written(dir.path(), VALUE)?;
    never_written(dir.path(), NEW_VALUE)?;
    Ok(())
}

#[tokio::test]
async fn a_value_is_given_only_by_a_person_through_a_trusted_screen_service() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (shared, _key) = broker(dir.path())?;
    let stranger = Ed25519Identity::load_or_generate(&dir.path().join("stranger.key"))?;
    let body = serde_json::to_vec(&json!({
        "operation": OPERATION, "name": "fixture-token", "class": "credential",
        "upstream": "https://upstream.example", "header": "authorization", "prefix": "", "value": VALUE,
    }))?;
    let refused = crate::values::add(
        State(Arc::clone(&shared)),
        vouched(&stranger, ("POST", "/_lys/add"), PERSON, &body)?,
    )
    .await
    .err()
    .ok_or("an untrusted service added a secret")?;
    assert!(!refused.1.contains(VALUE));
    assert_eq!(
        shared
            .broker
            .lock()
            .expect("test state lock poisoned")
            .store()
            .entries()
            .count(),
        0
    );
    never_written(dir.path(), VALUE)?;
    Ok(())
}
