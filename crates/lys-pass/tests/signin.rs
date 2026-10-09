#![cfg(test)]
//! A product signs a person in server-side: the addresses come from
//! discovery, the state is the browser's, the credential and the verifier
//! go only to the token endpoint, and the pass is verified before use.

use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use lys_pass::signin::{Secret, SignIn, Started, session_cookie, state_cookie};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const SECRET: &str = "issued-credential-never-shown";
const NOW: u64 = 100;

/// One request the double of Lys saw: its path and its body.
type Seen = Arc<Mutex<Vec<(String, String)>>>;

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

/// A pass for `issuer`, signed by the double's key.
fn pass(issuer: &str) -> String {
    let header = json!({"alg": "EdDSA", "typ": "JWT", "kid": "double"});
    let claims = json!({
        "iss": issuer, "sub": "holder", "aud": "sample", "iat": 90, "exp": 200,
        "holder": {"id": "holder", "kind": "person", "responsible": null},
        "rights": [{"resource": {"kind": "sample.file", "id": "child"},
            "actions": ["read"], "mode": "outright", "grant": "grant"}],
    });
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    );
    let signature = URL_SAFE_NO_PAD.encode(signing_key().sign(input.as_bytes()).to_bytes());
    format!("{input}.{signature}")
}

/// A double of Lys answering by path, recording every request. `discovered`
/// is the issuer its discovery document names, its own origin by default;
/// `token` is the token endpoint's status and body, by default a pass it
/// signs for its own origin with a refresh token.
async fn double(discovered: Option<&str>, token: Option<(u16, Value)>) -> Result<(Url, Seen)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let token = token.unwrap_or_else(|| {
        (
            200,
            json!({"access_token": pass(&origin), "token_type": "Bearer", "expires_in": 100,
                "id_token": null, "refresh_token": "refresh-1"}),
        )
    });
    let issuer = discovered.map_or_else(|| origin.clone(), str::to_owned);
    let seen: Seen = Arc::default();
    let record = Arc::clone(&seen);
    let keys = json!({"keys": [{"kty": "OKP", "crv": "Ed25519", "use": "sig", "alg": "EdDSA",
        "kid": "double", "x": URL_SAFE_NO_PAD.encode(signing_key().verifying_key().to_bytes())}]});
    let discovery = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{origin}/oauth/authorize"),
        "token_endpoint": format!("{origin}/oauth/token"),
        "jwks_uri": format!("{origin}/oauth/jwks"),
    });
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut bytes = Vec::new();
            let (path, body) = loop {
                let mut buffer = [0; 4096];
                let Ok(count) = socket.read(&mut buffer).await else {
                    return;
                };
                if count == 0 {
                    return;
                }
                bytes.extend_from_slice(&buffer[..count]);
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let Some((head, body)) = text.split_once("\r\n\r\n") else {
                    continue;
                };
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                if body.len() < length {
                    continue;
                }
                let path = head.split(' ').nth(1).unwrap_or_default().to_owned();
                break (path, body.to_owned());
            };
            record.lock().unwrap().push((path.clone(), body));
            let (status, answer) = match path.as_str() {
                "/.well-known/openid-configuration" => (200, discovery.clone()),
                "/oauth/jwks" => (200, keys.clone()),
                "/oauth/token" => token.clone(),
                _ => (404, json!({})),
            };
            let answer = answer.to_string();
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
                answer.len()
            );
            if socket.write_all(response.as_bytes()).await.is_err() {
                return;
            }
        }
    });
    Ok((Url::parse(&format!("{origin}/"))?, seen))
}

async fn sign_in(issuer: &Url) -> std::result::Result<SignIn, lys_pass::Error> {
    SignIn::connect(
        reqwest::Client::new(),
        issuer,
        "sample",
        "lys-client.sample",
        Secret::new(SECRET.to_owned())?,
        Url::parse("https://studio.example/callback")?,
    )
    .await
}

fn form(body: &str) -> Vec<(String, String)> {
    url::form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect()
}

#[tokio::test]
async fn a_start_sends_the_browser_to_the_discovered_address_with_s256() -> Result {
    let (issuer, _) = double(None, None).await?;
    let started = sign_in(&issuer).await?.start()?;
    let url = Url::parse(&started.url)?;
    assert_eq!(url.path(), "/oauth/authorize");
    let asked: std::collections::BTreeMap<String, String> =
        url.query_pairs().into_owned().collect();
    assert_eq!(asked["response_type"], "code");
    assert_eq!(asked["client_id"], "lys-client.sample");
    assert_eq!(asked["redirect_uri"], "https://studio.example/callback");
    assert_eq!(asked["state"], started.state);
    assert_eq!(asked["code_challenge_method"], "S256");
    assert_eq!(
        asked["code_challenge"],
        URL_SAFE_NO_PAD.encode(Sha256::digest(started.verifier.as_bytes()))
    );
    assert!(!started.url.contains(&started.verifier), "the verifier left the server");
    assert!(!started.url.contains(SECRET), "the credential left the server");
    for value in [&started.state, &started.verifier, &started.browser] {
        assert_eq!(value.len(), 43);
    }
    assert_ne!(started.state, started.verifier);
    assert_ne!(started.state, started.browser);
    Ok(())
}

#[tokio::test]
async fn a_code_is_exchanged_with_credential_and_verifier_then_verified() -> Result {
    let (issuer, seen) = double(None, None).await?;
    let door = sign_in(&issuer).await?;
    let started = door.start()?;
    let signed_in = door
        .finish("code-1", &started.state, &started.browser, &started, NOW)
        .await?;
    assert_eq!(signed_in.holder, "holder");
    assert_eq!(signed_in.expires_at, 200);
    assert_eq!(signed_in.refresh.as_deref(), Some("refresh-1"));
    let requests = seen.lock().unwrap().clone();
    let (_, body) = requests
        .iter()
        .find(|(path, _)| path == "/oauth/token")
        .ok_or("no token request was made")?;
    let sent: std::collections::BTreeMap<String, String> = form(body).into_iter().collect();
    assert_eq!(sent["grant_type"], "authorization_code");
    assert_eq!(sent["code"], "code-1");
    assert_eq!(sent["code_verifier"], started.verifier);
    assert_eq!(sent["client_id"], "lys-client.sample");
    assert_eq!(sent["client_secret"], SECRET);
    assert_eq!(sent["redirect_uri"], "https://studio.example/callback");
    assert!(
        requests.iter().any(|(path, _)| path == "/oauth/jwks"),
        "the pass was used without the published keys"
    );
    Ok(())
}

#[tokio::test]
async fn another_browsers_state_is_refused_before_lys_is_asked() -> Result {
    let (issuer, seen) = double(None, None).await?;
    let door = sign_in(&issuer).await?;
    let started = door.start()?;
    let other: Started = door.start()?;
    for (state, browser) in [
        (other.state.as_str(), started.browser.as_str()),
        (started.state.as_str(), other.browser.as_str()),
    ] {
        let refused = door
            .finish("code-1", state, browser, &started, NOW)
            .await
            .err()
            .ok_or("a sign-in finished for another browser")?;
        assert_eq!(refused.name(), "sign_in_state_refused");
    }
    assert!(
        seen.lock().unwrap().iter().all(|(path, _)| path != "/oauth/token"),
        "the code was sent for a state that was not this browser's"
    );
    Ok(())
}

#[tokio::test]
async fn a_discovery_document_naming_another_issuer_is_refused() -> Result {
    let (issuer, _) = double(Some("https://elsewhere.example"), None).await?;
    let refused = sign_in(&issuer)
        .await
        .err()
        .ok_or("another issuer's discovery was taken")?;
    assert_eq!(refused.name(), "wrong_issuer");
    Ok(())
}

#[tokio::test]
async fn an_issuer_refusal_is_named_and_carries_no_credential() -> Result {
    let refusal = json!({"error": "invalid_grant", "error_description": "the code was used",
        "refusal": "code_used", "reason": "the code was used"});
    let (issuer, _) = double(None, Some((400, refusal))).await?;
    let door = sign_in(&issuer).await?;
    let started = door.start()?;
    let refused = door
        .finish("code-1", &started.state, &started.browser, &started, NOW)
        .await
        .err()
        .ok_or("a refused exchange answered a pass")?;
    assert_eq!(refused.name(), "code_used");
    for shown in [refused.to_string(), format!("{refused:?}"), format!("{door:?}")] {
        assert!(!shown.contains(SECRET), "the credential was shown: {shown}");
    }
    Ok(())
}

#[tokio::test]
async fn a_refresh_is_verified_and_an_unreachable_lys_is_named() -> Result {
    let (issuer, _) = double(None, None).await?;
    let door = sign_in(&issuer).await?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let closed = Url::parse(&format!("http://127.0.0.1:{port}/"))?;
    let unreachable = sign_in(&closed).await.err().ok_or("a closed port answered")?;
    assert_eq!(unreachable.name(), "lys_could_not_be_asked");
    let refreshed = door.refresh("refresh-1", NOW).await?;
    assert_eq!(refreshed.holder, "holder");
    Ok(())
}

#[test]
fn cookies_are_http_only_and_secure_and_refuse_what_is_not_a_token() -> Result {
    assert_eq!(
        session_cookie("haem_session", "abc123", 3600)?,
        "haem_session=abc123; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=3600"
    );
    assert_eq!(
        state_cookie("haem_state", "x_y-z", 600)?,
        "haem_state=x_y-z; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=600"
    );
    for (name, value) in [("haem_session", "a;b"), ("", "a"), ("a b", "c"), ("n", "")] {
        assert!(session_cookie(name, value, 1).is_err(), "{name:?}={value:?} was set");
    }
    Ok(())
}

#[test]
fn a_secret_is_one_line_and_never_shown() {
    assert!(Secret::new(String::new()).is_err());
    assert!(Secret::new(format!("{SECRET}\n")).is_err());
    let shown = format!("{:?}", Secret::new(SECRET.to_owned()));
    assert!(!shown.contains(SECRET), "{shown}");
}
