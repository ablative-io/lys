#![cfg(test)]
//! One service whose administrator, Ada, gives Bea roots, and Bea's three
//! active agents, each certified by Bea so it signs its own requests: the
//! creator, the reviewer and a stranger that holds nothing until a test
//! gives it something (AGENTS-015).

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::grants::shipped_model;
use lys_identity::{Actor, AuthMethod, IdentityId, OperationId, Profile, Provenance, Transition};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::routes::open_directory;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::session::now;
use serde_json::{Value, json};

/// A test's outcome.
pub type TestResult = Result<(), Box<dyn Error>>;
/// A status and the JSON answered with it.
pub type Answer = (u16, Value);

/// Bea's subject at the fake issuer.
pub const BEA: &str = "bea-agent-starts-agent";

/// A secrets broker listing one live handle and nothing else, so a start's
/// credentials check has its record (SECRETS-002).
async fn broker(request: Request) -> Response {
    if request.uri().path() != "/_lys/handles" {
        return axum::Json(json!({})).into_response();
    }
    axum::Json(json!({ "holder": "any", "handles": [
        { "id": "h-live", "secret": "git-host token", "max_uses": 10, "used": 1,
          "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0,
          "parent": null },
    ]}))
    .into_response()
}

/// A fresh operation id.
///
/// # Errors
/// Refuses when no random bytes can be read.
pub fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0f)],
            ]
        })
        .map(char::from)
        .collect()
}

/// An agent holding a certified key, which signs its own requests.
pub struct Certified {
    /// The agent's id.
    pub id: String,
    key: Arc<Ed25519Identity>,
}

impl Certified {
    /// The signature header for `method path` over `body`.
    ///
    /// # Errors
    /// Refuses an unreadable clock or no random bytes.
    pub fn signature(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<String, Box<dyn Error>> {
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = hex(OperationId::generate()?.as_bytes());
        let cose =
            sign_attestation(&payload(method, path, body, at, &nonce), &self.key).to_cose_bytes();
        Ok(format!("{} {at} {nonce} {}", self.id, hex(&cose)))
    }
}

/// The service, its two people's sessions and Bea's three certified agents.
pub struct World {
    /// The service.
    pub service: Service,
    /// Ada's session: the administrator and root authority.
    pub ada: String,
    /// Bea's session.
    pub bea: String,
    /// Bea's person id.
    pub bea_person: String,
    /// The agent that registers, sets up and starts agents.
    pub creator: Certified,
    /// A second agent, which reviews profiles.
    pub reviewer: Certified,
    /// An agent holding nothing until a test gives it something.
    pub stranger: Certified,
    client: reqwest::Client,
    _keys: tempfile::TempDir,
}

impl World {
    /// Start the service on the shipped model, with Bea and her three
    /// active agents, each certified.
    ///
    /// # Errors
    /// Propagates a fixture step's failure.
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let model = shipped_model();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        tokio::spawn(async move { axum::serve(listener, Router::new().fallback(broker)).await });
        let keys = tempfile::tempdir()?;
        let key_file = keys.path().join("secrets-service.key");
        Ed25519Identity::load_or_generate(&key_file)?;
        let settings = SecretsSettings {
            broker: format!("http://{address}"),
            service: "identity".to_owned(),
            service_key_file: key_file,
        };
        let (service, (bea_person, agents)) =
            Service::start_asking(&model, None, Some(settings), |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
                let mut directory = open_directory(config)?;
                let actor = Actor::new(
                    config.administrator_binding()?,
                    Provenance::new(AuthMethod::Oidc, now()),
                );
                let bea = seeded.people[1].id;
                let mut agents = vec![seeded.people[1].agents[0].id];
                for name in ["Reviewer two", "Stranger"] {
                    let (agent, _) = directory.register_agent(
                        actor.clone(),
                        OperationId::generate()?,
                        bea,
                        Profile::new(name)?,
                        now(),
                    )?;
                    directory.transition(
                        actor.clone(),
                        OperationId::generate()?,
                        IdentityId::Agent(agent),
                        Transition::Activate,
                        "",
                        now(),
                    )?;
                    agents.push(agent);
                }
                Ok((bea.to_string(), agents))
            })
            .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let mut certified = Vec::new();
        for agent in agents {
            certified.push(enroll(&service, &bea, &agent.to_string()).await?);
        }
        let stranger = certified.pop().ok_or("no stranger")?;
        let reviewer = certified.pop().ok_or("no reviewer")?;
        let creator = certified.pop().ok_or("no creator")?;
        Ok(Self {
            service,
            ada,
            bea,
            bea_person,
            creator,
            reviewer,
            stranger,
            client: reqwest::Client::new(),
            _keys: keys,
        })
    }

    /// `agent`'s signed POST of `body` to `path`.
    ///
    /// # Errors
    /// Propagates a transport or decoding failure.
    pub async fn signed(
        &self,
        agent: &Certified,
        path: &str,
        body: &Value,
    ) -> Result<Answer, Box<dyn Error>> {
        let bytes = serde_json::to_vec(body)?;
        let signature = agent.signature("POST", path, &bytes)?;
        self.service
            .post_signed(path, (HEADER, &signature), bytes)
            .await
    }

    /// `agent`'s signed MCP message `body`, answered as JSON-RPC.
    ///
    /// # Errors
    /// Propagates a transport or decoding failure.
    pub async fn mcp(&self, agent: &Certified, body: &Value) -> Result<Value, Box<dyn Error>> {
        let bytes = serde_json::to_vec(body)?;
        let signature = agent.signature("POST", "/mcp", &bytes)?;
        let response = self
            .client
            .post(format!("{}/mcp", self.service.base))
            .header(HEADER, signature)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .body(bytes)
            .send()
            .await?;
        Ok(response.json().await?)
    }

    /// A root Ada issues to Bea: `action` alone on `resource`, which Bea
    /// may pass on to agents.
    ///
    /// # Errors
    /// Refuses anything but a recorded root.
    pub async fn root(&self, resource: &Value, action: &str) -> Result<String, Box<dyn Error>> {
        let body = json!({
            "operation": operation()?, "route": "api", "holder": self.bea_person,
            "resource": resource, "relation": format!("only.{action}"),
            "pass_on": {"kind": "to", "actions": [action], "recipients": ["agent"]},
            "window": {"starts_at": 0, "ends_at": null},
        });
        let (status, issued) = self
            .service
            .post("/grants/roots", Some(&self.ada), &body)
            .await?;
        assert_eq!(status, 200, "{issued}");
        Ok(issued["grant"].as_str().ok_or("no root")?.to_owned())
    }

    /// The body passing `action` on `resource` from `source` to `recipient`.
    ///
    /// # Errors
    /// Refuses when no random bytes can be read.
    pub fn passing(
        &self,
        source: &str,
        recipient: &str,
        resource: &Value,
        action: &str,
        pass_on: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": operation()?, "route": "api", "source": source,
            "recipient": recipient, "responsible": self.bea_person,
            "resource": resource, "relation": format!("only.{action}"),
            "pass_on": pass_on, "window": {"starts_at": 0, "ends_at": null},
        }))
    }

    /// Bea's grant to `agent` of `action` on `resource`, passable on to
    /// agents when `onward`, through a root Ada gives her for it.
    ///
    /// # Errors
    /// Refuses anything but a recorded grant.
    pub async fn give(
        &self,
        agent: &str,
        resource: &Value,
        action: &str,
        onward: bool,
    ) -> Result<String, Box<dyn Error>> {
        let root = self.root(resource, action).await?;
        let pass_on = if onward {
            json!({"kind": "to", "actions": [action], "recipients": ["agent"]})
        } else {
            json!({"kind": "use_only"})
        };
        let body = self.passing(&root, agent, resource, action, &pass_on)?;
        let (status, given) = self.service.post("/grants", Some(&self.bea), &body).await?;
        assert_eq!(status, 200, "{given}");
        Ok(given["grant"].as_str().ok_or("no grant")?.to_owned())
    }

    /// Bea's person as a resource.
    #[must_use]
    pub fn bea_resource(&self) -> Value {
        json!({"kind": "person", "id": self.bea_person})
    }

    /// The grants Bea may see.
    ///
    /// # Errors
    /// Refuses anything but a list.
    pub async fn beas_grants(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let (status, listed) = self.service.get("/grants", Some(&self.bea)).await?;
        assert_eq!(status, 200, "{listed}");
        Ok(listed["grants"].as_array().ok_or("no grants")?.clone())
    }

    /// A computer Bea names as her own, running nothing yet.
    ///
    /// # Errors
    /// Refuses anything but a named computer.
    pub async fn beas_computer(&self) -> Result<String, Box<dyn Error>> {
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": "Bea's laptop", "kind": "laptop",
            "runtime": "local launcher", "slots": 2, "may_run": [], "may_reach": [],
        });
        let (status, named) = self
            .service
            .post("/network/machines", Some(&self.bea), &body)
            .await?;
        assert_eq!(status, 200, "{named}");
        Ok(machine)
    }
}

/// `agent`, certified by `person`'s session over a key kept beside the service.
async fn enroll(service: &Service, person: &str, agent: &str) -> Result<Certified, Box<dyn Error>> {
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &service.dir.path().join(format!("{agent}.key")),
    )?);
    let body = json!({
        "operation": operation()?,
        "request": STANDARD.encode(create_certificate_request(&key, agent)?),
    });
    let (status, issued) = service
        .post(
            &format!("/agents/{agent}/certificates"),
            Some(person),
            &body,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    Ok(Certified {
        id: agent.to_owned(),
        key,
    })
}

/// Whether `answer` is the refusal `name`, or a start refused by it.
#[must_use]
pub fn refused_as(answer: &Answer, name: &str) -> bool {
    answer.1["refusal"] == name
        || answer.1["error"] == name
        || answer.1["refused"]
            .as_array()
            .is_some_and(|refused| refused.iter().any(|one| one["refusal"] == name))
}
