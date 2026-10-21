#![cfg(test)]

//! A product admits a person through its own person pass (DIRECTORY-113):
//! POST /people/admit asked with an approved app's pass is judged as the
//! person the pass speaks for, admits only with a first grant on a kind the
//! app owns, refuses before any step otherwise, and its handed link goes on
//! to the app's own origin, which the proof answers.

use std::error::Error;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::app_custody::secret;
use identity_contract::apps::registered;
use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "people-admit-app-first-run-code";
const ADMIN_EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";
const BEA: &str = "bea@example.test";
const APP: &str = "fixture_product";
/// The address the fixture app's registration lists, and its origin.
const BACK: &str = "https://app.example.test/signed-in";
const ORIGIN: &str = "https://app.example.test";
const VERIFIER: &str = "a-pass-verifier-of-enough-length-0123456789-abcdefghij";
const STEPS: [&str; 6] = ["register", "account", "bind", "activate", "grant", "letter"];

/// A service with a fake issuer, no mail settings, Lys's provider on, Ada
/// as the first-run administrator of Acme, and the fixture app approved.
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
            config.administrator = None;
            config.setup = Some(SetupSettings {
                code_file: config.log_dir.with_file_name("setup-code"),
                administrator_file: config.log_dir.with_file_name("administrator.json"),
                email: None,
                organisation: "Acme".to_owned(),
            });
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds: CODE_SECONDS,
                pass_seconds: PASS_SECONDS,
                rights_bytes: None,
            });
        },
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            let setup = config.setup.as_ref().ok_or("setup is configured")?;
            let digest: Vec<String> = Sha256::digest(CODE.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let pending = json!({ "purpose": "first-run", "sha256": digest.concat() });
            std::fs::write(&setup.code_file, pending.to_string())?;
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
                "email": ADMIN_EMAIL,
                "password": PASSWORD,
            })
            .to_string(),
        )
        .send()
        .await?;
    let ada = session_cookie(made).await?;
    registered(&service, &ada, APP).await?;
    Ok((service, rauthy, ada))
}

/// The fixture app's person pass for the person signed in with `cookie`.
async fn app_pass(service: &Service, cookie: &str) -> Result<String, Box<dyn Error>> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let sent = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", APP),
            ("redirect_uri", BACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(sent.status(), 303);
    let back = sent
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the answer sends the browser nowhere")?
        .to_str()?;
    let code = reqwest::Url::parse(back)?
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the app is given a code")?;
    let answer: Value = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Basic {}", STANDARD.encode(format!("{APP}:{}", secret()))),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", BACK),
            ("code_verifier", VERIFIER),
        ])
        .send()
        .await?
        .json()
        .await?;
    Ok(answer["access_token"]
        .as_str()
        .ok_or_else(|| format!("the token route issues a pass: {answer}"))?
        .to_owned())
}

/// POST `body` to /people/admit with `pass` as the bearer.
async fn admit_with(
    service: &Service,
    pass: &str,
    body: &Value,
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/people/admit", service.base))
        .bearer_auth(pass)
        .json(body)
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, answer.json().await?))
}

/// The act admitting Bea, with `grant` as its first grant.
fn bea(operation: &str, grant: Value) -> Value {
    json!({"operation": operation, "display_name": "Bea", "email": BEA, "grant": grant})
}

/// A member grant on `kind` `ws-1`, asked on the API route.
fn member_of(kind: &str) -> Value {
    json!({"route": "api", "resource": {"kind": kind, "id": "ws-1"}, "relation": "member"})
}

/// The people the directory lists, as Ada reads them.
async fn people(service: &Service, ada: &str) -> Result<usize, Box<dyn Error>> {
    let (status, listed) = service.get("/directory/people", Some(ada)).await?;
    assert_eq!(status, 200, "{listed}");
    let people = listed["people"]
        .as_array()
        .or_else(|| listed.as_array())
        .ok_or("the directory lists people")?;
    Ok(people.len())
}

/// The accounts the issuer holds for `email`.
fn accounts_for(rauthy: &FakeRauthy, email: &str) -> Result<usize, Box<dyn Error>> {
    Ok(rauthy
        .users()?
        .iter()
        .filter(|user| user["email"] == email)
        .count())
}

/// The token after `#/added/` in a handed link.
fn token_in(answer: &Value) -> Result<String, Box<dyn Error>> {
    let link = answer["letter"]["handed"]
        .as_str()
        .ok_or_else(|| format!("the answer hands a link: {answer}"))?;
    let (_, token) = link
        .split_once("#/added/")
        .ok_or_else(|| format!("the link is an admission link: {link}"))?;
    Ok(token.to_owned())
}

/// Post `token` to prove, answering the status and the body.
async fn prove(service: &Service, token: &str) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/organisations/found/prove", service.base))
        .json(&json!({ "proof": token }))
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, answer.json().await?))
}

#[tokio::test]
async fn a_product_admits_a_person_as_the_person_whose_pass_it_holds() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let asked = bea(&operation, member_of("fixture_product.workspace"));
    let (status, answer) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["receipt"]["completed"], json!(STEPS), "{answer}");
    assert!(answer["receipt"]["failed"].is_null(), "{answer}");
    let person = answer["person"].as_str().ok_or("a person")?;
    let grant = answer["grant"].as_str().ok_or("a first grant")?;
    token_in(&answer)?;
    let (status, held) = service.get(&format!("/grants/{grant}"), Some(&ada)).await?;
    assert_eq!(status, 200, "{held}");
    let text = held.to_string();
    for word in [person, "fixture_product.workspace", "ws-1", "member"] {
        assert!(text.contains(word), "the grant names {word}: {held}");
    }
    // Asked again with the same operation, the act replays: the same person
    // and no second account.
    let (status, again) = admit_with(&service, &pass, &asked).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["person"], answer["person"]);
    assert_eq!(again["receipt"]["logged"], answer["receipt"]["logged"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    Ok(())
}

#[tokio::test]
async fn a_product_is_refused_before_any_step_without_a_grant_on_its_own_kind() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let before = people(&service, &ada).await?;
    let asks = [
        (bea(&OperationId::generate()?.to_string(), Value::Null), 400, "app_admit_grant_required"),
        (bea(&OperationId::generate()?.to_string(), member_of("cambium.workspace")), 403, "not_your_app"),
        (bea(&OperationId::generate()?.to_string(), member_of("fixture_product.room")), 403, "kind_not_registered"),
    ];
    for (asked, status, refusal) in asks {
        let (answered, answer) = admit_with(&service, &pass, &asked).await?;
        assert_eq!((answered, answer["refusal"].as_str()), (status, Some(refusal)), "{answer}");
        assert_eq!(answer["receipt"]["completed"], json!([]), "{answer}");
    }
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    Ok(())
}

#[tokio::test]
async fn a_person_without_organisation_admit_admits_nobody_through_a_product() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let (status, answer) = admit_with(
        &service,
        &pass,
        &bea(&OperationId::generate()?.to_string(), member_of("fixture_product.workspace")),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    let bea_cookie = service
        .sign_in(identity_contract::fake_issuer::Login {
            subject: answer["subject"].as_str().ok_or("a subject")?.to_owned(),
            email: BEA.to_owned(),
        })
        .await?;
    let bea_pass = app_pass(&service, &bea_cookie).await?;
    let before = people(&service, &ada).await?;
    let asked = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Cy",
        "email": "cy@example.test", "grant": member_of("fixture_product.workspace")});
    let (status, refused) = admit_with(&service, &bea_pass, &asked).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!([]), "{refused}");
    assert_eq!(people(&service, &ada).await?, before);
    assert_eq!(accounts_for(&rauthy, "cy@example.test")?, 0);
    Ok(())
}

#[tokio::test]
async fn the_handed_link_goes_on_to_the_products_own_origin() -> TestResult {
    let (service, _rauthy, ada) = table().await?;
    let pass = app_pass(&service, &ada).await?;
    let (status, answer) = admit_with(
        &service,
        &pass,
        &bea(&OperationId::generate()?.to_string(), member_of("fixture_product.workspace")),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, proved) = prove(&service, &token_in(&answer)?).await?;
    assert_eq!(status, 200, "{proved}");
    assert_eq!(proved["continue_to"], ORIGIN, "{proved}");
    // The same act asked from Ada's own session goes on to nowhere.
    let asked = json!({"operation": OperationId::generate()?.to_string(), "display_name": "Cy",
        "email": "cy@example.test", "grant": member_of("fixture_product.workspace")});
    let (status, from_lys) = service.post("/people/admit", Some(&ada), &asked).await?;
    assert_eq!(status, 200, "{from_lys}");
    let (status, proved) = prove(&service, &token_in(&from_lys)?).await?;
    assert_eq!(status, 200, "{proved}");
    assert!(proved.get("continue_to").is_none(), "{proved}");
    // A body naming where to go is refused whole, nothing recorded.
    let mut named = bea(&OperationId::generate()?.to_string(), member_of("fixture_product.workspace"));
    named["continue_to"] = json!("https://elsewhere.example.test");
    let (status, refused) = admit_with(&service, &pass, &named).await?;
    assert_eq!((status, refused["refusal"].as_str()), (400, Some("RequestMalformed")), "{refused}");
    Ok(())
}
