//! The door to a haematite store: only an administrator is admitted, a verb
//! is carried to the store's service after a greeting, the service's own
//! refusal is a definite answer, and a service that does not answer is said
//! to be unreachable, never refused.
use identity_contract::apps::{Auth, get, login, post};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type TestResult = Result<(), Box<dyn Error>>;

fn read_frame(stream: &mut UnixStream) -> Option<Value> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).ok()?;
    let mut body = vec![0_u8; u32::from_be_bytes(header) as usize];
    stream.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn write_frame(stream: &mut UnixStream, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(value)?;
    let length = u32::try_from(body.len()).map_err(std::io::Error::other)?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(&body)
}

/// A service on `socket` that speaks `haem serve`'s framing and keeps every
/// request it read: it answers `hello`, refuses any verb sent before it,
/// answers `branches.list`, hangs up on `hang.up`, and refuses the rest by
/// the name `not_available`.
fn serving(socket: &Path) -> Result<Arc<Mutex<Vec<Value>>>, Box<dyn Error>> {
    let listener = UnixListener::bind(socket)?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut greeted = false;
            while let Some(request) = read_frame(&mut stream) {
                if let Ok(mut all) = kept.lock() {
                    all.push(request.clone());
                }
                let id = request["id"].clone();
                let method = request["operation"]["method"].as_str().unwrap_or_default();
                let answer = match method {
                    "hello" => {
                        greeted = true;
                        json!({"id": id, "result": {"protocol": "haem/1", "max_page": 200, "max_frame": 1_048_576}})
                    }
                    _ if !greeted => {
                        json!({"id": id, "error": {"code": "hello_required", "message": "send hello before other requests"}})
                    }
                    "branches.list" => {
                        json!({"id": id, "result": {"entries": [], "next_after": null, "asked": request["operation"]["params"]}})
                    }
                    "hang.up" => break,
                    other => {
                        json!({"id": id, "error": {"code": "not_available", "message": format!("no {other}")}})
                    }
                };
                if write_frame(&mut stream, &answer).is_err() {
                    break;
                }
            }
        }
    });
    Ok(seen)
}

async fn door(stores: Vec<(&'static str, PathBuf)>) -> Result<Service, Box<dyn Error>> {
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.haem_stores = stores
                .into_iter()
                .map(|(name, socket)| (name.to_owned(), socket))
                .collect();
        },
        |_| Ok(()),
    )
    .await?;
    Ok(service)
}

#[tokio::test]
async fn nobody_reaches_a_store_without_signing_in() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("s");
    let seen = serving(&socket)?;
    let service = door(vec![("ablative", socket)]).await?;
    let (status, refused) = get(&service, "/haem", Auth::Nobody).await?;
    assert_eq!((status, &refused["refusal"]), (401, &json!("NotSignedIn")));
    let verb = json!({"method": "branches.list", "params": {"limit": 200}});
    let (status, refused) = post(&service, "/haem/ablative", Auth::Nobody, &verb).await?;
    assert_eq!((status, &refused["refusal"]), (401, &json!("NotSignedIn")));
    assert!(
        seen.lock().map_err(|error| error.to_string())?.is_empty(),
        "a caller who is not admitted reaches the store's service"
    );
    Ok(())
}

#[tokio::test]
async fn an_administrator_is_answered_what_the_store_answered() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("s");
    let seen = serving(&socket)?;
    let service = door(vec![
        ("ablative", socket),
        ("trial", dir.path().join("absent")),
    ])
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let auth = Auth::Cookie(&cookie);

    let (status, stores) = get(&service, "/haem", auth).await?;
    assert_eq!(status, 200, "{stores}");
    assert_eq!(
        stores,
        json!({"stores": [{"name": "ablative"}, {"name": "trial"}]})
    );

    let verb = json!({"method": "branches.list", "params": {"limit": 200}});
    let (status, answer) = post(&service, "/haem/ablative", Auth::Cookie(&cookie), &verb).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer,
        json!({"entries": [], "next_after": null, "asked": {"limit": 200}})
    );
    assert_eq!(
        *seen.lock().map_err(|error| error.to_string())?,
        vec![
            json!({"id": "hello", "operation": {"method": "hello"}}),
            json!({"id": "verb", "operation": {"method": "branches.list", "params": {"limit": 200}}}),
        ],
        "one greeting, then the verb with its params, on one connection"
    );

    let (status, hello) = post(
        &service,
        "/haem/ablative",
        Auth::Cookie(&cookie),
        &json!({"method": "hello"}),
    )
    .await?;
    assert_eq!(status, 200, "{hello}");
    assert_eq!(hello["protocol"], "haem/1");
    Ok(())
}

#[tokio::test]
async fn a_refusal_is_definite_and_silence_is_not_a_refusal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("s");
    let _seen = serving(&socket)?;
    let service = door(vec![
        ("ablative", socket),
        ("trial", dir.path().join("absent")),
    ])
    .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let ask = |store: &'static str, body: Value| {
        let (service, cookie) = (&service, cookie.as_str());
        async move {
            post(
                service,
                &format!("/haem/{store}"),
                Auth::Cookie(cookie),
                &body,
            )
            .await
        }
    };

    let (status, refused) = ask("ablative", json!({"method": "edge.out", "params": {}})).await?;
    assert_eq!((status, &refused["refusal"]), (409, &json!("HaemRefused")));
    assert_eq!(refused["reason"], "HaemRefused: not_available: no edge.out");

    let (status, refused) = ask("ablative", json!({"method": "hang.up"})).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (502, &json!("HaemUnreachable"))
    );

    let (status, refused) = ask("trial", json!({"method": "hello"})).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (502, &json!("HaemUnreachable"))
    );

    let (status, refused) = ask("nowhere", json!({"method": "hello"})).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (404, &json!("HaemStoreUnknown"))
    );

    let (status, refused) = ask("ablative", json!({"method": "hello", "extra": 1})).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("RequestMalformed"))
    );
    let (status, refused) = ask("ablative", json!({"params": {}})).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("RequestMalformed"))
    );
    Ok(())
}

#[test]
fn a_store_is_configured_by_a_name_and_an_absolute_socket() -> TestResult {
    let refused = tokio::runtime::Runtime::new()?
        .block_on(door(vec![("ablative", PathBuf::from("relative/s"))]))
        .err()
        .ok_or("a store with a relative socket was served")?
        .to_string();
    assert!(
        refused.contains("haem_stores: the socket of `ablative` must be an absolute path"),
        "{refused}"
    );
    Ok(())
}
