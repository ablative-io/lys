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
use lys_runner::console_stop;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

/// Every handle the stand-in broker was asked to end, by id.
type Drops = Arc<Mutex<Vec<String>>>;

/// The broker path and the person each call acts for.
type BrokerCalls = Arc<Mutex<Vec<(String, String)>>>;

#[derive(Default)]
struct Setting {
    under_api: bool,
    upgrade: bool,
    refuse_handles: bool,
    previous: Option<console_stop::Request>,
}

/// A broker holding one handle for each holder, `h-` and the holder, that
/// ends each handle it is asked to and keeps the asking.
async fn broker(
    request: Request,
    drops: &Drops,
    calls: &BrokerCalls,
    refuse_handles: bool,
) -> Response {
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or_default().to_owned();
    let person = request
        .headers()
        .get("lys-on-behalf-of")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    calls
        .lock()
        .expect("fixture lock poisoned")
        .push((path.clone(), person));
    if path == "/_lys/handles" && refuse_handles {
        return (
            axum::http::StatusCode::FORBIDDEN,
            "fixture_handles_refused: the broker refuses ending these handles",
        )
            .into_response();
    }
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
    broker_calls: BrokerCalls,
    under_api: bool,
}

fn keep_console(
    config: &lys_identity_server::Config,
    key: &Arc<Ed25519Identity>,
    previous: console_stop::Request,
) -> TestResult {
    use lys_identity_server::cord_store::{CordStore, Pull, PullResult, Pulling};
    let mut cord = CordStore::open(&config.log_dir.with_file_name("cord"), Arc::clone(key))?;
    let pull = Pull {
        operation: previous.operation.clone(),
        by: "console".to_owned(),
        by_name: Some(previous.by),
        reason: previous.reason,
        kill: previous.kill,
        at: 123,
    };
    assert!(matches!(cord.pull(pull.clone())?, Pulling::Go(_)));
    cord.finish(PullResult {
        operation: previous.operation,
        pulled: pull,
        stopped: Vec::new(),
        still_running: Vec::new(),
        unreached: Vec::new(),
        handles_ended: Vec::new(),
        handles_refused: Vec::new(),
    })?;
    Ok(())
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        Self::set_with(Setting::default()).await
    }

    async fn set_with(setting: Setting) -> Result<Self, Box<dyn Error>> {
        let Setting {
            under_api,
            upgrade,
            refuse_handles,
            previous,
        } = setting;
        let drops = Drops::default();
        let broker_calls = BrokerCalls::default();
        let called = Arc::clone(&broker_calls);
        let kept = Arc::clone(&drops);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let stand_in = Router::new().fallback(move |request: Request| {
            let drops = Arc::clone(&kept);
            let calls = Arc::clone(&called);
            async move { broker(request, &drops, &calls, refuse_handles).await }
        });
        tokio::spawn(async move { axum::serve(listener, stand_in).await });
        let dir = tempfile::tempdir()?;
        let screens = under_api.then(|| dir.path().join("screens"));
        if let Some(screens) = &screens {
            std::fs::create_dir(screens)?;
            std::fs::write(screens.join("index.html"), "<title>Lys</title>")?;
        }
        let intent = upgrade.then(|| dir.path().join("upgrade.json"));
        if let Some(intent) = &intent {
            std::fs::write(intent, "{}")?;
        }
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
            move |config| {
                config.runner_socket = Some(adjusted);
                config.surface_dir = screens;
                config.operator_upgrade_file = intent;
            },
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                if let Some(previous) = previous {
                    keep_console(config, &key, previous)?;
                }
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
        let ada = Self::sign_in(&service, ADMINISTRATOR, under_api).await?;
        let bea = Self::sign_in(&service, BEA, under_api).await?;
        let mut table = Self {
            service,
            seeded,
            ada,
            bea,
            dir,
            serving: Some(serving),
            machine: String::new(),
            drops,
            broker_calls,
            under_api,
        };
        table.machine = table
            .named_machine("Runner box", Some(json!({ "kind": "lys" })))
            .await?;
        Ok(table)
    }

    async fn sign_in(
        service: &Service,
        subject: &str,
        under_api: bool,
    ) -> Result<String, Box<dyn Error>> {
        if !under_api {
            return service.sign_in(login(subject)).await;
        }
        service.issuer.sign_in_as(login(subject))?;
        let response = reqwest::Client::new()
            .post(format!("{}/api/sign-in", service.base))
            .json(&json!({
                "email": "shared@example.test",
                "password": identity_contract::harness::HARNESS_PASSWORD,
            }))
            .send()
            .await?;
        identity_contract::harness::session_cookie(response).await
    }

    fn path(&self, path: &str) -> String {
        let prefix = if self.under_api { "/api" } else { "" };
        format!("{prefix}{path}")
    }

    fn agent(&self, person: usize) -> String {
        self.seeded.people[person].agents[0].id.to_string()
    }

    fn agent_name(&self, person: usize) -> String {
        self.seeded.people[person].agents[0].display_name.clone()
    }

    async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self
            .service
            .post(&self.path(path), Some(&self.ada), body)
            .await?;
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
                &self.path(&format!("/agents/{agent}/start-command")),
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
            .post(&self.path("/runtime/stop-everything"), Some(cookie), body)
            .await
    }

    async fn release(&self, cookie: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post(
                &self.path("/runtime/stop-everything/release"),
                Some(cookie),
                body,
            )
            .await
    }

    /// The latest report on `agent`'s session `session`, as Ada reads it.
    async fn session(&self, agent: &str, session: &str) -> Result<Value, Box<dyn Error>> {
        let (status, sessions) = self
            .service
            .get(
                &self.path(&format!("/agents/{agent}/runtime/sessions")),
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

    fn signed(&self, body: &[u8]) -> Result<String, Box<dyn Error>> {
        let key = Ed25519Identity::load(&self.service.dir.path().join("service.key"))?;
        Ok(lys_runner::protocol::hex(
            &key.sign(&console_stop::signed_bytes(body)),
        ))
    }

    async fn console(
        &self,
        body: &[u8],
        signature: Option<&str>,
        cookie: Option<&str>,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let mut request = reqwest::Client::new()
            .post(format!(
                "{}{}",
                self.service.base,
                console_stop::path(self.under_api)
            ))
            .header("content-type", "application/json")
            .body(body.to_owned());
        if let Some(signature) = signature {
            request = request.header(console_stop::SIGNATURE_HEADER, signature);
        }
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        Ok((status, serde_json::from_slice(&response.bytes().await?)?))
    }

    async fn untouched(&self) -> TestResult {
        let (status, standing) = self
            .service
            .get(&self.path("/runtime/stop-everything"), Some(&self.ada))
            .await?;
        assert_eq!(status, 200, "{standing}");
        assert!(standing["pulled"].is_null(), "{standing}");
        assert!(self.dropped().is_empty());
        Ok(())
    }

    fn called_handles(&self) -> Vec<(String, String)> {
        self.broker_calls
            .lock()
            .expect("fixture lock poisoned")
            .iter()
            .filter(|(path, _)| matches!(path.as_str(), "/_lys/handles" | "/_lys/drop"))
            .cloned()
            .collect()
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

/// A start and a pull sent together: whichever the service takes first, once
/// both have answered no session of the agent is left running under the
/// pulled cord, and a start that was answered as started was stopped by it.
#[tokio::test(flavor = "multi_thread")]
async fn a_start_sent_as_the_cord_is_pulled_is_never_left_running() -> TestResult {
    let table = Table::set().await?;
    let adas = table.agent(0);
    table.profile(&adas).await?;
    for round in 0..6 {
        let body = json!({ "operation": operation()?, "reason": "a runaway loop", "kill": true });
        let (start, pull) = tokio::join!(table.start(&adas), table.pull(&table.ada, &body));
        let (start, (status, pulled)) = (start?, pull?);
        assert_eq!(status, 200, "round {round}: {pulled}");
        if start.0 != 200 {
            refused(&start, 409, "everything_stopped");
        }
        let (status, sessions) = table
            .service
            .get(
                &format!("/agents/{adas}/runtime/sessions"),
                Some(&table.ada),
            )
            .await?;
        assert_eq!(status, 200, "{sessions}");
        let live: Vec<&Value> = sessions["sessions"]
            .as_array()
            .map(|all| {
                all.iter()
                    .filter(|each| each["shown"] != "stopped")
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            live.is_empty(),
            "round {round}: a session runs under a pulled cord; the start answered {}: {sessions}",
            start.0
        );
        let release = json!({ "operation": operation()? });
        let (status, released) = table.release(&table.ada, &release).await?;
        assert_eq!(status, 200, "{released}");
    }
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

const CONSOLE_CLAIM: &str = "operator at the console of this computer";
const CONSOLE_REASON: &str = "the console asked every agent to stop";

fn console_request() -> Result<console_stop::Request, Box<dyn Error>> {
    Ok(console_stop::Request::new(
        operation()?,
        CONSOLE_CLAIM.to_owned(),
        CONSOLE_REASON.to_owned(),
        true,
    ))
}

async fn console_join(under_api: bool) -> TestResult {
    let table = Table::set_with(Setting {
        under_api,
        ..Setting::default()
    })
    .await?;
    let agent = table.agent(0);
    table.profile(&agent).await?;
    let session = table.started(&agent).await?;
    let request = console_request()?;
    let body = serde_json::to_vec(&request)?;
    let signature = table.signed(&body)?;
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let (status, pulled) = table.console(&body, Some(&signature), None).await?;
    assert_eq!(status, 200, "console join: {pulled}");
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    assert_eq!(pulled["pulled"]["operation"], request.operation);
    assert_eq!(pulled["pulled"]["by"], "console");
    assert_eq!(pulled["pulled"]["by_name"], CONSOLE_CLAIM);
    assert_eq!(pulled["pulled"]["reason"], CONSOLE_REASON);
    assert_eq!(pulled["pulled"]["kill"], true);
    let recorded = pulled["pulled"]["at"].as_u64().ok_or("no service time")?;
    assert!((before..=after).contains(&recorded), "{pulled}");
    assert_eq!(pulled["stopped"].as_array().map(Vec::len), Some(1));
    assert_eq!(pulled["stopped"][0]["session"], session);
    assert_eq!(pulled["still_running"], json!([]));
    assert_eq!(pulled["unreached"], json!([]));
    assert_eq!(pulled["handles_refused"], json!([]), "{pulled}");
    assert_eq!(table.dropped(), vec![format!("h-{agent}")]);
    let calls = table.called_handles();
    let owner = table.seeded.people[0].id.to_string();
    assert_eq!(
        calls,
        vec![
            ("/_lys/handles".to_owned(), owner.clone()),
            ("/_lys/drop".to_owned(), owner),
        ]
    );
    let kept = table.session(&agent, &session).await?;
    assert_eq!(kept["shown"], "stopped", "{kept}");
    assert_eq!(kept["last_reported"], "stopped", "{kept}");
    let (status, again) = table.console(&body, Some(&signature), None).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, pulled, "the same signed stop keeps its answer");
    assert_eq!(table.session(&agent, &session).await?, kept);
    assert_eq!(table.dropped(), vec![format!("h-{agent}")]);
    assert_eq!(table.called_handles(), calls);
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_stop_reaches_the_real_router_and_records_its_claim() -> TestResult {
    console_join(false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_stop_reaches_the_real_router_under_the_screens() -> TestResult {
    console_join(true).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_stop_without_a_signature_is_refused_without_a_pull() -> TestResult {
    let table = Table::set().await?;
    let body = serde_json::to_vec(&console_request()?)?;
    refused(&table.console(&body, None, None).await?, 401, "NotSignedIn");
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_stop_with_a_malformed_signature_is_refused_without_a_pull() -> TestResult {
    let table = Table::set().await?;
    let body = serde_json::to_vec(&console_request()?)?;
    refused(
        &table.console(&body, Some("not a signature"), None).await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_keys_console_signature_is_refused_without_a_pull() -> TestResult {
    let table = Table::set().await?;
    let body = serde_json::to_vec(&console_request()?)?;
    let key = Ed25519Identity::load_or_generate(&table.dir.path().join("other.key"))?;
    let signature = lys_runner::protocol::hex(&key.sign(&console_stop::signed_bytes(&body)));
    refused(
        &table.console(&body, Some(&signature), None).await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changing_even_whitespace_after_console_signing_is_refused_without_a_pull() -> TestResult {
    let table = Table::set().await?;
    let mut body = serde_json::to_vec(&console_request()?)?;
    let signature = table.signed(&body)?;
    body.push(b' ');
    refused(
        &table.console(&body, Some(&signature), None).await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_cookie_beside_a_console_signature_is_refused_without_a_pull() -> TestResult {
    let table = Table::set().await?;
    let body = serde_json::to_vec(&console_request()?)?;
    let signature = table.signed(&body)?;
    refused(
        &table
            .console(&body, Some(&signature), Some(&table.ada))
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_signed_console_body_with_an_unknown_member_is_refused_without_a_pull() -> TestResult {
    let table = Table::set().await?;
    let mut request = serde_json::to_value(console_request()?)?;
    request["at"] = json!(1);
    let body = serde_json::to_vec(&request)?;
    let signature = table.signed(&body)?;
    let answer = table.console(&body, Some(&signature), None).await?;
    refused(&answer, 400, "RequestMalformed");
    assert!(
        answer.1["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("unknown field `at`"))
    );
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_stop_is_admitted_while_the_upgrade_is_reversible() -> TestResult {
    let table = Table::set_with(Setting {
        upgrade: true,
        ..Setting::default()
    })
    .await?;
    let body = serde_json::to_vec(&console_request()?)?;
    let signature = table.signed(&body)?;
    let (status, answer) = table.console(&body, Some(&signature), None).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["pulled"]["by"], "console");
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_start_after_a_console_stop_names_the_console_claim_and_reason() -> TestResult {
    let table = Table::set().await?;
    let agent = table.agent(0);
    table.profile(&agent).await?;
    let body = serde_json::to_vec(&console_request()?)?;
    let signature = table.signed(&body)?;
    let (status, answer) = table.console(&body, Some(&signature), None).await?;
    assert_eq!(status, 200, "{answer}");
    let answer = table.start(&agent).await?;
    refused(&answer, 409, "everything_stopped");
    let words = answer.1["reason"].as_str().ok_or("no refusal words")?;
    assert!(words.contains(CONSOLE_CLAIM), "{words}");
    assert!(words.contains(CONSOLE_REASON), "{words}");
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_pull_names_the_broker_refusal_instead_of_skipping_handles() -> TestResult {
    let table = Table::set_with(Setting {
        refuse_handles: true,
        ..Setting::default()
    })
    .await?;
    let agent = table.agent(0);
    table.profile(&agent).await?;
    table.started(&agent).await?;
    let body = serde_json::to_vec(&console_request()?)?;
    let signature = table.signed(&body)?;
    let (status, answer) = table.console(&body, Some(&signature), None).await?;
    assert_eq!(status, 200, "{answer}");
    let refused = answer["handles_refused"]
        .as_array()
        .ok_or("no handle refusals")?;
    assert_eq!(refused.len(), 1, "{answer}");
    assert_eq!(refused[0]["agent"], agent);
    assert!(
        refused[0]["refusal"]
            .as_str()
            .is_some_and(|words| words.contains("fixture_handles_refused")),
        "{answer}"
    );
    assert!(table.dropped().is_empty());
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_pull_replayed_before_release_answers_the_same_pull() -> TestResult {
    let request = console_request()?;
    let table = Table::set_with(Setting {
        previous: Some(request.clone()),
        ..Setting::default()
    })
    .await?;
    let (status, standing) = table
        .service
        .get("/runtime/stop-everything", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{standing}");
    let body = serde_json::to_vec(&request)?;
    let signature = table.signed(&body)?;
    let (status, again) = table.console(&body, Some(&signature), None).await?;
    assert_eq!(status, 200, "permit replay before release: {again}");
    assert_eq!(again, standing["last"], "same retained answer");
    assert!(table.dropped().is_empty());
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_console_pull_replayed_after_release_is_refused_as_cord_reused() -> TestResult {
    let request = console_request()?;
    let table = Table::set_with(Setting {
        previous: Some(request.clone()),
        ..Setting::default()
    })
    .await?;
    let (status, released) = table
        .release(&table.ada, &json!({ "operation": operation()? }))
        .await?;
    assert_eq!(status, 200, "{released}");
    assert!(released["pulled"].is_null(), "{released}");
    let body = serde_json::to_vec(&request)?;
    let signature = table.signed(&body)?;
    refused(
        &table.console(&body, Some(&signature), None).await?,
        409,
        "cord_reused",
    );
    table.untouched().await?;
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_console_claim_cannot_reuse_a_kept_console_operation() -> TestResult {
    let mut request = console_request()?;
    let table = Table::set_with(Setting {
        previous: Some(request.clone()),
        ..Setting::default()
    })
    .await?;
    request.by = "another operator at the console of this computer".to_owned();
    let body = serde_json::to_vec(&request)?;
    let signature = table.signed(&body)?;
    refused(
        &table.console(&body, Some(&signature), None).await?,
        409,
        "cord_reused",
    );
    let (status, standing) = table
        .service
        .get("/runtime/stop-everything", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{standing}");
    assert_eq!(standing["pulled"]["by_name"], CONSOLE_CLAIM);
    assert!(table.dropped().is_empty());
    table.close()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_signed_in_pull_replayed_after_release_keeps_its_answer() -> TestResult {
    let table = Table::set().await?;
    let body = json!({ "operation": operation()?, "reason": "a signed-in stop" });
    let (status, pulled) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{pulled}");
    let (status, released) = table
        .release(&table.ada, &json!({ "operation": operation()? }))
        .await?;
    assert_eq!(status, 200, "{released}");
    let (status, again) = table.pull(&table.ada, &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(
        again, pulled,
        "the signed-in pull keeps its old replay behavior"
    );
    table.untouched().await?;
    table.close()
}
