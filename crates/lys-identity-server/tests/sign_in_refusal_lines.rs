#![cfg(test)]

//! DIRECTORY-099 R2: every refused person sign-in, on Lys's own sign-in
//! route and on the provider callback, is one WARN line in the service's log
//! naming the route, the refusal and the handle tried when it is a
//! well-formed email, and never the password. A sign-in that succeeds says
//! no such line.

use std::error::Error;
use std::sync::{Arc, Mutex};

use identity_contract::fake_issuer::{Account, Login};
use identity_contract::harness::{GRANT_MODEL, Service};
use lys_identity_server::Say;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const EMAIL: &str = "grace@example.test";
const PASSWORD: &str = "Compiler-Pioneer-1952";
const WRONG: &str = "Not-The-Password-1";
const REFUSED: &str = "a person's sign-in was refused";

/// Every line the service said, its own and its WARN events, kept in order.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn say(&self) -> Say {
        let lines = Arc::clone(&self.0);
        Arc::new(move |line: &str| match lines.lock() {
            Ok(mut lines) => lines.push(line.to_owned()),
            Err(poisoned) => panic!("sign_in_refusal_fixture_poisoned: {poisoned}"),
        })
    }

    fn lines(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(self
            .0
            .lock()
            .map_err(|poisoned| poisoned.to_string())?
            .clone())
    }

    /// The refused-sign-in lines said so far.
    fn refusals(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(self
            .lines()?
            .into_iter()
            .filter(|line| line.contains(REFUSED))
            .collect())
    }

    /// No line said holds either password the tests type.
    fn holds_no_password(&self) -> TestResult {
        for line in self.lines()? {
            assert!(
                !line.contains(PASSWORD) && !line.contains(WRONG),
                "a password reached the log: {line}"
            );
        }
        Ok(())
    }
}

/// The service, saying its lines and its WARN events to one capture, with
/// Grace's account held at the issuer.
async fn started() -> Result<(Service, Said), Box<dyn Error>> {
    let said = Said::default();
    lys_identity_server::warn_lines::install(said.say())?;
    let (service, ()) = Service::start_saying(
        GRANT_MODEL,
        None,
        None,
        None,
        |_| {},
        Some(said.say()),
        |_| Ok(()),
    )
    .await?;
    service.issuer.hold(Account {
        login: Login {
            subject: "grace-subject".to_owned(),
            email: EMAIL.to_owned(),
        },
        password: PASSWORD.to_owned(),
        second_factor: false,
    })?;
    Ok((service, said))
}

/// POST /api/sign-in with `email` and `password`: the status and the body.
async fn posted(
    service: &Service,
    email: &str,
    password: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/sign-in", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json!({ "email": email, "password": password }).to_string())
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, serde_json::from_str(&answer.text().await?)?))
}

/// The one refused-sign-in line said so far, which must hold `expected`.
fn one_line_holding(said: &Said, expected: &str) -> TestResult {
    let lines = said.refusals()?;
    assert_eq!(lines.len(), 1, "{lines:#?}");
    assert!(lines[0].contains(expected), "{lines:#?}");
    assert!(
        lines[0].starts_with("WARN lys_identity_server::sign_in_refusal_line: "),
        "{lines:#?}"
    );
    Ok(())
}

#[tokio::test]
async fn a_wrong_password_for_a_held_account_says_one_line_naming_its_email() -> TestResult {
    let (service, said) = started().await?;
    let (status, body) = posted(&service, EMAIL, WRONG).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "SignInRefused");
    one_line_holding(
        &said,
        "refusal=SignInRefused route=POST /api/sign-in handle=grace@example.test",
    )?;
    said.holds_no_password()
}

#[tokio::test]
async fn an_unknown_email_says_one_line_naming_it_trimmed_and_lowercased() -> TestResult {
    let (service, said) = started().await?;
    let (status, body) = posted(&service, " Nobody@Example.TEST ", WRONG).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "SignInRefused");
    one_line_holding(
        &said,
        "refusal=SignInRefused route=POST /api/sign-in handle=nobody@example.test",
    )?;
    said.holds_no_password()
}

#[tokio::test]
async fn an_empty_field_says_one_line_each() -> TestResult {
    let (service, said) = started().await?;
    let (status, body) = posted(&service, EMAIL, "").await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "SignInRefused");
    one_line_holding(
        &said,
        "refusal=SignInRefused route=POST /api/sign-in handle=grace@example.test",
    )?;
    let (status, body) = posted(&service, "  ", WRONG).await?;
    assert_eq!(status, 401, "{body}");
    let lines = said.refusals()?;
    assert_eq!(lines.len(), 2, "{lines:#?}");
    assert!(
        lines[1].contains("refusal=SignInRefused route=POST /api/sign-in handle=none"),
        "{lines:#?}"
    );
    said.holds_no_password()
}

#[tokio::test]
async fn a_password_typed_into_the_email_box_is_never_written() -> TestResult {
    let (service, said) = started().await?;
    let (status, body) = posted(&service, WRONG, PASSWORD).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "SignInRefused");
    one_line_holding(
        &said,
        "refusal=SignInRefused route=POST /api/sign-in handle=not-an-email",
    )?;
    said.holds_no_password()
}

#[tokio::test]
async fn a_sign_in_that_succeeds_says_no_refusal_line() -> TestResult {
    let (service, said) = started().await?;
    let (status, body) = posted(&service, EMAIL, PASSWORD).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["signed_in"]["subject"], "grace-subject");
    let lines = said.refusals()?;
    assert!(lines.is_empty(), "{lines:#?}");
    said.holds_no_password()
}

#[tokio::test]
async fn a_provider_callback_the_issuer_will_not_admit_says_one_line_naming_its_email()
-> TestResult {
    let (service, said) = started().await?;
    service.issuer.refuse_provider_callback(json!({
        "error": "NotFound",
        "message": "User not found",
        "email": "callie@example.test",
        "access_token": "callback-private-token-sentinel",
    }))?;
    let (back, binding) = service
        .issuer_answer(Login {
            subject: "callie-subject".to_owned(),
            email: "callie@example.test".to_owned(),
        })
        .await?;
    let (status, to, _) = service.get_page(&back, &binding).await?;
    assert_eq!(status, 303);
    assert_eq!(
        to.as_deref(),
        Some("/#/sign-in?refused=NotAdmittedYet&email=callie%40example.test")
    );
    one_line_holding(
        &said,
        "refusal=NotAdmittedYet route=GET /auth/v1/providers/callback handle=callie@example.test",
    )?;
    for line in said.lines()? {
        assert!(
            !line.contains("callback-private-token-sentinel"),
            "an issuer body reached the log: {line}"
        );
    }
    said.holds_no_password()
}

/// The provider routes write their refusals through the WARN sink alone:
/// nothing in sign_in_upstream.rs writes to the process's stderr.
#[test]
fn the_provider_routes_write_nothing_to_stderr() {
    let source = include_str!("../src/sign_in_upstream.rs");
    for writer in ["eprintln!", "eprint!", "stderr()"] {
        assert!(
            !source.contains(writer),
            "sign_in_upstream.rs writes to stderr with {writer}"
        );
    }
}
