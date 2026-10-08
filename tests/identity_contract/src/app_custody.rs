//! A named custody stand-in for app contract tests. Real broker sealing and
//! signed requests are covered by the broker and secrets API test suites.
//!
//! The stand-in also holds apps' virtual client credentials (DIRECTORY-081)
//! as the broker does: it issues each once, keeps only its digest, confirms
//! one only while the apps' record sends it as live, ends one the record no
//! longer holds live when it meets it, and can be taken down and brought
//! back, refusing every request while down.
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, http::HeaderMap};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;

/// Fixed fixture bytes, never used by a production service.
pub fn secret() -> String {
    "ab".repeat(32)
}
/// The app credential held by this stand-in, not returned by approval.
pub fn credential(app: &str) -> String {
    format!("lys-app.{app}.{}", secret())
}

/// One client credential the stand-in issued.
#[derive(Clone, Debug)]
struct Held {
    app: String,
    credential_id: String,
    digest: String,
    ended: bool,
}

#[derive(Default)]
struct Kept {
    down: bool,
    /// Apps whose sealed client secret the stand-in no longer holds.
    lost: Vec<String>,
    issued: Vec<Held>,
    current: BTreeMap<String, String>,
    bearers: BTreeMap<String, (String, String)>,
}

/// A running stand-in: its address, and its credentials as it holds them.
#[derive(Clone)]
pub struct Custody {
    /// The address the service is configured with.
    pub base: String,
    kept: Arc<Mutex<Kept>>,
}

impl Custody {
    /// What the stand-in holds; a lock poisoned by a panicking test is
    /// refused by name, never recovered.
    fn kept(&self) -> std::io::Result<std::sync::MutexGuard<'_, Kept>> {
        self.kept
            .lock()
            .map_err(|error| std::io::Error::other(format!("fixture_lock_poisoned: {error}")))
    }

    /// Take the stand-in down: every request is refused until `up`.
    ///
    /// # Errors
    /// Returns `fixture_lock_poisoned` when the stand-in's lock is poisoned.
    pub fn down(&self) -> std::io::Result<()> {
        self.kept()?.down = true;
        Ok(())
    }

    /// Bring the stand-in back.
    ///
    /// # Errors
    /// Returns `fixture_lock_poisoned` when the stand-in's lock is poisoned.
    pub fn up(&self) -> std::io::Result<()> {
        self.kept()?.down = false;
        Ok(())
    }

    /// The stand-in no longer holds `app`'s sealed client secret, as a
    /// broker whose store was lost: issuing to it is refused in the broker's
    /// words.
    ///
    /// # Errors
    /// Returns `fixture_lock_poisoned` when the stand-in's lock is poisoned.
    pub fn lose(&self, app: &str) -> std::io::Result<()> {
        self.kept()?.lost.push(app.to_owned());
        Ok(())
    }

    /// The ids of `app`'s credentials the stand-in has ended.
    ///
    /// # Errors
    /// Returns `fixture_lock_poisoned` when the stand-in's lock is poisoned.
    pub fn ended(&self, app: &str) -> std::io::Result<Vec<String>> {
        Ok(self
            .kept()?
            .issued
            .iter()
            .filter(|held| held.app == app && held.ended)
            .map(|held| held.credential_id.clone())
            .collect())
    }
}

/// Start this test's isolated broker endpoint.
///
/// # Errors
/// Returns listener binding errors.
pub async fn start() -> Result<String, Box<dyn Error>> {
    Ok(serve().await?.base)
}

/// Start this test's isolated broker endpoint, answering its handle.
///
/// # Errors
/// Returns listener binding errors.
pub async fn serve() -> Result<Custody, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let custody = Custody {
        base: format!("http://{}", listener.local_addr()?),
        kept: Arc::default(),
    };
    let router = axum::Router::new()
        .route("/_lys/apps/prepare", axum::routing::post(prepare))
        .route("/_lys/apps/bearer/issue", axum::routing::post(issue_bearer))
        .route("/_lys/apps/client", axum::routing::post(authenticate))
        .route("/_lys/apps/client/issue", axum::routing::post(issue))
        .route("/_lys/apps/client/end", axum::routing::post(end))
        .with_state(custody.clone());
    tokio::spawn(async move { axum::serve(listener, router).await });
    Ok(custody)
}

fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn owner(headers: &HeaderMap) -> String {
    headers
        .get("lys-on-behalf-of")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing")
        .to_owned()
}

fn refused(status: StatusCode, words: &str) -> Response {
    (status, format!("{words}\n")).into_response()
}

fn down() -> Response {
    refused(
        StatusCode::SERVICE_UNAVAILABLE,
        "Unavailable: the custody stand-in is down",
    )
}

/// The stand-in's lock was poisoned: every request is refused by that name.
fn poisoned(error: &std::io::Error) -> Response {
    refused(StatusCode::SERVICE_UNAVAILABLE, &error.to_string())
}

/// What the stand-in holds, or the refusal its poisoned lock answers.
macro_rules! kept {
    ($custody:expr) => {
        match $custody.kept() {
            Ok(kept) => kept,
            Err(error) => return poisoned(&error),
        }
    };
}

async fn prepare(
    State(custody): State<Custody>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if kept!(custody).down {
        return down();
    }
    let owner = owner(&headers);
    let app = body["app"].as_str().unwrap_or("missing");
    kept!(custody).lost.retain(|lost| lost != app);
    let prefix = format!("lys-app-{owner}-{app}");
    Json(
        json!({"app": app, "client_secret_ref": format!("{prefix}-client"),
        "api_credential_ref": format!("{prefix}-api"),
        "client_secret_sha256": digest(&secret())}),
    )
    .into_response()
}

async fn issue_bearer(
    State(custody): State<Custody>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let mut kept = kept!(custody);
    if kept.down { return down(); }
    let Some(app) = body["app"].as_str() else { return refused(StatusCode::BAD_REQUEST, "RequestMalformed"); };
    let Some(operation) = body["operation"].as_str() else { return refused(StatusCode::BAD_REQUEST, "RequestMalformed"); };
    let repeated = kept.bearers.get(operation).cloned();
    let secret = match &repeated {
        Some((held_app, secret)) if held_app == app => secret.clone(),
        Some(_) => return refused(StatusCode::CONFLICT, "OperationReused"),
        None => digest(operation),
    };
    let current = kept.current.get(app).cloned().unwrap_or_else(|| digest(&self::secret()));
    let next = digest(&secret);
    if body["expected_digest"].as_str() != Some(current.as_str()) && current != next {
        return refused(StatusCode::FORBIDDEN, "AppClientCustodyMismatch");
    }
    kept.current.insert(app.to_owned(), next.clone());
    kept.bearers.insert(operation.to_owned(), (app.to_owned(), secret.clone()));
    let owner = owner(&headers);
    Json(json!({"app": app, "owner": owner, "client_secret_ref": format!("lys-app-{owner}-{app}-client"),
        "api_credential_ref": format!("lys-app-{owner}-{app}-api-{operation}"), "client_secret_sha256": next,
        "credential": if repeated.is_some() { None } else { Some(format!("lys-app.{app}.{secret}")) }})).into_response()
}

async fn issue(
    State(custody): State<Custody>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let mut kept = kept!(custody);
    if kept.down {
        return down();
    }
    let app = body["app"].as_str().unwrap_or("missing").to_owned();
    if kept.lost.contains(&app) {
        return refused(
            StatusCode::FORBIDDEN,
            &format!(
                "AppClientNoCustody: no client secret is sealed for the app {app} (act: approve the app, which seals its client secret, before issuing it a credential)"
            ),
        );
    }
    let credential_id = format!("{:016x}", kept.issued.len() + 1);
    let value = format!(
        "lys-client.{app}.{}",
        digest(&format!("{app}/{credential_id}"))
    );
    kept.issued.push(Held {
        app: app.clone(),
        credential_id: credential_id.clone(),
        digest: digest(&value),
        ended: false,
    });
    let current = kept.current.get(&app).cloned().unwrap_or_else(|| digest(&secret()));
    Json(json!({"app": app, "credential_id": credential_id,
        "owner": owner(&headers), "value": value, "client_secret_sha256": current}))
    .into_response()
}

async fn authenticate(State(custody): State<Custody>, Json(body): Json<Value>) -> Response {
    let mut kept = kept!(custody);
    if kept.down {
        return down();
    }
    let app = body["app"].as_str().unwrap_or("missing");
    let current = kept.current.get(app).cloned().unwrap_or_else(|| digest(&secret()));
    if body["secret_sha256"].as_str() != Some(current.as_str()) {
        return refused(
            StatusCode::FORBIDDEN,
            "AppClientCustodyMismatch: the sealed client secret is not the approved one",
        );
    }
    let presented = digest(body["presented"].as_str().unwrap_or_default());
    let live: Vec<&str> = body["live"]
        .as_array()
        .map(|live| live.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let credential_refused = || {
        refused(
            StatusCode::FORBIDDEN,
            "AppClientCredentialRefused: no live client credential of this app holds that value",
        )
    };
    let Some(held) = kept
        .issued
        .iter_mut()
        .find(|held| held.app == app && held.digest == presented && !held.ended)
    else {
        return credential_refused();
    };
    if !live.contains(&held.credential_id.as_str()) {
        held.ended = true;
        return credential_refused();
    }
    Json(json!({"app": app, "credential_id": held.credential_id})).into_response()
}

async fn end(State(custody): State<Custody>, Json(body): Json<Value>) -> Response {
    let mut kept = kept!(custody);
    if kept.down {
        return down();
    }
    let app = body["app"].as_str().unwrap_or("missing").to_owned();
    let asked: Vec<&str> = body["credential_ids"]
        .as_array()
        .map(|ids| ids.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut ended = Vec::new();
    for held in &mut kept.issued {
        if held.app == app && !held.ended && asked.contains(&held.credential_id.as_str()) {
            held.ended = true;
            ended.push(held.credential_id.clone());
        }
    }
    Json(json!({"app": app, "ended": ended})).into_response()
}
