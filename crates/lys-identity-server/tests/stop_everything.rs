#![cfg(test)]
//! The master off switch through the service's routes, against Lys's own
//! runner started here: a pull ends every running session on every computer
//! and records each stopped with who and why, ends the agents' credential
//! handles, and is kept once; while it stands every start is refused
//! `everything_stopped` and after its release starts go through; only the
//! administrator pulls or releases it, and anyone signed in reads how it
//! stands; a dialled computer with no bridge in is named unreached and its
//! session is never recorded stopped. Every wait ends on an answer, never a
//! clock.

#[path = "support/harness_description.rs"]
mod harness_description;
#[path = "support/stub_program.rs"]
mod stub_program;

use std::error::Error;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::to_bytes;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

/// Every handle the stand-in broker was asked to end, by id.
type Drops = Arc<Mutex<Vec<String>>>;

/// A broker holding one handle for each holder, `h-` and the holder, that
/// ends each handle it is asked to and keeps the asking.
async fn broker(request: Request, drops: &Drops) -> Response {
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or_default().to_owned();
    if path == "/_lys/handles" {
        let holder = query.strip_prefix("holder=").unwrap_or_default();
        return axum::Json(json!({ "holder": holder, "handles": [
            { "id": format!("h-{holder}"), "secret": "model account", "max_uses": 10, "used": 1,
              "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0, "parent": null },
        ]}))
        .into_response();
    }
    if path == "/_lys/drop" {
        let Ok(bytes) = to_bytes(request.into_body(), usize::MAX).await else {
            return axum::Json(json!({ "refusal": "unreadable" })).into_response();
        };
        let asked: Value = serde_json::from_slice(&bytes).unwrap_or_default();
        let handle = asked["handle"].as_str().unwrap_or_default().to_owned();
        drops
            .lock()
            .expect("fixture lock poisoned")
            .push(handle.clone());
        return axum::Json(json!({ "handle": handle, "outcome": "ended" })).into_response();
    }
    axum::Json(json!({})).into_response()
}

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

/// One service, broker and real runner; Ada (administrator) and Bea, each
/// answering for one agent; one computer whose runner is Lys's own.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    dir: tempfile::TempDir,
    serving: Option<Serving>,
    machine: String,
    drops: Drops,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let drops = Drops::default();
        let kept = Arc::clone(&drops);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let stand_in = Router::new().fallback(move |request: Request| {
            let drops = Arc::clone(&kept);
            async move { broker(request, &drops).await }
        });
        tokio::spawn(async move { axum::serve(listener, stand_in).await });
        let dir = tempfile::tempdir()?;
        let key_file = dir.path().join("secrets-service.key");
        Ed25519Identity::load_or_generate(&key_file)?;
        let settings = SecretsSettings {
            broker: format!("http://{address}"),
            service: "identity".to_owned(),
            service_key_file: key_file,
        };
        let socket = dir.path().join("runner.sock");
        let state = dir.path().join("runner-state");
        let adjusted = socket.clone();
        let (service, (seeded, serving)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| config.runner_socket = Some(adjusted),
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
                let key = Ed25519Identity::load(&config.event_key_file)?;
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 1 << 16,
                })?;
                Ok((seeded, runner.spawn()))
            },
        )
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let mut table = Self {
            service,
            seeded,
            ada,
            bea,
            dir,
            serving: Some(serving),
            machine: String::new(),
            drops,
        };
        table.machine = table
            .named_machine("Runner box", Some(json!({ "kind": "lys" })))
            .await?;
        Ok(table)
    }

    fn agent(&self, person: usize) -> String {
        self.seeded.people[person].agents[0].id.to_string()
    }

    fn agent_name(&self, person: usize) -> String {
        self.seeded.people[person].agents[0].display_name.clone()
    }

    async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    async fn named_machine(
        &self,
        name: &str,
        runner: Option<Value>,
    ) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        let body = json!({
            "operation": id, "name": name, "kind": "laptop", "runtime": "sh",
            "slots": 2, "may_run": [self.agent(0), self.agent(1)], "may_reach": [],
        });
        self.ok("/network/machines", &body).await?;
        if let Some(runner) = runner {
            self.ok(
                &format!("/network/machines/{id}/runner"),
                &json!({ "runner": runner }),
            )
            .await?;
        }
        Ok(id)
    }

    async fn profile(&self, agent: &str) -> TestResult {
        let path = format!("/agents/{agent}/provisioning");
        let program = stub_program::write(self.dir.path(), stub_program::SHELL)?;
        let mut harness = harness_description::declared();
        harness["program"] = json!(program);
        let body = json!({
            "operation": operation()?, "from_version": 0, "working_folder": "/tmp",
            "model_access": ["claude-fable-5-1"], "tools": ["read"], "skills": [],
            "mcp_servers": [], "permissions": {"default_mode": "plan"}, "harness": harness,
            "instructions": "", "note": "", "session": {},
        });
        self.ok(&path, &body).await?;
        self.ok(
            &format!("{path}/1/review"),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(())
    }

    /// Ask to start `agent` on the runner's computer, as Ada.
    async fn start(&self, agent: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({ "machine": self.machine, "operation": operation()? });
        self.service
            .post(
                &format!("/agents/{agent}/start-command"),
                Some(&self.ada),
                &body,
            )
            .await
    }

    /// Start `agent`, answering its session.
    async fn started(&self, agent: &str) -> Result<String, Box<dyn Error>> {
        let (status, started) = self.start(agent).await?;
        assert_eq!(status, 200, "{started}");
        assert_eq!(started["runner"]["state"], "running", "{started}");
        Ok(started["session"].as_str().ok_or("no session")?.to_owned())
    }

    async fn pull(&self, cookie: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post("/runtime/stop-everything", Some(cookie), body)
            .await
    }

    async fn release(&self, cookie: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post("/runtime/stop-everything/release", Some(cookie), body)
            .await
    }

    /// The latest report on `agent`'s session `session`, as Ada reads it.
    async fn session(&self, agent: &str, session: &str) -> Result<Value, Box<dyn Error>> {
        let (status, sessions) = self
            .service
            .get(
                &format!("/agents/{agent}/runtime/sessions"),
                Some(&self.ada),
            )
            .await?;
        assert_eq!(status, 200, "{sessions}");
        sessions["sessions"]
            .as_array()
            .and_then(|all| all.iter().find(|each| each["session"] == session))
            .cloned()
            .ok_or_else(|| format!("no session {session} in {sessions}").into())
    }

    fn dropped(&self) -> Vec<String> {
        let mut drops = self.drops.lock().expect("fixture lock poisoned").clone();
        drops.sort();
        drops
    }

    fn close(mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pull_stops_every_running_session_records_who_and_why_and_is_kept_once() -> TestResult {
    let table = Table::set().await?;
    let (adas, beas) = (table.agent(0), table.agent(1));
    table.profile(&adas).await?;
    table.profile(&beas).await?;
    let first = table.started(&adas).await?;
    let second = table.started(&beas).await?;

    let body = json!({ "operation": operation()?, "reason": "a runaway loop is spending money" });
    let (status, pulled) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{pulled}");
    assert_eq!(pulled["operation"], body["operation"]);
    assert_eq!(
        pulled["pulled"]["reason"],
        "a runaway loop is spending money"
    );
    assert_eq!(
        pulled["pulled"]["by"],
        table.seeded.people[0].id.to_string()
    );
    assert_eq!(pulled["pulled"]["kill"], false);
    let mut stopped: Vec<(String, String, String)> = pulled["stopped"]
        .as_array()
        .ok_or("no stopped list")?
        .iter()
        .map(|each| {
            (
                each["session"].as_str().unwrap_or_default().to_owned(),
                each["agent_name"].as_str().unwrap_or_default().to_owned(),
                each["machine_name"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    stopped.sort();
    let mut expected = vec![
        (first.clone(), table.agent_name(0), "Runner box".to_owned()),
        (second.clone(), table.agent_name(1), "Runner box".to_owned()),
    ];
    expected.sort();
    assert_eq!(stopped, expected, "{pulled}");
    assert_eq!(pulled["still_running"], json!([]), "{pulled}");
    assert_eq!(pulled["unreached"], json!([]), "{pulled}");
    assert_eq!(pulled["handles_refused"], json!([]), "{pulled}");
    let mut handles = vec![format!("h-{adas}"), format!("h-{beas}")];
    handles.sort();
    assert_eq!(table.dropped(), handles, "each agent's handles are ended");

    for (agent, session) in [(&adas, &first), (&beas, &second)] {
        let kept = table.session(agent, session).await?;
        assert_eq!(kept["shown"], "stopped", "{kept}");
        assert_eq!(kept["last_reported"], "stopped", "{kept}");
        let what = kept["what"].as_str().unwrap_or_default();
        assert!(
            what.contains("everything stopped by") && what.contains("a runaway loop"),
            "who and why: {kept}"
        );
        assert!(
            kept["stop_asked_at"].is_u64(),
            "asked before it ended: {kept}"
        );
    }

    let (status, again) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(
        again, pulled,
        "the same pull sent again answers what it did"
    );
    assert_eq!(table.dropped(), handles, "and ends nothing twice");
    let changed = json!({ "operation": body["operation"], "reason": "other words" });
    refused(&table.pull(&table.ada, &changed).await?, 409, "cord_reused");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_start_while_everything_is_stopped_is_refused_and_goes_through_after_release()
-> TestResult {
    let table = Table::set().await?;
    let adas = table.agent(0);
    table.profile(&adas).await?;
    let body = json!({ "operation": operation()?, "reason": "the building is on fire" });
    let (status, pulled) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{pulled}");
    assert_eq!(pulled["stopped"], json!([]), "{pulled}");

    let answer = table.start(&adas).await?;
    refused(&answer, 409, "everything_stopped");
    let words = answer.1["reason"].as_str().unwrap_or_default();
    assert!(words.contains("the building is on fire"), "why: {words}");
    assert!(
        words.contains(&table.seeded.people[0].display_name),
        "who: {words}"
    );
    assert!(words.contains("No agent can be started"), "{words}");

    let release = json!({ "operation": operation()? });
    let (status, released) = table.release(&table.ada, &release).await?;
    assert_eq!(status, 200, "{released}");
    assert_eq!(released["pulled"], Value::Null, "{released}");
    assert_eq!(released["released"]["operation"], release["operation"]);
    assert_eq!(released["released"]["pull"], body["operation"]);
    assert_eq!(released["last"]["operation"], body["operation"]);
    let (status, again) = table.release(&table.ada, &release).await?;
    assert_eq!(
        (status, &again),
        (200, &released),
        "the same release answers the same"
    );
    refused(
        &table
            .release(&table.ada, &json!({ "operation": operation()? }))
            .await?,
        409,
        "cord_not_pulled",
    );

    table.started(&adas).await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_administrator_pulls_or_releases_and_anyone_signed_in_reads_how_it_stands()
-> TestResult {
    let table = Table::set().await?;
    refused(
        &table.service.get("/runtime/stop-everything", None).await?,
        401,
        "NotSignedIn",
    );
    let (status, stands) = table
        .service
        .get("/runtime/stop-everything", Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{stands}");
    assert_eq!(
        stands,
        json!({ "pulled": null, "last": null, "released": null, "may_pull": false })
    );

    let body = json!({ "operation": operation()?, "reason": "testing the cord" });
    refused(&table.pull(&table.bea, &body).await?, 403, "NotAdmitted");
    refused(
        &table
            .pull(
                &table.ada,
                &json!({ "operation": operation()?, "reason": " " }),
            )
            .await?,
        400,
        "RequestMalformed",
    );
    let (status, pulled) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{pulled}");

    let (status, stands) = table
        .service
        .get("/runtime/stop-everything", Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{stands}");
    assert_eq!(stands["pulled"]["reason"], "testing the cord");
    assert_eq!(
        stands["pulled"]["by_name"],
        table.seeded.people[0].display_name.as_str()
    );
    assert!(stands["pulled"]["at"].is_u64(), "{stands}");
    assert_eq!(stands["last"], pulled);
    assert_eq!(stands["may_pull"], false);
    let (status, own) = table
        .service
        .get("/runtime/stop-everything", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{own}");
    assert_eq!(own["may_pull"], true);

    let release = json!({ "operation": operation()? });
    refused(
        &table.release(&table.bea, &release).await?,
        403,
        "NotAdmitted",
    );
    let (status, released) = table.release(&table.ada, &release).await?;
    assert_eq!(status, 200, "{released}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dialled_computer_with_no_bridge_in_is_unreached_and_its_session_is_not_recorded_stopped()
-> TestResult {
    let table = Table::set().await?;
    let away = table
        .named_machine(
            "Away box",
            Some(json!({ "kind": "dialled", "key": "ab".repeat(32) })),
        )
        .await?;
    let beas = table.agent(1);
    let session = operation()?;
    let report = json!({
        "operation": operation()?, "state": "starting", "machine": away,
        "what": "launched", "confirmation": "",
    });
    let (status, answer) = table
        .service
        .post(
            &format!("/agents/{beas}/runtime/sessions/{session}/reports"),
            Some(&table.bea),
            &report,
        )
        .await?;
    assert_eq!(status, 200, "{answer}");

    let body =
        json!({ "operation": operation()?, "reason": "the away box is misbehaving", "kill": true });
    let (status, pulled) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{pulled}");
    assert_eq!(pulled["pulled"]["kill"], true);
    assert_eq!(pulled["stopped"], json!([]), "{pulled}");
    assert_eq!(
        pulled["unreached"],
        json!([{
            "machine": away, "machine_name": "Away box", "refusal": "runner_not_dialled_in",
            "reason": "this computer's runner is not connected to Lys right now",
        }]),
        "{pulled}"
    );
    assert_eq!(
        pulled["still_running"],
        json!([{
            "session": session, "agent": beas, "agent_name": table.agent_name(1),
            "machine": away, "machine_name": "Away box",
        }]),
        "{pulled}"
    );

    let kept = table.session(&beas, &session).await?;
    assert_eq!(kept["last_reported"], "stop_asked", "{kept}");
    assert_eq!(
        kept["shown"], "unconfirmed",
        "asked is never stopped: {kept}"
    );
    table.close()
}
