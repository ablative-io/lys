//! Lys as the `OpenID` provider a product is registered with (DIRECTORY-047
//! R3). A fixture product signs a person in through Lys with openidconnect's
//! own client, a second implementation of the protocol: it discovers Lys at
//! Lys's origin, sends the person to Lys, exchanges the code with PKCE, and
//! verifies the ID token against the keys Lys publishes. Each of the four
//! refusals has its own test.

use std::error::Error;

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity_server::provider::{CODE_SECONDS, ProductClient, ProviderSettings};
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

const PRODUCT: &str = "fixture-product";
const SECRET: &str = "fixture-product-secret";
const CALLBACK: &str = "http://product.example.test/auth/callback";

fn hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    digits.concat()
}

/// A service with the fixture product registered, codes living
/// `code_seconds`, and the administrator set up as a person, signed in.
async fn table(code_seconds: u64) -> Result<(Service, String, String), Box<dyn Error>> {
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                clients: vec![ProductClient {
                    client_id: PRODUCT.to_owned(),
                    secret_sha256: hex(&Sha256::digest(SECRET.as_bytes())),
                    redirect_uris: vec![CALLBACK.to_owned()],
                }],
                code_seconds,
            });
        },
        |config| {
            let key = config.log_dir.with_file_name("provider.key");
            std::fs::write(key, [5u8; 32])?;
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
            format!("Basic {}", STANDARD.encode(format!("{PRODUCT}:{SECRET}"))),
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
    assert_eq!(answer.status(), 200);
    let body: Value = answer.json().await?;
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
        Some(ClientSecret::new(SECRET.to_owned())),
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
    assert_eq!(body["refusal"], "RedirectUnregistered");
    assert!(!body.to_string().contains(service.issuer.loopback()));
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
