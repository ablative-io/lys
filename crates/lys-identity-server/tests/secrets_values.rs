//! Adding a secret, changing its value and retiring it, against a stand-in broker that records what
//! reaches it. A secret's value travels to the broker in the body exactly as
//! the browser sent it, unread, signed for the signed-in person; a broker
//! refusal passes through by name; and the value appears in no answer,
//! no line the service says and no file it keeps.

use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::secrets_api::SecretsSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const VALUE: &str = "sentinel-identity-value-41c0a9e7d25b";

/// One request as the stand-in broker received it.
#[derive(Clone, Debug)]
struct Received {
    path: String,
    on_behalf_of: String,
    body: Vec<u8>,
}

type Log = Arc<Mutex<Vec<Received>>>;

/// The stand-in broker's refusal, as the broker writes one.
fn refused(status: StatusCode, name: &str) -> Response {
    (
        status,
        format!("{name}: the stand-in broker refuses this\n"),
    )
        .into_response()
}

async fn broker(State(log): State<Log>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let Ok(body) = axum::body::to_bytes(body, 1 << 20).await else {
        return (StatusCode::BAD_REQUEST, "unreadable body").into_response();
    };
    let on_behalf_of = parts
        .headers
        .get("lys-on-behalf-of")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let path = parts.uri.path().to_owned();
    log.lock().expect("fixture lock poisoned").push(Received {
        path: path.clone(),
        on_behalf_of: on_behalf_of.clone(),
        body: body.to_vec(),
    });
    let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let named = |member: &str| asked[member].as_str().unwrap_or_default().to_owned();
    match path.as_str() {
        "/_lys/add" => match named("name").as_str() {
            "taken" => refused(StatusCode::CONFLICT, "SecretExists"),
            "retired" => refused(StatusCode::CONFLICT, "SecretRetired"),
            "reused" => refused(StatusCode::BAD_REQUEST, "OperationReused"),
            "nowhere" => refused(StatusCode::BAD_REQUEST, "RouteInvalid"),
            "empty" => refused(StatusCode::BAD_REQUEST, "ValueEmpty"),
            name => axum::Json(json!({
                "secret": name, "class": "credential", "owner": on_behalf_of,
                "sequence": 1, "operation": named("operation"), "repeated": false,
            }))
            .into_response(),
        },
        "/_lys/replace" => match named("secret").as_str() {
            "not-mine" => refused(StatusCode::FORBIDDEN, "LendingNotPermitted"),
            secret => axum::Json(json!({
                "secret": secret, "sequence": 2, "operation": named("operation"), "repeated": false,
            }))
            .into_response(),
        },
        "/_lys/retire" => axum::Json(json!({
            "secret": named("secret"), "retired_at": 1, "operation": named("operation"),
            "repeated": false,
        }))
        .into_response(),
        "/_lys/model/register" => match named("name").as_str() {
            "Taken" => refused(StatusCode::CONFLICT, "ModelAccountNameTaken"),
            name => axum::Json(json!({
                "account": format!("model-account-{}", named("operation")), "name": name,
                "harness": named("harness"), "registered_by": on_behalf_of,
                "registered_at": 1, "operation": named("operation"), "repeated": false,
            }))
            .into_response(),
        },
        "/_lys/model/retire" => match named("account").as_str() {
            "unknown" => refused(StatusCode::NOT_FOUND, "ModelAccountUnknown"),
            account => axum::Json(json!({
                "account": account, "retired_at": 1, "operation": named("operation"),
                "repeated": false,
            }))
            .into_response(),
        },
        "/_lys/model/accounts" => axum::Json(json!({ "accounts": [] })).into_response(),
        _ => axum::Json(json!({ "path": path })).into_response(),
    }
}

struct Setup {
    service: Service,
    log: Log,
    said: Arc<Mutex<Vec<String>>>,
    person: String,
}

async fn setup(subject: &str) -> Result<Setup, Box<dyn Error>> {
    let log: Log = Arc::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = Router::new().fallback(broker).with_state(Arc::clone(&log));
    tokio::spawn(async move { axum::serve(listener, app).await });
    let keys = tempfile::TempDir::new()?;
    let key_file = keys.path().join("secrets-service.key");
    Ed25519Identity::load_or_generate(&key_file)?;
    let settings = SecretsSettings {
        broker: format!("http://{address}"),
        service: "identity".to_owned(),
        service_key_file: key_file,
    };
    let said = Arc::new(Mutex::new(Vec::new()));
    let saying = Arc::clone(&said);
    let say: lys_identity_server::Say = Arc::new(move |line| {
        saying
            .lock()
            .expect("fixture lock poisoned")
            .push(line.to_owned());
    });
    let (service, seeded) = Box::pin(Service::start_saying(
        GRANT_MODEL,
        None,
        Some(settings),
        None,
        |_| {},
        Some(say),
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    ))
    .await?;
    drop(keys);
    let person = seeded
        .people
        .iter()
        .find(|person| person.subject == subject)
        .ok_or("no seeded person")?
        .id
        .to_string();
    Ok(Setup {
        service,
        log,
        said,
        person,
    })
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn received(log: &Log) -> Vec<Received> {
    log.lock().expect("fixture lock poisoned").clone()
}

/// Neither the answers, nor any line the service said, nor any file under
/// its folder carries `text`.
fn never_kept(setup: &Setup, text: &str) -> TestResult {
    assert!(
        !setup
            .said
            .lock()
            .expect("fixture lock poisoned")
            .iter()
            .any(|line| line.contains(text)),
        "a line the service said carries the value"
    );
    never_written(setup.service.dir.path(), text)
}

fn never_written(dir: &Path, text: &str) -> TestResult {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            never_written(&path, text)?;
        } else {
            let bytes = std::fs::read(&path)?;
            assert!(
                !bytes
                    .windows(text.len())
                    .any(|window| window == text.as_bytes()),
                "{} carries the value",
                path.display()
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn a_secret_is_added_changed_and_retired_with_its_value_passed_unread_and_never_kept()
-> TestResult {
    let setup = setup(BEA).await?;
    let add = |name: &str| {
        json!({
            "operation": "op-add-0123456789abcdef", "name": name, "class": "credential",
            "upstream": "https://api.example.test", "header": "authorization",
            "prefix": "Bearer ", "value": VALUE,
        })
    };
    let (status, body) = setup
        .service
        .post("/secrets/add", None, &add("token"))
        .await?;
    assert_eq!(status, 401, "{body}");
    assert!(received(&setup.log).is_empty());

    let cookie = setup.service.sign_in(login(BEA)).await?;
    let (status, body) = setup
        .service
        .post("/secrets/add", Some(&cookie), &add("token"))
        .await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["owner"], setup.person.as_str());
    assert!(!body.to_string().contains(VALUE));
    let asked = received(&setup.log);
    assert_eq!(asked[0].path, "/_lys/add");
    assert_eq!(asked[0].on_behalf_of, setup.person);
    assert_eq!(asked[0].body, add("token").to_string().into_bytes());

    for (name, status, refusal) in [
        ("taken", 409, "SecretExists"),
        ("retired", 409, "SecretRetired"),
        ("reused", 400, "OperationReused"),
        ("nowhere", 400, "RouteInvalid"),
        ("empty", 400, "ValueEmpty"),
    ] {
        let (answered, body) = setup
            .service
            .post("/secrets/add", Some(&cookie), &add(name))
            .await?;
        assert_eq!(answered, status, "{body}");
        assert_eq!(body["refusal"], refusal);
        assert!(!body.to_string().contains(VALUE));
    }

    let replace = |secret: &str| json!({ "operation": "op-replace-0123456789abcdef", "secret": secret, "value": VALUE });
    let (status, body) = setup
        .service
        .post("/secrets/replace", Some(&cookie), &replace("token"))
        .await?;
    assert_eq!(status, 200, "{body}");
    assert!(!body.to_string().contains(VALUE));
    let (status, body) = setup
        .service
        .post("/secrets/replace", Some(&cookie), &replace("not-mine"))
        .await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "LendingNotPermitted");
    let retire = json!({ "operation": "op-retire-0123456789abcdef", "secret": "token" });
    let (status, body) = setup
        .service
        .post("/secrets/retire", Some(&cookie), &retire)
        .await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        received(&setup.log).last().map(|asked| asked.body.clone()),
        Some(retire.to_string().into_bytes())
    );
    never_kept(&setup, VALUE)
}
