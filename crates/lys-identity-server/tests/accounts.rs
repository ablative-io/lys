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
            config.password_policy = Some(accounts::PasswordPolicy {
                length_min: 14,
                length_max: 128,
                lower_case: Some(1),
                upper_case: Some(1),
                digits: Some(1),
                special: None,
                not_recently_used: None,
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

async fn product_code(
    service: &Service,
    cookie: &str,
    verifier: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let answer = client
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", PRODUCT),
            ("redirect_uri", CALLBACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("state", "accounts-product"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(answer.status().as_u16(), 303);
    let back = reqwest::Url::parse(
        answer
            .headers()
            .get(reqwest::header::LOCATION)
            .ok_or("missing product redirect")?
            .to_str()?,
    )?;
    back.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| "missing product code".into())
}

async fn product_token(
    service: &Service,
    code: &str,
    verifier: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!("{PRODUCT}:{}", secret()))
            ),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", CALLBACK),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;
    Ok((answer.status().as_u16(), answer.json().await?))
}

async fn product_user(service: &Service, token: &str) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(token)
        .send()
        .await?;
    Ok((answer.status().as_u16(), answer.json().await?))
}

#[tokio::test]
async fn disabling_an_account_revokes_existing_sessions_codes_and_tokens_across_restart()
-> TestResult {
    let (mut service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let cookie = service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let verifier = "accounts-product-verifier-of-enough-length-0123456789";
    let code = product_code(&service, &cookie, verifier).await?;
    let issued = product_token(&service, &code, verifier).await?;
    assert_eq!(issued.0, 200);
    let token = issued.1["access_token"]
        .as_str()
        .ok_or("no access token")?
        .to_owned();
    assert_eq!(product_user(&service, &token).await?.0, 200);
    let pending = product_code(&service, &cookie, verifier).await?;
    let disabled = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(disabled.0, 200, "{}", disabled.1);
    assert_eq!(disabled.1["enabled"], false);
    let kept: Value = serde_json::from_slice(&std::fs::read(
        service.dir.path().join("provider.tokens.json"),
    )?)?;
    assert!(
        kept["tokens"]
            .as_array()
            .ok_or("no stored access table")?
            .is_empty()
    );
    let signed_out = service.get("/me", Some(&cookie)).await?;
    assert_eq!(signed_out.0, 401, "{}", signed_out.1);
    assert_eq!(signed_out.1["refusal"], "NotSignedIn");
    let revoked = product_user(&service, &token).await?;
    assert_eq!(revoked.0, 401, "{}", revoked.1);
    assert_eq!(revoked.1["refusal"], "TokenUnknown");
    let code_refused = product_token(&service, &pending, verifier).await?;
    assert_eq!(code_refused.0, 400, "{}", code_refused.1);
    assert_eq!(code_refused.1["error"], "invalid_grant");
    assert_eq!(code_refused.1["refusal"], "CodeUnknown");
    service.restart().await?;
    let reopened = service.get("/me", Some(&cookie)).await?;
    assert_eq!(reopened.0, 401, "{}", reopened.1);
    assert_eq!(product_user(&service, &token).await?.0, 401);
    assert!(
        service
            .sign_in_with("bea@example.test", "Bea-Password-000111")
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn a_callback_prepared_before_disable_cannot_create_a_new_session() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let user = rauthy
        .users()?
        .into_iter()
        .find(|user| user["email"] == "bea@example.test")
        .ok_or("account missing")?;
    let subject = user["id"].as_str().ok_or("account id missing")?.to_owned();
    let (callback, binding) = service
        .issuer_answer(identity_contract::fake_issuer::Login {
            subject,
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let disabled = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(disabled.0, 200, "{}", disabled.1);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let answer = client
        .get(format!("{}{callback}", service.base))
        .header(reqwest::header::COOKIE, binding)
        .send()
        .await?;
    let session = answer
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find(|cookie| cookie.starts_with(&format!("{}=", lys_identity_server::session::COOKIE)));
    assert!(
        session.is_none(),
        "a pre-disable callback recreated a signed-in session"
    );
    Ok(())
}

#[tokio::test]
async fn a_failed_session_revoke_is_named_and_keeps_the_old_cookie_refused() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let cookie = service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let sessions = service.dir.path().join("sessions.json");
    std::fs::rename(&sessions, service.dir.path().join("saved-sessions.json"))?;
    std::fs::create_dir(&sessions)?;
    let refused = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(refused.0, 503, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "SessionsUnavailable");
    let signed_out = service.get("/me", Some(&cookie)).await?;
    assert_eq!(signed_out.0, 401, "{}", signed_out.1);
    assert_eq!(signed_out.1["refusal"], "NotSignedIn");
    let user = rauthy
        .users()?
        .into_iter()
        .find(|user| user["email"] == "bea@example.test")
        .ok_or("account missing")?;
    assert_eq!(
        user["enabled"], true,
        "issuer changes only after durable session revocation"
    );
    Ok(())
}

#[tokio::test]
async fn a_failed_token_revoke_is_named_and_leaves_the_issuer_account_enabled() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let person = bea(&service, &rauthy, &ada).await?;
    let cookie = service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    let verifier = "accounts-product-verifier-of-enough-length-0123456789";
    let code = product_code(&service, &cookie, verifier).await?;
    let issued = product_token(&service, &code, verifier).await?;
    assert_eq!(issued.0, 200, "{}", issued.1);
    let tokens = service.dir.path().join("provider.tokens.json");
    std::fs::rename(&tokens, service.dir.path().join("saved-tokens.json"))?;
    std::fs::create_dir(&tokens)?;
    let refused = service
        .post(
            &format!("/directory/people/{person}/account/enabled"),
            Some(&ada),
            &json!({ "enabled": false }),
        )
        .await?;
    assert_eq!(refused.0, 503, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "ProviderUnavailable");
    let user = rauthy
        .users()?
        .into_iter()
        .find(|user| user["email"] == "bea@example.test")
        .ok_or("account missing")?;
    assert_eq!(
        user["enabled"], true,
        "issuer changes only after the provider's tokens are revoked"
    );
    Ok(())
}

#[tokio::test]
async fn a_sign_in_the_issuer_api_refuses_is_named_and_begins_no_session() -> TestResult {
    let (mut service, rauthy, ada) = table().await?;
    bea(&service, &rauthy, &ada).await?;
    std::fs::write(rauthy.api_key_file(), "lys$not-the-issuer-key")?;
    service.restart().await?;
    let refused = service
        .post(
            "/sign-in",
            None,
            &json!({ "email": "bea@example.test", "password": "Bea-Password-000111" }),
        )
        .await?;
    assert_eq!(refused.0, 502, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "SignInProvidersRefused");
    Ok(())
}

#[tokio::test]
async fn the_administrator_is_refused_an_email_or_password_the_policy_refuses() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let bea = bea(&service, &rauthy, &ada).await?;
    let (status, email) = service
        .post(
            &format!("/directory/people/{bea}/account/email"),
            Some(&ada),
            &json!({ "email": "not-an-email" }),
        )
        .await?;
    assert_eq!(status, 400, "{email}");
    assert_eq!(email["refusal"], "AccountRefused", "{email}");
    let (status, password) = service
        .post(
            &format!("/directory/people/{bea}/account/password"),
            Some(&ada),
            &json!({ "password": "short" }),
        )
        .await?;
    assert_eq!(status, 400, "{password}");
    assert_eq!(password["refusal"], "AccountRefused", "{password}");
    service
        .sign_in_with("bea@example.test", "Bea-Password-000111")
        .await?;
    Ok(())
}
