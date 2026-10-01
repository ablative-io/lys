#![cfg(test)]
//! Counting gates on the proxy's request path (SECRETS-005 R3, R4). A
//! permission check held open by the test holds up its own call and no
//! screen route; a hundred proxied requests read `routes.json` no time after
//! start; and a route another process adds is served on the next request.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, Denied, Holder, IssuedHandle, PermissionCheck, Permitted, Presentation, Relation,
    Secret, SecretsError, ServiceWindow, new_operation_id, request_digest, to_hex,
};
use tempfile::TempDir;

use crate::files::{FileGrants, Layout, Route, now_ms};
use crate::serve::{Permissions, Shared, forward};
use crate::spice::Grants;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SECRET: &str = "token";
const HOLDER: &str = "agent:noor";
const PERSON: &str = "person:tom";

/// A proxy's shared state over a broker made in a temporary directory,
/// holding one sealed secret routed to a port nothing listens on and one
/// handle on it.
struct Served {
    dir: TempDir,
    root: PathBuf,
    keys: PathBuf,
    shared: Arc<Shared>,
    agent: Ed25519Identity,
    issued: IssuedHandle,
}

fn route() -> Route {
    Route {
        upstream: "http://127.0.0.1:9".to_owned(),
        header: "authorization".to_owned(),
        prefix: "Bearer ".to_owned(),
        spend_header: None,
    }
}

fn served(permissions: Permissions) -> TestResult<Served> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("broker");
    let keys = dir.path().join("keys");
    let layout = Layout::new(&root, &keys);
    layout.prepare()?;
    FileGrants::new(layout.grants()).set(Relation::Use, HOLDER, SECRET, Some(PERSON))?;
    let grants = Grants::File(FileGrants::new(layout.grants()));
    let mut broker = Broker::create(&layout.paths(), grants, Box::new(now_ms))?;
    broker.seal(SECRET, PERSON, &Secret::from_slice(b"test value"))?;
    let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let holder = Holder {
        identity: HOLDER.to_owned(),
        key: agent.public_key_bytes(),
    };
    let issued = broker.issue(&holder, SECRET, 1000, now_ms() + 600_000)?;
    layout.add_route(SECRET, route())?;
    let shared = Arc::new(Shared {
        broker: Mutex::new(broker),
        layout,
        client: reqwest::Client::new(),
        window: Mutex::new(ServiceWindow::new()),
        permissions,
    });
    Ok(Served {
        dir,
        root,
        keys,
        shared,
        agent,
        issued,
    })
}

/// A request for `path` carrying the handle and a presentation signed for
/// that very request.
fn signed(served: &Served, path: &str) -> TestResult<Request> {
    let digest = request_digest("GET", path, b"")?;
    let presentation = Presentation::sign(
        &served.issued.id,
        &new_operation_id()?,
        now_ms(),
        digest,
        &served.agent,
    )?;
    let [handle_id, operation, signed_at, signature] = presentation.to_wire();
    Ok(axum::http::Request::builder()
        .method("GET")
        .uri(path)
        .header("lys-handle", to_hex(served.issued.token.expose()))
        .header("lys-handle-id", handle_id)
        .header("lys-operation", operation)
        .header("lys-signed-at", signed_at)
        .header("lys-presentation", signature)
        .body(Body::empty())?)
}

/// A request for `path` carrying no handle.
fn unsigned(path: &str) -> TestResult<Request> {
    Ok(axum::http::Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())?)
}

fn status(answered: Result<Response, (StatusCode, SecretsError)>) -> StatusCode {
    match answered {
        Ok(response) => response.status(),
        Err((status, _)) => status,
    }
}

/// Whether the held check has been entered, and whether it was let go.
#[derive(Debug, Default)]
struct Held {
    entered: bool,
    released: bool,
}

/// A permission source whose every `use` check is held open until the test
/// lets it go, and that permits it then.
#[derive(Debug, Default)]
struct Gate {
    held: Mutex<Held>,
    changed: Condvar,
}

impl Gate {
    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held.lock().expect("test state lock poisoned")
    }

    fn wait_entered(&self) {
        let mut held = self.held();
        while !held.entered {
            held = self.changed.wait(held).expect("test state lock poisoned");
        }
    }

    fn release(&self) {
        self.held().released = true;
        self.changed.notify_all();
    }

    fn released(&self) -> bool {
        self.held().released
    }
}

fn denied() -> Denied {
    Denied {
        reason: "the gate grants nothing but use".to_owned(),
        no_person_root: false,
    }
}

impl PermissionCheck for Gate {
    fn may_use(&self, _: &str, _: &str) -> Result<Permitted, Denied> {
        let mut held = self.held();
        held.entered = true;
        self.changed.notify_all();
        while !held.released {
            held = self.changed.wait(held).expect("test state lock poisoned");
        }
        Ok(Permitted {
            person: PERSON.to_owned(),
            ends_at_ms: None,
        })
    }

    fn may_read(&self, _: &str, _: &str) -> Result<Permitted, Denied> {
        Err(denied())
    }

    fn may_lend(&self, _: &str, _: &str) -> Result<Permitted, Denied> {
        Err(denied())
    }

    fn member_of(&self, _: &str, _: &str) -> Result<Permitted, Denied> {
        Err(denied())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_screen_route_answers_while_a_permission_check_is_held_open() -> TestResult {
    let gate = Arc::new(Gate::default());
    let held: Arc<Gate> = Arc::clone(&gate);
    let permissions: Permissions = held;
    let served = served(permissions)?;
    let proxied = {
        let shared = Arc::clone(&served.shared);
        let request = signed(&served, "/token/v1/models")?;
        tokio::spawn(async move { status(forward(&shared, request).await) })
    };
    let entered = Arc::clone(&gate);
    let waited = tokio::task::spawn_blocking(move || entered.wait_entered());
    waited.await?;

    let screen = crate::view::audit(
        State(Arc::clone(&served.shared)),
        signed(&served, "/_lys/audit")?,
    )
    .await;
    assert!(
        !gate.released(),
        "the screen route answered while the permission check was still held open"
    );
    let lines = screen.map_err(|(status, text)| format!("{status}: {text}"))?;
    assert!(lines.0["size"].as_u64().is_some_and(|size| size > 0));

    gate.release();
    assert_eq!(
        proxied.await?,
        StatusCode::BAD_GATEWAY,
        "the held call was admitted once let go, and its upstream does not listen"
    );
    served.dir.close()?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hundred_proxied_requests_read_routes_json_no_time_after_start() -> TestResult {
    let file: Permissions = Arc::new(FileGrants::new(PathBuf::from("no-grants.json")));
    let served = served(file)?;
    let shared = &served.shared;
    shared.layout.routes()?;
    let reads = || shared.layout.routes.reads.load(Ordering::Relaxed);
    let at_start = reads();
    assert_eq!(at_start, 1);
    for _ in 0..100 {
        assert_eq!(
            status(forward(shared, unsigned("/token/v1/models")?).await),
            StatusCode::BAD_REQUEST,
            "the route was found and the request refused for carrying no handle"
        );
    }
    assert_eq!(
        reads() - at_start,
        0,
        "a hundred proxied requests read routes.json no time after start"
    );

    assert_eq!(
        status(forward(shared, unsigned("/added/v1")?).await),
        StatusCode::NOT_FOUND
    );
    Layout::new(&served.root, &served.keys).add_route("added", route())?;
    assert_eq!(
        status(forward(shared, unsigned("/added/v1")?).await),
        StatusCode::BAD_REQUEST,
        "a route another process added is served on the next request"
    );
    assert_eq!(reads() - at_start, 1);
    served.dir.close()?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signing_rpc_returns_only_cose_and_names_a_revoked_lease() -> TestResult {
    let gate = Arc::new(Gate::default());
    gate.release();
    let served = served(gate)?;
    let issued = {
        let mut broker = served.shared.broker.lock().expect("fixture broker");
        broker.seal_once(
            "signing",
            lys_secrets::EntryClass::Key,
            PERSON,
            &Secret::from_slice(&[9; 32]),
        )?;
        FileGrants::new(served.shared.layout.grants()).set(
            Relation::Use,
            HOLDER,
            "signing",
            Some(PERSON),
        )?;
        broker.issue(
            &Holder {
                identity: HOLDER.to_owned(),
                key: served.agent.public_key_bytes(),
            },
            "signing",
            2,
            now_ms() + 600_000,
        )?
    };
    let payload = b"exact payload";
    let request = || -> TestResult<Request> {
        let presentation = Presentation::sign(
            &issued.id,
            &new_operation_id()?,
            now_ms(),
            request_digest("SIGN", "signing", payload)?,
            &served.agent,
        )?;
        let [id, operation, time, signature] = presentation.to_wire();
        Ok(axum::http::Request::builder()
            .method("POST")
            .uri("/_lys/sign/signing")
            .header("lys-handle", to_hex(issued.token.expose()))
            .header("lys-handle-id", id)
            .header("lys-operation", operation)
            .header("lys-signed-at", time)
            .header("lys-presentation", signature)
            .body(Body::from(payload.to_vec()))?)
    };
    let response = crate::signing::sign(
        State(Arc::clone(&served.shared)),
        axum::extract::Path("signing".to_owned()),
        request()?,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1024).await?;
    let seed = zeroize::Zeroizing::new([9; 32]);
    let key = Ed25519Identity::from_seed(&seed);
    lys_core::attestation::verify_attestation_bytes_by_signer(
        &bytes,
        payload,
        &key.public_key_bytes(),
    )?;
    assert!(!bytes.windows(32).any(|window| window == seed.as_slice()));
    served
        .shared
        .broker
        .lock()
        .expect("fixture broker")
        .drop_handle(&issued.id)?;
    let response = crate::signing::sign(
        State(Arc::clone(&served.shared)),
        axum::extract::Path("signing".to_owned()),
        request()?,
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let bytes = axum::body::to_bytes(response.into_body(), 1024).await?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(value["refusal"], "HandleDropped");
    Ok(())
}
