//! OAuth and tool-result helpers for person-consent HTTP regressions.
#![cfg(test)]
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use identity_contract::harness::Service;
use reqwest::{StatusCode, redirect::Policy};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
const REDIRECT: &str = "https://app.example/callback";
const VERIFIER: &str = "a-verifier-of-at-least-forty-three-characters-long";
pub(super) fn client() -> Result<reqwest::Client, Box<dyn Error>> {
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

pub(super) async fn connected_mode(
    service: &Service,
    cookie: &str,
    mode: Option<&str>,
) -> Result<(String, Value), Box<dyn Error>> {
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
    let mut form = vec![("asking", asking.as_str()), ("decision", "approve")];
    if let Some(mode) = mode {
        form.push(("mode", mode));
    }
    let answered = client
        .post(format!("{}/oauth/mcp/consent", service.base))
        .header("cookie", cookie)
        .form(&form)
        .send()
        .await?;
    assert_eq!(
        answered.status(),
        StatusCode::SEE_OTHER,
        "OAuth consent for mode {mode:?} was refused"
    );
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

pub(super) async fn connected(
    service: &Service,
    cookie: &str,
) -> Result<(String, Value), Box<dyn Error>> {
    connected_mode(service, cookie, None).await
}
pub(super) fn body(result: &Value) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(
        result["content"][0]["text"]
            .as_str()
            .ok_or("MCP answer has no JSON text")?,
    )?)
}

pub(super) fn unhex(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let text = value.as_str().ok_or("no hex text")?;
    if !text.len().is_multiple_of(2) {
        return Err("hex text has an odd length".into());
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)?;
            u8::from_str_radix(pair, 16).map_err(Box::<dyn Error>::from)
        })
        .collect()
}
