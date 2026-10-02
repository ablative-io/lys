//! An app outside Lys connects through the MCP authorization flow end to
//! end: it registers, the person approves it, it exchanges the code for
//! tokens, and it acts as the agent the approval made, never as the person.

use std::error::Error;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use identity_contract::harness::{ADMINISTRATOR, Service};
use reqwest::StatusCode;
use reqwest::redirect::Policy;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const REDIRECT: &str = "https://app.example/callback";
const VERIFIER: &str = "a-verifier-of-at-least-forty-three-characters-long";

async fn service() -> Result<(Service, String), Box<dyn Error>> {
    use lys_identity::{
        Actor, AuthMethod, Directory, LoginBinding, OperationId, Profile, Provenance,
    };
    let (service, ()) = Service::start_with(|config| {
        lys_log_store::FileLeafStore::create(&config.log_dir, &config.log_origin)?;
        let path = config.log_dir.clone();
        let key = lys_identity::signer::load_service_key(&config.event_key_file)?;
        lys_identity::directory_migration::migrate(
            lys_log_store::FileLeafStore::open(&path)?,
            &key,
        )?;
        let mut directory = Directory::open(
            Box::new(move || lys_log_store::FileLeafStore::open(&path)),
            key,
        )?;
        directory.setup_person(
            Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            ),
            OperationId::generate()?,
            Profile::new("Owner")?,
            1,
        )?;
        Ok(())
    })
    .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    Ok((service, cookie))
}

fn client() -> Result<reqwest::Client, Box<dyn Error>> {
    Ok(reqwest::Client::builder()
        .redirect(Policy::none())
        .build()?)
}

fn located(response: &reqwest::Response) -> Result<reqwest::Url, Box<dyn Error>> {
    let location = response
        .headers()
        .get("location")
        .ok_or("no location")?
        .to_str()?;
    Ok(reqwest::Url::parse(location)?)
}

fn pair(url: &reqwest::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// Register, approve and exchange; answers the client id and token answer.
async fn connected(service: &Service, cookie: &str) -> Result<(String, Value), Box<dyn Error>> {
    let client = client()?;
    let registered: Value = client
        .post(format!("{}/oauth/mcp/register", service.base))
        .json(&json!({"client_name":"Notes", "redirect_uris":[REDIRECT]}))
        .send()
        .await?
        .json()
        .await?;
    let client_id = registered["client_id"]
        .as_str()
        .ok_or("no client id")?
        .to_owned();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let page = client
        .get(format!("{}/oauth/mcp/authorize", service.base))
        .query(&[
            ("response_type", "code"),
            ("client_id", client_id.as_str()),
            ("redirect_uri", REDIRECT),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", "kept"),
        ])
        .header("cookie", cookie)
        .send()
        .await?;
    assert_eq!(page.status(), StatusCode::OK);
    let page = page.text().await?;
    assert!(page.contains("Connect Notes"), "{page}");
    let asking = page
        .split("name=\"asking\" value=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .ok_or("no approval on the page")?
        .to_owned();
    let answered = client
        .post(format!("{}/oauth/mcp/consent", service.base))
        .header("cookie", cookie)
        .form(&[("asking", asking.as_str()), ("decision", "approve")])
        .send()
        .await?;
    assert_eq!(answered.status(), StatusCode::SEE_OTHER);
    let back = located(&answered)?;
    assert_eq!(pair(&back, "state").as_deref(), Some("kept"));
    let code = pair(&back, "code").ok_or("no code")?;
    let tokens: Value = client
        .post(format!("{}/oauth/mcp/token", service.base))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("code", code.as_str()),
            ("code_verifier", VERIFIER),
            ("redirect_uri", REDIRECT),
        ])
        .send()
        .await?
        .json()
        .await?;
    Ok((client_id, tokens))
}

async fn tree(service: &Service, token: &str) -> Result<reqwest::Response, Box<dyn Error>> {
    let client = client()?;
    Ok(client
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .header("authorization", format!("Bearer {token}"))
        .json(
            &json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
                "name":"read", "arguments":{"method":"GET","path":"/tree"}
            }}),
        )
        .send()
        .await?)
}

#[tokio::test]
async fn an_approved_app_acts_as_its_own_agent_never_as_the_person() -> Result<(), Box<dyn Error>> {
    let (service, cookie) = service().await?;
    let (_, tokens) = connected(&service, &cookie).await?;
    assert_eq!(tokens["token_type"], "Bearer", "{tokens}");
    let access = tokens["access_token"].as_str().ok_or("no access token")?;
    let called: Value = tree(&service, access).await?.json().await?;
    assert_eq!(
        called["result"]["structuredContent"]["status"], 200,
        "{called}"
    );
    let root = &called["result"]["structuredContent"]["body"]["root"];
    assert_eq!(root["name"], "Notes", "{called}");
    let (_, own) = service.get("/tree", Some(&cookie)).await?;
    assert_ne!(root["id"], own["root"]["id"], "the app acted as the person");
    Ok(())
}

#[tokio::test]
async fn a_refresh_token_is_spent_when_it_is_used() -> Result<(), Box<dyn Error>> {
    let (service, cookie) = service().await?;
    let (client_id, tokens) = connected(&service, &cookie).await?;
    let refresh = tokens["refresh_token"].as_str().ok_or("no refresh token")?;
    let exchange = |token: &str| {
        let form = vec![
            ("grant_type".to_owned(), "refresh_token".to_owned()),
            ("client_id".to_owned(), client_id.clone()),
            ("refresh_token".to_owned(), token.to_owned()),
        ];
        let base = service.base.clone();
        async move {
            client()?
                .post(format!("{base}/oauth/mcp/token"))
                .form(&form)
                .send()
                .await
                .map_err(Box::<dyn Error>::from)
        }
    };
    let renewed = exchange(refresh).await?;
    assert_eq!(renewed.status(), StatusCode::OK);
    let renewed: Value = renewed.json().await?;
    assert!(renewed["access_token"].is_string(), "{renewed}");
    let again = exchange(refresh).await?;
    assert_eq!(again.status(), StatusCode::BAD_REQUEST);
    let again: Value = again.json().await?;
    assert_eq!(again["error"], "invalid_grant");
    Ok(())
}

#[tokio::test]
async fn a_wrong_verifier_or_a_reused_code_gets_no_tokens() -> Result<(), Box<dyn Error>> {
    let (service, cookie) = service().await?;
    let (client_id, _) = connected(&service, &cookie).await?;
    let refused = client()?
        .post(format!("{}/oauth/mcp/token", service.base))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("code", "not-a-code"),
            ("code_verifier", VERIFIER),
            ("redirect_uri", REDIRECT),
        ])
        .send()
        .await?;
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    let refused: Value = refused.json().await?;
    assert_eq!(refused["error"], "invalid_grant");
    Ok(())
}

#[tokio::test]
async fn an_unknown_bearer_token_is_refused_and_told_where_to_authorize()
-> Result<(), Box<dyn Error>> {
    let (service, _) = service().await?;
    let response = tree(&service, "not-a-token").await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let challenge = response
        .headers()
        .get("www-authenticate")
        .ok_or("no challenge")?
        .to_str()?
        .to_owned();
    assert!(
        challenge.contains("/.well-known/oauth-protected-resource"),
        "{challenge}"
    );
    let metadata: Value = client()?
        .get(format!(
            "{}/.well-known/oauth-protected-resource",
            service.base
        ))
        .send()
        .await?
        .json()
        .await?;
    assert!(
        metadata["resource"]
            .as_str()
            .is_some_and(|resource| resource.ends_with("/mcp")),
        "{metadata}"
    );
    let server: Value = client()?
        .get(format!(
            "{}/.well-known/oauth-authorization-server",
            service.base
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(server["code_challenge_methods_supported"], json!(["S256"]));
    Ok(())
}

#[tokio::test]
async fn an_app_asking_while_signed_out_is_sent_to_sign_in_first() -> Result<(), Box<dyn Error>> {
    let (service, _) = service().await?;
    let registered: Value = client()?
        .post(format!("{}/oauth/mcp/register", service.base))
        .json(&json!({"client_name":"Notes", "redirect_uris":[REDIRECT]}))
        .send()
        .await?
        .json()
        .await?;
    let client_id = registered["client_id"].as_str().ok_or("no client id")?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let response = client()?
        .get(format!("{}/oauth/mcp/authorize", service.base))
        .query(&[
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", REDIRECT),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .ok_or("no location")?
        .to_str()?;
    assert!(
        location.starts_with("/#/sign-in?continue=%2Foauth%2Fmcp%2Fauthorize%3F"),
        "{location}"
    );
    Ok(())
}

#[tokio::test]
async fn an_app_is_given_only_what_the_person_holds_to_pass_on() -> Result<(), Box<dyn Error>> {
    let (service, cookie) = service().await?;
    let client = client()?;
    let registered: Value = client
        .post(format!("{}/oauth/mcp/register", service.base))
        .json(&json!({"client_name":"Notes", "redirect_uris":[REDIRECT]}))
        .send()
        .await?
        .json()
        .await?;
    let client_id = registered["client_id"].as_str().ok_or("no client id")?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let page = client
        .get(format!("{}/oauth/mcp/authorize", service.base))
        .query(&[
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", REDIRECT),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header("cookie", &cookie)
        .send()
        .await?
        .text()
        .await?;
    assert!(page.contains("starts with no permissions"), "{page}");
    let asking = page
        .split("name=\"asking\" value=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .ok_or("no approval on the page")?
        .to_owned();
    let forged = client
        .post(format!("{}/oauth/mcp/consent", service.base))
        .header("cookie", &cookie)
        .form(&[
            ("asking", asking.as_str()),
            ("decision", "approve"),
            ("grant", "not-a-grant:owner"),
        ])
        .send()
        .await?;
    assert_eq!(forged.status(), StatusCode::BAD_REQUEST);
    let refused: Value = forged.json().await?;
    assert_eq!(refused["refusal"], "RequestMalformed", "{refused}");
    Ok(())
}
