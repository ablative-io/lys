//! Person consent reads own records and never carries a person's decision.

#[path = "../src/mcp_endpoint/person_test_support.rs"]
mod support;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::draft_event::{Created, DraftEvent, Target};
use lys_identity::{Actor, AuthMethod, IdentityId, OperationId, Provenance};
use serde_json::{Value, json};
use std::error::Error;
use std::sync::Arc;
use support::{client, connected, connected_mode};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

struct Table {
    service: Service,
    owner: String,
    other: String,
    person: String,
    other_person: String,
    draft: String,
    other_draft: String,
}

impl Table {
    async fn start() -> Result<Self> {
        let (service, (seeded, drafts)) = Service::start_with(|config| {
            let seeded = identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "other"],
            )?;
            let mut directory =
                identity_contract::lys_identity_server::routes::open_directory(config)?;
            let mut drafts = Vec::new();
            for person in &seeded.people {
                let operation = OperationId::generate()?;
                let actor = Actor::new(
                    lys_identity::LoginBinding::new(&config.issuer, &person.subject)?,
                    Provenance::new(AuthMethod::Oidc, lys_identity_server::session::now()),
                );
                directory.record_draft(DraftEvent::Created(Arc::new(Created {
                    operation,
                    actor,
                    recorded_at: lys_identity_server::session::now(),
                    target: Target {
                        kind: "person".to_owned(),
                        id: person.id.to_string(),
                        action: "write".to_owned(),
                    },
                    method: "POST".to_owned(),
                    path: format!("/identities/{}/profile", person.id),
                    body: br#"{"display_name":"Prepared"}"#.to_vec(),
                    note: person.display_name.clone(),
                    reviewer: IdentityId::Person(person.id),
                    corrects: None,
                    evidence: None,
                    request_signature: None,
                })))?;
                drafts.push(operation.to_string());
            }
            Ok((seeded, drafts))
        })
        .await?;
        let owner = service
            .sign_in(identity_contract::apps::login(ADMINISTRATOR))
            .await?;
        let other = service
            .sign_in(identity_contract::apps::login("other"))
            .await?;
        Ok(Self {
            service,
            owner,
            other,
            person: seeded.people[0].id.to_string(),
            other_person: seeded.people[1].id.to_string(),
            draft: drafts[0].clone(),
            other_draft: drafts[1].clone(),
        })
    }

    async fn connect(&self, cookie: &str) -> Result<(String, Value)> {
        connected_mode(&self.service, cookie, Some("person_reads")).await
    }

    async fn call(&self, tokens: &Value, name: &str, arguments: Value) -> Result<Value> {
        let client = client()?;
        let response = client
            .post(format!("{}/mcp", self.service.base))
            .header("accept", "application/json, text/event-stream")
            .bearer_auth(tokens["access_token"].as_str().ok_or("no access token")?)
            .json(&json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
                "params":{"name":name, "arguments":arguments}}))
            .send()
            .await?;
        assert_eq!(response.status(), 200, "MCP transport refused");
        let answer: Value = response.json().await?;
        assert!(answer["result"].is_object(), "{answer}");
        support::body(&answer["result"])
    }
}

#[tokio::test]
async fn person_connection_reads_me_agents_and_own_raised_and_waiting_drafts() -> Result {
    let table = Table::start().await?;
    let (connection, tokens) = table.connect(&table.owner).await?;
    let (_, expected) = table.service.get("/me", Some(&table.owner)).await?;
    let me = table.call(&tokens, "read", json!({"path":"/me"})).await?;
    assert_eq!(me["status"], 200, "person /me read was refused: {me}");
    assert_eq!(me["body"], expected);
    assert_eq!(me["receipt"]["read"]["client"], connection);
    assert!(
        me["receipt"]["read"]["connection"]
            .as_str()
            .is_some_and(|id| id.parse::<lys_identity::AgentId>().is_ok())
    );
    assert_eq!(me["receipt"]["read"]["person"], table.person);
    assert_eq!(me["receipt"]["read"]["route"], "/me");
    let (_, dashboard) = table
        .service
        .get("/dashboard?limit=50", Some(&table.owner))
        .await?;
    let agents = table
        .call(&tokens, "read", json!({"path":"/dashboard?limit=50"}))
        .await?;
    assert_eq!(agents["status"], 200, "{agents}");
    assert_eq!(agents["body"]["agents"], dashboard["agents"]);
    let drafts = table
        .call(&tokens, "drafts", json!({"path":"/drafts?state=waiting"}))
        .await?;
    assert_eq!(drafts["status"], 200, "{drafts}");
    assert_eq!(drafts["body"]["drafts"][0]["id"], table.draft);
    assert_eq!(
        drafts["body"]["drafts"]
            .as_array()
            .ok_or("no drafts")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn person_connection_agents_read_contains_only_own_agents_and_paging() -> Result {
    let table = Table::start().await?;
    let (_, other_tokens) = table.connect(&table.other).await?;
    let other = table
        .call(&other_tokens, "read", json!({"path":"/me"}))
        .await?;
    let other_connection = other["receipt"]["read"]["connection"]
        .as_str()
        .ok_or("no other connection")?;
    let (_, tokens) = table.connect(&table.owner).await?;
    for (path, expected_fields) in [
        ("/dashboard", vec!["agents"]),
        ("/dashboard?limit=1", vec!["agents", "next", "total"]),
    ] {
        let answer = table.call(&tokens, "read", json!({"path":path})).await?;
        assert_eq!(answer["status"], 200, "own agents were refused");
        let fields = answer["body"]
            .as_object()
            .ok_or("own-agent view is not an object")?
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            fields, expected_fields,
            "unconsented dashboard fields escaped"
        );
        assert!(!answer["body"].to_string().contains(other_connection));
        let (_, expected) = table.service.get(path, Some(&table.owner)).await?;
        assert_eq!(answer["body"]["agents"], expected["agents"]);
    }
    Ok(())
}

#[tokio::test]
async fn person_connection_refuses_each_draft_decision_with_its_browser_url() -> Result {
    let table = Table::start().await?;
    let (_, tokens) = table.connect(&table.owner).await?;
    let before = table.service.log_size().await?;
    for tool in ["read", "change", "drafts"] {
        for decision in ["approve", "refuse", "correct"] {
            let answer = table
                .call(
                    &tokens,
                    tool,
                    json!({
                        "method":"POST", "path":format!("/drafts/{}/{decision}", table.draft),
                        "body":{"operation":OperationId::generate()?.to_string()}
                    }),
                )
                .await?;
            assert_eq!(answer["status"], 403, "{answer}");
            assert_eq!(
                answer["body"]["refusal"], "PersonPresenceRequired",
                "{answer}"
            );
            assert_eq!(
                answer["body"]["reason"],
                format!(
                    "deciding a draft needs you in Lys: {}/#/access/drafts?draft={}",
                    table.service.base, table.draft
                )
            );
        }
    }
    assert_eq!(
        table.service.log_size().await?,
        before,
        "a decision reached the directory"
    );
    let (_, drafts) = table.service.get("/drafts", Some(&table.owner)).await?;
    assert_eq!(drafts["drafts"][0]["state"], "waiting");
    Ok(())
}

#[tokio::test]
async fn person_connection_cannot_read_another_persons_drafts_even_as_administrator() -> Result {
    let table = Table::start().await?;
    for (cookie, own, foreign, person) in [
        (
            &table.owner,
            &table.draft,
            &table.other_draft,
            &table.person,
        ),
        (
            &table.other,
            &table.other_draft,
            &table.draft,
            &table.other_person,
        ),
    ] {
        let (_, tokens) = table.connect(cookie).await?;
        for path in ["/drafts", "/drafts?state=all&limit=1"] {
            let answer = table.call(&tokens, "read", json!({"path":path})).await?;
            assert_eq!(answer["status"], 200, "{answer}");
            let rows = answer["body"]["drafts"].as_array().ok_or("no drafts")?;
            assert_eq!(rows.len(), 1, "{answer}");
            assert_eq!(rows[0]["id"], *own);
            assert_eq!(rows[0]["responsible"]["id"], *person);
            assert!(!answer.to_string().contains(foreign.as_str()));
        }
        for path in [
            format!("/drafts/{foreign}"),
            "/product-drafts".to_owned(),
            format!("/directory/people/{person}/sessions"),
            "/api/me".to_owned(),
        ] {
            let answer = table.call(&tokens, "read", json!({"path":path})).await?;
            assert_eq!(answer["status"], 403, "{answer}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn agent_mode_keeps_its_own_identity_and_cannot_read_person_views() -> Result {
    let mut table = Table::start().await?;
    let (_, tokens) = connected(&table.service, &table.owner).await?;
    let path = table.service.dir.path().join("connected-apps.json");
    let mut legacy: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    legacy
        .as_object_mut()
        .ok_or("no store object")?
        .remove("format");
    for issued in legacy["tokens"]
        .as_object_mut()
        .ok_or("no token digests")?
        .values_mut()
    {
        issued
            .as_object_mut()
            .ok_or("no issued-token object")?
            .remove("person_reads");
    }
    std::fs::write(path, serde_json::to_vec(&legacy)?)?;
    table.service.restart().await?;
    let tree = table.call(&tokens, "read", json!({"path":"/tree"})).await?;
    assert_eq!(tree["status"], 200, "{tree}");
    assert_eq!(tree["body"]["root"]["name"], "Notes");
    assert_ne!(tree["body"]["root"]["id"], table.person);
    for path in ["/me", "/dashboard", "/drafts"] {
        let answer = table.call(&tokens, "read", json!({"path":path})).await?;
        assert_ne!(answer["status"], 200, "agent gained person views: {answer}");
    }
    Ok(())
}

#[tokio::test]
async fn person_connection_keeps_read_scope_through_refresh_and_restart() -> Result {
    let mut table = Table::start().await?;
    let (connection, tokens) = table.connect(&table.owner).await?;
    table.service.restart().await?;
    let refreshed: Value = client()?
        .post(format!("{}/oauth/mcp/token", table.service.base))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", connection.as_str()),
            (
                "refresh_token",
                tokens["refresh_token"].as_str().ok_or("no refresh")?,
            ),
        ])
        .send()
        .await?
        .json()
        .await?;
    let answer = table
        .call(&refreshed, "read", json!({"path":"/me"}))
        .await?;
    assert_eq!(answer["status"], 200, "{answer}");
    assert_eq!(answer["body"]["person"]["id"], table.person);
    let change = table
        .call(
            &refreshed,
            "change",
            json!({"method":"POST", "path":"/grants/roots", "body":{}}),
        )
        .await?;
    assert_eq!(change["status"], 403, "{change}");
    assert_eq!(change["body"]["refusal"], "PersonPresenceRequired");
    let direct = client()?
        .get(format!("{}/me", table.service.base))
        .bearer_auth(refreshed["access_token"].as_str().ok_or("no access")?)
        .send()
        .await?;
    assert_ne!(
        direct.status(),
        200,
        "person MCP credential escaped the MCP scope"
    );
    Ok(())
}

#[tokio::test]
async fn person_connection_loses_its_reads_when_its_connection_is_suspended() -> Result {
    let table = Table::start().await?;
    let (_, tokens) = table.connect(&table.owner).await?;
    let answer = table.call(&tokens, "read", json!({"path":"/me"})).await?;
    assert_eq!(answer["status"], 200, "{answer}");
    let connection = answer["receipt"]["read"]["connection"]
        .as_str()
        .ok_or("no connection id")?;
    let (status, changed) = table.service.post(
        &format!("/identities/{connection}/transitions"), Some(&table.owner),
        &json!({"operation":OperationId::generate()?.to_string(), "transition":"suspend", "reason":"End this connection"})
    ).await?;
    assert_eq!(status, 200, "{changed}");
    let refused = table.call(&tokens, "read", json!({"path":"/me"})).await?;
    assert_ne!(
        refused["status"], 200,
        "suspended connection kept its person reads: {refused}"
    );
    Ok(())
}

#[tokio::test]
async fn person_connection_discovery_names_only_consented_reads_and_keeps_a_signed_durable_receipt()
-> Result {
    use lys_core::attestation::verify_attestation_bytes_by_signer;
    use lys_core::merkle::{InclusionProof, RootHash, verify_inclusion_raw};
    use lys_log_store::LeafStore;
    let mut table = Table::start().await?;
    let (_, tokens) = table.connect(&table.owner).await?;
    let discovered = table.call(&tokens, "what-can-I-do", json!({})).await?;
    assert_eq!(discovered["status"], 200, "{discovered}");
    assert_eq!(discovered["body"]["connection"]["mode"], "person_reads");
    assert_eq!(discovered["body"]["connection"]["person"], table.person);
    assert_eq!(discovered["body"]["resources"], json!([]));
    assert_eq!(
        discovered["body"]["routes"],
        json!([
            {"method":"GET", "path":"/me"}, {"method":"GET", "path":"/dashboard"}, {"method":"GET", "path":"/drafts"}
        ])
    );
    let receipt = &discovered["receipt"];
    let message = support::unhex(&receipt["message"])?;
    let payload = support::unhex(&receipt["payload"])?;
    let signed: Value = serde_json::from_slice(&message)?;
    assert_eq!(serde_json::from_slice::<Value>(&payload)?, receipt["read"]);
    assert_eq!(signed["read"], receipt["read"]);
    let signature: Vec<u8> = serde_json::from_value(signed["signature"].clone())?;
    let (_, service_key) = table.service.get("/service-key", None).await?;
    let key: [u8; 32] = support::unhex(&service_key["ed25519"])?
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("service key has {} bytes, expected 32", bytes.len()))?;
    verify_attestation_bytes_by_signer(&signature, &payload, &key)?;
    let root: [u8; 32] = support::unhex(&receipt["checkpoint"]["root"])?
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("receipt root has {} bytes, expected 32", bytes.len()))?;
    let size = receipt["checkpoint"]["tree_size"]
        .as_u64()
        .ok_or("no tree size")?;
    let index = receipt["index"].as_u64().ok_or("no read index")?;
    verify_inclusion_raw(
        &RootHash::from_parts(root, size),
        &message,
        index,
        &InclusionProof::try_from_bytes(support::unhex(&receipt["inclusion_proof"])?)?,
    )?;
    let store = lys_log_store::FileLeafStore::open_read_only(
        &table.service.dir.path().join("mcp-person-reads"),
    )?;
    assert_eq!(
        store.leaf(index)?.ok_or("read receipt was not durable")?,
        message
    );
    drop(store);
    table.service.restart().await?;
    let reread = table.call(&tokens, "read", json!({"path":"/me"})).await?;
    assert_eq!(reread["status"], 200, "{reread}");
    assert_eq!(reread["receipt"]["index"], index + 1);
    Ok(())
}

#[tokio::test]
async fn person_connection_does_not_serve_a_read_when_its_receipt_cannot_be_kept() -> Result {
    let table = Table::start().await?;
    let (_, tokens) = table.connect(&table.owner).await?;
    std::fs::write(
        table.service.dir.path().join("mcp-person-reads"),
        "not a receipt store",
    )?;
    let answer = table.call(&tokens, "read", json!({"path":"/me"})).await?;
    assert_eq!(answer["status"], 503, "{answer}");
    assert_eq!(answer["body"]["refusal"], "McpReadReceiptUnavailable");
    assert!(
        answer["body"].get("person").is_none(),
        "private data was served without its audit: {answer}"
    );
    Ok(())
}
