#![cfg(test)]
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use identity_contract::apps::op;
use tower::ServiceExt;

use serde_json::{Value, json};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Proxy {
    anthropic: AtomicBool,
    bindings: Mutex<Vec<Value>>,
}
struct Serving(tokio::task::JoinHandle<std::io::Result<()>>);
impl Drop for Serving {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn answer(State(proxy): State<Arc<Proxy>>, request: Request) -> Response {
    match request.uri().path() {
        "/_lys/handles"=>Json(json!({"handles":[{"id":"first","secret":"login","dropped":false}]})).into_response(),
        "/_lys/secrets"=>Json(json!({"secrets":[{"name":"login","class":"credential","header":"authorization","upstream":if proxy.anthropic.load(Ordering::SeqCst) {"https://api.anthropic.com"} else {"https://api.openai.com"}}]})).into_response(),
        "/_lys/account/key"=>"ab".repeat(32).into_response(),
        "/_lys/account/bind"=>{
            if request.headers().get("lys-service").and_then(|h|h.to_str().ok())!=Some("identity")
                || !request.headers().contains_key("lys-service-signature") {
                return (StatusCode::BAD_REQUEST,"fixture_binding_unsigned").into_response();
            }
            let person=request.headers().get("lys-on-behalf-of").and_then(|h|h.to_str().ok()).map(str::to_owned);
            let bytes=match axum::body::to_bytes(request.into_body(),usize::MAX).await {
                Ok(bytes)=>bytes,
                Err(error)=>return (StatusCode::BAD_REQUEST,error.to_string()).into_response(),
            };
            let body:Value=match serde_json::from_slice(&bytes) {
                Ok(body)=>body,
                Err(error)=>return (StatusCode::BAD_REQUEST,error.to_string()).into_response(),
            };
            match proxy.bindings.lock() {
                Ok(mut bindings)=>bindings.push(json!({"person":person,"body":body})),
                Err(error)=>return (StatusCode::INTERNAL_SERVER_ERROR,error.to_string()).into_response(),
            }
            Json(json!({"bound":true})).into_response()
        }
        _=>(StatusCode::NOT_FOUND,"fixture_route_unknown").into_response(),
    }
}

fn profile(wire: &str, operation: &str, from_version: u32) -> Value {
    json!({"operation":operation,"from_version":from_version,"model_access":[],"tools":[],"skills":[],"mcp_servers":[],"instructions":"","note":"",
        "harness":{"name":"seat","program":"/opt/seat/host","package":"seat 1","description":{
            "rendering_contract":"seat-program/v1","models":{"minimum":0,"maximum":0,"further_encoding":{"kind":"array"}},
            "permissions":{"modes":[],"rule_forms":[]},"mcp":{"transports":[],"working_directory":false,"handle_variables":false,"channel_policies":[]},
            "seat_program":{"executable":"/opt/seat/host","executable_sha256":"a".repeat(64),"arguments":["{seat_file}"],"seat_file":"/opt/seat/seat.json","seat_file_sha256":"b".repeat(64),"adapter":"acp/1","model_wire":wire}}},
        "session":{"stall_after":300,"accounts":{"accounts":["first"],"variable":"LYS_HANDLE_ACCOUNT","limit":{"signal":"plan_window"}}}})
}

#[tokio::test]
async fn both_seat_program_wires_bind_the_run_at_the_proxy_without_receiving_a_credential()
-> Result<(), Box<dyn Error>> {
    let proxy = Arc::new(Proxy::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}", listener.local_addr()?);
    let router = Router::new()
        .fallback(answer)
        .with_state(Arc::clone(&proxy));
    let serving = Serving(tokio::spawn(
        async move { axum::serve(listener, router).await },
    ));
    let keys = tempfile::tempdir()?;
    let (fixture, cookie, person) = prepared(keys.path(), &address).await?;
    let agent = fixture.agent.clone();
    let path = format!("/agents/{agent}/provisioning");
    let router = crate::provisioning_api::routes().with_state(Arc::clone(&fixture.state));
    for (from_version, wire, provider, variable) in [
        (
            0,
            "openai-responses",
            "https://api.openai.com",
            "CODEX_API_KEY",
        ),
        (
            1,
            "anthropic-messages",
            "https://api.anthropic.com",
            "ANTHROPIC_AUTH_TOKEN",
        ),
    ] {
        proxy
            .anthropic
            .store(wire == "anthropic-messages", Ordering::SeqCst);
        let request = axum::http::Request::post(&path)
            .header("cookie", &cookie)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                profile(wire, &op()?, from_version).to_string(),
            ))?;
        let answer = router.clone().oneshot(request).await?;
        let status = answer.status();
        let body = axum::body::to_bytes(answer.into_body(), usize::MAX).await?;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let version = from_version + 1;
        let run = format!("{version:032x}");
        let mut launch: lys_runner::Launch = serde_json::from_value(
            json!({"session":"fixture","program":"/opt/seat/host","arguments":[],"directory":"/opt/seat","columns":80,"rows":24,
            "environment":{"LYS_RUN":run,"LYS_PROVISIONING_VERSION":version.to_string()},
            "rotation":{"accounts":["first"],"variable":"LYS_HANDLE_ACCOUNT","limit":{"signal":"plan_window"}}}),
        )?;
        crate::model_account_start::bind(&fixture.state, &agent, &person, &mut launch).await?;
        assert_eq!(launch.environment["LYS_ACCOUNT_VARIABLE"], variable);
        assert_eq!(launch.environment[variable], "lys-account.first");
        if wire == "anthropic-messages" {
            assert_eq!(launch.environment["ANTHROPIC_API_KEY"], "");
            assert_eq!(launch.environment["CLAUDE_CODE_OAUTH_TOKEN"], "");
        }
        let bindings = proxy.bindings.lock().map_err(|error| error.to_string())?;
        let binding = bindings.last().ok_or("binding absent")?;
        assert_eq!(binding["person"], person);
        assert_eq!(binding["body"]["run"], run);
        assert_eq!(binding["body"]["provider"], provider);
        assert_eq!(binding["body"]["agent"], agent);
        assert_eq!(binding["body"]["accounts"], json!(["first"]));
        assert_eq!(bindings.len(), usize::try_from(version)?);
    }
    drop(router);
    fixture.finish().await?;
    drop(serving);
    Ok(())
}

async fn prepared(
    keys: &std::path::Path,
    address: &str,
) -> Result<(crate::budgets_feed::entry_tests::Fixture, String, String), Box<dyn Error>> {
    let service_key_file = keys.join("service.key");
    lys_core::Ed25519Identity::load_or_generate(&service_key_file)?;
    let secrets = crate::secrets_api::SecretsSettings {
        broker: address.to_owned(),
        service: "identity".into(),
        service_key_file,
    };
    let mut fixture = crate::budgets_feed::entry_tests::fixture().await?;
    let state = Arc::get_mut(&mut fixture.state).ok_or("fixture state already shared")?;
    state.model_proxy = Some(format!("{address}/anthropic"));
    state.secrets = Some(crate::secrets_api::SecretsBroker::open(&secrets)?);
    state.provisioning = Some(Mutex::new(
        crate::provisioning_store::ProvisioningStore::open(&keys.join("provisioning.json"))?,
    ));
    let binding = state
        .admission
        .administrator_login()?
        .ok_or("administrator absent")?;
    let actor = lys_identity::Actor::new(
        binding,
        lys_identity::Provenance::new(lys_identity::AuthMethod::Oidc, crate::session::now()),
    );
    let person = crate::routes::with_directory(state, |directory| {
        Ok(crate::read_api::own_person(directory.projection()?, &actor)?.to_string())
    })?;
    let cookie = state.sessions.begin(actor)?;
    let cookie = cookie
        .split(';')
        .next()
        .ok_or("session cookie absent")?
        .to_owned();
    Ok((fixture, cookie, person))
}
