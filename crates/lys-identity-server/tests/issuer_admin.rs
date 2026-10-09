#![cfg(test)]

//! The issuer owns administrator authentication behind the public route.

use std::error::Error;
use std::future::IntoFuture;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use axum::routing::any;
use identity_contract::harness::{GRANT_MODEL, Service};
use tokio::sync::oneshot;

type TestResult = Result<(), Box<dyn Error>>;
type Seen = Arc<Mutex<Vec<Observed>>>;

struct Observed {
    path: String,
    cookie: String,
    host: String,
    forwarded_host: String,
    forwarded_address: String,
    authorization: Option<HeaderValue>,
}

struct Fixture {
    seen: Seen,
    discovery_api: Mutex<Option<String>>,
    http: reqwest::Client,
}

fn failed(message: String) -> Response {
    let mut answer = Response::new(Body::from(message));
    *answer.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    answer
}

async fn issuer(State(fixture): State<Arc<Fixture>>, request: Request) -> Response {
    let path = request.uri().to_string();
    if matches!(request.uri().path(), "/auth/v1/.well-known/openid-configuration" | "/auth/v1/jwks") {
        let native = match fixture.discovery_api.lock() {
            Ok(held) => held.clone(),
            Err(error) => return failed(format!("discovery_lock_failed: {error}")),
        };
        let Some(native) = native else { return failed("discovery_api_absent".to_owned()); };
        let Some(suffix) = path.strip_prefix("/auth/v1") else { return failed("discovery_path_invalid".to_owned()); };
        return match fixture.http.get(format!("{native}{suffix}")).send().await {
            Ok(answer) => {
                let answer: axum::http::Response<reqwest::Body> = answer.into();
                answer.map(Body::new)
            }
            Err(error) => failed(format!("discovery_forward_failed: {}", error.without_url())),
        };
    }
    let cookie = request.headers().get(header::COOKIE)
        .and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned();
    let text = |name: &str| request.headers().get(name)
        .and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned();
    let observed = Observed {
        path: path.clone(), cookie,
        host: text("host"), forwarded_host: text("x-forwarded-host"),
        forwarded_address: text("x-forwarded-for"),
        authorization: request.headers().get(header::AUTHORIZATION).cloned(),
    };
    match fixture.seen.lock() {
        Ok(mut held) => held.push(observed),
        Err(_) => {
            let mut answer = Response::new(Body::from("observer lock failed"));
            *answer.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return answer;
        }
    }
    let mut answer = Response::new(if path == "/auth/v1/users/import" {
        request.into_body()
    } else {
        Body::from(path.clone())
    });
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
    let fixture = Arc::new(Fixture {
        seen: Arc::clone(&seen), discovery_api: Mutex::new(None),
        http: reqwest::Client::builder().no_proxy().build()?,
    });
    let configuring = Arc::clone(&fixture);
    let (stop, stopped) = oneshot::channel::<()>();
    let task = tokio::spawn(axum::serve(listener, axum::Router::new()
        .route("/auth/v1/{*path}", any(issuer)).with_state(fixture))
        .with_graceful_shutdown(async move {
            if let Err(error) = stopped.await {
                panic!("issuer_shutdown_signal_failed: {error}");
            }
        }).into_future());
    let started = Service::start_adjusted(GRANT_MODEL, None, None, None,
        move |config| {
            match configuring.discovery_api.lock() {
                Ok(mut held) => *held = Some(config.sign_in_api()),
                Err(error) => panic!("discovery_configuration_lock_failed: {error}"),
            }
            config.sign_in_api = Some(address);
        }, |_| Ok(())).await;
    let (mut service, ()) = match started {
        Ok(service) => service,
        Err(error) => {
            let failure = std::io::Error::other(format!("service_startup_failed: {error}"));
            drop(error);
            let sent = stop.send(()).map_err(|()| "issuer stopped before shutdown");
            let joined = task.await;
            sent?;
            joined??;
            return Err(failure.into());
        }
    };
    let checks = async {
        let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()?;
        for path in ["/auth/v1/admin", "/auth/v1/assets/admin.js", "/auth/v1/users", "/auth/v1/oidc/callback?code=opaque&state=held", "/auth/v1/api_keys"] {
            let answer = client.get(format!("{}{path}", service.base))
                .header(header::COOKIE, "issuer_session=held; lys_directory_session=private")
                .header(header::HOST, "untrusted.example.test")
                .header("x-forwarded-for", "198.51.100.60")
                .header("x-forwarded-host", "untrusted.example.test")
                .send().await?;
            assert_eq!(answer.status(), if path == "/auth/v1/api_keys" { 401 } else { 200 });
            assert_eq!(answer.headers().get_all(header::SET_COOKIE).iter().count(), 2);
            assert_eq!(answer.text().await?, path);
        }
        let body = vec![b'a'; 32_768];
        let answer = client.post(format!("{}/auth/v1/users/import", service.base))
            .header(header::COOKIE, "issuer_session=held; lys_provider=private")
            .body(body.clone()).send().await?;
        assert_eq!(answer.status(), 200);
        assert_eq!(answer.bytes().await?.as_ref(), body.as_slice());
        let callback = client.get(format!("{}/auth/v1/providers/callback?state=unknown&code=opaque", service.base)).send().await?;
        assert_eq!(callback.status(), 303);
        assert!(callback.headers().get(header::LOCATION).ok_or("callback location")?.to_str()?.contains("SignInStateUnknown"));
        let held = seen.lock().map_err(|_| "observer lock failed")?;
        assert_eq!(held.len(), 6, "the Lys callback never reaches the native issuer");
        let authority = service.base.strip_prefix("http://").ok_or("fixture origin")?;
        for observed in held.iter() {
            assert_eq!(observed.cookie, "issuer_session=held", "{}", observed.path);
            assert_eq!(observed.host, authority);
            assert_eq!(observed.forwarded_host, authority);
            assert_eq!(observed.forwarded_address, "127.0.0.1");
            assert!(observed.authorization.is_none(), "no service credential is injected");
        }
        Ok::<_, Box<dyn Error>>(())
    }.await;
    let closed = service.close();
    let sent = stop.send(()).map_err(|()| "issuer stopped before shutdown");
    let joined = task.await;
    closed?;
    sent?;
    joined??;
    checks
}
