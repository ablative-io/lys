#![cfg(test)]
//! Account draws retain the source handle's authority and never expose its value.
use crate::files::{FileGrants, now_ms};
use crate::values_tests::{Outcome, PERSON, broker, never_written, upstream, vouched};
use axum::extract::State;
use lys_core::Ed25519Identity;
use lys_secrets::{Holder, Relation, to_hex};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn account_binding_checks_holder_and_provider_and_keeps_the_token_sealed() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (shared, service) = broker(dir.path())?;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = upstream(seen).await?;
    let token = "account-binding-sentinel-739abc";
    let body = serde_json::to_vec(
        &json!({"operation":"binding-add-0123456789", "name":"anthropic-account", "class":"credential", "upstream":base, "header":"authorization", "prefix":"Bearer ", "value":token}),
    )?;
    let added = crate::values::add(
        State(Arc::clone(&shared)),
        vouched(&service, ("POST", "/_lys/add"), PERSON, &body)?,
    )
    .await
    .map_err(|(_, e)| e)?;
    assert!(added.0.get("value").is_none());
    grant_account(&shared)?;
    let key = Ed25519Identity::load_or_generate(&dir.path().join("agent.key"))?;
    let held = shared.broker.lock().map_err(|e| e.to_string())?.issue(
        &Holder {
            identity: "agent-a".to_owned(),
            key: key.public_key_bytes(),
        },
        "anthropic-account",
        10,
        now_ms() + 600_000,
    )?;
    let proxy = Ed25519Identity::load_or_generate(&dir.path().join("proxy.key"))?;
    let asked = |agent: &str, provider: &str| {
        serde_json::to_vec(
            &json!({"broker":"http://127.0.0.1:8472","run":"0123456789abcdef0123456789abcdef","agent":agent,"provider":provider,"accounts":[held.id.as_str()],"proxy_key":to_hex(&proxy.public_key_bytes())}),
        )
    };
    for (agent, provider, refusal) in [
        ("agent-b", base.as_str(), "account_handle_unheld"),
        (
            "agent-a",
            "https://api.openai.com",
            "account_provider_mismatch",
        ),
    ] {
        let error = crate::account_binding::bind(
            State(Arc::clone(&shared)),
            vouched(
                &service,
                ("POST", "/_lys/account/bind"),
                PERSON,
                &asked(agent, provider)?,
            )?,
        )
        .await
        .expect_err("invalid binding admitted");
        assert!(error.1.starts_with(refusal), "{}", error.1);
        assert!(error.1.contains(held.id.as_str()));
    }
    let binding = crate::account_binding::bind(
        State(Arc::clone(&shared)),
        vouched(
            &service,
            ("POST", "/_lys/account/bind"),
            PERSON,
            &asked("agent-a", &base)?,
        )?,
    )
    .await
    .map_err(|(_, e)| e)?
    .0;
    assert_eq!(binding["draws"][0]["source"], held.id.as_str());
    assert!(!binding.to_string().contains(token));
    let draw = lys_secrets::IssuedHandle {
        id: lys_secrets::HandleId::from_text(
            binding["draws"][0]["id"].as_str().ok_or("id absent")?,
        ),
        token: lys_secrets::HandleToken::from_bytes(
            &lys_secrets::from_hex(binding["draws"][0]["token"].as_str().ok_or("draw absent")?)
                .ok_or("draw malformed")?,
        ),
    };
    let (status, answer) =
        crate::values_tests::call(&shared, ("anthropic-account", &draw), &proxy).await?;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(!answer.contains(token));
    shared
        .broker
        .lock()
        .map_err(|e| e.to_string())?
        .drop_handle(&held.id)?;
    let (status, answer) =
        crate::values_tests::call(&shared, ("anthropic-account", &draw), &proxy).await?;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    assert!(answer.contains("HandleDropped"));
    never_written(dir.path(), token)?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_account_proxy_replaces_placeholder_records_source_and_never_exposes_value() -> Outcome
{
    use lys_identity_server::model_account_start::model_proxy::forward::{
        Base, Proxy, ProxyConfig,
    };
    let dir = tempfile::tempdir()?;
    let (shared, service) = broker(dir.path())?;
    let fixture = account_fixture(dir.path(), &shared, &service).await?;
    let base = &fixture.base;
    let token = fixture.token;
    let seen = &fixture.seen;
    let handle = &fixture.handle;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let broker_url = format!("http://{}", listener.local_addr()?);
    let broker_routes = crate::router::routes(Arc::clone(&shared));
    let broker_task = tokio::spawn(async move { axum::serve(listener, broker_routes).await });
    let state = dir.path().join("proxy-state");
    let started = Proxy::start(ProxyConfig {
        home: dir.path().join("home"),
        state: state.clone(),
        anthropic: Base::parse(base)?,
        openai: Base::parse(base)?,
        chatgpt: Base::parse(base)?,
    })?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let proxy_url = format!("http://{}", listener.local_addr()?);
    let proxy = Arc::clone(&started.proxy);
    let proxy_task = tokio::spawn(async move { proxy.serve(listener).await });
    let client = reqwest::Client::new();
    let proxy_key = client
        .get(format!("{proxy_url}/_lys/account/key"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let run = "0123456789abcdef0123456789abcdef";
    let binding = serde_json::to_vec(
        &json!({"broker":broker_url,"run":run,"agent":"agent-a","provider":base,"accounts":[handle.id.as_str()],"proxy_key":proxy_key.trim()}),
    )?;
    let signed = vouched(&service, ("POST", "/_lys/account/bind"), PERSON, &binding)?;
    let response = client
        .post(format!("{proxy_url}/_lys/account/bind"))
        .headers(signed.headers().clone())
        .body(binding.clone())
        .send()
        .await?;
    let status = response.status();
    let answer = response.text().await?;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    assert!(!answer.contains(token));
    let denied = client
        .post(format!("{proxy_url}/_lys/account/bind"))
        .body(binding)
        .send()
        .await?;
    let expected = crate::callers::refused(&lys_secrets::SecretsError::HandleUnknown);
    assert_eq!(denied.status().as_u16(), expected.0.as_u16());
    assert_eq!(denied.text().await?, expected.1);
    let selector = format!("lys-account.{}", handle.id);
    let response = client
        .post(format!("{proxy_url}/{run}/anthropic/v1/messages"))
        .header("authorization", format!("Bearer {selector}"))
        .body(r#"{"model":"fixture"}"#)
        .send()
        .await?;
    let status = response.status();
    let answer = response.text().await?;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    assert!(!answer.contains(token));
    assert_eq!(
        seen.lock().map_err(|e| e.to_string())?[0],
        format!("Bearer {token}")
    );
    let reports = started.reports;
    let report = tokio::task::spawn_blocking(move || reports.recv()).await??;
    assert!(!serde_json::to_string(&report)?.contains(token));
    let bytes = std::fs::read_to_string(state.join("usage").join(format!("{run}.jsonl")))?;
    let lines = bytes
        .lines()
        .map(
            serde_json::from_str::<
                lys_identity_server::model_account_start::model_proxy::usage::UsageLine,
            >,
        )
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(lines[0].account.as_deref(), Some(handle.id.as_str()));
    let forged = client
        .post(format!("{proxy_url}/{run}/anthropic/v1/messages"))
        .header("authorization", "Bearer lys-account.unheld")
        .body("{}")
        .send()
        .await?;
    assert_eq!(forged.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(seen.lock().map_err(|e| e.to_string())?.len(), 1);
    claude_proof(dir.path(), &proxy_url, run, selector, token).await?;
    never_written(dir.path(), token)?;
    proxy_task.abort();
    broker_task.abort();
    Ok(())
}

#[tokio::test]
async fn live_account_hold_returns_only_the_id_and_requires_the_agents_grant() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (shared, service) = broker(dir.path())?;
    let value = "hold-sentinel-value-44912";
    let body = serde_json::to_vec(
        &json!({"operation":"hold-add-0123456789","name":"anthropic-account","class":"credential","upstream":"https://api.anthropic.com","header":"authorization","prefix":"Bearer ","value":value}),
    )?;
    let added = crate::values::add(
        State(Arc::clone(&shared)),
        vouched(&service, ("POST", "/_lys/add"), PERSON, &body)?,
    )
    .await
    .map_err(|(_, e)| e)?;
    assert!(added.0.get("value").is_none());
    let proxy = Ed25519Identity::load_or_generate(&dir.path().join("proxy.key"))?;
    let asked = serde_json::to_vec(
        &json!({"agent":"agent-a","secret":"anthropic-account","proxy_key":to_hex(&proxy.public_key_bytes()),"max_uses":10,"not_after_ms":now_ms()+600_000}),
    )?;
    assert!(
        crate::account_binding::hold(
            State(Arc::clone(&shared)),
            vouched(&service, ("POST", "/_lys/account/hold"), PERSON, &asked)?
        )
        .await
        .is_err()
    );
    grant_account(&shared)?;
    let receipt = crate::account_binding::hold(
        State(Arc::clone(&shared)),
        vouched(&service, ("POST", "/_lys/account/hold"), PERSON, &asked)?,
    )
    .await
    .map_err(|(_, e)| e)?
    .0;
    assert!(receipt["handle"].as_str().is_some());
    assert!(receipt.get("token").is_none());
    assert!(!receipt.to_string().contains(value));
    never_written(dir.path(), value)?;
    Ok(())
}

struct AccountFixture {
    base: String,
    token: &'static str,
    seen: Arc<Mutex<Vec<String>>>,
    handle: lys_secrets::IssuedHandle,
    upstream: tokio::task::JoinHandle<std::io::Result<()>>,
}
impl Drop for AccountFixture {
    fn drop(&mut self) {
        self.upstream.abort();
    }
}
async fn account_fixture(
    dir: &std::path::Path,
    shared: &Arc<crate::serve::Shared>,
    service: &Ed25519Identity,
) -> Outcome<AccountFixture> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let token = "run-proxy-sentinel-348f01";
    let observed = Arc::clone(&seen);
    let router=axum::Router::new().fallback(move|headers:axum::http::HeaderMap| {
        let observed=Arc::clone(&observed);
        async move {
            let value=headers.get("authorization").and_then(|v|v.to_str().ok()).unwrap_or("missing").to_owned();
            observed.lock().expect("fixture lock").push(value);
            axum::Json(json!({"id":"msg_fixture","type":"message","role":"assistant","model":"claude-haiku-4-5","content":[{"type":"text","text":"account proxy answered"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":3}}))
        }
    });
    let upstream_task = tokio::spawn(async move { axum::serve(listener, router).await });
    let add = serde_json::to_vec(
        &json!({"operation":"proxy-add-0123456789","name":"anthropic-account","class":"credential","upstream":base,"header":"authorization","prefix":"Bearer ","value":token}),
    )?;
    let request = vouched(service, ("POST", "/_lys/add"), PERSON, &add)?;
    let receipt = crate::values::add(State(Arc::clone(shared)), request)
        .await
        .map_err(|(_, e)| e)?
        .0;
    assert!(!receipt.to_string().contains(token));
    grant_account(shared)?;
    let key = Ed25519Identity::load_or_generate(&dir.join("agent.key"))?;
    let handle = shared.broker.lock().map_err(|e| e.to_string())?.issue(
        &Holder {
            identity: "agent-a".to_owned(),
            key: key.public_key_bytes(),
        },
        "anthropic-account",
        100,
        now_ms() + 600_000,
    )?;
    Ok(AccountFixture {
        base,
        token,
        seen,
        handle,
        upstream: upstream_task,
    })
}
async fn claude_proof(
    dir: &std::path::Path,
    proxy_url: &str,
    run: &str,
    selector: String,
    token: &str,
) -> Outcome {
    if let Some(claude) = std::env::var_os("LYS_ACCOUNT_CLAUDE_PROOF") {
        let config = dir.join("claude-config");
        std::fs::create_dir(&config)?;
        let base = format!("{proxy_url}/{run}/anthropic");
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(claude)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", &config)
                .env("CLAUDE_CONFIG_DIR", &config)
                .env("ANTHROPIC_BASE_URL", base)
                .env("ANTHROPIC_AUTH_TOKEN", selector)
                .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
                .env("DISABLE_AUTOUPDATER", "1")
                .env("DISABLE_TELEMETRY", "1")
                .env("DISABLE_ERROR_REPORTING", "1")
                .current_dir(&config)
                .args([
                    "--print",
                    "--model",
                    "claude-haiku-4-5",
                    "--setting-sources=",
                    "--strict-mcp-config",
                    "--mcp-config",
                    "{\"mcpServers\":{}}",
                    "Reply with one word.",
                ])
                .output()
        })
        .await??;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let answer = String::from_utf8(output.stdout)?;
        assert!(answer.contains("account proxy answered"), "{answer}");
        assert!(!answer.contains(token));
        println!(
            "Claude account proof: empty login, proxy plus broker, exit 0, account proxy answered"
        );
    }
    Ok(())
}

fn grant_account(shared: &crate::serve::Shared) -> Outcome {
    FileGrants::new(shared.layout.grants()).set(
        Relation::Use,
        "agent-a",
        "anthropic-account",
        Some(PERSON),
    )?;
    FileGrants::new(shared.layout.grants()).set(
        Relation::Member,
        "agent-a",
        &lys_secrets::Scope::Personal(PERSON.to_owned()).target(),
        Some(PERSON),
    )?;
    Ok(())
}
