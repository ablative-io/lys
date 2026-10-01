#![cfg(test)]

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent_signature::{HEADER, payload};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 15)],
            ]
        })
        .map(char::from)
        .collect()
}

struct Table {
    service: Service,
    client: reqwest::Client,
    cookie: String,
    person: String,
    target: String,
    issuer: Issuer,
    agent: String,
    key: Arc<Ed25519Identity>,
}

impl Table {
    async fn grant(&self, person: &str) -> TestResult<String> {
        let resource = json!({"kind": "account", "id": person});
        let (status, root) = self
            .service
            .post(
                "/grants/roots",
                Some(&self.cookie),
                &json!({
                    "operation": operation()?, "route": "api", "holder": self.person,
                    "resource": resource, "relation": "writer",
                    "pass_on": {"kind": "to", "actions": ["write"], "recipients": ["agent"]},
                    "window": {"starts_at": 0, "ends_at": null},
                }),
            )
            .await?;
        assert_eq!(status, 200, "{root}");
        let (status, answer) = self
            .service
            .post(
                "/grants",
                Some(&self.cookie),
                &json!({
                    "operation": operation()?, "route": "api", "source": root["grant"],
                    "recipient": self.agent, "responsible": self.person,
                    "resource": resource, "relation": "writer", "pass_on": {"kind": "use_only"},
                    "window": {"starts_at": 0, "ends_at": null},
                }),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer["grant"]
            .as_str()
            .ok_or("grant id is missing")?
            .to_owned())
    }

    async fn team(&self, members: &[&str]) -> TestResult<String> {
        let id = operation()?;
        let (status, answer) = self
            .service
            .post(
                "/teams",
                Some(&self.cookie),
                &json!({"operation": id, "name": "Team"}),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        for member in members {
            let (status, answer) = self
                .service
                .post(
                    &format!("/teams/{id}/members"),
                    Some(&self.cookie),
                    &json!({"operation": operation()?, "member": member}),
                )
                .await?;
            assert_eq!(status, 200, "{answer}");
        }
        Ok(id)
    }

    async fn fresh() -> TestResult<Self> {
        let issuer = Issuer::open().await?;
        let settings = serde_json::from_value(
            json!({"api": issuer.base, "api_key_file": issuer.keys.path().join("api.key")}),
        )?;
        let (service, (person, target, agent, key)) = Service::start_adjusted(
            r#"{"version":1,"relations":{"writer":["write"]}}"#,
            None,
            None,
            Some(settings),
            |config| {
                config.requests_dir = None;
                config.network_file = None;
                config.roles_file = None;
                config.provisioning_file = None;
                config.runtime_dir = None;
                config.service_accounts_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| {
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                crate::configuration_store::ConfigurationStore::open(
                    &config.log_dir.with_file_name("organisation"),
                    Arc::clone(&key),
                )?;
                crate::runner_acts::ActStore::open(
                    &config.log_dir.with_file_name("runner-acts"),
                    Arc::clone(&key),
                )?;
                lys_identity::start::LaunchRecords::open(
                    &config.log_dir.with_file_name("launch-records"),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?;
                crate::apps_store::AppStore::open(&config.apps_dir(), Arc::clone(&key))?;
                crate::certificates_store::CertificateStore::open(
                    config
                        .certificates_dir
                        .as_deref()
                        .ok_or("certificate fixture is disabled")?,
                    Arc::clone(&key),
                )?;
                crate::teams_store::TeamStore::open(
                    config
                        .teams_dir
                        .as_deref()
                        .ok_or("team fixture is disabled")?,
                    Arc::clone(&key),
                )?;
                FileLeafStore::create(&config.log_dir, &config.log_origin)?;
                let path = config.log_dir.clone();
                let mut directory = Directory::open(
                    Box::new(move || FileLeafStore::open(&path)),
                    Ed25519Identity::load(&config.event_key_file)?,
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
                let target_actor = Actor::new(
                    LoginBinding::new(&config.issuer, "target")?,
                    Provenance::new(AuthMethod::Oidc, 1),
                );
                let (target, _) = directory.setup_person(
                    target_actor,
                    OperationId::generate()?,
                    Profile::new("Target")?,
                    1,
                )?;
                let (agent, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Agent")?,
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
                    person.to_string(),
                    target.to_string(),
                    agent.to_string(),
                    key,
                ))
            },
        )
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        let (status, answer) = service.post(
            &format!("/agents/{agent}/certificates"), Some(&cookie),
            &json!({"operation": operation()?, "request": STANDARD.encode(create_certificate_request(&key, &agent)?)}),
        ).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(Self {
            service,
            client: reqwest::Client::new(),
            cookie,
            person,
            target,
            issuer,
            agent,
            key,
        })
    }

    async fn signed(&self, path: &str, body: &Value, cookie: bool) -> TestResult<(u16, Value)> {
        let bytes = serde_json::to_vec(body)?;
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = hex(&Sha256::digest(operation()?.as_bytes()));
        let signed = payload("POST", path, &bytes, at, &nonce);
        let signature = hex(&sign_attestation(&signed, &self.key).to_cose_bytes());
        let mut request = self
            .client
            .post(format!("{}{path}", self.service.base))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(HEADER, format!("{} {at} {nonce} {signature}", self.agent))
            .body(bytes);
        if cookie {
            request = request.header(reqwest::header::COOKIE, &self.cookie);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await?))
    }

    async fn enabled(&self, person: &str, enabled: bool, cookie: bool) -> TestResult<(u16, Value)> {
        self.signed(
            &format!("/directory/people/{person}/account/enabled"),
            &json!({"enabled": enabled}),
            cookie,
        )
        .await
    }
}

struct Issuer {
    base: String,
    keys: tempfile::TempDir,
    users: Arc<tokio::sync::Mutex<std::collections::HashMap<String, Value>>>,
    serving: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}
impl Drop for Issuer {
    fn drop(&mut self) {
        self.serving.abort();
    }
}
impl Issuer {
    async fn open() -> TestResult<Self> {
        let keys = tempfile::tempdir()?;
        std::fs::write(keys.path().join("api.key"), "test$synthetic-key")?;
        let users = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::from(
            [ADMINISTRATOR, "target"].map(|id| {
                (
                    id.to_owned(),
                    json!({
                        "id": id, "email": "person@example.test", "language": "en", "roles": [],
                        "groups": ["group"], "enabled": true, "email_verified": false,
                        "user_values": {"saved": "kept"}
                    }),
                )
            }),
        )));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let router = axum::Router::new()
            .route(
                "/users/{id}",
                axum::routing::get(issuer_read).put(issuer_write),
            )
            .with_state(Arc::clone(&users));
        let serving = tokio::spawn(async move { axum::serve(listener, router).await });
        Ok(Self {
            base,
            keys,
            users,
            serving,
        })
    }
}
type Users = Arc<tokio::sync::Mutex<std::collections::HashMap<String, Value>>>;
async fn issuer_read(
    axum::extract::State(users): axum::extract::State<Users>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> axum::Json<Value> {
    axum::Json(users.lock().await.get(&id).cloned().unwrap_or(Value::Null))
}
async fn issuer_write(
    axum::extract::State(users): axum::extract::State<Users>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::Json(update): axum::Json<Value>,
) -> axum::Json<Value> {
    users.lock().await.insert(id, update);
    axum::Json(Value::Null)
}

#[tokio::test]
async fn a_signed_account_write_refuses_an_administrator_cookie() -> TestResult {
    let table = Table::fresh().await?;
    let before = table.issuer.users.lock().await.clone();
    let (status, answer) = table.enabled(&table.target, false, true).await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "AgentSignatureRefused", "{answer}");
    assert_eq!(*table.issuer.users.lock().await, before);
    Ok(())
}

#[tokio::test]
async fn account_write_needs_a_live_grant_and_current_holding() -> TestResult {
    let table = Table::fresh().await?;
    let (status, answer) = table.enabled(&table.target, false, false).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotHeld", "{answer}");
    let grant = table.grant(&table.target).await?;
    let (status, answer) = table.enabled(&table.target, false, false).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "HoldingNotHeld", "{answer}");
    table.team(&[&table.agent, &table.target]).await?;
    let (status, answer) = table
        .service
        .post(
            &format!("/grants/{grant}/revoke"),
            Some(&table.cookie),
            &json!({"operation": operation()?, "route": "api", "reason": "Revoked"}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = table.enabled(&table.target, false, false).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "Revoked", "{answer}");
    assert_eq!(table.issuer.users.lock().await["target"]["enabled"], true);
    Ok(())
}

#[tokio::test]
async fn a_held_account_grant_disables_sessions_and_reenables_without_password_or_mail()
-> TestResult {
    let table = Table::fresh().await?;
    let target_cookie = table
        .service
        .sign_in(Login {
            subject: "target".to_owned(),
            email: "person@example.test".to_owned(),
        })
        .await?;
    table.grant(&table.target).await?;
    table.team(&[&table.agent, &table.target]).await?;
    let (status, answer) = table.enabled(&table.target, false, false).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["enabled"], false);
    let (status, answer) = table.service.get("/me", Some(&target_cookie)).await?;
    assert_eq!(status, 401, "{answer}");
    let user = table.issuer.users.lock().await["target"].clone();
    assert_eq!(user["enabled"], false);
    assert_eq!(user["email_verified"], false);
    assert_eq!(user["groups"], json!(["group"]));
    assert_eq!(user["user_values"], json!({"saved": "kept"}));
    assert!(user.get("password").is_none());
    let (status, answer) = table.enabled(&table.target, true, false).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["enabled"], true);
    let (status, answer) = table.service.get("/me", Some(&target_cookie)).await?;
    assert_eq!(
        status, 401,
        "reenabling must not revive an old session: {answer}"
    );
    let new_cookie = table
        .service
        .sign_in(Login {
            subject: "target".to_owned(),
            email: "person@example.test".to_owned(),
        })
        .await?;
    let (status, answer) = table.service.get("/me", Some(&new_cookie)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        table.issuer.users.lock().await.len(),
        2,
        "no account created"
    );
    Ok(())
}

#[tokio::test]
async fn an_agent_cannot_disable_its_responsible_administrator() -> TestResult {
    let table = Table::fresh().await?;
    table.grant(&table.person).await?;
    table.team(&[&table.agent, &table.person]).await?;
    let (status, answer) = table.enabled(&table.person, false, false).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted", "{answer}");
    assert!(
        answer["reason"]
            .as_str()
            .ok_or("refusal words missing")?
            .contains("responsible"),
        "{answer}"
    );
    assert_eq!(
        table.issuer.users.lock().await[ADMINISTRATOR]["enabled"],
        true
    );
    Ok(())
}
