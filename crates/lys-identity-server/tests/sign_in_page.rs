#![cfg(test)]

//! Password sign-in on Lys's own page (DIRECTORY-047 R2): the browser posts
//! to Lys and is never sent to the issuer, a wrong password and an unknown
//! email are one refusal that names neither the issuer nor which was wrong,
//! failed sign-ins bar only the address they came from, and an account that
//! asks for a second factor is refused by name. A provider sign-in passes
//! through the provider's page and Lys's pages only.

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};

use identity_contract::fake_issuer::{Account, FAILURES_BARRED, Login};
use identity_contract::harness::Service;
use lys_identity_server::Config;
use lys_identity_server::oidc::Oidc;
use lys_identity_server::sign_in::{Attempt, IssuerSignIn};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

#[path = "support/public_sign_in.rs"]
mod public_sign_in;

const EMAIL: &str = "grace@example.test";
const PASSWORD: &str = "Compiler-Pioneer-1952";

fn account(subject: &str, email: &str, second_factor: bool) -> Account {
    Account {
        login: Login {
            subject: subject.to_owned(),
            email: email.to_owned(),
        },
        password: PASSWORD.to_owned(),
        second_factor,
    }
}

/// Everything a browser sees of one answer: the address asked, where it was
/// sent next, and the body.
struct Seen {
    asked: String,
    location: Option<String>,
    status: u16,
    body: String,
}

async fn browse(
    client: &reqwest::Client,
    request: reqwest::RequestBuilder,
) -> Result<Seen, Box<dyn Error>> {
    let request = request.build()?;
    let asked = request.url().to_string();
    let answer = client.execute(request).await?;
    let location = answer
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let status = answer.status().as_u16();
    Ok(Seen {
        asked,
        location,
        status,
        body: answer.text().await?,
    })
}

fn sign_in_body(email: &str, password: &str) -> String {
    json!({ "email": email, "password": password }).to_string()
}

#[tokio::test]
async fn every_address_the_browser_is_sent_to_is_on_lys_origin() -> TestResult {
    let service = Service::start().await?;
    service
        .issuer
        .hold(account("grace-subject", EMAIL, false))?;
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let first = browse(&browser, browser.get(format!("{}/login", service.base))).await?;
    let posted = browse(
        &browser,
        browser
            .post(format!("{}/sign-in", service.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(sign_in_body(EMAIL, PASSWORD)),
    )
    .await?;
    assert_eq!(posted.status, 200, "{}", posted.body);
    let body: Value = serde_json::from_str(&posted.body)?;
    assert_eq!(body["signed_in"]["subject"], "grace-subject");

    let mut visited = Vec::new();
    for seen in [&first, &posted] {
        visited.push(seen.asked.clone());
        if let Some(location) = &seen.location {
            let absolute = if location.starts_with('/') {
                format!("{}{location}", service.base)
            } else {
                location.clone()
            };
            visited.push(absolute);
        }
    }
    assert_eq!(visited.len(), 3, "{visited:?}");
    for url in &visited {
        assert!(
            url.starts_with(&service.base),
            "{url} is not on Lys's origin"
        );
    }
    assert!(
        service.issuer.issuer().starts_with(&service.base),
        "the issuer is named on Lys's origin, as installed"
    );
    let loopback = service.issuer.loopback();
    let port = loopback.rsplit(':').next().ok_or("the issuer has a port")?;
    let mut read = 0;
    for seen in [&first, &posted] {
        for text in [Some(&seen.body), seen.location.as_ref()]
            .into_iter()
            .flatten()
        {
            read += 1;
            assert!(
                !text.contains(loopback) && !text.contains(&format!(":{port}")),
                "no answer gives the issuer's loopback address or port: {text}"
            );
        }
    }
    assert_eq!(read, 3, "two bodies and one redirect were read");
    let forwarded = service.issuer.forwarded()?;
    assert_eq!(
        forwarded.len(),
        3,
        "three steps reached the issuer: {forwarded:?}"
    );
    assert!(
        forwarded
            .iter()
            .all(|value| value.as_deref() == Some("127.0.0.1")),
        "every step carried the person's own address: {forwarded:?}"
    );
    Ok(())
}

#[tokio::test]
async fn a_wrong_password_and_an_unknown_email_are_one_refusal() -> TestResult {
    let service = Service::start().await?;
    service
        .issuer
        .hold(account("grace-subject", EMAIL, false))?;
    let client = reqwest::Client::new();
    let mut answers = Vec::new();
    for (email, password) in [
        (EMAIL, "Not-The-Password-1"),
        ("nobody@example.test", "Not-The-Password-1"),
    ] {
        let answer = client
            .post(format!("{}/sign-in", service.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(sign_in_body(email, password))
            .send()
            .await?;
        assert!(answer.headers().get(reqwest::header::SET_COOKIE).is_none());
        answers.push((answer.status().as_u16(), answer.text().await?));
    }
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0], answers[1], "the two refusals are the same");
    let (status, body) = &answers[0];
    assert_eq!(*status, 401);
    let body: Value = serde_json::from_str(body)?;
    assert_eq!(body["refusal"], "SignInRefused");
    let words = body.to_string().to_ascii_lowercase();
    for named in [
        "rauthy",
        "password is wrong",
        "no account",
        "unknown",
        "issuer",
    ] {
        assert!(!words.contains(named), "the refusal names {named}: {words}");
    }
    Ok(())
}

#[tokio::test]
async fn an_account_asking_for_a_second_factor_is_refused_by_name() -> TestResult {
    let service = Service::start().await?;
    service
        .issuer
        .hold(account("passkey-subject", EMAIL, true))?;
    let answer = reqwest::Client::new()
        .post(format!("{}/sign-in", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(sign_in_body(EMAIL, PASSWORD))
        .send()
        .await?;
    assert_eq!(answer.status(), 403);
    let body: Value = serde_json::from_str(&answer.text().await?)?;
    assert_eq!(body["refusal"], "SecondFactorUnsupported");
    Ok(())
}

#[tokio::test]
async fn failed_sign_ins_from_one_address_do_not_bar_another() -> TestResult {
    let (service, config) = Service::start_with(|config: &Config| Ok(config.clone())).await?;
    service
        .issuer
        .hold(account("grace-subject", EMAIL, false))?;
    let oidc = Oidc::discover(&config).await?;
    let issuer = IssuerSignIn::configured(&config)?;
    let noisy = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    let quiet = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 7));
    let attempt = |password, address| Attempt {
        email: EMAIL,
        password,
        address,
    };
    let mut refusals = Vec::new();
    for _ in 0..FAILURES_BARRED {
        let refused = issuer
            .password(&oidc, &attempt("Not-The-Password-1", noisy))
            .await
            .err()
            .map(|error| error.name());
        refusals.push(refused);
    }
    assert_eq!(refusals.len(), 3);
    assert!(
        refusals
            .iter()
            .all(|name| name.as_deref() == Some("SignInRefused"))
    );
    let barred = issuer
        .password(&oidc, &attempt(PASSWORD, noisy))
        .await
        .err()
        .map(|error| error.name());
    assert_eq!(barred.as_deref(), Some("SignInThrottled"));
    let actor = issuer.password(&oidc, &attempt(PASSWORD, quiet)).await?;
    assert_eq!(actor.binding().subject(), "grace-subject");
    let forwarded = service.issuer.forwarded()?;
    assert!(forwarded.contains(&Some("198.51.100.7".to_owned())));
    assert!(forwarded.contains(&Some("192.0.2.1".to_owned())));
    Ok(())
}

#[tokio::test]
async fn a_provider_sign_in_passes_through_the_provider_and_lys_only() -> TestResult {
    let service = Service::start().await?;
    service.issuer.sign_in_as(Login {
        subject: "google-subject".to_owned(),
        email: "g@example.test".to_owned(),
    })?;
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut visited = Vec::new();
    let mut cookie = None;
    let mut next = format!("{}/sign-in/providers/google-provider", service.base);
    for _ in 0..3 {
        visited.push(next.clone());
        let mut request = browser.get(&next);
        if next.starts_with(&service.base)
            && let Some(cookie) = &cookie
        {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let answer = request.send().await?;
        assert_eq!(answer.status(), 303, "{next}");
        if let Some(set) = answer.headers().get(reqwest::header::SET_COOKIE) {
            cookie = set.to_str()?.split(';').next().map(str::to_owned);
        }
        let location = answer
            .headers()
            .get(reqwest::header::LOCATION)
            .ok_or("every step sends the browser on")?
            .to_str()?
            .to_owned();
        next = if location.starts_with('/') {
            format!("{}{location}", service.base)
        } else {
            location
        };
    }
    visited.push(next.clone());
    assert_eq!(next, format!("{}/#/me", service.base), "{visited:?}");
    assert_eq!(visited.len(), 4);
    let provider = service.issuer.provider_base();
    assert!(visited[1].starts_with(provider), "{visited:?}");
    for url in &visited {
        assert!(
            url.starts_with(&service.base) || url.starts_with(provider),
            "{url} is neither Lys's nor the provider's"
        );
        assert!(!url.starts_with(service.issuer.loopback()), "{url}");
    }
    let cookie = cookie.ok_or("the provider sign-in began a session")?;
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 403, "signed in, bound to no person yet: {me}");
    assert_eq!(me["refusal"], "NoPerson");
    Ok(())
}

#[tokio::test]
async fn a_provider_the_issuer_does_not_hold_lands_on_lys_sign_in_by_name() -> TestResult {
    let service = Service::start().await?;
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let answer = browser
        .get(format!(
            "{}/sign-in/providers/unknown-provider",
            service.base
        ))
        .send()
        .await?;
    assert_eq!(answer.status(), 303);
    assert_eq!(
        answer.headers().get(reqwest::header::LOCATION),
        Some(&reqwest::header::HeaderValue::from_static(
            "/#/sign-in?refused=SignInFailed"
        ))
    );
    let forged = browser
        .get(format!(
            "{}/auth/v1/providers/callback?code=forged&state=never-begun",
            service.base
        ))
        .send()
        .await?;
    assert_eq!(
        forged.headers().get(reqwest::header::LOCATION),
        Some(&reqwest::header::HeaderValue::from_static(
            "/#/sign-in?refused=SignInStateUnknown"
        ))
    );
    Ok(())
}

/// The screens' page as the surface ships it, served as the install serves it.
const PAGE: &str = include_str!("../../../surface/identity/index.html");

#[tokio::test]
async fn the_served_sign_in_page_loads_nothing_from_the_issuer() -> TestResult {
    let screens = tempfile::TempDir::new()?;
    std::fs::write(screens.path().join("index.html"), PAGE)?;
    let dir = screens.path().to_path_buf();
    let (service, ()) = Service::start_adjusted(
        identity_contract::harness::GRANT_MODEL,
        None,
        None,
        None,
        |config| config.surface_dir = Some(dir),
        |_| Ok(()),
    )
    .await?;
    let page = reqwest::get(format!("{}/", service.base))
        .await?
        .text()
        .await?;
    assert!(page.contains("<title>Lys</title>"), "{page}");
    let issuer = service.issuer.issuer();
    let mut references = 0;
    for attribute in ["href=\"", "src=\""] {
        for (at, _) in page.match_indices(attribute) {
            let rest = page.get(at + attribute.len()..).unwrap_or_default();
            let target = rest.split('"').next().unwrap_or_default();
            references += 1;
            assert!(!target.starts_with(issuer), "{target}");
            assert!(!target.contains("/auth/v1"), "{target}");
            assert!(
                target.starts_with('/') || target.starts_with("https://fonts."),
                "{target} is neither Lys's own nor the shared fonts"
            );
        }
    }
    assert!(
        references >= 2,
        "the page's stylesheet and script were read"
    );
    Ok(())
}

#[tokio::test]
async fn lys_limits_attempts_even_when_the_issuer_accepts_every_password() -> TestResult {
    let service = Service::start().await?;
    service.issuer.hold(account("person", EMAIL, false))?;
    let browser = reqwest::Client::new();
    for _ in 0..10 {
        let answer = browser
            .post(format!("{}/sign-in", service.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(sign_in_body(EMAIL, PASSWORD))
            .send()
            .await?;
        assert_eq!(answer.status(), 200);
    }
    let answer = browser
        .post(format!("{}/sign-in", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "203.0.113.99")
        .header("forwarded", "for=203.0.113.99")
        .body(sign_in_body(EMAIL, PASSWORD))
        .send()
        .await?;
    assert_eq!(answer.status(), 429);
    assert!(answer.headers().get(reqwest::header::SET_COOKIE).is_none());
    let body: Value = serde_json::from_str(&answer.text().await?)?;
    assert_eq!(body["refusal"], "SignInThrottled");
    Ok(())
}

#[derive(Clone)]
struct IssuerRefusalFixture {
    status: axum::http::StatusCode,
    error: &'static str,
    expires: u64,
    pow: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    authorize: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl IssuerRefusalFixture {
    fn new(status: u16, error: &'static str, expires: u64) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            status: axum::http::StatusCode::from_u16(status)?,
            error,
            expires,
            pow: std::sync::Arc::default(),
            authorize: std::sync::Arc::default(),
        })
    }
}

async fn refusal_challenge(
    axum::extract::State(fixture): axum::extract::State<IssuerRefusalFixture>,
) -> String {
    fixture
        .pow
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("1:10:{}:salt:challenge:", fixture.expires)
}

async fn refuse_authorization(
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
    let discovery = json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/oidc/authorize"),
        "token_endpoint": format!("{base}/oidc/token"),
        "jwks_uri": format!("{base}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
    });
    let routes = axum::Router::new()
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
        .with_state(fixture.clone());
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
        "session_seconds": 60, "secure_cookie": false,
        "grant_log_dir": directory.path(), "grant_log_origin": "test",
        "grant_model_file": directory.path().join("unused-model"),
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
struct RefusalLog {
    text: std::sync::Arc<std::sync::Mutex<String>>,
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
    issuer_refusal(403, "Forbidden", 4_102_444_800, "IssuerRefused", 502, 1).await
}
#[tokio::test]
async fn a_bad_request_names_the_issuer_refusal() -> TestResult {
    issuer_refusal(400, "BadRequest", 4_102_444_800, "IssuerRefused", 502, 1).await
}
#[tokio::test]
async fn an_unauthorized_credential_answer_names_the_password_refusal() -> TestResult {
    issuer_refusal(401, "Unauthorized", 4_102_444_800, "SignInRefused", 401, 1).await
}
#[tokio::test]
async fn a_challenge_already_expired_never_posts_credentials() -> TestResult {
    issuer_refusal(403, "Forbidden", 1, "IssuerChallengeExpired", 503, 0).await
}

#[tokio::test]
async fn the_public_password_route_names_refusals_without_retry_or_private_data() -> TestResult {
    let cases = [
        (403, "Forbidden", 4_102_444_800, "IssuerRefused", 502, 1),
        (400, "BadRequest", 4_102_444_800, "IssuerRefused", 502, 1),
        (401, "Unauthorized", 4_102_444_800, "SignInRefused", 401, 1),
        (401, "Forbidden", 4_102_444_800, "IssuerRefused", 502, 1),
        (429, "TooManyRequests", 4_102_444_800, "SignInThrottled", 429, 1),
        (200, "SecondFactor", 4_102_444_800, "SecondFactorUnsupported", 403, 1),
        (403, "Forbidden", 1, "IssuerChallengeExpired", 503, 0),
    ];
    let mut service = public_sign_in::PublicSignIn::start().await?;
    let outcome = async {
        let mut answers = Vec::with_capacity(cases.len());
        for (status, error, expires, _, _, _) in cases {
            answers.push(service.request(status, error, expires, false).await?);
        }
        let malformed = service.request(401, "Unauthorized", 4_102_444_800, true).await?;
        Ok::<_, Box<dyn Error>>((answers, malformed))
    }
    .await;
    let cleanup = service.close().await;
    let (answers, malformed) = match (outcome, cleanup) {
        (Ok(answers), Ok(())) => answers,
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup)) => {
            return Err(format!("public sign-in fixture failed: {error}; cleanup failed: {cleanup}").into());
        }
    };
    assert_eq!(answers.len(), cases.len());
    for (seen, (status, error, _, named, wire_status, authorize)) in answers.iter().zip(cases) {
        assert_eq!(seen.status, wire_status, "{}", seen.body);
        let body: Value = serde_json::from_str(&seen.body)?;
        assert_eq!(body["refusal"], named);
        assert!(!seen.cookie);
        assert_eq!(seen.pow, 1, "one challenge, no retry");
        assert_eq!(seen.authorize, authorize);
        assert_public_refusal_privacy(seen);
        if named == "IssuerRefused" {
            assert!(seen.body.contains(&status.to_string()) && seen.body.contains(error));
            assert!(seen.log.contains("WARN") && seen.log.contains(error));
            assert!(seen.log.contains(&status.to_string()));
        } else if named == "IssuerChallengeExpired" {
            assert!(seen.body.contains("sign-in again"));
            assert!(seen.log.contains("WARN") && seen.log.contains(named));
        }
    }
    assert_eq!(malformed.status, 400, "{}", malformed.body);
    let body: Value = serde_json::from_str(&malformed.body)?;
    assert_eq!(body["refusal"], "RequestMalformed");
    assert!(!malformed.cookie);
    assert_eq!(malformed.pow, 0);
    assert_eq!(malformed.authorize, 0);
    assert_public_refusal_privacy(&malformed);
    Ok(())
}

fn assert_public_refusal_privacy(seen: &public_sign_in::Observed) {
    for sentinel in [
        "issuer-private-message-sentinel",
        "issuer-private-email-sentinel",
        "issuer-private-token-sentinel",
        "attempt-private-email-sentinel",
        "attempt-private-password-sentinel",
        "fixture-request-token",
    ] {
        assert!(!seen.body.contains(sentinel), "private data in public refusal body");
        assert!(!seen.log.contains(sentinel), "private data in public refusal log");
    }
}

#[tokio::test]
async fn the_public_provider_start_refuses_expiry_before_posting_to_the_provider() -> TestResult {
    let mut service = public_sign_in::PublicSignIn::start().await?;
    let outcome = async {
        let expired = service.provider_start(1).await?;
        let live = service.provider_start(4_102_444_800).await?;
        Ok::<_, Box<dyn Error>>((expired, live))
    }
    .await;
    let cleanup = service.close().await;
    let (expired, live) = match (outcome, cleanup) {
        (Ok(answers), Ok(())) => answers,
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup)) => {
            return Err(format!("public provider fixture failed: {error}; cleanup failed: {cleanup}").into());
        }
    };
    assert_eq!(expired.status, 303);
    assert_eq!(
        expired.location.as_deref(),
        Some("/#/sign-in?refused=IssuerChallengeExpired")
    );
    assert!(!expired.cookie);
    assert_eq!(expired.pow, 1, "one challenge, no retry");
    assert_eq!(expired.authorize, 0);
    assert_eq!(expired.provider_posts, 0);
    assert!(expired.log.contains("WARN") && expired.log.contains("IssuerChallengeExpired"));
    assert_public_refusal_privacy(&expired);

    assert_eq!(live.status, 303);
    assert!(live.cookie);
    assert_eq!(live.pow, 1, "one challenge, no retry");
    assert_eq!(live.authorize, 0);
    assert_eq!(live.provider_posts, 1);
    let location = reqwest::Url::parse(live.location.as_deref().ok_or("live provider redirect missing")?)?;
    assert_eq!(location.scheme(), "https");
    assert_eq!(location.host_str(), Some("provider.example.test"));
    assert_eq!(location.path(), "/login");
    let query: Vec<_> = location.query_pairs().collect();
    assert_eq!(query.len(), 2);
    assert!(query.iter().any(|(name, value)| name == "redirect_uri"
        && value == "http://127.0.0.1/auth/v1/providers/callback"));
    assert!(query.iter().any(|(name, value)| name == "state" && value == "fixture-provider-state"));
    assert!(!live.log.contains("IssuerChallengeExpired"));
    assert_public_refusal_privacy(&live);
    Ok(())
}
