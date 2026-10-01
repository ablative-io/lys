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

fn unhex(value: &str) -> TestResult<Vec<u8>> {
    (0..value.len())
        .step_by(2)
        .map(|at| {
            let pair = value
                .get(at..at + 2)
                .ok_or("signed event has an odd number of hex digits")?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect()
}

struct Table {
    service: Service,
    client: reqwest::Client,
    cookie: String,
    person: String,
    agent: String,
    key: Arc<Ed25519Identity>,
}

impl Table {
    async fn event(&self, answer: &Value) -> TestResult<lys_identity::IdentityEvent> {
        let index = answer["receipt"]["log"]["index"]
            .as_u64()
            .ok_or("receipt names no leaf")?;
        let (status, leaf) = self
            .service
            .get(&format!("/receipts/{index}"), None)
            .await?;
        assert_eq!(status, 200, "{leaf}");
        let (status, key) = self.service.get("/service-key", None).await?;
        assert_eq!(status, 200, "{key}");
        let bytes = unhex(key["ed25519"].as_str().ok_or("service key is missing")?)?;
        let key = <[u8; 32]>::try_from(bytes.as_slice())?;
        let message = unhex(leaf["message"].as_str().ok_or("signed event is missing")?)?;
        Ok(lys_identity::verify_event(&message, &key)?.event()?.clone())
    }

    async fn grant(&self, person: &str) -> TestResult<String> {
        let resource = json!({"kind": "person", "id": person});
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

    async fn profile(&self, name: &str) -> TestResult<(u16, Value)> {
        self.signed(
            &format!("/identities/{}/profile", self.person),
            &json!({"operation": operation()?, "display_name": name}),
            false,
        )
        .await
    }

    async fn fresh() -> TestResult<Self> {
        let (service, (person, agent, key)) = Service::start_adjusted(
            r#"{"version":1,"relations":{"writer":["write"]}}"#,
            None,
            None,
            None,
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
                Ok((person.to_string(), agent.to_string(), key))
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

    async fn record(&self, person: &str) -> TestResult<Value> {
        let (status, answer) = self
            .service
            .get(&format!("/identities/{person}"), Some(&self.cookie))
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}

#[tokio::test]
async fn a_signed_person_profile_write_refuses_an_administrator_cookie() -> TestResult {
    let table = Table::fresh().await?;
    let before = table.record(&table.person).await?;
    let (status, answer) = table
        .signed(
            &format!("/identities/{}/profile", table.person),
            &json!({"operation": operation()?, "display_name": "Changed"}),
            true,
        )
        .await?;
    assert_eq!(
        status, 401,
        "an agent cannot borrow an administrator cookie: {answer}"
    );
    assert_eq!(answer["refusal"], "AgentSignatureRefused", "{answer}");
    assert_eq!(table.record(&table.person).await?, before);
    let agent_before = table.record(&table.agent).await?;
    let (status, answer) = table
        .signed(
            &format!("/identities/{}/profile", table.agent),
            &json!({"operation": operation()?, "display_name": "Agent changed"}),
            false,
        )
        .await?;
    assert_eq!(
        status, 403,
        "the agent-profile branch remains outside this permission: {answer}"
    );
    assert_eq!(answer["refusal"], "NotAdmitted", "{answer}");
    assert_eq!(table.record(&table.agent).await?, agent_before);
    let (status, answer) = table
        .service
        .post(
            &format!("/identities/{}/profile", table.person),
            Some(&table.cookie),
            &json!({"operation": operation()?, "display_name": "Human edit"}),
        )
        .await?;
    assert_eq!(
        status, 200,
        "the administrator's profile path remains available: {answer}"
    );
    assert_eq!(
        table.record(&table.person).await?["display_name"],
        "Human edit"
    );
    Ok(())
}

#[tokio::test]
async fn an_agent_needs_a_live_person_write_grant_and_a_shared_admitted_team() -> TestResult {
    let table = Table::fresh().await?;
    table.team(&[&table.agent, &table.person]).await?;
    let before = table.record(&table.person).await?;
    let (status, answer) = table.profile("Without grant").await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotHeld", "{answer}");
    assert_eq!(table.record(&table.person).await?, before);
    let grant = table.grant(&table.person).await?;
    let (status, answer) = table.profile("Changed").await?;
    assert_eq!(
        status, 200,
        "a signature alone suffices with a live grant and holding: {answer}"
    );
    assert_eq!(answer["receipt"]["identity"], table.person);
    let event = table.event(&answer).await?;
    assert_eq!(
        event
            .actor()
            .provenance()
            .agent()
            .map(|agent| agent.to_string()),
        Some(table.agent.clone())
    );
    assert!(matches!(
        event.actor().provenance().method(),
        AuthMethod::AgentSignature(_)
    ));
    // Email and password take no agent at all; enabled takes an agent only
    // under an account grant, which a person profile grant is not.
    for (suffix, body, refused, refusal) in [
        (
            "email",
            json!({"email": "changed@example.test"}),
            401,
            "NotSignedIn",
        ),
        (
            "password",
            json!({"password": "Fixture-password"}),
            401,
            "NotSignedIn",
        ),
        ("enabled", json!({"enabled": true}), 403, "NotHeld"),
    ] {
        let (status, answer) = table
            .signed(
                &format!("/directory/people/{}/account/{suffix}", table.person),
                &body,
                false,
            )
            .await?;
        assert_eq!(
            status, refused,
            "a person profile grant does not admit an account write: {answer}"
        );
        assert_eq!(answer["refusal"], refusal, "{answer}");
    }
    assert_eq!(
        table.record(&table.person).await?["display_name"],
        "Changed"
    );
    let (status, answer) = table
        .service
        .post(
            &format!("/grants/{grant}/revoke"),
            Some(&table.cookie),
            &json!({"operation": operation()?, "route": "api", "reason": "ended"}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let before = table.record(&table.person).await?;
    let (status, answer) = table.profile("After revoke").await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "Revoked", "{answer}");
    assert_eq!(table.record(&table.person).await?, before);
    Ok(())
}

#[tokio::test]
async fn a_person_grant_does_not_replace_either_team_membership() -> TestResult {
    let table = Table::fresh().await?;
    table.grant(&table.person).await?;
    let team = table.team(&[&table.agent]).await?;
    let before = table.record(&table.person).await?;
    let (status, answer) = table.profile("Person absent").await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "HoldingNotHeld", "{answer}");
    assert_eq!(table.record(&table.person).await?, before);
    let (status, answer) = table
        .service
        .post(
            &format!("/teams/{team}/members/{}/remove", table.agent),
            Some(&table.cookie),
            &json!({"operation": operation()?}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = table
        .service
        .post(
            &format!("/teams/{team}/members"),
            Some(&table.cookie),
            &json!({"operation": operation()?, "member": table.person}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = table.profile("Agent absent").await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "HoldingNotHeld", "{answer}");
    assert_eq!(table.record(&table.person).await?, before);
    Ok(())
}

#[test]
fn held_memberships_and_retired_teams_do_not_admit_a_person_profile_write() -> TestResult {
    let agent = lys_identity::AgentId::generate()?;
    let person = lys_identity::PersonId::generate()?;
    let id = operation()?;
    let by = crate::read_views::Login {
        provider: "https://issuer.test".to_owned(),
        subject: "owner".to_owned(),
    };
    let mut team = crate::teams_state::Team {
        created: crate::teams_state::Created {
            id: id.clone(),
            owner: person.to_string(),
            name: "Team".to_owned(),
            description: String::new(),
            by: by.clone(),
            at: 1,
        },
        parent: None,
        lead: None,
        members: vec![agent.to_string(), person.to_string()],
        retired: None,
        held: Vec::new(),
        changes: Vec::new(),
    };
    super::people_giving::holds(std::slice::from_ref(&team), agent, person)?;
    for member in [agent.to_string(), person.to_string()] {
        team.held = vec![crate::teams_state::Hold {
            operation: operation()?,
            team: id.clone(),
            member,
            reason: "not admitted".to_owned(),
            at: 2,
        }];
        assert_eq!(
            super::people_giving::holds(std::slice::from_ref(&team), agent, person)
                .err()
                .ok_or("held membership was admitted")?
                .name(),
            "HoldingNotHeld"
        );
    }
    team.held.clear();
    team.retired = Some(crate::teams_state::Changed {
        operation: operation()?,
        team: id,
        member: String::new(),
        by,
        at: 3,
    });
    assert_eq!(
        super::people_giving::holds(std::slice::from_ref(&team), agent, person)
            .err()
            .ok_or("retired team was admitted")?
            .name(),
        "HoldingNotHeld"
    );
    Ok(())
}
