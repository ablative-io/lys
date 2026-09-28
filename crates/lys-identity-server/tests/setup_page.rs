//! First-run setup on Lys's own setup page (DIRECTORY-047 R1): with no
//! administrator configured, the page takes the person's name, email and
//! password against a one-time code, makes exactly one account at the
//! issuer, makes that person the administrator, signs them in, and the code
//! is then used. A password code sets the administrator a new password.
//!
//! The code file is written here by hand from its documented format, apart
//! from the install that writes it in use, so the service is checked against
//! a second writer of the format.

use std::error::Error;
use std::path::{Path, PathBuf};

use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use lys_identity::OperationId;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "k3Jd9-first-run-code-Qm2";
const EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";

fn hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    digits.concat()
}

/// Write the pending code file as `lys identity install` writes it.
fn write_code(path: &Path, purpose: &str, code: &str) -> TestResult {
    let pending = json!({ "purpose": purpose, "sha256": hex(&Sha256::digest(code.as_bytes())) });
    std::fs::write(path, pending.to_string())?;
    Ok(())
}

struct Table {
    service: Service,
    rauthy: FakeRauthy,
    code_file: PathBuf,
    administrator_file: PathBuf,
}

/// A service with no administrator, first-run setup configured, and a
/// first-run code pending.
async fn table() -> Result<Table, Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, (code_file, administrator_file)) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        |config| {
            config.administrator = None;
            config.setup = Some(SetupSettings {
                code_file: config.log_dir.with_file_name("setup-code"),
                administrator_file: config.log_dir.with_file_name("administrator.json"),
                email: None,
            });
        },
        |config| {
            let setup = config.setup.as_ref().ok_or("setup is configured")?;
            write_code(&setup.code_file, "first-run", CODE)?;
            Ok((setup.code_file.clone(), setup.administrator_file.clone()))
        },
    )
    .await?;
    rauthy.link(service.issuer.clone());
    Ok(Table {
        service,
        rauthy,
        code_file,
        administrator_file,
    })
}

fn administrator(code: &str, email: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "code": code,
        "operation": OperationId::generate()?.to_string(),
        "display_name": "Ada Lovelace",
        "email": email,
        "password": PASSWORD,
    }))
}

/// POST `body` to `path` as a browser does, answering the status, the body
/// and the session cookie the answer began, if it began one.
async fn post_raw(
    service: &Service,
    path: &str,
    body: &Value,
) -> Result<(u16, String, Option<String>), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}{path}", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?;
    let status = answer.status().as_u16();
    let cookie = answer
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned);
    Ok((status, answer.text().await?, cookie))
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

#[tokio::test]
async fn the_setup_route_answers_the_setup_screen_for_the_pending_code_only() -> TestResult {
    let table = table().await?;
    let service = &table.service;
    let (status, opened) = service
        .post("/setup/open", None, &json!({ "code": CODE }))
        .await?;
    assert_eq!(status, 200, "{opened}");
    assert_eq!(opened["purpose"], "first-run");
    assert_eq!(
        opened["email"],
        Value::Null,
        "no email is filled in for the person"
    );
    assert_eq!(opened["policy"]["length_min"], 14);
    let wrong = service
        .post("/setup/open", None, &json!({ "code": "not-the-code" }))
        .await?;
    refused(&wrong, 401, "SetupCodeRefused");
    assert_eq!(
        table.rauthy.user_count(),
        0,
        "opening the page makes nothing"
    );
    Ok(())
}

#[tokio::test]
async fn a_setup_retried_after_its_account_was_made_signs_in_with_the_password_it_set() -> TestResult
{
    let table = table().await?;
    let key = format!("API-Key {API_KEY}");
    let send = |method: reqwest::Method, path: String, body: &Value| {
        reqwest::Client::new()
            .request(method, format!("{}{path}", table.rauthy.api()))
            .header(reqwest::header::AUTHORIZATION, &key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
    };
    let made = send(
        reqwest::Method::POST,
        "/users".to_owned(),
        &json!({ "email": EMAIL, "given_name": "Ada Lovelace" }),
    )
    .await?
    .text()
    .await?;
    let made: Value = serde_json::from_str(&made)?;
    let id = made["id"].as_str().ok_or("an account id")?;
    let password = json!({ "email": EMAIL, "password": PASSWORD, "enabled": true });
    let set = send(reqwest::Method::PUT, format!("/users/{id}"), &password).await?;
    assert_eq!(set.status(), 200, "the earlier attempt set the password");
    let again = send(reqwest::Method::PUT, format!("/users/{id}"), &password).await?;
    assert_eq!(
        again.status(),
        400,
        "the issuer refuses a password set again"
    );

    let (status, text, cookie) = post_raw(
        &table.service,
        "/setup/administrator",
        &administrator(CODE, EMAIL)?,
    )
    .await?;
    assert_eq!(status, 200, "{text}");
    assert!(cookie.is_some(), "the retried setup signs the person in");
    assert_eq!(
        table.rauthy.user_count(),
        1,
        "the account is found, not made again"
    );
    assert_eq!(table.rauthy.users()[0]["id"], id);
    Ok(())
}

#[tokio::test]
async fn completing_setup_makes_one_administrator_and_signs_them_in() -> TestResult {
    let table = table().await?;
    let service = &table.service;
    assert_eq!(service.get("/directory/people", None).await?.0, 401);
    let (status, text, cookie) = post_raw(
        service,
        "/setup/administrator",
        &administrator(CODE, EMAIL)?,
    )
    .await?;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text)?;
    assert_eq!(answer["receipt"]["change_kind"], 7);
    let cookie = cookie.ok_or("setup signs the person in")?;

    let users = table.rauthy.users();
    assert_eq!(users.len(), 1, "exactly one account is made: {users:?}");
    assert_eq!(users[0]["email"], EMAIL);
    assert!(
        !serde_json::to_string(&users)?.contains(PASSWORD),
        "the password is not kept by the account"
    );
    let (status, me) = service.get("/me", Some(&cookie)).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["person"]["state"], "active");
    assert_eq!(me["signed_in"]["subject"], users[0]["id"]);
    let (status, people) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(status, 200, "{people}");
    assert_eq!(
        people["people"].as_array().map(Vec::len),
        Some(1),
        "{people}"
    );

    assert!(!table.code_file.exists(), "the used code is removed");
    let recorded = std::fs::read_to_string(&table.administrator_file)?;
    let recorded: Value = serde_json::from_str(&recorded)?;
    assert_eq!(recorded["subject"], users[0]["id"]);
    assert!(
        !std::fs::read_to_string(&table.administrator_file)?.contains(EMAIL),
        "the administrator is recorded by login, never by email"
    );

    let again = service
        .post(
            "/setup/administrator",
            None,
            &administrator(CODE, "eve@example.test")?,
        )
        .await?;
    refused(&again, 409, "SetupClosed");
    let reopened = service
        .post("/setup/open", None, &json!({ "code": CODE }))
        .await?;
    refused(&reopened, 409, "SetupClosed");
    assert_eq!(table.rauthy.user_count(), 1, "a closed setup makes nothing");
    let signed_in = service.sign_in_with(EMAIL, PASSWORD).await?;
    assert_eq!(service.get("/me", Some(&signed_in)).await?.0, 200);
    Ok(())
}

#[tokio::test]
async fn a_wrong_code_makes_nothing_and_a_weak_password_is_refused_before_anything_is_sent()
-> TestResult {
    let table = table().await?;
    let service = &table.service;
    let wrong = service
        .post(
            "/setup/administrator",
            None,
            &administrator("guess", EMAIL)?,
        )
        .await?;
    refused(&wrong, 401, "SetupCodeRefused");
    let mut weak = administrator(CODE, EMAIL)?;
    weak["password"] = json!("short");
    let weak = service.post("/setup/administrator", None, &weak).await?;
    refused(&weak, 400, "AccountRefused");
    let mut unshaped = administrator(CODE, EMAIL)?;
    unshaped["email"] = json!("ada");
    let unshaped = service
        .post("/setup/administrator", None, &unshaped)
        .await?;
    refused(&unshaped, 400, "AccountRefused");
    assert_eq!(table.rauthy.user_count(), 0, "nothing was made");
    assert!(table.code_file.exists(), "an unused code stays");
    Ok(())
}

#[tokio::test]
async fn a_password_code_sets_the_administrator_a_new_password() -> TestResult {
    let table = table().await?;
    let service = &table.service;
    let (status, text, _) = post_raw(
        service,
        "/setup/administrator",
        &administrator(CODE, EMAIL)?,
    )
    .await?;
    assert_eq!(status, 200, "{text}");
    let used = service
        .post(
            "/setup/password",
            None,
            &json!({ "code": CODE, "password": "Another-Password-99" }),
        )
        .await?;
    refused(&used, 401, "SetupCodeRefused");

    let fresh = "fresh-password-code-7Tq";
    write_code(&table.code_file, "password", fresh)?;
    let (status, opened) = service
        .post("/setup/open", None, &json!({ "code": fresh }))
        .await?;
    assert_eq!(status, 200, "{opened}");
    assert_eq!(opened["purpose"], "password");
    assert_eq!(
        opened["email"], EMAIL,
        "the page names whose password is set"
    );
    let renewed = "Difference-Engine-1822";
    let reset = reqwest::Client::new()
        .post(format!("{}/setup/password", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(json!({ "code": fresh, "password": renewed }).to_string())
        .send()
        .await?;
    let cookie = session_cookie(reset).await?;
    assert_eq!(service.get("/me", Some(&cookie)).await?.0, 200);
    assert!(!table.code_file.exists(), "the password code is used");
    let old = service.sign_in_with(EMAIL, PASSWORD).await;
    assert!(old.is_err(), "the old password no longer signs in");
    service.sign_in_with(EMAIL, renewed).await?;
    assert_eq!(table.rauthy.user_count(), 1, "no second account is made");
    Ok(())
}
