#![cfg(test)]

use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::extract::{ConnectInfo, State};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use lys_identity_server::Config;
use serde_json::json;
use tokio::sync::{oneshot, watch};
use tower::ServiceExt;
use tracing::instrument::WithSubscriber;

use super::{IssuerRefusalFixture, RefusalLog, refusal_challenge, refuse_authorization};

type Failure = Box<dyn Error>;
type Cases = watch::Receiver<IssuerRefusalFixture>;

pub(super) struct Observed {
    pub status: u16,
    pub location: Option<String>,
    pub body: String,
    pub log: String,
    pub cookie: bool,
    pub pow: usize,
    pub authorize: usize,
    pub provider_posts: usize,
}

struct Issuer {
    stop: Option<oneshot::Sender<()>>,
    serving: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}

impl Issuer {
    async fn close(&mut self) -> Result<(), Failure> {
        let mut failures = Vec::new();
        if let Some(stop) = self.stop.take()
            && stop.send(()).is_err()
        {
            failures.push("public_issuer_shutdown_receiver_lost".to_owned());
        }
        if let Some(serving) = self.serving.take() {
            match serving.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => failures.push(format!("public_issuer_serve_failed: {error}")),
                Err(error) => failures.push(format!("public_issuer_join_failed: {error}")),
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; ").into())
        }
    }
}

impl Drop for Issuer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take()
            && stop.send(()).is_err()
        {
            eprintln!("public_issuer_shutdown_receiver_lost");
        }
        if let Some(serving) = self.serving.take() {
            eprintln!("public_issuer_dropped_without_join");
            serving.abort();
        }
    }
}

pub(super) struct PublicSignIn {
    router: Option<Router>,
    next: watch::Sender<IssuerRefusalFixture>,
    provider_posts: Arc<AtomicUsize>,
    issuer: Issuer,
    directory: tempfile::TempDir,
}

async fn challenge(State(cases): State<Cases>) -> String {
    let fixture = cases.borrow().clone();
    refusal_challenge(State(fixture)).await
}

async fn refuse(State(cases): State<Cases>) -> Response {
    let fixture = cases.borrow().clone();
    refuse_authorization(State(fixture)).await.into_response()
}

impl PublicSignIn {
    pub(super) async fn start() -> Result<Self, Failure> {
        let directory = tempfile::tempdir()?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let discovery = json!({
            "issuer": base,
            "authorization_endpoint": format!("{base}/oidc/authorize"),
            "token_endpoint": format!("{base}/oidc/token"),
            "jwks_uri": format!("{base}/jwks"),
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["EdDSA"],
        });
        let (next, cases) = watch::channel(IssuerRefusalFixture::new(
            401,
            "Unauthorized",
            4_102_444_800,
        )?);
        let provider_posts = Arc::new(AtomicUsize::new(0));
        let issuer_provider_posts = Arc::clone(&provider_posts);
        let routes = Router::new()
            .route("/.well-known/openid-configuration", axum::routing::get(move || {
                let discovery = discovery.clone();
                async move { Json(discovery) }
            }))
            .route("/jwks", axum::routing::get(|| async { Json(json!({"keys": []})) }))
            .route("/oidc/authorize", axum::routing::get(|| async {
                "<template id=\"tpl_csrf_token\">fixture-request-token</template>"
            }).post(refuse))
            .route("/pow", axum::routing::post(challenge))
            .route("/providers/login", axum::routing::post(move || {
                let provider_posts = Arc::clone(&issuer_provider_posts);
                async move {
                    provider_posts.fetch_add(1, Ordering::SeqCst);
                    (
                        StatusCode::ACCEPTED,
                        [(header::LOCATION,
                            "https://provider.example.test/login?redirect_uri=http%3A%2F%2F127.0.0.1%2Fauth%2Fv1%2Fproviders%2Fcallback&state=fixture-provider-state")],
                        "fixture-request-token",
                    )
                }
            }))
            .with_state(cases);
        let (stop, stopped) = oneshot::channel();
        let serving = tokio::spawn(async move {
            axum::serve(listener, routes)
                .with_graceful_shutdown(async move {
                    if stopped.await.is_err() {
                        eprintln!("public_issuer_shutdown_sender_lost");
                    }
                })
                .await
        });
        let mut issuer = Issuer {
            stop: Some(stop),
            serving: Some(serving),
        };
        let outcome = async {
            let key = directory.path().join("service.key");
            let secret = directory.path().join("client.secret");
            let model = directory.path().join("grant-model.json");
            std::fs::write(&key, [9; 32])?;
            std::fs::write(&secret, "fixture-client-secret")?;
            for file in [&key, &secret] {
                std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))?;
            }
            std::fs::write(&model, identity_contract::harness::GRANT_MODEL)?;
            let config: Config = serde_json::from_value(json!({
                "listen": "127.0.0.1:0", "log_dir": directory.path().join("log"),
                "log_origin": "test", "event_key_file": key,
                "issuer": base, "client_id": "test", "client_secret_file": secret,
                "redirect_url": "http://127.0.0.1/callback",
                "administrator": {"issuer": base, "subject": "administrator"},
                "link_audit_source": {"issuer": base, "subject": "test"},
                "session_seconds": 60, "secure_cookie": false,
                "grant_log_dir": directory.path().join("grants"), "grant_log_origin": "test",
                "grant_model_file": model,
            }))?;
            config.validate()?;
            Ok::<_, Failure>(lys_identity_server::service(&config).await?)
        }
        .await
        // Words, not the boxed error: the cleanup below awaits, and a boxed
        // error is not Send.
        .map_err(|error| error.to_string());
        match outcome {
            Ok(service) => Ok(Self {
                router: Some(service),
                next,
                provider_posts,
                issuer,
                directory,
            }),
            Err(error) => match issuer.close().await {
                Ok(()) => Err(error.into()),
                Err(cleanup) => Err(format!(
                    "public sign-in start failed: {error}; cleanup failed: {cleanup}"
                )
                .into()),
            },
        }
    }

    pub(super) async fn request(
        &mut self,
        status: u16,
        error: &'static str,
        expires: u64,
        malformed: bool,
    ) -> Result<Observed, Failure> {
        let fixture = IssuerRefusalFixture::new(status, error, expires)?;
        drop(self.next.send_replace(fixture.clone()));
        self.provider_posts.store(0, Ordering::SeqCst);
        let mut body = json!({
            "email": "attempt-private-email-sentinel",
            "password": "attempt-private-password-sentinel",
        });
        if malformed {
            body["unexpected"] = json!(true);
        }
        let mut request = Request::builder()
            .method("POST")
            .uri("/sign-in")
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(body.to_string()))?;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((Ipv4Addr::LOCALHOST, 1234))));
        let logs = RefusalLog::default();
        let captured = logs.clone();
        let response = self
            .router
            .as_ref()
            .ok_or("public sign-in router is closed")?
            .clone()
            .oneshot(request)
            .with_subscriber(logs)
            .await?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(header::LOCATION)
            .map(|value| value.to_str().map(str::to_owned))
            .transpose()?;
        let cookie = response
            .headers()
            .contains_key(axum::http::header::SET_COOKIE);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let log = captured
            .text
            .lock()
            .map_err(|error| {
                std::io::Error::other(format!("public log capture lock poisoned: {error}"))
            })?
            .clone();
        Ok(Observed {
            status,
            location,
            body: String::from_utf8(body.to_vec())?,
            log,
            cookie,
            pow: fixture.pow.load(Ordering::SeqCst),
            authorize: fixture.authorize.load(Ordering::SeqCst),
            provider_posts: self.provider_posts.load(Ordering::SeqCst),
        })
    }

    pub(super) async fn provider_start(&mut self, expires: u64) -> Result<Observed, Failure> {
        let fixture = IssuerRefusalFixture::new(401, "Unauthorized", expires)?;
        drop(self.next.send_replace(fixture.clone()));
        self.provider_posts.store(0, Ordering::SeqCst);
        let mut request = Request::builder()
            .method("GET")
            .uri("/sign-in/providers/example")
            .body(axum::body::Body::empty())?;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((Ipv4Addr::LOCALHOST, 1234))));
        let logs = RefusalLog::default();
        let captured = logs.clone();
        let response = self
            .router
            .as_ref()
            .ok_or("public sign-in router is closed")?
            .clone()
            .oneshot(request)
            .with_subscriber(logs)
            .await?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(header::LOCATION)
            .map(|value| value.to_str().map(str::to_owned))
            .transpose()?;
        let cookie = response.headers().contains_key(header::SET_COOKIE);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let log = captured
            .text
            .lock()
            .map_err(|error| {
                std::io::Error::other(format!("public log capture lock poisoned: {error}"))
            })?
            .clone();
        Ok(Observed {
            status,
            location,
            body: String::from_utf8(body.to_vec())?,
            log,
            cookie,
            pow: fixture.pow.load(Ordering::SeqCst),
            authorize: fixture.authorize.load(Ordering::SeqCst),
            provider_posts: self.provider_posts.load(Ordering::SeqCst),
        })
    }

    pub(super) async fn close(mut self) -> Result<(), Failure> {
        drop(self.router.take());
        let shutdown = self.issuer.close().await;
        let cleanup = self
            .directory
            .close()
            .map_err(|error| -> Failure { error.into() });
        match (shutdown, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => Err(format!(
                "public sign-in shutdown failed: {error}; directory cleanup failed: {cleanup}"
            )
            .into()),
        }
    }
}
