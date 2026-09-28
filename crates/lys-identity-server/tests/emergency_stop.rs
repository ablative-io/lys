//! An emergency stop suspends the agent, withdraws its certificates, asks
//! every open session's runtime to end it, and names the broker's refusal
//! when handles cannot be ended; the sessions stay unconfirmed until a
//! runtime reports; the same stop sent again answers the same; anyone but
//! the person responsible or the administrator is refused.

use std::error::Error;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_core::ca::create_certificate_request;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

/// Bea's active agent with a certificate and a session starting on a
/// machine, Ada's own agent, and the cookies of Ada (administrator) and Bea.
struct Table {
    service: Service,
    agent: String,
    session: String,
    serial: String,
    ada: String,
    bea: String,
    adas_agent: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let agent = seeded.people[1].agents[0].id.to_string();
        let machine = operation()?;
        let named = json!({
            "operation": machine, "name": "Laptop 2", "kind": "laptop", "runtime": "local launcher",
            "slots": 2, "may_run": [agent], "may_reach": [],
        });
        let (status, answer) = service
            .post("/network/machines", Some(&ada), &named)
            .await?;
        assert_eq!(status, 200, "{answer}");
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join("agent.key"),
        )?);
        let serial = operation()?;
        let issue = json!({
            "operation": serial,
            "request": STANDARD.encode(create_certificate_request(&key, &agent)?),
        });
        let (status, answer) = service
            .post(&format!("/agents/{agent}/certificates"), Some(&bea), &issue)
            .await?;
        assert_eq!(status, 200, "{answer}");
        let session = operation()?;
        let report = json!({
            "operation": operation()?, "state": "starting", "machine": machine,
            "what": "launched", "confirmation": "",
        });
        let (status, answer) = service
            .post(
                &format!("/agents/{agent}/runtime/sessions/{session}/reports"),
                Some(&bea),
                &report,
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(Self {
            service,
            agent,
            session,
            serial,
            ada,
            bea,
            adas_agent: seeded.people[0].agents[0].id.to_string(),
        })
    }

    fn path(&self) -> String {
        format!("/agents/{}/stop", self.agent)
    }
}

#[tokio::test]
async fn a_stop_suspends_withdraws_and_asks_and_is_kept_once() -> TestResult {
    let table = Table::set().await?;
    let body = json!({ "operation": operation()?, "reason": "leaked its key" });
    let (status, stopped) = table
        .service
        .post(&table.path(), Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{stopped}");
    assert_eq!(stopped["state"], "suspended");
    assert_eq!(stopped["certificates_withdrawn"], json!([table.serial]));
    assert_eq!(stopped["sessions_asked"], json!([table.session]));
    assert_eq!(stopped["credentials_ended"], Value::Null);
    assert!(
        stopped["credentials_refused"]
            .as_str()
            .is_some_and(|reason| reason.starts_with("SecretsUnavailable")),
        "{stopped}"
    );

    let (status, agent) = table
        .service
        .get(&format!("/agents/{}", table.agent), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{agent}");
    assert_eq!(agent["state"], "suspended");

    let (status, certificates) = table
        .service
        .get(
            &format!("/agents/{}/certificates", table.agent),
            Some(&table.bea),
        )
        .await?;
    assert_eq!(status, 200, "{certificates}");
    assert_eq!(
        certificates["certificates"][0]["withdrawn"]["reason"],
        "emergency stop: leaked its key"
    );

    let (status, sessions) = table
        .service
        .get(
            &format!("/agents/{}/runtime/sessions", table.agent),
            Some(&table.bea),
        )
        .await?;
    assert_eq!(status, 200, "{sessions}");
    let session = &sessions["sessions"][0];
    assert_eq!(session["shown"], "unconfirmed", "asked is not stopped");
    assert_eq!(session["last_reported"], "stop_asked");
    assert!(session["stop_asked_at"].is_u64(), "{session}");

    let (status, again) = table
        .service
        .post(&table.path(), Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, stopped, "the same stop sent again answers the same");
    refused(
        &table
            .service
            .post(
                &table.path(),
                Some(&table.bea),
                &json!({ "operation": body["operation"], "reason": "other words" }),
            )
            .await?,
        409,
        "StopReused",
    );

    let report = json!({
        "operation": operation()?, "state": "stopped", "machine": session["machine"],
        "what": "", "confirmation": "exited 143",
    });
    let (status, ended) = table
        .service
        .post(
            &format!(
                "/agents/{}/runtime/sessions/{}/reports",
                table.agent, table.session
            ),
            Some(&table.bea),
            &report,
        )
        .await?;
    assert_eq!(status, 200, "{ended}");
    assert_eq!(ended["shown"], "stopped");
    Ok(())
}

#[tokio::test]
async fn only_the_person_responsible_or_the_administrator_stops_an_agent() -> TestResult {
    let table = Table::set().await?;
    let body = json!({ "operation": operation()?, "reason": "leaked its key" });
    refused(
        &table.service.post(&table.path(), None, &json!({})).await?,
        401,
        "NotSignedIn",
    );
    refused(
        &table
            .service
            .post(
                &format!("/agents/{}/stop", table.adas_agent),
                Some(&table.bea),
                &body,
            )
            .await?,
        404,
        "AgentNotVisible",
    );
    refused(
        &table
            .service
            .post(
                &table.path(),
                Some(&table.ada),
                &json!({ "operation": operation()?, "reason": "" }),
            )
            .await?,
        400,
        "RequestMalformed",
    );
    let (status, stopped) = table
        .service
        .post(&table.path(), Some(&table.ada), &body)
        .await?;
    assert_eq!(status, 200, "{stopped}");
    let asked = json!({
        "operation": operation()?, "state": "stop_asked", "machine": stopped["agent"],
    });
    refused(
        &table
            .service
            .post(
                &format!(
                    "/agents/{}/runtime/sessions/{}/reports",
                    table.agent, table.session
                ),
                Some(&table.ada),
                &asked,
            )
            .await?,
        400,
        "RequestMalformed",
    );
    Ok(())
}
