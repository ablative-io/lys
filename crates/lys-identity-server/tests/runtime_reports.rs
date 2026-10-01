//! The runtime routes: a found session is reported by a signed-in identity
//! and kept without an identity, stopped only with the runtime's
//! confirmation, and listed to the administrator; an agent's session is
//! reported by the agent, the person responsible for it or the
//! administrator, and by no one else; each refusal is by name.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::{AgentId, OperationId};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
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

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    machine: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.certificates_dir = None;
                config.homes_dir = None;
                config.roles_file = None;
                config.provisioning_file = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
        )
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let machine = operation()?;
        let agent = seeded.people[0].agents[0].id.to_string();
        let body = json!({
            "operation": machine, "name": "Laptop 2", "kind": "laptop", "runtime": "local launcher",
            "slots": 2, "may_run": [agent], "may_reach": [],
        });
        let (status, named) = service.post("/network/machines", Some(&ada), &body).await?;
        assert_eq!(status, 200, "{named}");
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
            machine,
        })
    }

    fn body(&self, state: &str, confirmation: &str) -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": operation()?, "state": state, "machine": self.machine,
            "what": "a session with no identity, using a person's git-host token",
            "confirmation": confirmation,
        }))
    }
}

#[tokio::test]
async fn a_found_session_is_kept_without_an_identity_and_stops_only_when_confirmed() -> TestResult {
    let table = Table::set().await?;
    let session = operation()?;
    let path = format!("/runtime/found/{session}/reports");

    let unsigned = table
        .service
        .post(&path, None, &table.body("running", "")?)
        .await?;
    refused(&unsigned, 401, "NotSignedIn");
    let starting = table
        .service
        .post(&path, Some(&table.bea), &table.body("starting", "")?)
        .await?;
    refused(&starting, 400, "RequestMalformed");
    let mut elsewhere = table.body("running", "")?;
    elsewhere["machine"] = json!(operation()?);
    let elsewhere = table
        .service
        .post(&path, Some(&table.bea), &elsewhere)
        .await?;
    refused(&elsewhere, 404, "MachineUnknown");
    let unbegun = table
        .service
        .post(&path, Some(&table.bea), &table.body("stopped", "exited 0")?)
        .await?;
    refused(&unbegun, 404, "RuntimeSessionUnknown");

    let (status, seen) = table
        .service
        .post(&path, Some(&table.bea), &table.body("running", "")?)
        .await?;
    assert_eq!(status, 200, "{seen}");
    assert_eq!(
        seen["agent"],
        Value::Null,
        "a found session is never given an identity"
    );
    assert_eq!(seen["shown"], "running");
    assert_eq!(seen["machine_name"], "Laptop 2");
    assert_eq!(seen["runtime"], "local launcher");
    assert_eq!(seen["reported_by"], table.seeded.people[1].id.to_string());
    assert_eq!(seen["stopped"], Value::Null);

    let (status, network) = table.service.get("/network", Some(&table.bea)).await?;
    assert_eq!(status, 200, "{network}");
    assert_eq!(network["reports_served"], true);
    assert_eq!(
        network["machines"][0]["last_report_at"],
        seen["last_report_at"]
    );

    let unconfirmed = table
        .service
        .post(&path, Some(&table.bea), &table.body("stopped", "")?)
        .await?;
    refused(&unconfirmed, 400, "RequestMalformed");
    let (status, still) = table
        .service
        .get("/runtime/found", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{still}");
    assert_eq!(
        still["sessions"][0]["shown"], "running",
        "a stop without confirmation moves nothing"
    );

    let (status, stopped) = table
        .service
        .post(&path, Some(&table.bea), &table.body("stopped", "exited 0")?)
        .await?;
    assert_eq!(status, 200, "{stopped}");
    assert_eq!(stopped["shown"], "stopped");
    assert_eq!(stopped["stopped"]["confirmation"], "exited 0");
    let after = table
        .service
        .post(&path, Some(&table.bea), &table.body("running", "")?)
        .await?;
    refused(&after, 409, "RuntimeSessionStopped");

    refused(
        &table
            .service
            .get("/runtime/found", Some(&table.bea))
            .await?,
        403,
        "NotAdmitted",
    );
    let (status, found) = table
        .service
        .get("/runtime/found", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{found}");
    assert_eq!(found["sessions"].as_array().map(Vec::len), Some(1));
    let (status, identified) = table
        .service
        .get("/runtime/sessions", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{identified}");
    assert_eq!(
        identified["sessions"],
        json!([]),
        "a found session is not an agent's"
    );
    Ok(())
}

#[tokio::test]
async fn only_those_answering_for_the_agent_report_its_sessions() -> TestResult {
    let table = Table::set().await?;
    let agent = table.seeded.people[0].agents[0].id.to_string();
    let session = operation()?;
    let path = format!("/agents/{agent}/runtime/sessions/{session}/reports");

    let by_other = table
        .service
        .post(&path, Some(&table.bea), &table.body("starting", "")?)
        .await?;
    refused(&by_other, 403, "NotAdmitted");
    let (status, by_administrator) = table
        .service
        .post(&path, Some(&table.ada), &table.body("starting", "")?)
        .await?;
    assert_eq!(status, 200, "{by_administrator}");
    assert_eq!(by_administrator["shown"], "unconfirmed");
    let unheld = format!(
        "/agents/{}/runtime/sessions/{session}/reports",
        AgentId::generate()?
    );
    let unheld = table
        .service
        .post(&unheld, Some(&table.ada), &table.body("starting", "")?)
        .await?;
    refused(&unheld, 404, "AgentNotVisible");

    let listed = format!("/agents/{agent}/runtime/sessions");
    let (status, held) = table.service.get(&listed, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["sessions"].as_array().map(Vec::len), Some(1), "{held}");
    refused(
        &table.service.get(&listed, Some(&table.bea)).await?,
        404,
        "AgentNotVisible",
    );
    Ok(())
}

#[tokio::test]
async fn forged_browser_reports_cannot_change_an_agents_session() -> TestResult {
    let table = Table::set().await?;
    let agent = table.seeded.people[0].agents[0].id.to_string();
    let session = operation()?;
    let path = format!("/agents/{agent}/runtime/sessions/{session}/reports");
    let body = table.body("starting", "")?;
    let client = reqwest::Client::new();
    for (kind, origin) in [
        ("text/plain", None),
        ("application/json", Some("http://127.0.0.1:1")),
    ] {
        let mut request = client
            .post(format!("{}{path}", table.service.base))
            .header("cookie", &table.ada)
            .header("content-type", kind)
            .body(body.to_string());
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let response = request.send().await?;
        assert_eq!(response.status(), 400);
        let answer: Value = serde_json::from_str(&response.text().await?)?;
        assert_eq!(answer["refusal"], "RequestMalformed", "{answer}");
    }
    let (status, held) = table
        .service
        .get(
            &format!("/agents/{agent}/runtime/sessions"),
            Some(&table.ada),
        )
        .await?;
    assert_eq!(status, 200, "{held}");
    assert_eq!(held["sessions"], json!([]));
    let response = client
        .post(format!("{}{path}", table.service.base))
        .header("cookie", &table.ada)
        .header("content-type", "application/json")
        .header("origin", &table.service.base)
        .body(body.to_string())
        .send()
        .await?;
    assert_eq!(response.status(), 200, "{}", response.text().await?);
    Ok(())
}
