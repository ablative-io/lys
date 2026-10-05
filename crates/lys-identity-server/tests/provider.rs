//! Lys as the `OpenID` provider a product is registered with (DIRECTORY-047
//! R3). A fixture product signs a person in through Lys with openidconnect's
//! own client, a second implementation of the protocol: it discovers Lys at
//! Lys's origin, sends the person to Lys, exchanges the code with PKCE, and
//! verifies the ID token against the keys Lys publishes. Each of the four
//! refusals has its own test.

use std::error::Error;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::apps::{
    Auth, approve, approve_with, ok, post, registration, workspace_schema,
};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::provider::{CODE_SECONDS, ProviderSettings};
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::secrets_api::SecretsSettings;
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreJwsSigningAlgorithm, CoreProviderMetadata,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, RedirectUrl, TokenResponse,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

/// The fixture product: an app registered and approved in `table`, so the
/// provider serves it as a client with no configuration naming it.
const PRODUCT: &str = "fixture_product";
const CALLBACK: &str = "https://product.example.test/auth/callback";
/// A second app, registered during a test to be approved and retired while
/// the service runs.
const SECOND: &str = "second_product";
const BACK: &str = "https://second.example.test/callback";

/// The product's client secret, as the custody fixture issued it at approval.
fn secret() -> String {
    identity_contract::app_custody::secret()
}

/// A service with the fixture product registered, codes living
/// `code_seconds`, and the administrator set up as a person, signed in.
/// A service with the provider on and the fixture product registered and
/// approved by the administrator, answering the service, the administrator's
/// cookie and their person id.
async fn table(code_seconds: u64) -> Result<(Service, String, String), Box<dyn Error>> {
    let broker = identity_contract::app_custody::start().await?;
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| {
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
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
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds,
            });
        },
        |config| {
            let key = config.log_dir.with_file_name("provider.key");
            std::fs::write(key, [5u8; 32])?;
            let signing = Arc::new(load_service_key(&config.event_key_file)?);
            drop(open_directory(config)?);
            drop(ConfigurationStore::open(
                &config.log_dir.with_file_name("organisation"),
                Arc::clone(&signing),
            )?);
            drop(ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&signing),
            )?);
            drop(lys_identity::start::LaunchRecords::open(
                &config.log_dir.with_file_name("launch-records"),
                load_service_key(&config.event_key_file)?,
            )?);
            drop(lys_identity_server::apps_api::opened(
                config,
                signing,
                &|_| {},
            )?);
            Ok(())
        },
    )
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let body = json!({ "operation": OperationId::generate()?.to_string(), "display_name": "Ada" });
    let (status, set_up) = service.post("/setup", Some(&cookie), &body).await?;
    assert_eq!(status, 200, "{set_up}");
    let person = set_up["person"].as_str().ok_or("a person")?.to_owned();
    // The product is an app the administrator registers and approves; the
    // provider reads it from the apps' record at each request.
    let mut body = registration(PRODUCT, &workspace_schema(PRODUCT))?;
    body["redirects"] = json!([CALLBACK]);
    ok(post(&service, "/apps", Auth::Cookie(&cookie), &body).await?)?;
    approve(&service, &cookie, PRODUCT).await?;
    Ok((service, cookie, person))
}

fn browser() -> Result<reqwest::Client, Box<dyn Error>> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

fn location(answer: &reqwest::Response) -> Result<String, Box<dyn Error>> {
    Ok(answer
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or("the answer sends the browser nowhere")?
        .to_str()?
        .to_owned())
}

fn challenge_of(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A code for the fixture product with `challenge`, as a signed-in browser gets it.
async fn code(service: &Service, cookie: &str, challenge: &str) -> Result<String, Box<dyn Error>> {
    let client = browser()?;
    let answer = client
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", PRODUCT),
            ("redirect_uri", CALLBACK),
            ("response_type", "code"),
            ("scope", "openid"),
            ("state", "product-state"),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(answer.status(), 303);
    let back = reqwest::Url::parse(&location(&answer)?)?;
    assert!(back.as_str().starts_with(CALLBACK), "{back}");
    let code = back
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the product is given a code")?;
    Ok(code)
}

/// Exchange `code` with `verifier` as the fixture product does, answering
/// the status and body.
async fn exchange(
    service: &Service,
    code: &str,
    verifier: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    exchange_at(service, code, verifier, CALLBACK).await
}

/// Exchange `code` with `verifier`, naming `redirect` as the address the
/// code was issued for.
async fn exchange_at(
    service: &Service,
    code: &str,
    verifier: &str,
    redirect: &str,
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
            ("redirect_uri", redirect),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, serde_json::from_str(&answer.text().await?)?))
}

#[tokio::test]
async fn replaying_a_code_revokes_the_access_it_issued() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let given = code(&service, &cookie, &challenge_of("verifier")).await?;
    let (status, answer) = exchange(&service, &given, "verifier").await?;
    assert_eq!(status, 200);
    let access = answer["access_token"].as_str().ok_or("no access token")?;
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .get(format!("{}/oauth/userinfo", service.base))
            .bearer_auth(access)
            .send()
            .await?
            .status(),
        200
    );
    let (status, refused) = exchange(&service, &given, "verifier").await?;
    assert_eq!(status, 400);
    assert_eq!(refused["refusal"], "CodeUsed");
    let refused = client
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(access)
        .send()
        .await?;
    assert_eq!(refused.status(), 401);
    let body: Value = refused.json().await?;
    assert_eq!(body["refusal"], "TokenUnknown");
    Ok(())
}

#[tokio::test]
async fn an_issued_access_token_survives_a_service_restart() -> TestResult {
    let (mut service, cookie, person) = table(CODE_SECONDS).await?;
    let given = code(&service, &cookie, &challenge_of("verifier")).await?;
    let (status, answer) = exchange(&service, &given, "verifier").await?;
    assert_eq!(status, 200);
    let access = answer["access_token"].as_str().ok_or("no access token")?;
    service.restart().await?;
    let answer = reqwest::Client::new()
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(access)
        .send()
        .await?;
    let status = answer.status();
    let body: Value = answer.json().await?;
    assert_eq!(status, 200, "refusal: {}", body["refusal"]);
    assert_eq!(body["sub"], person);
    Ok(())
}

#[tokio::test]
async fn a_product_signs_in_through_lys_and_verifies_the_token_against_lys_keys() -> TestResult {
    let (service, cookie, person) = table(CODE_SECONDS).await?;
    let http = openidconnect::reqwest::ClientBuilder::new()
        .redirect(openidconnect::reqwest::redirect::Policy::none())
        .build()?;
    let metadata =
        CoreProviderMetadata::discover_async(IssuerUrl::new(service.base.clone())?, &http).await?;
    assert_eq!(metadata.issuer().as_str(), service.base);
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(PRODUCT.to_owned()),
        Some(ClientSecret::new(secret())),
    )
    .set_redirect_uri(RedirectUrl::new(CALLBACK.to_owned())?);
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, state, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(challenge)
        .url();
    assert!(url.as_str().starts_with(&service.base), "{url}");

    let visitor = browser()?;
    let unsigned = visitor.get(url.as_str()).send().await?;
    assert_eq!(unsigned.status(), 303);
    let to_sign_in = location(&unsigned)?;
    assert!(
        to_sign_in.starts_with("/#/sign-in?continue="),
        "{to_sign_in}"
    );
    let signed = visitor
        .get(url.as_str())
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    let back = reqwest::Url::parse(&location(&signed)?)?;
    let pairs: Vec<(String, String)> = back
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let value = |name: &str| {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(value("state").as_deref(), Some(state.secret().as_str()));
    let code = value("code").ok_or("a code")?;
    for seen in [url.as_str(), &format!("{}{to_sign_in}", service.base)] {
        assert!(
            seen.starts_with(&service.base),
            "{seen} is not on Lys's origin"
        );
        assert!(!seen.contains(service.issuer.loopback()), "{seen}");
    }

    let answer = client
        .exchange_code(AuthorizationCode::new(code))?
        .set_pkce_verifier(verifier)
        .request_async(&http)
        .await?;
    let token = answer.id_token().ok_or("an ID token")?;
    let checker = client
        .id_token_verifier()
        .set_allowed_algs([CoreJwsSigningAlgorithm::EdDsa]);
    let claims = token.claims(&checker, &nonce)?;
    assert_eq!(claims.issuer().as_str(), service.base);
    assert_eq!(claims.subject().as_str(), person);
    let info = reqwest::Client::new()
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(answer.access_token().secret())
        .send()
        .await?
        .text()
        .await?;
    let info: Value = serde_json::from_str(&info)?;
    assert_eq!(info["sub"], person.as_str());
    Ok(())
}

#[tokio::test]
async fn a_redirect_address_not_registered_is_refused_and_never_followed() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let client = browser()?;
    let answer = client
        .get(format!("{}/oauth/authorize", service.base))
        .query(&[
            ("client_id", PRODUCT),
            ("redirect_uri", "http://elsewhere.example.test/callback"),
            ("response_type", "code"),
            ("code_challenge", &challenge_of("v")),
            ("code_challenge_method", "S256"),
        ])
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    assert_eq!(answer.status(), 400);
    assert!(answer.headers().get(reqwest::header::LOCATION).is_none());
    let body: Value = serde_json::from_str(&answer.text().await?)?;
    assert_eq!(body["refusal"], "redirect_invalid");
    assert!(!body.to_string().contains(service.issuer.loopback()));
    Ok(())
}

/// The authorize answer for `client_id` and `redirect`, as a signed-in
/// browser gets it: the status, the Location header if any, and the body.
async fn authorize_answer(
    service: &Service,
    cookie: &str,
    client_id: &str,
    redirect: &str,
) -> Result<(u16, Option<String>, Value), Box<dyn Error>> {
    authorize_scoped(service, cookie, client_id, redirect, Some("openid")).await
}

/// The authorize answer for `client_id`, `redirect` and the `scope` words
/// given, or no scope at all.
async fn authorize_scoped(
    service: &Service,
    cookie: &str,
    client_id: &str,
    redirect: &str,
    scope: Option<&str>,
) -> Result<(u16, Option<String>, Value), Box<dyn Error>> {
    let browser = browser()?;
    let challenge = challenge_of("v");
    let mut query = vec![
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
    ];
    if let Some(scope) = scope {
        query.push(("scope", scope));
    }
    let answer = browser
        .get(format!("{}/oauth/authorize", service.base))
        .query(&query)
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?;
    let status = answer.status().as_u16();
    let location = answer
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let text = answer.text().await?;
    let body = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text)?
    };
    Ok((status, location, body))
}

/// DIRECTORY-079 R1 with DIRECTORY-067 R1 and R2: an app is a sign-in client
/// from the request after its approval to the request after its retirement,
/// judged from the apps' record each time, with nothing restarted; before
/// approval and after retirement it is refused by name, as are an id no app
/// holds and a wrong secret; no refusal sends the browser anywhere.
#[tokio::test]
async fn an_app_is_a_client_from_its_approval_to_its_retirement_with_no_restart() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let mut body = registration(SECOND, &workspace_schema(SECOND))?;
    body["redirects"] = json!([BACK]);
    ok(post(&service, "/apps", Auth::Cookie(&cookie), &body).await?)?;
    // Registered and pending: refused app_not_approved, the browser sent nowhere.
    let (status, location, refusal) = authorize_answer(&service, &cookie, SECOND, BACK).await?;
    assert_eq!((status, location), (403, None), "{refusal}");
    assert_eq!(refusal["refusal"], "app_not_approved");
    // Approved: the next request is served, with no restart.
    approve(&service, &cookie, SECOND).await?;
    let (status, location, _) = authorize_answer(&service, &cookie, SECOND, BACK).await?;
    assert_eq!(status, 303);
    let back = reqwest::Url::parse(&location.ok_or("a code is sent back")?)?;
    assert!(back.as_str().starts_with(BACK), "{back}");
    let code = back
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("the product is given a code")?;
    // A wrong secret at the exchange: credential_refused, naming no secret.
    let wrong = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!("{SECOND}:not-the-secret"))
            ),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", BACK),
            ("code_verifier", "v"),
        ])
        .send()
        .await?;
    assert_eq!(wrong.status().as_u16(), 401);
    let refused: Value = serde_json::from_str(&wrong.text().await?)?;
    assert_eq!(refused["error"], "invalid_client");
    assert_eq!(refused["refusal"], "credential_refused");
    assert!(!refused.to_string().contains("not-the-secret"));
    // Retired: the next authorize and the code issued before it are refused app_retired.
    let retire = json!({"operation": identity_contract::apps::op()?, "reason": "done"});
    ok(post(
        &service,
        &format!("/apps/{SECOND}/retire"),
        Auth::Cookie(&cookie),
        &retire,
    )
    .await?)?;
    let (status, location, refusal) = authorize_answer(&service, &cookie, SECOND, BACK).await?;
    assert_eq!((status, location), (403, None), "{refusal}");
    assert_eq!(refusal["refusal"], "app_retired");
    let late = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!("{SECOND}:{}", secret()))
            ),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", BACK),
            ("code_verifier", "v"),
        ])
        .send()
        .await?;
    assert_eq!(late.status().as_u16(), 403);
    let refused: Value = serde_json::from_str(&late.text().await?)?;
    assert_eq!(refused["refusal"], "app_retired");
    // An id no app holds: credential_refused, naming no app and no secret.
    let (status, location, refusal) =
        authorize_answer(&service, &cookie, "nobody_here", BACK).await?;
    assert_eq!((status, location), (401, None), "{refusal}");
    assert_eq!(refusal["refusal"], "credential_refused");
    assert!(!refusal.to_string().contains("nobody_here"));
    // The first product, approved in the fixture, still signs in.
    let (status, _, _) = authorize_answer(&service, &cookie, PRODUCT, CALLBACK).await?;
    assert_eq!(status, 303);
    Ok(())
}

/// DIRECTORY-079 R2: the approval carries the app's return addresses and the
/// provider admits them at the next authorize; an administrator's change on
/// the Apps screen's route is in force at the next authorize, admitting the
/// new address and refusing the old one by name, with no restart; the app's
/// view answers the settings as last set, the name setting included.
#[tokio::test]
async fn the_sign_in_settings_are_in_force_at_the_provider_s_next_request() -> TestResult {
    const MOVED: &str = "https://second.example.test/moved-here";
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let mut body = registration(SECOND, &workspace_schema(SECOND))?;
    body["redirects"] = json!([BACK]);
    ok(post(&service, "/apps", Auth::Cookie(&cookie), &body).await?)?;
    let approval = approve_with(&service, &cookie, SECOND, json!([BACK]), false).await?;
    assert_eq!(approval["app"]["sign_in"]["redirects"], json!([BACK]));
    assert_eq!(approval["app"]["sign_in"]["profile"], false);
    let (status, _, _) = authorize_answer(&service, &cookie, SECOND, BACK).await?;
    assert_eq!(status, 303, "the approved address is admitted");
    let (status, location, refusal) = authorize_answer(&service, &cookie, SECOND, MOVED).await?;
    assert_eq!((status, location), (400, None), "{refusal}");
    assert_eq!(refusal["refusal"], "redirect_invalid");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|words| words.contains(MOVED))
    );
    // The administrator moves the app's return address and turns the name on.
    let change =
        json!({"operation": identity_contract::apps::op()?, "redirects": [MOVED], "profile": true});
    let changed = ok(post(
        &service,
        &format!("/apps/{SECOND}/sign_in"),
        Auth::Cookie(&cookie),
        &change,
    )
    .await?)?;
    assert_eq!(changed["sign_in"]["redirects"], json!([MOVED]));
    assert_eq!(changed["sign_in"]["profile"], true);
    let (status, _, _) = authorize_answer(&service, &cookie, SECOND, MOVED).await?;
    assert_eq!(status, 303, "the next authorize follows the change");
    let (status, location, refusal) = authorize_answer(&service, &cookie, SECOND, BACK).await?;
    assert_eq!((status, location), (400, None), "{refusal}");
    assert_eq!(refusal["refusal"], "redirect_invalid");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|words| words.contains(BACK))
    );
    // The name turned off again: the view follows; the token follows in R3.
    let off = json!({"operation": identity_contract::apps::op()?, "redirects": [MOVED], "profile": false});
    let read = ok(post(
        &service,
        &format!("/apps/{SECOND}/sign_in"),
        Auth::Cookie(&cookie),
        &off,
    )
    .await?)?;
    assert_eq!(read["sign_in"]["profile"], false);
    let view = ok(identity_contract::apps::get(
        &service,
        &format!("/apps/{SECOND}"),
        Auth::Cookie(&cookie),
    )
    .await?)?;
    assert_eq!(view["sign_in"], read["sign_in"]);
    assert_eq!(
        view["redirects"],
        json!([BACK]),
        "the registration's addresses stay what was asked for"
    );
    Ok(())
}

/// DIRECTORY-079 R1, acceptance 4: a declined app's client id is refused
/// `app_not_approved` naming the app, at authorize and at token, and the
/// browser is sent nowhere.
#[tokio::test]
async fn a_declined_apps_client_id_is_refused_app_not_approved_at_authorize_and_token() -> TestResult
{
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let mut body = registration(SECOND, &workspace_schema(SECOND))?;
    body["redirects"] = json!([BACK]);
    ok(post(&service, "/apps", Auth::Cookie(&cookie), &body).await?)?;
    let decline = json!({"operation": identity_contract::apps::op()?, "reason": "not this one"});
    ok(post(
        &service,
        &format!("/apps/{SECOND}/decline"),
        Auth::Cookie(&cookie),
        &decline,
    )
    .await?)?;
    let (status, location, refusal) = authorize_answer(&service, &cookie, SECOND, BACK).await?;
    assert_eq!((status, location), (403, None), "{refusal}");
    assert_eq!(refusal["refusal"], "app_not_approved");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|words| words.contains(SECOND))
    );
    let token = reqwest::Client::new()
        .post(format!("{}/oauth/token", service.base))
        .header(
            reqwest::header::AUTHORIZATION,
            format!(
                "Basic {}",
                STANDARD.encode(format!("{SECOND}:{}", secret()))
            ),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", "a-code-nobody-issued"),
            ("redirect_uri", BACK),
            ("code_verifier", "v"),
        ])
        .send()
        .await?;
    assert_eq!(token.status().as_u16(), 403);
    let refused: Value = serde_json::from_str(&token.text().await?)?;
    assert_eq!(refused["refusal"], "app_not_approved");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|words| words.contains(SECOND))
    );
    Ok(())
}

#[tokio::test]
async fn a_code_used_twice_is_refused() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let verifier = "a-verifier-of-enough-length-for-pkce-0123456789";
    let code = code(&service, &cookie, &challenge_of(verifier)).await?;
    let (status, first) = exchange(&service, &code, verifier).await?;
    assert_eq!(status, 200, "{first}");
    let (status, second) = exchange(&service, &code, verifier).await?;
    assert_eq!(status, 400, "{second}");
    assert_eq!(second["refusal"], "CodeUsed");
    assert_eq!(second["error"], "invalid_grant");
    Ok(())
}

#[tokio::test]
async fn a_wrong_pkce_verifier_is_refused() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let verifier = "a-verifier-of-enough-length-for-pkce-0123456789";
    let code = code(&service, &cookie, &challenge_of(verifier)).await?;
    let other = "another-verifier-of-enough-length-0123456789";
    let (status, refused) = exchange(&service, &code, other).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "VerifierWrong");
    Ok(())
}

#[tokio::test]
async fn a_code_past_its_instant_is_refused() -> TestResult {
    let (service, cookie, _) = table(0).await?;
    let verifier = "a-verifier-of-enough-length-for-pkce-0123456789";
    let code = code(&service, &cookie, &challenge_of(verifier)).await?;
    let (status, refused) = exchange(&service, &code, verifier).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "CodeExpired");
    Ok(())
}

#[tokio::test]
async fn a_code_exchanged_for_another_redirect_is_refused_and_not_used_up() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let verifier = "a-verifier-of-enough-length-for-pkce-0123456789";
    let code = code(&service, &cookie, &challenge_of(verifier)).await?;
    let elsewhere = "http://elsewhere.example.test/callback";
    let (status, refused) = exchange_at(&service, &code, verifier, elsewhere).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "RedirectUnregistered");
    assert!(!refused.to_string().contains("elsewhere"), "{refused}");
    let (status, tokens) = exchange(&service, &code, verifier).await?;
    assert_eq!(
        status, 200,
        "the code is still good where it was issued: {tokens}"
    );
    Ok(())
}

async fn userinfo_with(service: &Service, token: &str) -> Result<(u16, Value), Box<dyn Error>> {
    let response = reqwest::Client::new()
        .get(format!("{}/oauth/userinfo", service.base))
        .bearer_auth(token)
        .send()
        .await?;
    Ok((response.status().as_u16(), response.json().await?))
}

async fn end_current_session(service: &Service, cookie: &str) -> TestResult {
    let (status, sessions) = service.get("/sessions", Some(cookie)).await?;
    assert_eq!(status, 200, "{sessions}");
    let id = sessions["sessions"]
        .as_array()
        .ok_or("sessions is not an array")?
        .iter()
        .find(|session| session["current"] == true)
        .and_then(|session| session["id"].as_str())
        .ok_or("current session is absent")?;
    let (status, ended) = service
        .post(&format!("/sessions/{id}/end"), Some(cookie), &json!({}))
        .await?;
    assert_eq!(status, 200, "{ended}");
    let (status, ended) = service.get("/me", Some(cookie)).await?;
    assert_eq!(status, 401, "{ended}");
    Ok(())
}

#[tokio::test]
async fn a_product_token_stops_answering_when_its_lys_session_ends() -> TestResult {
    let (service, cookie, person) = table(CODE_SECONDS).await?;
    let verifier = "a-session-bound-verifier-of-enough-length-0123456789";
    let issued = code(&service, &cookie, &challenge_of(verifier)).await?;
    let (status, answer) = exchange(&service, &issued, verifier).await?;
    assert_eq!(status, 200, "{answer}");
    let token = answer["access_token"].as_str().ok_or("no access token")?;
    let (status, active) = userinfo_with(&service, token).await?;
    assert_eq!(status, 200, "{active}");
    assert_eq!(active["sub"], person);
    end_current_session(&service, &cookie).await?;
    let (status, refused) = userinfo_with(&service, token).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "TokenUnknown");
    Ok(())
}

#[tokio::test]
async fn a_code_from_an_ended_session_cannot_create_product_tokens() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let verifier = "an-ended-session-verifier-of-enough-length-0123456789";
    let issued = code(&service, &cookie, &challenge_of(verifier)).await?;
    end_current_session(&service, &cookie).await?;
    let (status, refused) = exchange(&service, &issued, verifier).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "CodeExpired");
    assert!(refused.get("access_token").is_none());
    assert!(refused.get("id_token").is_none());
    Ok(())
}

#[tokio::test]
async fn a_product_identity_token_has_a_short_validity_window() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let verifier = "a-short-lived-id-token-verifier-of-enough-length-0123456789";
    let issued = code(&service, &cookie, &challenge_of(verifier)).await?;
    let (status, answer) = exchange(&service, &issued, verifier).await?;
    assert_eq!(status, 200, "{answer}");
    let jwt = answer["id_token"].as_str().ok_or("no identity token")?;
    let claims = jwt
        .split('.')
        .nth(1)
        .ok_or("identity token has no claims")?;
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims)?)?;
    let issued = claims["iat"].as_u64().ok_or("no issue instant")?;
    let expires = claims["exp"].as_u64().ok_or("no expiry instant")?;
    assert!(expires > issued);
    assert!(
        expires - issued <= 300,
        "identity token outlives five minutes"
    );
    Ok(())
}

/// The claims an identity token carries.
fn claims_of(answer: &Value) -> Result<Value, Box<dyn Error>> {
    let jwt = answer["id_token"].as_str().ok_or("no identity token")?;
    let claims = jwt
        .split('.')
        .nth(1)
        .ok_or("identity token has no claims")?;
    Ok(serde_json::from_slice(&URL_SAFE_NO_PAD.decode(claims)?)?)
}

/// A code for the fixture product asking for `scope`, from a signed-in browser.
async fn code_scoped(
    service: &Service,
    cookie: &str,
    scope: &str,
) -> Result<String, Box<dyn Error>> {
    let (status, location, body) =
        authorize_scoped(service, cookie, PRODUCT, CALLBACK, Some(scope)).await?;
    assert_eq!(status, 303, "{body}");
    let back = reqwest::Url::parse(&location.ok_or("a code is sent back")?)?;
    assert!(back.as_str().starts_with(CALLBACK), "{back}");
    back.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| "the product is given a code".into())
}

/// Set the fixture product's sign-in settings: its address, and whether it is
/// given the name.
async fn set_profile(service: &Service, cookie: &str, profile: bool) -> TestResult {
    let change = json!({
        "operation": identity_contract::apps::op()?,
        "redirects": [CALLBACK],
        "profile": profile,
    });
    let view = ok(post(
        service,
        &format!("/apps/{PRODUCT}/sign_in"),
        Auth::Cookie(cookie),
        &change,
    )
    .await?)?;
    assert_eq!(view["sign_in"]["profile"], profile);
    Ok(())
}

/// DIRECTORY-079 R3: the name is given only for the `profile` scope and only
/// while the app's sign-in settings grant it. Asked for when not granted, the
/// authorization is refused `ScopeNotGranted` naming the app and the browser
/// is sent nowhere. Granted, the identity token and the user information
/// carry the person's name as the directory holds it at that issue; a token
/// for `openid` alone carries none even then; and when the setting is turned
/// off, the token already issued stops giving the name at the next request
/// and the next authorization asking for it is refused.
#[tokio::test]
async fn the_name_follows_the_profile_scope_and_the_apps_setting() -> TestResult {
    let (service, cookie, person) = table(CODE_SECONDS).await?;
    // Approved with the name off: asking for it is refused, sent nowhere.
    let (status, location, refusal) =
        authorize_scoped(&service, &cookie, PRODUCT, CALLBACK, Some("openid profile")).await?;
    assert_eq!((status, location), (403, None), "{refusal}");
    assert_eq!(refusal["refusal"], "ScopeNotGranted");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|words| words.contains(PRODUCT) && words.contains("profile")),
        "{refusal}"
    );
    // The administrator turns the name on: the next issue carries it.
    set_profile(&service, &cookie, true).await?;
    let issued = code_scoped(&service, &cookie, "openid profile").await?;
    let (status, answer) = exchange(&service, &issued, "v").await?;
    assert_eq!(status, 200, "{answer}");
    let claims = claims_of(&answer)?;
    assert_eq!(claims["sub"], person);
    assert_eq!(claims["name"], "Ada", "{claims}");
    let with_name = answer["access_token"].as_str().ok_or("a token")?.to_owned();
    let (status, info) = userinfo_with(&service, &with_name).await?;
    assert_eq!(status, 200, "{info}");
    assert_eq!(info, json!({ "sub": person, "name": "Ada" }));
    // Asked for openid alone, the name is not given even while granted.
    let issued = code_scoped(&service, &cookie, "openid").await?;
    let (status, answer) = exchange(&service, &issued, "v").await?;
    assert_eq!(status, 200, "{answer}");
    assert!(claims_of(&answer)?.get("name").is_none());
    let plain = answer["access_token"].as_str().ok_or("a token")?;
    let (status, info) = userinfo_with(&service, plain).await?;
    assert_eq!(status, 200, "{info}");
    assert_eq!(info, json!({ "sub": person }));
    // Turned off again: the token follows the setting at its next request,
    // and the next authorization asking for the name is refused.
    set_profile(&service, &cookie, false).await?;
    let (status, info) = userinfo_with(&service, &with_name).await?;
    assert_eq!(status, 200, "{info}");
    assert_eq!(info, json!({ "sub": person }));
    let (status, location, refusal) =
        authorize_scoped(&service, &cookie, PRODUCT, CALLBACK, Some("openid profile")).await?;
    assert_eq!((status, location), (403, None), "{refusal}");
    assert_eq!(refusal["refusal"], "ScopeNotGranted");
    Ok(())
}

/// DIRECTORY-079 R3: the setting is judged again at the exchange. A code
/// issued while the name was granted, exchanged after the administrator took
/// the name away, yields a token without the name claim, and its user
/// information carries none either.
#[tokio::test]
async fn a_setting_taken_away_between_authorize_and_exchange_withholds_the_name() -> TestResult {
    let (service, cookie, person) = table(CODE_SECONDS).await?;
    set_profile(&service, &cookie, true).await?;
    let issued = code_scoped(&service, &cookie, "openid profile").await?;
    set_profile(&service, &cookie, false).await?;
    let (status, answer) = exchange(&service, &issued, "v").await?;
    assert_eq!(status, 200, "{answer}");
    let claims = claims_of(&answer)?;
    assert_eq!(claims["sub"], person);
    assert!(claims.get("name").is_none(), "{claims}");
    let token = answer["access_token"].as_str().ok_or("a token")?;
    let (status, info) = userinfo_with(&service, token).await?;
    assert_eq!(status, 200, "{info}");
    assert_eq!(info, json!({ "sub": person }));
    Ok(())
}

/// DIRECTORY-079 R3: a scope Lys does not serve is refused `ScopeUnknown`
/// naming the word and the browser is sent nowhere; an authorization without
/// the `openid` scope is malformed; and discovery lists exactly the scopes
/// and claims Lys serves.
#[tokio::test]
async fn a_scope_lys_does_not_serve_is_refused_and_discovery_says_what_is_served() -> TestResult {
    let (service, cookie, _) = table(CODE_SECONDS).await?;
    let (status, location, refusal) =
        authorize_scoped(&service, &cookie, PRODUCT, CALLBACK, Some("openid email")).await?;
    assert_eq!((status, location), (400, None), "{refusal}");
    assert_eq!(refusal["refusal"], "ScopeUnknown");
    assert!(
        refusal["reason"]
            .as_str()
            .is_some_and(|words| words.contains("email")),
        "{refusal}"
    );
    let (status, location, refusal) =
        authorize_scoped(&service, &cookie, PRODUCT, CALLBACK, None).await?;
    assert_eq!((status, location), (400, None), "{refusal}");
    assert_eq!(refusal["refusal"], "RequestMalformed");
    let discovery: Value =
        reqwest::get(format!("{}/.well-known/openid-configuration", service.base))
            .await?
            .json()
            .await?;
    assert_eq!(discovery["scopes_supported"], json!(["openid", "profile"]));
    assert_eq!(
        discovery["claims_supported"],
        json!([
            "iss",
            "sub",
            "aud",
            "iat",
            "exp",
            "auth_time",
            "nonce",
            "name"
        ])
    );
    Ok(())
}
