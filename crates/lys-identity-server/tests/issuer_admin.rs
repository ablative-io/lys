//! The issuer owns administrator authentication behind the public route.

use std::error::Error;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::any;
use identity_contract::harness::{GRANT_MODEL, Service};
use tokio::sync::oneshot;

type TestResult = Result<(), Box<dyn Error>>;
type Seen = Arc<Mutex<Vec<(String, String)>>>;

async fn issuer(State(seen): State<Seen>, request: Request) -> Response {
    let path = request.uri().to_string();
    let cookie = request.headers().get(header::COOKIE)
        .and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned();
    match seen.lock() {
        Ok(mut held) => held.push((path.clone(), cookie)),
        Err(_) => return Response::new(Body::from("observer lock failed")),
    }
    let mut answer = Response::new(Body::from(path.clone()));
    *answer.status_mut() = if path.starts_with("/auth/v1/api_keys") {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::OK
    };
    answer.headers_mut().append(header::SET_COOKIE, HeaderValue::from_static("issuer_session=challenge; HttpOnly; Secure"));
    answer.headers_mut().append(header::SET_COOKIE, HeaderValue::from_static("issuer_csrf=proof; Secure"));
    answer
}

#[tokio::test]
async fn administrator_pages_and_native_callback_keep_issuer_authentication() -> TestResult {
    let seen = Seen::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}/auth/v1", listener.local_addr()?);
    let (stop, stopped) = oneshot::channel::<()>();
    let task = tokio::spawn(axum::serve(listener, axum::Router::new()
        .route("/auth/v1/{*path}", any(issuer)).with_state(Arc::clone(&seen)))
        .with_graceful_shutdown(async { let _ = stopped.await; }).into_future());
    let (mut service, ()) = Service::start_adjusted(GRANT_MODEL, None, None, None,
        move |config| config.sign_in_api = Some(address), |_| Ok(())).await?;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()?;
    let checks = async {
        for path in ["/auth/v1/admin", "/auth/v1/assets/admin.js", "/auth/v1/users", "/auth/v1/oidc/callback?code=opaque&state=held", "/auth/v1/api_keys"] {
            let answer = client.get(format!("{}{path}", service.base))
                .header(header::COOKIE, "issuer_session=held; lys_directory_session=private")
                .send().await?;
            assert_eq!(answer.status(), if path == "/auth/v1/api_keys" { 401 } else { 200 });
            assert_eq!(answer.headers().get_all(header::SET_COOKIE).iter().count(), 2);
            assert_eq!(answer.text().await?, path);
        }
        let callback = client.get(format!("{}/auth/v1/providers/callback?state=unknown&code=opaque", service.base)).send().await?;
        assert_eq!(callback.status(), 303);
        assert!(callback.headers().get(header::LOCATION).ok_or("callback location")?.to_str()?.contains("SignInStateUnknown"));
        let held = seen.lock().map_err(|_| "observer lock failed")?;
        assert_eq!(held.len(), 5, "the Lys callback never reaches the native issuer");
        assert!(held.iter().all(|(_, cookie)| cookie == "issuer_session=held"));
        Ok::<_, Box<dyn Error>>(())
    }.await;
    service.close()?;
    stop.send(()).map_err(|()| "issuer stopped before shutdown")?;
    task.await??;
    checks
}
