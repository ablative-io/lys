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
    service.issuer.hold(account("grace-subject", EMAIL, false));
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
    let forwarded = service.issuer.forwarded();
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
    service.issuer.hold(account("grace-subject", EMAIL, false));
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
    service.issuer.hold(account("passkey-subject", EMAIL, true));
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
    service.issuer.hold(account("grace-subject", EMAIL, false));
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
    let forwarded = service.issuer.forwarded();
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
    });
    let browser = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut visited = Vec::new();
    let mut cookie = None;
    let mut next = format!("{}/sign-in/providers/google-provider", service.base);
    for _ in 0..3 {
        visited.push(next.clone());
        let answer = browser.get(&next).send().await?;
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
