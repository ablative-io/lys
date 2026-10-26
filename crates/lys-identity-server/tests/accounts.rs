#![cfg(test)]

//! A person's own account and the administrator's administration of
//! accounts, carried out by the service at the issuer (DIRECTORY-047 R5): a
//! changed email signs in and the old one does not, a password reset by the
//! administrator signs in and the old password does not, and a disabled
//! person cannot sign in until enabled again.

use std::error::Error;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};

use identity_contract::apps::{Auth, approve, ok, post, registration, workspace_schema};
use identity_contract::fake_rauthy::{API_KEY, FakeRauthy};
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::accounts;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::provider::ProviderSettings;
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::{SignInProviders, SignInProvidersSettings};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "accounts-first-run-code";
const EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";
/// The fixture product: an app the administrator registers and approves in
/// `table`, served by the provider from the apps' record.
const PRODUCT: &str = "accounts_fixture";
const CALLBACK: &str = "https://product.example.test/callback";

/// The product's client secret, as the custody fixture issued it at approval.
fn secret() -> String {
    identity_contract::app_custody::secret()
}

fn held_account() -> Value {
    json!({
        "email": EMAIL,
        "given_name": "Ada",
        "family_name": "Lovelace",
        "language": "en",
        "roles": ["staff"],
        "groups": ["accounts"],
        "enabled": true,
        "email_verified": true,
        "user_expires": 2_000_000_000,
        "user_values": {"city": "Melbourne", "phone": "+61000000000"},
    })
}

fn account_api(issuer: &FakeRauthy) -> Result<SignInProviders, Box<dyn Error>> {
    Ok(SignInProviders::open(
        &SignInProvidersSettings {
            api: issuer.api().to_owned(),
            api_key_file: issuer.api_key_file(),
        },
        None,
    )?)
}

#[tokio::test]
async fn account_update_refuses_each_missing_mandatory_member_before_put() -> TestResult {
    let issuer = FakeRauthy::start().await?;
    let api = account_api(&issuer)?;
    let mut checked = 0;
    for field in [
        "email",
        "language",
        "roles",
        "enabled",
        "email_verified",
        "user_values",
    ] {
        let mut user =
            issuer_api(&issuer, reqwest::Method::POST, "/users", &held_account()).await?;
        let id = user["id"]
            .as_str()
            .ok_or("the issuer names the account")?
            .to_owned();
        user.as_object_mut()
            .ok_or("an account object")?
            .remove(field);
        issuer_api(
            &issuer,
            reqwest::Method::PUT,
            &format!("/users/{id}"),
            &user,
        )
        .await?;
        let before = issuer.users()?;
        let requests = issuer.request_count();
        let mut changed = false;
        let answer = accounts::change(&api, &id, |update| {
            changed = true;
            update["enabled"] = json!(false);
        })
        .await;
        let refusal = answer
            .err()
            .ok_or("an incomplete account must be refused")?;
        assert_eq!(
            refusal.name(),
            "SignInProvidersUnavailable",
            "{field}: {refusal}"
        );
        assert!(refusal.to_string().contains(field), "{field}: {refusal}");
        assert!(!changed, "{field}: refuse before applying the change");
        assert_eq!(
            issuer.request_count(),
            requests + 1,
            "{field}: only GET reaches the issuer"
        );
        assert_eq!(
            issuer.users()?,
            before,
            "{field}: held records survive without a PUT"
        );
        checked += 1;
    }
    assert_eq!(checked, 6);
    Ok(())
}

#[tokio::test]
async fn account_update_preserves_other_values_and_never_copies_a_held_password() -> TestResult {
    let issuer = FakeRauthy::start().await?;
    let api = account_api(&issuer)?;
    let mut request = held_account();
    request["password"] = json!("held-password-must-not-be-sent");
    request["created_at"] = json!(123);
    let mut expected = issuer_api(&issuer, reqwest::Method::POST, "/users", &request).await?;
    let id = expected["id"]
        .as_str()
        .ok_or("the issuer names the account")?
        .to_owned();
    let fields = expected.as_object_mut().ok_or("an account object")?;
    fields.remove("password");
    fields.remove("created_at");
    fields.insert("enabled".to_owned(), json!(false));
    accounts::change(&api, &id, |update| update["enabled"] = json!(false)).await?;
    assert_eq!(issuer.users()?, vec![expected]);
    Ok(())
}

#[tokio::test]
async fn account_update_keeps_absent_optionals_distinct_from_explicit_null() -> TestResult {
    let issuer = FakeRauthy::start().await?;
    let api = account_api(&issuer)?;
    let mut checked = 0;
    for present in [false, true] {
        let mut request = held_account();
        let fields = request.as_object_mut().ok_or("an account object")?;
        for field in ["given_name", "family_name", "groups", "user_expires"] {
            if present {
                fields.insert(field.to_owned(), Value::Null);
            } else {
                fields.remove(field);
            }
        }
        let mut expected = issuer_api(&issuer, reqwest::Method::POST, "/users", &request).await?;
        let id = expected["id"]
            .as_str()
            .ok_or("the issuer names the account")?
            .to_owned();
        expected["enabled"] = json!(false);
        accounts::change(&api, &id, |update| update["enabled"] = json!(false)).await?;
        let held = issuer.users()?;
        let actual = held
            .iter()
            .find(|user| user["id"] == id)
            .ok_or("the account is still held")?;
        assert_eq!(actual, &expected, "explicit null present: {present}");
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}

/// A service whose administrator Ada was made on the setup page, signed in.
async fn table() -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    let (service, api, cookie, _sink) = table_with_mail(false).await?;
    Ok((service, api, cookie))
}

async fn table_with_mail(
    mail: bool,
) -> Result<
    (
        Service,
        FakeRauthy,
        String,
        identity_contract::mail_sink::MailSink,
    ),
    Box<dyn Error>,
> {
    let sink = identity_contract::mail_sink::MailSink::start().await?;
    let port = sink.port();
    let rauthy = FakeRauthy::start().await?;
    let broker = identity_contract::app_custody::start().await?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        move |config| {
            config.password_policy = Some(changes::fixture_password_policy());
            config.mail = mail.then(|| lys_identity_server::mail::MailSettings {
                host: "127.0.0.1".to_owned(),
                port,
                user: None,
                password_file: None,
                from: "Lys <lys@example.test>".to_owned(),
                tls: lys_identity_server::mail::Tls::None,
                sent_file: config.log_dir.with_file_name("letters-sent.jsonl"),
            });
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds: 60,
                pass_seconds: lys_identity_server::provider::PASS_SECONDS,
                rights_bytes: None,
            });
            config.administrator = None;
            config.setup = Some(SetupSettings {
                code_file: config.log_dir.with_file_name("setup-code"),
                administrator_file: config.log_dir.with_file_name("administrator.json"),
                email: None,
                organisation: "Example organisation".to_owned(),
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
            std::fs::write(config.log_dir.with_file_name("provider.key"), [5u8; 32])?;
            let key = Arc::new(load_service_key(&config.event_key_file)?);
            drop(open_directory(config)?);
            drop(ConfigurationStore::open(
                &config.log_dir.with_file_name("organisation"),
                Arc::clone(&key),
            )?);
            drop(ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            drop(lys_identity::start::LaunchRecords::open(
                &config.log_dir.with_file_name("launch-records"),
                load_service_key(&config.event_key_file)?,
            )?);
            drop(lys_identity_server::apps_api::opened(config, key, &|_| {})?);
            Ok(())
        },
    )
    .await?;
    rauthy.link(service.issuer.clone())?;
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
    // The product is an app Ada registers and approves; the provider reads it
    // from the apps' record at each request.
    let mut body = registration(PRODUCT, &workspace_schema(PRODUCT))?;
    body["redirects"] = json!([CALLBACK]);
    ok(post(&service, "/apps", Auth::Cookie(&ada), &body).await?)?;
    approve(&service, &ada, PRODUCT).await?;
    Ok((service, rauthy, ada, sink))
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
    // This test exercises an Active non-administrator's account permissions.
    let (status, answer) = service
        .post(
            &format!("/identities/{id}/transitions"),
            Some(ada),
            &json!({ "operation": OperationId::generate()?.to_string(), "transition": "activate" }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok(id)
}

#[path = "accounts/changes.rs"]
mod changes;
