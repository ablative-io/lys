//! Shared admitted HTTP fixtures for transport and receipt tests.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::error::Error;
use tower::ServiceExt;

pub(super) async fn certified_agent() -> Result<
    (
        identity_contract::harness::Service,
        String,
        String,
        std::sync::Arc<lys_core::Ed25519Identity>,
    ),
    Box<dyn Error>,
> {
    use base64::Engine;
    use identity_contract::harness::{ADMINISTRATOR, Service};
    use lys_identity::{
        Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
        Transition,
    };
    let (service, (agent, key)) =
        Service::start_judging(&lys_identity::grants::shipped_model(), None, |config| {
            lys_log_store::FileLeafStore::create(&config.log_dir, &config.log_origin)?;
            let path = config.log_dir.clone();
            let key = lys_core::Ed25519Identity::load(&config.event_key_file)?;
            lys_identity::directory_migration::migrate(
                lys_log_store::FileLeafStore::open(&path)?,
                &key,
            )?;
            let mut directory = Directory::open(
                Box::new(move || lys_log_store::FileLeafStore::open(&path)),
                key,
            )?;
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let (person, _) = directory.setup_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Owner")?,
                1,
            )?;
            let (agent, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new("Caller")?,
                2,
            )?;
            directory.transition(
                actor,
                OperationId::generate()?,
                IdentityId::Agent(agent),
                Transition::Activate,
                "",
                3,
            )?;
            Ok((
                agent.to_string(),
                std::sync::Arc::new(lys_core::Ed25519Identity::load(&config.event_key_file)?),
            ))
        })
        .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let request = lys_core::ca::create_certificate_request(&key, &agent)?;
    let (status, answer) = service
        .post(
            &format!("/agents/{agent}/certificates"),
            Some(&cookie),
            &json!({
                "operation":lys_identity::OperationId::generate()?.to_string(),
                "request":base64::engine::general_purpose::STANDARD.encode(request)
            }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok((service, cookie, agent, key))
}

pub(super) fn change_call(id: u64, path: &str, body: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call", "params":{
            "name":"change", "arguments":{"method":"POST","path":path,"body":body}
        }
    }))?)
}

pub(super) async fn leaves(
    service: &identity_contract::harness::Service,
    cookie: &str,
) -> Result<u64, Box<dyn Error>> {
    let (status, page) = service.get("/receipts/0", Some(cookie)).await?;
    assert_eq!(status, 200, "{page}");
    Ok(page["checkpoint"]["tree_size"]
        .as_u64()
        .ok_or("the receipt checkpoint has no tree size")?)
}

pub(super) fn nobody() -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": lys_identity::OperationId::generate()?.to_string(),
        "display_name": "Nobody"
    }))
}

pub(super) fn signed(
    agent: &str,
    key: &lys_core::Ed25519Identity,
    bytes: &[u8],
) -> Result<String, Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    use std::time::{SystemTime, UNIX_EPOCH};
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = crate::routes::hex(&Sha256::digest(
        lys_identity::OperationId::generate()?
            .to_string()
            .as_bytes(),
    ));
    let payload = crate::agent_signature::payload("POST", "/mcp", bytes, at, &nonce);
    let signature =
        crate::routes::hex(&lys_core::attestation::sign_attestation(&payload, key).to_cose_bytes());
    Ok(format!("{agent} {at} {nonce} {signature}"))
}

pub(super) async fn post_signed(
    base: &str,
    bytes: Vec<u8>,
    signature: &str,
) -> Result<reqwest::Response, Box<dyn Error>> {
    Ok(reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .header(crate::agent_signature::HEADER, signature)
        .body(bytes)
        .send()
        .await?)
}

pub(super) fn tree_call(id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call", "params":{
            "name":"read", "arguments":{"method":"GET","path":"/tree"}
        }
    }))?)
}

pub(super) fn register(router: Router, origin: &str) -> Result<Router, crate::error::ServerError> {
    Ok(router.clone().merge(super::routes(router, origin)?))
}

pub(super) fn request(message: &Value) -> Result<Request<Body>, Box<dyn Error>> {
    Ok(Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .body(Body::from(serde_json::to_vec(message)?))?)
}

pub(super) async fn answer(
    router: Router,
    message: &Value,
) -> Result<(StatusCode, Value), Box<dyn Error>> {
    let request = request(message)?;
    let response = router.oneshot(request).await?;
    let status = response.status();
    let body = to_bytes(response.into_body(), 2 * 1024 * 1024).await?;
    Ok((status, serde_json::from_slice(&body)?))
}
