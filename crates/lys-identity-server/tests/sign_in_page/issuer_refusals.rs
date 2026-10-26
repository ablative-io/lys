//! The issuer's refusals of a password sign-in, each named by Lys and
//! never retried (moved from `sign_in_page.rs` whole so each file stays under
//! the 500-line gate; DIRECTORY-107 touched the parent).

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};

use lys_identity_server::Config;
use lys_identity_server::oidc::Oidc;
use lys_identity_server::sign_in::{Attempt, IssuerSignIn};
use serde_json::json;

use super::TestResult;

#[derive(Clone)]
pub(super) struct IssuerRefusalFixture {
    pub(super) status: axum::http::StatusCode,
    pub(super) error: &'static str,
    pub(super) expires: u64,
    pub(super) pow: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub(super) authorize: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl IssuerRefusalFixture {
    pub(super) fn new(
        status: u16,
        error: &'static str,
        expires: u64,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            status: axum::http::StatusCode::from_u16(status)?,
            error,
            expires,
            pow: std::sync::Arc::default(),
            authorize: std::sync::Arc::default(),
        })
    }
}

pub(super) async fn refusal_challenge(
    axum::extract::State(fixture): axum::extract::State<IssuerRefusalFixture>,
) -> String {
    fixture
        .pow
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("1:10:{}:salt:challenge:", fixture.expires)
}

pub(super) async fn refuse_authorization(
    axum::extract::State(fixture): axum::extract::State<IssuerRefusalFixture>,
) -> impl axum::response::IntoResponse {
    fixture
        .authorize
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    (
        fixture.status,
        axum::Json(json!({
            "error": fixture.error,
            "message": "issuer-private-message-sentinel",
            "email": "issuer-private-email-sentinel",
            "token": "issuer-private-token-sentinel",
        })),
    )
}

async fn issuer_refusal(
    status: u16,
    error: &'static str,
    expires: u64,
    named: &str,
    wire_status: u16,
    authorize_count: usize,
) -> TestResult {
    use axum::response::IntoResponse;
    use std::sync::atomic::Ordering;

    let fixture = IssuerRefusalFixture::new(status, error, expires)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let routes = issuer_refusal_routes(&base, fixture.clone());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(async move {
        axum::serve(listener, routes)
            .with_graceful_shutdown(async move {
                match stopped.await {
                    Ok(()) | Err(_) => {}
                }
            })
            .await
    });
    let directory = tempfile::tempdir()?;
    let secret = directory.path().join("client-secret");
    std::fs::write(&secret, "fixture-client-secret")?;
    let config: Config = serde_json::from_value(json!({
        "listen": "127.0.0.1:0", "log_dir": directory.path(), "log_origin": "test",
        "event_key_file": directory.path().join("unused-key"),
        "issuer": base, "client_id": "test", "client_secret_file": secret,
        "redirect_url": format!("{base}/callback"),
        "link_audit_source": {"issuer": base, "subject": "test"},
        "reviews": {"period_days": 30}, "session_seconds": 60, "secure_cookie": false,
        "grant_log_dir": directory.path(), "grant_log_origin": "test",
        "grant_model_file": directory.path().join("unused-model"),
        "stores_receipt_file": directory.path().join("install").join("stores.json"),
        "stores_accepted_file": directory.path().join("install").join("stores.accepted.json"),
    }))?;
    let oidc = Oidc::discover(&config).await?;
    let issuer = IssuerSignIn::new(base, config.redirect_url)?;
    let logs = RefusalLog::default();
    let captured = logs.clone();
    let refusal = {
        let guard = tracing::subscriber::set_default(logs);
        let result = issuer
            .password(
                &oidc,
                &Attempt {
                    email: "attempt-private-email-sentinel",
                    password: "attempt-private-password-sentinel",
                    address: IpAddr::V4(Ipv4Addr::LOCALHOST),
                },
            )
            .await;
        drop(guard);
        result.err().ok_or("the fixture issuer refused")?
    };
    let name = refusal.name();
    let response = refusal.into_response();
    let response_status = response.status().as_u16();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body = String::from_utf8(body.to_vec())?;
    stop.send(()).map_err(|()| "the fixture was serving")?;
    serving.await??;
    assert_eq!(
        fixture.pow.load(Ordering::SeqCst),
        1,
        "one challenge, no retry"
    );
    assert_eq!(fixture.authorize.load(Ordering::SeqCst), authorize_count);
    let log = captured
        .text
        .lock()
        .map_err(|error| std::io::Error::other(format!("log capture lock poisoned: {error}")))?
        .clone();
    for sentinel in [
        "issuer-private-message-sentinel",
        "issuer-private-email-sentinel",
        "issuer-private-token-sentinel",
        "attempt-private-email-sentinel",
        "attempt-private-password-sentinel",
        "fixture-request-token",
    ] {
        assert!(!body.contains(sentinel), "private data in refusal body");
        assert!(!log.contains(sentinel), "private data in refusal log");
    }
    assert_eq!(name, named, "{body}");
    assert_eq!(response_status, wire_status, "{body}");
    if named == "IssuerRefused" {
        assert!(body.contains(&status.to_string()) && body.contains(error));
        assert!(log.contains("WARN") && log.contains(&status.to_string()) && log.contains(error));
    } else if named == "IssuerChallengeExpired" {
        assert!(log.contains("WARN") && log.contains("IssuerChallengeExpired"));
        assert!(body.contains("sign-in again"));
    }
    Ok(())
}

#[derive(Clone, Default)]
pub(super) struct RefusalLog {
    pub(super) text: std::sync::Arc<std::sync::Mutex<String>>,
}

impl tracing::Subscriber for RefusalLog {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        *metadata.level() == tracing::Level::WARN
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut text = match self.text.lock() {
            Ok(text) => text,
            Err(error) => panic!("log capture lock poisoned: {error}"),
        };
        text.push_str(event.metadata().level().as_str());
        event.record(&mut RefusalFields(&mut text));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

struct RefusalFields<'a>(&'a mut String);
impl tracing::field::Visit for RefusalFields<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        if let Err(error) = write!(self.0, " {}={value:?}", field.name()) {
            panic!("log capture formatting failed: {error}");
        }
    }
}

#[tokio::test]
async fn a_future_forbidden_challenge_names_the_issuer_refusal() -> TestResult {
    issuer_refusal(403, "Forbidden", 4_102_444_800, "IssuerRefused", 503, 1).await
}
#[tokio::test]
async fn a_bad_request_names_the_issuer_refusal() -> TestResult {
    issuer_refusal(400, "BadRequest", 4_102_444_800, "IssuerRefused", 503, 1).await
}
#[tokio::test]
async fn an_unauthorized_credential_answer_names_the_password_refusal() -> TestResult {
    issuer_refusal(401, "Unauthorized", 4_102_444_800, "SignInRefused", 401, 1).await
}
#[tokio::test]
async fn a_challenge_already_expired_never_posts_credentials() -> TestResult {
    issuer_refusal(403, "Forbidden", 1, "IssuerChallengeExpired", 503, 0).await
}

fn issuer_refusal_routes(base: &str, fixture: IssuerRefusalFixture) -> axum::Router {
    let discovery = json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/oidc/authorize"),
        "token_endpoint": format!("{base}/oidc/token"),
        "jwks_uri": format!("{base}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
    });
    axum::Router::new()
        .route(
            "/.well-known/openid-configuration",
            axum::routing::get(move || {
                let discovery = discovery.clone();
                async move { axum::Json(discovery) }
            }),
        )
        .route(
            "/jwks",
            axum::routing::get(|| async { axum::Json(json!({"keys": []})) }),
        )
        .route(
            "/oidc/authorize",
            axum::routing::get(|| async {
                "<template id=\"tpl_csrf_token\">fixture-request-token</template>"
            })
            .post(refuse_authorization),
        )
        .route("/pow", axum::routing::post(refusal_challenge))
        .with_state(fixture)
}
