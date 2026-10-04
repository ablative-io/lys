#![cfg(test)]
//! The whole path of a call on a model account, against a stand-in
//! upstream: Lys's model proxy is handed a call carrying the run's
//! placeholder, presents the draw to this broker with its own key, the
//! broker writes the sealed token into the call and sends it on, and takes
//! the token out of what comes back. The stand-in upstream is sent the real
//! token; the run's answer, the proxy's record of the call and every file
//! the proxy and the broker keep outside the sealed entries never carry it.
//!
//! This proves the broker admits the proxy's presentation, written in its
//! own form in `lys-home`, and proves nothing of the provider: whether the
//! provider takes the swapped token is proved only once Tom registers a real
//! token after the install.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use lys_core::Ed25519Identity;
use lys_home::proxy::account::{Broker, placeholder};
use lys_home::proxy::forward::{Base, Proxy, ProxyConfig};
use lys_secrets::{Relation, ServiceKey, to_hex};
use serde_json::json;

use crate::files::FileGrants;
use crate::model::PROXY_SERVICE;
use crate::values_tests::{Outcome, PERSON, broker, never_written, upstream, vouched};

const TOKEN: &str = "sentinel-proxied-token-0c9e4a7b31d8";
const AGENT: &str = "agent-proxied";
const OPERATION: &str = "op-proxied-0123456789abcdef0123456";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_call_on_a_model_account_goes_through_the_proxy_and_the_broker_with_the_token_only_upstream()
-> Outcome {
    let dir = tempfile::tempdir()?;
    let (shared, key) = broker(dir.path())?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = upstream(Arc::clone(&seen)).await?;
    let register = serde_json::to_vec(&json!({
        "operation": OPERATION, "name": "Main account", "harness": "Fixture program",
        "upstream": base, "header": "authorization", "prefix": "Bearer ", "token": TOKEN,
    }))?;
    let registered = crate::model::register(
        State(Arc::clone(&shared)),
        vouched(&key, ("POST", "/_lys/model/register"), PERSON, &register)?,
    )
    .await
    .map_err(|(status, words)| format!("{status} {words}"))?
    .0;
    let account = registered["account"]
        .as_str()
        .ok_or("no account")?
        .to_owned();
    let proxy_key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("proxy.key"),
    )?);
    shared.layout.trust_service(ServiceKey {
        name: PROXY_SERVICE.to_owned(),
        public_key: to_hex(&proxy_key.public_key_bytes()),
    })?;
    FileGrants::new(shared.layout.grants()).set(Relation::Use, AGENT, &account, Some(PERSON))?;
    let draw = serde_json::to_vec(&json!({ "account": account, "agent": AGENT }))?;
    let drawn = crate::model::draw(
        State(Arc::clone(&shared)),
        vouched(&key, ("POST", "/_lys/model/draw"), PERSON, &draw)?,
    )
    .await
    .map_err(|(status, words)| format!("{status} {words}"))?
    .0;
    let given = placeholder(
        &account,
        drawn["handle"].as_str().ok_or("no handle id")?,
        drawn["token"].as_str().ok_or("no handle")?,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let broker_base = format!("http://{}", listener.local_addr()?);
    let routes = crate::router::routes(Arc::clone(&shared));
    tokio::spawn(async move { axum::serve(listener, routes).await });

    let nowhere = Base::parse("http://127.0.0.1:9")?;
    let started = Proxy::start(ProxyConfig {
        home: dir.path().join("proxy-home"),
        state: dir.path().join("proxy-state"),
        anthropic: nowhere.clone(),
        chatgpt: nowhere.clone(),
        openai: nowhere,
        broker: Some(Broker {
            base: Base::parse(&broker_base)?,
            key: Arc::clone(&proxy_key),
        }),
    })?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let proxied = format!("http://{}", listener.local_addr()?);
    let proxy = Arc::clone(&started.proxy);
    tokio::spawn(async move { proxy.serve(listener).await });

    let answer = reqwest::Client::new()
        .post(format!("{proxied}/anthropic/v1/messages"))
        .header("authorization", format!("Bearer {given}"))
        .header("content-type", "application/json")
        .body(r#"{"model":"fixture","metadata":{"user_id":"user_0_account_0_session_0"}}"#)
        .send()
        .await?;
    let status = answer.status();
    let text = answer.text().await?;
    assert_eq!(status, reqwest::StatusCode::OK, "{text}");
    assert!(
        !text.contains(TOKEN),
        "the run's answer carried the token: {text}"
    );
    assert_eq!(
        seen.lock().expect("test state lock poisoned").last(),
        Some(&format!("Bearer {TOKEN}")),
        "the upstream was sent the sealed token"
    );
    // The proxy records the call off the forwarding path; its report says
    // the record is written.
    let reports = started.reports;
    let recorded =
        tokio::task::spawn_blocking(move || reports.recv().map(|_report| reports)).await??;
    never_written(dir.path(), TOKEN)?;
    drop(recorded);

    // A call whose placeholder names a draw on another account opens
    // nothing, and nothing more reaches the upstream.
    let forged = given.replacen(&account, "model-account-elsewhere", 1);
    let refused = reqwest::Client::new()
        .post(format!("{proxied}/anthropic/v1/messages"))
        .header("authorization", format!("Bearer {forged}"))
        .body("{}")
        .send()
        .await?;
    assert_ne!(refused.status(), reqwest::StatusCode::OK);
    assert_eq!(seen.lock().expect("test state lock poisoned").len(), 1);
    Ok(())
}
