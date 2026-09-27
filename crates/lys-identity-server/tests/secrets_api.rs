//! The secrets screens' routes, against a stand-in broker that records what
//! reaches it: only a signed-in person is served, the person the broker is
//! told of is the session's and never the browser's, the request the broker
//! receives is the one the service signed, a body is forwarded unchanged, a
//! secret's route is taken off before it leaves, and a broker refusal
//! passes through by name and status.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, verify_attestation_by_signer};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const ADA: &str = "ada-subject";
const BEA: &str = "bea-subject";

/// One request as the stand-in broker received it.
#[derive(Clone, Debug)]
struct Received {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Received {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(known, _value)| known == name)
            .map(|(_name, value)| value.as_str())
    }
}

type Log = Arc<Mutex<Vec<Received>>>;

async fn broker(State(log): State<Log>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let Ok(body) = axum::body::to_bytes(body, 1 << 20).await else {
        return (StatusCode::BAD_REQUEST, "unreadable body").into_response();
    };
    let path = parts.uri.path_and_query().map_or_else(
        || parts.uri.path().to_owned(),
        |whole| whole.as_str().to_owned(),
    );
    log.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(Received {
            method: parts.method.as_str().to_owned(),
            path: path.clone(),
            headers: parts
                .headers
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (name.as_str().to_owned(), value.to_owned()))
                })
                .collect(),
            body: body.to_vec(),
        });
    answer(&path, &body)
}

fn answer(path: &str, body: &Bytes) -> Response {
    match path {
        "/_lys/secrets" => axum::Json(json!({ "secrets": [{
            "name": "token", "class": "Credential", "owner": "person-owner", "sequence": 1,
            "upstream": "https://api.example.test", "header": "authorization",
        }]}))
        .into_response(),
        "/_lys/scope" => (
            StatusCode::FORBIDDEN,
            "LendingNotPermitted: someone may not lend token\n",
        )
            .into_response(),
        "/_lys/recipients" => match serde_json::from_slice::<Value>(body) {
            Ok(asked) => axum::Json(asked).into_response(),
            Err(_error) => (StatusCode::BAD_REQUEST, "Encoding: not JSON\n").into_response(),
        },
        _ => axum::Json(json!({ "path": path })).into_response(),
    }
}

struct Setup {
    service: Service,
    seeded: Seeded,
    log: Log,
    key: [u8; 32],
}

async fn setup() -> Result<Setup, Box<dyn Error>> {
    let log: Log = Arc::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = Router::new().fallback(broker).with_state(Arc::clone(&log));
    tokio::spawn(async move { axum::serve(listener, app).await });
    let keys = tempfile::TempDir::new()?;
    let key_file = keys.path().join("secrets-service.key");
    let key = Ed25519Identity::load_or_generate(&key_file)?.public_key_bytes();
    let settings = SecretsSettings {
        broker: format!("http://{address}"),
        service: "identity".to_owned(),
        service_key_file: key_file,
    };
    // The service reads its key once, at start, so the key file may go after.
    let (service, seeded) = Service::start_asking(GRANT_MODEL, None, Some(settings), |config| {
        Ok(seed_configured(config, [ADA, BEA])?)
    })
    .await?;
    drop(keys);
    Ok(Setup {
        service,
        seeded,
        log,
        key,
    })
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn received(log: &Log) -> Vec<Received> {
    log.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if text.len() % 2 != 0 {
        return Err("odd hex".into());
    }
    (0..text.len())
        .step_by(2)
        .map(|at| Ok(u8::from_str_radix(&text[at..at + 2], 16)?))
        .collect()
}

fn field(into: &mut Vec<u8>, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    into.extend_from_slice(&u32::try_from(bytes.len())?.to_be_bytes());
    into.extend_from_slice(bytes);
    Ok(())
}

/// Whether `request` carries a signature by `key` over exactly the method,
/// path and body the broker received, for the person it names.
fn signed_as_received(request: &Received, key: &[u8; 32]) -> Result<(), Box<dyn Error>> {
    let value = |name: &str| request.header(name).ok_or(format!("no {name} header"));
    let mut digest = Vec::new();
    field(&mut digest, b"lys-secrets/request/v1")?;
    field(&mut digest, request.method.as_bytes())?;
    field(&mut digest, request.path.as_bytes())?;
    field(&mut digest, &Sha256::digest(&request.body))?;
    let mut payload = Vec::new();
    field(&mut payload, b"lys-secrets/on-behalf/v1")?;
    field(&mut payload, value("lys-service")?.as_bytes())?;
    field(&mut payload, value("lys-on-behalf-of")?.as_bytes())?;
    field(&mut payload, &unhex(value("lys-operation")?)?)?;
    field(
        &mut payload,
        &value("lys-signed-at")?.parse::<i64>()?.to_be_bytes(),
    )?;
    field(&mut payload, &Sha256::digest(&digest))?;
    let attestation = Attestation::from_cose_bytes(&unhex(value("lys-service-signature")?)?)?;
    verify_attestation_by_signer(&attestation, &payload, key)?;
    Ok(())
}

#[tokio::test]
async fn nobody_signed_in_is_refused_and_nothing_reaches_the_broker() -> TestResult {
    let setup = setup().await?;
    let (status, body) = setup.service.get("/secrets", None).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    assert!(received(&setup.log).is_empty());
    Ok(())
}

#[tokio::test]
async fn the_signed_in_person_is_the_one_the_broker_is_asked_for_and_routes_never_leave()
-> TestResult {
    let setup = setup().await?;
    let cookie = setup.service.sign_in(login(ADA)).await?;
    let ada = setup
        .seeded
        .people
        .iter()
        .find(|person| person.subject == ADA)
        .ok_or("no seeded Ada")?;
    let (status, body) = setup.service.get("/secrets", Some(&cookie)).await?;
    assert_eq!(status, 200, "{body}");
    let entry = &body["secrets"][0];
    assert_eq!(entry["name"], "token");
    assert!(
        entry.get("upstream").is_none() && entry.get("header").is_none(),
        "{body}"
    );
    let asked = received(&setup.log);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].path, "/_lys/secrets");
    assert_eq!(asked[0].header("lys-service"), Some("identity"));
    assert_eq!(
        asked[0].header("lys-on-behalf-of"),
        Some(ada.id.to_string().as_str())
    );
    signed_as_received(&asked[0], &setup.key)?;
    Ok(())
}

#[tokio::test]
async fn a_change_is_forwarded_unchanged_and_a_refusal_passes_through_by_name() -> TestResult {
    let setup = setup().await?;
    let cookie = setup.service.sign_in(login(BEA)).await?;
    let change = json!({ "secret": "token", "scope": "team:accounts" });
    let (status, body) = setup
        .service
        .post("/secrets/scope", Some(&cookie), &change)
        .await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "LendingNotPermitted");
    let recipients = json!({ "secret": "token", "recipients": "people_only" });
    let (status, body) = setup
        .service
        .post("/secrets/recipients", Some(&cookie), &recipients)
        .await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, recipients);
    let asked = received(&setup.log);
    assert_eq!(asked.len(), 2);
    assert_eq!(asked[0].body, change.to_string().into_bytes());
    assert_eq!(asked[1].path, "/_lys/recipients");
    for request in &asked {
        signed_as_received(request, &setup.key)?;
    }
    Ok(())
}

#[tokio::test]
async fn a_revocation_is_asked_by_handle_and_the_query_is_signed() -> TestResult {
    let setup = setup().await?;
    let cookie = setup.service.sign_in(login(ADA)).await?;
    let (status, body) = setup
        .service
        .get("/secrets/revocation?handle=h-1", Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{body}");
    let (status, body) = setup
        .service
        .get("/secrets/revocation", Some(&cookie))
        .await?;
    assert_eq!(status, 400, "{body}");
    let asked = received(&setup.log);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].path, "/_lys/revocation?handle=h-1");
    signed_as_received(&asked[0], &setup.key)?;
    Ok(())
}

#[tokio::test]
async fn a_login_bound_to_no_person_is_refused_by_name() -> TestResult {
    let setup = setup().await?;
    let cookie = setup.service.sign_in(login("nobody-subject")).await?;
    let (status, body) = setup.service.get("/secrets", Some(&cookie)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NoPerson");
    assert!(received(&setup.log).is_empty());
    Ok(())
}

#[tokio::test]
async fn without_a_broker_configured_the_routes_say_so() -> TestResult {
    let (service, _seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADA, BEA])?)).await?;
    let cookie = service.sign_in(login(ADA)).await?;
    let (status, body) = service.get("/secrets", Some(&cookie)).await?;
    assert_eq!(status, 502, "{body}");
    assert_eq!(body["refusal"], "SecretsUnavailable");
    Ok(())
}
