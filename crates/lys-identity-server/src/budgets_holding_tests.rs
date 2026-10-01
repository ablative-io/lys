#![cfg(test)]

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::apps::{Auth, send};
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

struct Table {
    service: Service,
    cookie: String,
    giver: String,
    target: String,
    person: String,
    key: Arc<Ed25519Identity>,
}

fn body(amount: u64, version: u64) -> Value {
    json!({"limits": [{"unit": "tokens", "amount": amount, "period": "week", "act": "stop"}], "version": version})
}

impl Table {
    async fn fresh() -> TestResult<Self> {
        let (service, (giver, target, person, key)) = Service::start_adjusted(
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
                config.teams_dir = None;
                config.stops_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| {
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
                let (giver, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Giver")?,
                    2,
                )?;
                let (target, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Recipient")?,
                    2,
                )?;
                for agent in [giver, target] {
                    directory.transition(
                        actor.clone(),
                        OperationId::generate()?,
                        IdentityId::Agent(agent),
                        Transition::Activate,
                        "",
                        3,
                    )?;
                }
                Ok((
                    giver.to_string(),
                    target.to_string(),
                    person.to_string(),
                    Arc::new(Ed25519Identity::load(&config.event_key_file)?),
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
        let request = json!({
            "operation": OperationId::generate()?.to_string(),
            "request": STANDARD.encode(create_certificate_request(&key, &giver)?),
        });
        let (status, answer) = service
            .post(
                &format!("/agents/{giver}/certificates"),
                Some(&cookie),
                &request,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        let table = Self {
            service,
            cookie,
            giver,
            target,
            person,
            key,
        };
        let (status, answer) = table.put_person(&table.giver, &body(100, 0)).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(table)
    }

    async fn put_person(&self, agent: &str, body: &Value) -> TestResult<(u16, Value)> {
        send(
            &self.service,
            reqwest::Method::PUT,
            &format!("/budgets/agent/{agent}"),
            Auth::Cookie(&self.cookie),
            Some(body),
        )
        .await
    }

    async fn grant(&self) -> TestResult {
        let resource = json!({"kind": "budget", "id": format!("agent.{}", self.target)});
        let request = json!({
            "operation": OperationId::generate()?.to_string(), "route": "api", "holder": self.person,
            "resource": resource, "relation": "writer",
            "pass_on": {"kind": "to", "actions": ["write"], "recipients": ["agent"]},
            "window": {"starts_at": 0, "ends_at": null},
        });
        let (status, root) = self
            .service
            .post("/grants/roots", Some(&self.cookie), &request)
            .await?;
        assert_eq!(status, 200, "{root}");
        let request = json!({
            "operation": OperationId::generate()?.to_string(), "route": "api",
            "source": root["grant"], "recipient": self.giver, "responsible": self.person,
            "resource": resource, "relation": "writer", "pass_on": {"kind": "use_only"},
            "window": {"starts_at": 0, "ends_at": null},
        });
        let (status, answer) = self
            .service
            .post("/grants", Some(&self.cookie), &request)
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(())
    }

    async fn give(&self, body: &Value, cookie: bool) -> TestResult<(u16, Value)> {
        let path = format!("/budgets/agent/{}", self.target);
        let bytes = serde_json::to_vec(body)?;
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = crate::routes::hex(&Sha256::digest(
            OperationId::generate()?.to_string().as_bytes(),
        ));
        let signed = crate::agent_signature::payload("PUT", &path, &bytes, at, &nonce);
        let signature = crate::routes::hex(&sign_attestation(&signed, &self.key).to_cose_bytes());
        let mut request = reqwest::Client::new().put(format!("{}{path}", self.service.base));
        if cookie {
            request = request.header(reqwest::header::COOKIE, &self.cookie);
        }
        let response = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                crate::agent_signature::HEADER,
                format!("{} {at} {nonce} {signature}", self.giver),
            )
            .body(bytes)
            .send()
            .await?;
        let status = response.status().as_u16();
        Ok((status, response.json().await?))
    }
}

#[tokio::test]
async fn an_agent_cannot_give_a_budget_beyond_its_own_holding() -> TestResult {
    let table = Table::fresh().await?;
    table.grant().await?;
    let (status, answer) = table.give(&body(100, 0), false).await?;
    assert_eq!(status, 200, "the holding boundary is included: {answer}");
    let (status, answer) = table.give(&body(101, 1), false).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "HoldingNotHeld", "{answer}");
    let (status, held) = table
        .service
        .get(
            &format!("/budgets/agent/{}", table.target),
            Some(&table.cookie),
        )
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["version"], 1);
    assert_eq!(held["limits"][0]["amount"], 100);
    Ok(())
}

#[tokio::test]
async fn an_agent_without_a_grant_is_refused_on_budgets() -> TestResult {
    let table = Table::fresh().await?;
    let (status, answer) = table.give(&body(50, 0), true).await?;
    assert_eq!(
        status, 401,
        "an administrator cookie must not lend authority to its agent: {answer}"
    );
    assert_eq!(answer["refusal"], "AgentSignatureRefused", "{answer}");
    let (status, answer) = table.give(&body(50, 0), false).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotHeld", "{answer}");
    let (status, held) = table
        .service
        .get(
            &format!("/budgets/agent/{}", table.target),
            Some(&table.cookie),
        )
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["version"], 0);
    assert_eq!(held["limits"], json!([]));
    Ok(())
}

#[test]
fn a_holding_covers_the_exact_unit_period_zone_and_tightest_amount() -> TestResult {
    use crate::budgets_limits::{Limit, Limits};
    use crate::budgets_state::{Act, Held, Holder, HolderKind, Leaf, Length, Measure};

    let giver = Holder {
        kind: HolderKind::Agent,
        id: "giver".to_owned(),
    };
    let limit = Limit {
        unit: Measure::Tokens,
        amount: 50.into(),
        period: Some(Length::Week),
        act: Act::Stop,
        zone: None,
    };
    let mut held = Held::default();
    held.hold(Leaf::LimitsSet(Limits {
        holder: giver.clone(),
        limits: vec![
            limit.clone(),
            Limit {
                amount: 100.into(),
                act: Act::Tell,
                ..limit.clone()
            },
        ],
        warn_at: None,
        version: 1,
        by: "owner".to_owned(),
        at: 1,
    }))?;
    crate::budgets_holding::holds(
        &held,
        &giver,
        std::slice::from_ref(&limit),
        "Australia/Melbourne",
    )?;
    for requested in [
        Limit {
            amount: 51.into(),
            ..limit.clone()
        },
        Limit {
            unit: Measure::RunningMs,
            ..limit.clone()
        },
        Limit {
            period: Some(Length::Day),
            ..limit.clone()
        },
        Limit {
            zone: Some("UTC".to_owned()),
            ..limit.clone()
        },
    ] {
        let error =
            crate::budgets_holding::holds(&held, &giver, &[requested], "Australia/Melbourne")
                .err()
                .ok_or("an uncovered budget was given")?;
        assert_eq!(error.name(), "HoldingNotHeld");
        assert_eq!(error.status(), axum::http::StatusCode::FORBIDDEN);
    }
    assert!(crate::budgets_holding::holds(&held, &giver, &[], "Australia/Melbourne").is_err());
    assert!(
        crate::budgets_holding::holds(
            &held,
            &Holder {
                id: "unheld".to_owned(),
                ..giver
            },
            &[limit],
            "Australia/Melbourne"
        )
        .is_err()
    );
    Ok(())
}
