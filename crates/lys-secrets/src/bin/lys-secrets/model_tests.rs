#![cfg(test)]
//! A model account is registered with its token, drawn on for an agent and
//! used through the model proxy's key, against a stand-in upstream. The
//! upstream is sent the real token; the draw's answer, every answer that
//! comes back, and every file the broker writes outside its sealed entries
//! never carry it. Only the model proxy's key presents a draw, only an
//! agent granted the account is given one, and a retired account gives none
//! and ends every draw on it.
//!
//! This is proved against a stand-in upstream only. Whether the provider
//! itself takes the swapped token is proved once a real token is registered
//! after the install.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use lys_core::Ed25519Identity;
use lys_secrets::{HandleId, HandleToken, IssuedHandle, Relation, ServiceKey, from_hex, to_hex};
use serde_json::{Value, json};

use crate::files::FileGrants;
use crate::model::PROXY_SERVICE;
use crate::values_tests::{Outcome, PERSON, broker, call, never_written, upstream, vouched};

const TOKEN: &str = "sentinel-model-token-5be04c9d1a7f";
const AGENT: &str = "agent-drawing";
const OPERATION: &str = "op-model-0123456789abcdef01234567";

type Answered = Result<axum::Json<Value>, (StatusCode, String)>;

fn answered(answer: Answered) -> Outcome<Value> {
    answer
        .map(|json| json.0)
        .map_err(|(status, words)| format!("{status} {words}").into())
}

fn refused(answer: Answered) -> Outcome<(StatusCode, String)> {
    match answer {
        Ok(json) => Err(format!("answered where a refusal was due: {}", json.0).into()),
        Err(refusal) => Ok(refusal),
    }
}

/// The draw an answer carries, as the proxy holds it.
fn drawn(answer: &Value) -> Outcome<IssuedHandle> {
    let token = from_hex(answer["token"].as_str().ok_or("no handle")?).ok_or("not hex")?;
    Ok(IssuedHandle {
        id: HandleId::from_text(answer["handle"].as_str().ok_or("no handle id")?),
        token: HandleToken::from_bytes(&token),
    })
}

#[tokio::test]
async fn a_model_account_is_drawn_on_through_the_proxy_key_and_its_token_goes_only_upstream()
-> Outcome {
    let dir = tempfile::tempdir()?;
    let (shared, key) = broker(dir.path())?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = upstream(Arc::clone(&seen)).await?;
    let register = serde_json::to_vec(&json!({
        "operation": OPERATION, "name": "Main account", "harness": "Fixture program",
        "upstream": base, "header": "authorization", "prefix": "Bearer ", "token": TOKEN,
    }))?;
    let registered = answered(
        crate::model::register(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/model/register"), PERSON, &register)?,
        )
        .await,
    )?;
    assert!(!registered.to_string().contains(TOKEN));
    let account = registered["account"].as_str().ok_or("no account")?.to_owned();
    assert_eq!(account, format!("model-account-{OPERATION}"));
    assert_eq!(registered["registered_by"], PERSON);

    let draw = serde_json::to_vec(&json!({ "account": account, "agent": AGENT }))?;
    let untrusted = refused(
        crate::model::draw(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/model/draw"), PERSON, &draw)?,
        )
        .await,
    )?;
    assert!(untrusted.1.starts_with("ModelProxyUntrusted"), "{}", untrusted.1);
    let proxy = Ed25519Identity::load_or_generate(&dir.path().join("proxy.key"))?;
    shared.layout.trust_service(ServiceKey {
        name: PROXY_SERVICE.to_owned(),
        public_key: to_hex(&proxy.public_key_bytes()),
    })?;
    let ungranted = refused(
        crate::model::draw(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/model/draw"), PERSON, &draw)?,
        )
        .await,
    )?;
    assert_eq!(ungranted.0, StatusCode::FORBIDDEN);
    assert!(
        ungranted.1.starts_with("ModelAccountNotDrawable"),
        "{}",
        ungranted.1
    );

    FileGrants::new(shared.layout.grants()).set(Relation::Use, AGENT, &account, Some(PERSON))?;
    let answer = answered(
        crate::model::draw(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/model/draw"), PERSON, &draw)?,
        )
        .await,
    )?;
    assert!(!answer.to_string().contains(TOKEN));
    let issued = drawn(&answer)?;
    // Presented with the model proxy's key, the call is sent with the real
    // token, and the token is taken out of what comes back.
    let (status, text) = call(&shared, (&account, &issued), &proxy).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(!text.contains(TOKEN), "the answer carried the token: {text}");
    assert_eq!(
        seen.lock().expect("test state lock poisoned").last(),
        Some(&format!("Bearer {TOKEN}"))
    );
    // Presented with any other key, the draw opens nothing.
    let agent = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let (status, _) = call(&shared, (&account, &issued), &agent).await?;
    assert_ne!(status, StatusCode::OK);
    assert_eq!(seen.lock().expect("test state lock poisoned").len(), 1);

    let listed = answered(
        crate::model::accounts(
            State(Arc::clone(&shared)),
            vouched(&key, ("GET", "/_lys/model/accounts"), PERSON, b"")?,
        )
        .await,
    )?;
    assert_eq!(listed["accounts"][0]["name"], "Main account");
    assert_eq!(listed["accounts"][0]["draws"][0]["identity"], AGENT);
    assert_eq!(listed["accounts"][0]["draws"][0]["calls"], 1);
    assert!(!listed.to_string().contains(TOKEN));

    let retire = serde_json::to_vec(&json!({
        "operation": "op-model-retire-0123456789abcdef", "account": account,
    }))?;
    let retired = answered(
        crate::model::retire(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/model/retire"), PERSON, &retire)?,
        )
        .await,
    )?;
    assert!(retired["retired_at"].as_i64().is_some());
    let after = refused(
        crate::model::draw(
            State(Arc::clone(&shared)),
            vouched(&key, ("POST", "/_lys/model/draw"), PERSON, &draw)?,
        )
        .await,
    )?;
    assert!(after.1.starts_with("ModelAccountRetired"), "{}", after.1);
    let (status, _) = call(&shared, (&account, &issued), &proxy).await?;
    assert_ne!(status, StatusCode::OK, "a retired account's draw was used");
    assert_eq!(seen.lock().expect("test state lock poisoned").len(), 1);
    never_written(dir.path(), TOKEN)?;
    Ok(())
}
