//! Person-read connections cannot inherit administrator gifts; native terminals carry session authority.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use identity_contract::apps::login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_log_store::{FileLeafStore, LeafStore};
use reqwest::{Client, redirect::Policy};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const VERIFIER: &str = "a-verifier-of-at-least-forty-three-characters-long";

struct Table {
    service: Service,
    cookie: String,
    person: String,
    other: String,
    source: Value,
    client: Client,
}

impl Table {
    async fn start() -> Result<Self> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(
                identity_contract::lys_identity_server::dev_seed::seed_configured(
                    config,
                    [ADMINISTRATOR, "other"],
                )?,
            )
        })
        .await?;
        let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
        let person = seeded.people[0].id.to_string();
        let other = seeded.people[1].id.to_string();
        let root = root_body(&person)?;
        let (status, source) = service.post("/grants/roots", Some(&cookie), &root).await?;
        assert_eq!(status, 200, "administrator fixture root: {source}");
        Ok(Self {
            service,
            cookie,
            person,
            other,
            source,
            client: Client::builder().redirect(Policy::none()).build()?,
        })
    }

    async fn connect(&self, native: bool) -> Result<String> {
        let redirect = if native {
            "http://127.0.0.1:23456/lys/sign-in"
        } else {
            "https://app.example/callback"
        };
        let client_id = if native {
            "lys-cli".to_owned()
        } else {
            let response = self
                .client
                .post(format!("{}/oauth/mcp/register", self.service.base))
                .json(&json!({"client_name":"Authority proof", "redirect_uris":[redirect]}))
                .send()
                .await?;
            assert_eq!(response.status(), 201, "client registration status");
            let registered: Value = response.json().await?;
            registered["client_id"]
                .as_str()
                .ok_or("no client id")?
                .to_owned()
        };
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
        let page = self
            .client
            .get(format!("{}/oauth/mcp/authorize", self.service.base))
            .query(&[
                ("response_type", "code"),
                ("client_id", client_id.as_str()),
                ("redirect_uri", redirect),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
            ])
            .header("cookie", &self.cookie)
            .send()
            .await?;
        assert_eq!(page.status(), 200, "authorization page status");
        let page = page.text().await?;
        let asking = page
            .split("name=\"asking\" value=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .ok_or("no consent form")?;
        let mut form = vec![("asking", asking), ("decision", "approve")];
        if !native {
            form.push(("mode", "person_reads"));
        }
        let response = self
            .client
            .post(format!("{}/oauth/mcp/consent", self.service.base))
            .header("cookie", &self.cookie)
            .form(&form)
            .send()
            .await?;
        assert_eq!(response.status(), 303, "consent status");
        let back = reqwest::Url::parse(
            response
                .headers()
                .get("location")
                .ok_or("no consent redirect")?
                .to_str()?,
        )?;
        let code = back
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned())
            .ok_or("no code")?;
        let response = self
            .client
            .post(format!("{}/oauth/mcp/token", self.service.base))
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", client_id.as_str()),
                ("code", code.as_str()),
                ("code_verifier", VERIFIER),
                ("redirect_uri", redirect),
            ])
            .send()
            .await?;
        assert_eq!(response.status(), 200, "token exchange status");
        let tokens: Value = response.json().await?;
        Ok(tokens["access_token"]
            .as_str()
            .ok_or("no access token")?
            .to_owned())
    }

    fn requests(&self) -> Result<Vec<(&'static str, Value)>> {
        let mut held = json!({"operation":operation()?, "route":"api",
            "source":self.source["grant"], "recipient":self.other, "responsible":self.other,
            "resource":{"kind":"doc", "id":"authority-proof"}, "relation":"beta",
            "pass_on":{"kind":"use_only"}, "window":{"starts_at":0,"ends_at":null}});
        let mut standing = held.clone();
        standing["operation"] = json!(operation()?);
        standing["source"] = json!("administrator");
        held["operation"] = json!(operation()?);
        Ok(vec![
            ("/grants", held),
            ("/grants/roots", root_body(&self.other)?),
            ("/grants", standing),
        ])
    }

    async fn tool(
        &self,
        token: &str,
        tool: &str,
        method: &str,
        path: &str,
        body: &Value,
    ) -> Result<Value> {
        let mut arguments = json!({"method":method, "path":path});
        if !body.is_null() {
            arguments["body"] = body.clone();
        }
        let response = self
            .client
            .post(format!("{}/mcp", self.service.base))
            .header("accept", "application/json, text/event-stream")
            .bearer_auth(token)
            .json(&json!({"jsonrpc":"2.0", "id":1, "method":"tools/call",
                "params":{"name":tool, "arguments":arguments}}))
            .send()
            .await?;
        assert_eq!(response.status(), 200, "MCP transport status");
        let answer: Value = response.json().await?;
        Ok(serde_json::from_str(
            answer["result"]["content"][0]["text"]
                .as_str()
                .ok_or("no tool answer")?,
        )?)
    }

    async fn direct(&self, token: &str, path: &str, body: &Value) -> Result<(u16, Value)> {
        let response = self
            .client
            .post(format!("{}{path}", self.service.base))
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;
        Ok((response.status().as_u16(), response.json().await?))
    }

    fn stored(&self) -> Result<Vec<Vec<Vec<u8>>>> {
        ["grant-log", "log"]
            .into_iter()
            .map(|name| {
                let store = FileLeafStore::open_read_only(&self.service.dir.path().join(name))?;
                (0..store.extent())
                    .map(|index| {
                        store
                            .leaf(index)?
                            .ok_or_else(|| "missing durable leaf".into())
                    })
                    .collect()
            })
            .collect()
    }

    async fn grants(&self) -> Result<Value> {
        let (status, answer) = self.service.get("/grants", Some(&self.cookie)).await?;
        assert_eq!(status, 200, "grant book readback status");
        Ok(answer)
    }
}

fn operation() -> Result<String> {
    Ok(OperationId::generate()?.to_string())
}

fn root_body(holder: &str) -> Result<Value> {
    Ok(
        json!({"operation":operation()?, "route":"api", "holder":holder,
        "resource":{"kind":"doc", "id":"authority-proof"}, "relation":"alpha",
        "pass_on":{"kind":"to", "actions":["read"], "recipients":["person"]},
        "window":{"starts_at":0,"ends_at":null}}),
    )
}

#[tokio::test]
async fn person_reads_connection_cannot_give_or_delegate_through_tools_or_direct_bearer() -> Result
{
    let table = Table::start().await?;
    let token = table.connect(false).await?;
    assert!(
        !token.starts_with("lys-cli."),
        "non-native connection token kind"
    );
    let me = table
        .tool(&token, "read", "GET", "/me", &Value::Null)
        .await?;
    assert_eq!(me["status"], 200, "consented connection reads its person");
    assert_eq!(me["body"]["person"]["id"], table.person);
    let before = table.stored()?;
    let grants = table.grants().await?;
    let mut refused_statuses = Vec::new();
    for (path, body) in table.requests()? {
        for tool in ["change", "read"] {
            let answer = table.tool(&token, tool, "POST", path, &body).await?;
            assert_eq!(answer["status"], 403, "{tool} {path}: {answer}");
            assert_eq!(answer["body"]["refusal"], "PersonPresenceRequired");
            assert!(
                answer["body"]["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("needs you in Lys:")),
                "browser refusal must name the required presence"
            );
            assert!(
                table.stored()? == before,
                "MCP refusal changed grant book or directory"
            );
            assert_eq!(table.grants().await?, grants, "MCP refusal changed grants");
        }
        let (status, answer) = table.direct(&token, path, &body).await?;
        if ![401, 403].contains(&status) {
            refused_statuses.push(format!("direct {path}: {status}: {answer}"));
        }
        assert!(
            answer["refusal"].as_str().is_some(),
            "direct refusal must be named"
        );
        assert!(
            table.stored()? == before,
            "direct refusal changed grant book or directory"
        );
        assert_eq!(
            table.grants().await?,
            grants,
            "direct refusal changed grants"
        );
    }
    for (path, body) in table.requests()? {
        let (status, answer) = table.service.post(path, Some(&table.cookie), &body).await?;
        assert_eq!(status, 200, "browser authority control {path}: {answer}");
        let grants = table.grants().await?;
        assert!(
            grants["grants"]
                .as_array()
                .ok_or("no grants")?
                .iter()
                .any(|grant| grant["id"] == answer["grant"]),
            "browser grant readback"
        );
    }
    assert!(
        refused_statuses.is_empty(),
        "{}",
        refused_statuses.join("; ")
    );
    Ok(())
}

#[tokio::test]
async fn native_lys_cli_carries_approving_administrator_session_authority_but_not_standing_gifts()
-> Result {
    let table = Table::start().await?;
    let token = table.connect(true).await?;
    assert!(
        token.starts_with("lys-cli."),
        "native connection token kind"
    );
    let response = table
        .client
        .get(format!("{}/identities", table.service.base))
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(
        response.status(),
        200,
        "native token carries administrator authority"
    );
    let response = table
        .client
        .get(format!("{}/me", table.service.base))
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let me: Value = response.json().await?;
    assert_eq!(me["person"]["id"], table.person);
    let before = table.stored()?;
    let (status, registered) = table
        .direct(
            &token,
            "/people",
            &json!({
                "operation":operation()?, "display_name":"Native administrator control"
            }),
        )
        .await?;
    assert_eq!(
        status, 200,
        "native session may perform an administrator act: {registered}"
    );
    let person = registered["person"]
        .as_str()
        .ok_or("no registered person")?;
    let (status, _) = table
        .service
        .get(&format!("/identities/{person}"), Some(&table.cookie))
        .await?;
    assert_eq!(status, 200, "native administrator act directory readback");
    let after = table.stored()?;
    assert!(
        after[0] == before[0],
        "person registration must not create a grant"
    );
    assert!(
        after[1] != before[1],
        "native administrator act must be durable"
    );
    let before = table.stored()?;
    let requests = table.requests()?;
    let (path, body) = &requests[2];
    let (status, answer) = table.direct(&token, path, body).await?;
    assert_eq!(
        status, 403,
        "native standing gift still needs browser presence"
    );
    assert_eq!(answer["refusal"], "RootAuthorityRefused");
    assert!(
        table.stored()? == before,
        "refused standing gift wrote records"
    );
    Ok(())
}
