//! A person's own account and the administrator's administration of
//! accounts, carried out by the service at the issuer (DIRECTORY-047 R5): a
//! changed email signs in and the old one does not, a password reset by the
//! administrator signs in and the old password does not, and a disabled
//! person cannot sign in until enabled again.

use std::error::Error;

use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use lys_identity::OperationId;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "accounts-first-run-code";
const EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";

/// A service whose administrator Ada was made on the setup page, signed in.
async fn table() -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) = Service::start_adjusted(
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
            let digest: Vec<String> = Sha256::digest(CODE.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let pending = json!({ "purpose": "first-run", "sha256": digest.concat() });
            std::fs::write(&setup.code_file, pending.to_string())?;
            Ok(())
        },
    )
    .await?;
    rauthy.link(service.issuer.clone());
    let made = reqwest::Client::new()
        .post(format!("{}/setup/administrator", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "code": CODE,
                "operation": OperationId::generate()?.to_string(),
                "display_name": "Ada Lovelace",
                "email": EMAIL,
                "password": PASSWORD,
            })
            .to_string(),
        )
        .send()
        .await?;
    let ada = session_cookie(made).await?;
    Ok((service, rauthy, ada))
}

/// Call the issuer's administration API as the installation's key does.
async fn issuer_api(
    rauthy: &FakeRauthy,
    method: reqwest::Method,
    path: &str,
    body: &Value,
) -> Result<Value, Box<dyn Error>> {
    let text = reqwest::Client::new()
        .request(method, format!("{}{path}", rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?
        .text()
        .await?;
    Ok(serde_json::from_str(&text)?)
}

/// Register Bea in the directory with an account of her own at the issuer,
/// answering her person id.
async fn bea(service: &Service, rauthy: &FakeRauthy, ada: &str) -> Result<String, Box<dyn Error>> {
    let made = issuer_api(
        rauthy,
        reqwest::Method::POST,
        "/users",
        &json!({ "email": "bea@example.test" }),
    )
    .await?;
    let subject = made["id"]
        .as_str()
        .ok_or("the issuer names the account")?
        .to_owned();
    let mut account = made.clone();
    account["password"] = json!("Bea-Password-000111");
    account["enabled"] = json!(true);
    issuer_api(
        rauthy,
        reqwest::Method::PUT,
        &format!("/users/{subject}"),
        &account,
    )
    .await?;
    let (status, person) = service
        .post(
            "/people",
            Some(ada),
            &json!({ "operation": OperationId::generate()?.to_string(), "display_name": "Bea" }),
        )
        .await?;
    assert_eq!(status, 200, "{person}");
    let id = person["person"].as_str().ok_or("a person")?.to_owned();
    let bound = json!({
        "operation": OperationId::generate()?.to_string(),
        "issuer": service.issuer.issuer(),
        "subject": subject,
    });
    let (status, answer) = service
        .post(&format!("/people/{id}/logins"), Some(ada), &bound)
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(id)
}

#[tokio::test]
async fn a_changed_email_signs_in_and_the_old_one_does_not() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let (status, shown) = service.get("/me/account", Some(&ada)).await?;
    assert_eq!(status, 200, "{shown}");
    assert_eq!(shown["email"], EMAIL);
    let wrong = service
        .post(
            "/me/account/email",
            Some(&ada),
            &json!({ "email": "ada@lovelace.test", "password": "Not-The-Password-9" }),
        )
        .await?;
    assert_eq!(wrong.0, 400, "{}", wrong.1);
    assert_eq!(wrong.1["refusal"], "AccountRefused");
    let (status, changed) = service
        .post(
            "/me/account/email",
            Some(&ada),
            &json!({ "email": "ada@lovelace.test", "password": PASSWORD }),
        )
        .await?;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["email"], "ada@lovelace.test");
    let signed_in = service.sign_in_with("ada@lovelace.test", PASSWORD).await?;
    assert_eq!(service.get("/me", Some(&signed_in)).await?.0, 200);
    assert!(
        service.sign_in_with(EMAIL, PASSWORD).await.is_err(),
        "the old email no longer signs in"
    );
    Ok(())
}

#[tokio::test]
async fn a_changed_password_needs_the_current_one() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let renewed = "Difference-Engine-1822";
    let wrong = service
        .post(
            "/me/account/password",
            Some(&ada),
            &json!({ "current": "Not-The-Password-9", "password": renewed }),
        )
        .await?;
    assert_eq!(wrong.1["refusal"], "AccountRefused", "{}", wrong.1);
    let (status, answer) = service
        .post(
            "/me/account/password",
            Some(&ada),
            &json!({ "current": PASSWORD, "password": renewed }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    service.sign_in_with(EMAIL, renewed).await?;
    assert!(service.sign_in_with(EMAIL, PASSWORD).await.is_err());
    Ok(())
}

#[tokio::test]
async fn the_administrator_resets_a_password_and_disables_a_person() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let bea = bea(&service, &rauthy, &ada).await?;
    service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let (status, reset) = service
        .post(
            &format!("/directory/people/{bea}/account/password"),
            Some(&ada),
            &json!({ "password": "Bea-New-Password-222" }),
        )
        .await?;
    assert_eq!(status, 200, "{reset}");
    let bea_session = service
        .sign_in_with("bea@example.test", "Bea-New-Password-222")
        .await?;
    assert!(
        service
            .sign_in_with("bea@example.test", "Bea-Password-000111")
            .await
            .is_err()
    );

    let not_admin = service
        .post(
            &format!("/directory/people/{bea}/account/enabled"),
            Some(&bea_session),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(not_admin.1["refusal"], "NotAdmitted", "{}", not_admin.1);
    let (status, off) = service
        .post(
            &format!("/directory/people/{bea}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(status, 200, "{off}");
    assert_eq!(off["enabled"], false);
    assert!(
        service
            .sign_in_with("bea@example.test", "Bea-New-Password-222")
            .await
            .is_err()
    );
    let (status, on) = service
        .post(
            &format!("/directory/people/{bea}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": true }),
        )
        .await?;
    assert_eq!(status, 200, "{on}");
    service
        .sign_in_with("bea@example.test", "Bea-New-Password-222")
        .await?;

    let (status, moved) = service
        .post(
            &format!("/directory/people/{bea}/account/email"),
            Some(&ada),
            &json!({ "email": "bea@new.example.test" }),
        )
        .await?;
    assert_eq!(status, 200, "{moved}");
    service
        .sign_in_with("bea@new.example.test", "Bea-New-Password-222")
        .await?;
    Ok(())
}
