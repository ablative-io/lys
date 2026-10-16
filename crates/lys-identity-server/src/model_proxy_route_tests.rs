//! A run on a dialled-in computer reaches Lys's proxy through Lys's public
//! address: keyed model calls only, carried as they came and streamed.

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::routing::any;

const KEY: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn only_a_keyed_model_call_is_carried() {
    assert!(super::keyed_call(&format!("/{KEY}/anthropic/v1/messages")));
    assert!(super::keyed_call(&format!("/{KEY}/openai/v1/responses")));
    assert!(!super::keyed_call("/anthropic/v1/messages"));
    assert!(!super::keyed_call(&format!("/{KEY}/_lys/account/bind")));
    assert!(!super::keyed_call("/_lys/account/key"));
    assert!(!super::keyed_call("/api/me"));
    assert!(!super::keyed_call("/0123456789ABCDEF0123456789ABCDEF/anthropic/v1/messages"));
}

#[test]
fn the_dialled_address_is_the_public_origin_with_the_proxy_path() {
    assert_eq!(super::path_of("http://127.0.0.1:8484/anthropic"), "/anthropic");
    assert_eq!(super::path_of("http://127.0.0.1:8484"), "");
    assert_eq!(super::origin_of("http://127.0.0.1:8484/anthropic"), "http://127.0.0.1:8484");
    let public = format!("https://lys.example{}", super::path_of("http://127.0.0.1:8484/anthropic"));
    assert_eq!(
        lys_home::harness::rendering::keyed(&public, KEY).as_deref(),
        Some(format!("https://lys.example/{KEY}/anthropic").as_str())
    );
}

#[tokio::test]
async fn a_keyed_call_reaches_the_proxy_whole_and_streams_back()
-> Result<(), Box<dyn std::error::Error>> {
    // A stand-in proxy that answers with what it was sent.
    let proxy = axum::Router::new().fallback(any(|request: Request| async move {
        let path = request.uri().path().to_owned();
        let host = request
            .headers()
            .get("host")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let key = request
            .headers()
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let body = axum::body::to_bytes(request.into_body(), usize::MAX)
            .await
            .unwrap_or_default();
        format!("{path}|{host}|{key}|{}", String::from_utf8_lossy(&body))
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move { axum::serve(listener, proxy).await });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/{KEY}/anthropic/v1/messages?beta=true"))
        .header("host", "lys.example")
        .header("x-api-key", "caller-key")
        .body(Body::from("{\"model\":\"m\"}"))?;
    let response = super::forward(&format!("http://{address}/anthropic"), request).await;
    assert_eq!(response.status(), StatusCode::OK);
    let text = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let text = String::from_utf8(text.to_vec())?;
    assert_eq!(
        text,
        format!("/{KEY}/anthropic/v1/messages|{address}|caller-key|{{\"model\":\"m\"}}")
    );
    Ok(())
}

#[tokio::test]
async fn an_unreachable_proxy_is_named_never_hung() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    drop(listener);
    let request = Request::builder()
        .method("POST")
        .uri(format!("/{KEY}/anthropic/v1/messages"))
        .body(Body::empty())?;
    let response = super::forward(&format!("http://{address}/anthropic"), request).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let text = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    assert!(String::from_utf8(text.to_vec())?.contains("could not reach its model proxy"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_start_on_a_dialled_in_computer_is_given_the_public_proxy()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::network_store::{Machine, NetworkStore};
    use crate::runner_client::RunnerRecord;
    let (dir, mut state, _, person, agent, _) =
        crate::grant_settlement_tests::person_fixture().await?;
    state.model_proxy = Some("http://127.0.0.1:8484/anthropic".to_owned());
    state.dialled_model_proxy = Some("https://lys.example/anthropic".to_owned());
    let mut network = NetworkStore::open(&dir.path().join("network.json"))?;
    for id in ["near", "far"] {
        network.name(Machine {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: "computer".to_owned(),
            runtime: Some("sh".to_owned()),
            slots: 1,
            may_run: vec![agent.to_string()],
            may_run_roles: Vec::new(),
            may_reach: Vec::new(),
            named_by: person.to_string(),
            named_at: 1,
            retired: None,
            team: None,
            creation_team: None,
        })?;
    }
    network.name_runner(
        "near",
        Some(RunnerRecord::Socket {
            path: dir.path().join("runner.sock").to_string_lossy().into_owned(),
        }),
    )?;
    network.name_runner(
        "far",
        Some(RunnerRecord::Dialled {
            key: "ab".repeat(32),
            runner: None,
        }),
    )?;
    state.network = Some(std::sync::Mutex::new(network));
    // The computer beside Lys reaches the proxy on loopback; a computer that
    // dials in is given Lys's public address, where loopback names nothing.
    assert_eq!(
        super::for_machine(&state, "near")?,
        Some("http://127.0.0.1:8484/anthropic")
    );
    assert_eq!(
        super::for_machine(&state, "far")?,
        Some("https://lys.example/anthropic")
    );
    Ok(())
}
