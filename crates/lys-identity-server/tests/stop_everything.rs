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
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_identity_server::dev_seed::{Seeded, SeededAgent, SeededPerson};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner, Serving, console_stop};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

/// Every handle the stand-in broker was asked to end, by id.
type Drops = Arc<Mutex<BrokerProbe>>;

#[derive(Default)]
struct BrokerProbe {
    drops: Vec<String>,
    callers: Vec<(String, String)>,
    refuse_drop: bool,
}

/// A broker holding one handle for each holder, `h-` and the holder, that
/// ends each handle it is asked to and keeps the asking.
async fn broker(request: Request, drops: &Drops) -> Response {
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or_default().to_owned();
    let caller = request
        .headers()
        .get("lys-on-behalf-of")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let refuse_drop = {
        let mut probe = drops.lock().expect("fixture lock poisoned");
        probe.callers.push((path.clone(), caller));
        probe.refuse_drop
    };
    if path == "/_lys/handles" {
        let holder = query.strip_prefix("holder=").unwrap_or_default();
        return axum::Json(json!({ "holder": holder, "handles": [
            { "id": format!("h-{holder}"), "secret": "model account", "max_uses": 10, "used": 1,
              "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0, "parent": null },
        ]}))
        .into_response();
    }
    if path == "/_lys/drop" {
        if refuse_drop {
            return (
                axum::http::StatusCode::FORBIDDEN,
                "BrokerOwnerRefused: custody denied",
            )
                .into_response();
        }
        let Ok(bytes) = to_bytes(request.into_body(), usize::MAX).await else {
            return axum::Json(json!({ "refusal": "unreadable" })).into_response();
        };
        let asked: Value = serde_json::from_slice(&bytes).unwrap_or_default();
        let handle = asked["handle"].as_str().unwrap_or_default().to_owned();
        drops
            .lock()
            .expect("fixture lock poisoned")
            .drops
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

/// Stop cases need only one active agent per person, with every seed event
/// still written through the directory's typed API.
fn seed_stop(config: &lys_identity_server::Config) -> Result<Seeded, Box<dyn Error>> {
    let mut directory = lys_identity_server::routes::open_directory(config)?;
    let actor = Actor::new(
        config.administrator_binding()?,
        Provenance::new(AuthMethod::Oidc, lys_identity_server::session::now()),
    );
    let mut people = Vec::new();
    for (subject, name, agent_name) in [
        (ADMINISTRATOR, "Ada (test person)", "Scribe"),
        (BEA, "Bea (test person)", "Reviewer"),
    ] {
        let at = lys_identity_server::session::now();
        let (id, _) = directory.register_person(
            actor.clone(),
            OperationId::generate()?,
            Profile::new(name)?,
            at,
        )?;
        directory.bind_login(
            actor.clone(),
            OperationId::generate()?,
            id,
            LoginBinding::new(&config.issuer, subject)?,
            at,
        )?;
        directory.transition(
            actor.clone(),
            OperationId::generate()?,
            IdentityId::Person(id),
            Transition::Activate,
            "",
            at,
        )?;
        let (agent, _) = directory.register_agent(
            actor.clone(),
            OperationId::generate()?,
            id,
            Profile::new(agent_name)?,
            at,
        )?;
        directory.transition(
            actor.clone(),
            OperationId::generate()?,
            IdentityId::Agent(agent),
            Transition::Activate,
            "",
            at,
        )?;
        people.push(SeededPerson {
            id,
            display_name: name.to_owned(),
            subject: subject.to_owned(),
            agents: vec![SeededAgent {
                id: agent,
                display_name: agent_name.to_owned(),
                state: LifecycleState::Active,
            }],
        });
    }
    let (tree_size, _) = directory.log()?.head()?;
    assert_eq!(tree_size, 10);
    Ok(Seeded { people, tree_size })
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
    console_base: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        Self::set_surface(false).await
    }

    async fn set_surface(surface: bool) -> Result<Self, Box<dyn Error>> {
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
        let screens = dir.path().join("screens");
        if surface {
            std::fs::create_dir(&screens)?;
            std::fs::write(screens.join("index.html"), "<html></html>")?;
        }
        let (mut service, (seeded, serving)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| {
                config.runner_socket = Some(adjusted);
                config.surface_dir = surface.then_some(screens);
            },
            move |config| {
                let seeded = seed_stop(config)?;
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
        let console_base = service.base.clone();
        if surface {
            service.base.push_str("/api");
        }
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
            console_base,
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
        let mut drops = self
            .drops
            .lock()
            .expect("fixture lock poisoned")
            .drops
            .clone();
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

async fn console_join(surface: bool) -> TestResult {
    let table = Table::set_surface(surface).await?;
    let agent = table.agent(0);
    table.profile(&agent).await?;
    let session = table.started(&agent).await?;
    let body = console_stop::Body {
        operation: operation()?,
        by: "the operator at the console of this computer".to_owned(),
        reason: "stop a runaway process".to_owned(),
        kill: true,
    };
    let bytes = serde_json::to_vec(&body)?;
    let key = Ed25519Identity::load(&table.service.dir.path().join("service.key"))?;
    let signature = lys_runner::protocol::hex(&key.sign(&console_stop::signed_bytes(&bytes)));
    let response = reqwest::Client::new()
        .post(format!(
            "{}{}",
            table.console_base,
            console_stop::route(surface)
        ))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(console_stop::SIGNATURE, signature)
        .body(bytes)
        .send()
        .await?;
    let status = response.status().as_u16();
    let pulled: Value = response.json().await?;
    assert_eq!(status, 200, "console stop answered {status}: {pulled}");
    assert_eq!(pulled["pulled"]["by"], "console", "{pulled}");
    assert_eq!(pulled["pulled"]["by_name"], body.by, "{pulled}");
    assert_eq!(pulled["pulled"]["reason"], body.reason, "{pulled}");
    assert_eq!(pulled["pulled"]["kill"], true, "{pulled}");
    assert_eq!(pulled["stopped"][0]["session"], session, "{pulled}");
    assert_eq!(table.session(&agent, &session).await?["shown"], "stopped");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn console_stop_joins_the_bare_service_and_stops_a_running_session() -> TestResult {
    console_join(false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn console_stop_joins_the_surface_service_and_stops_a_running_session() -> TestResult {
    console_join(true).await
}

/// Authentication and replay cases need the real service, but no runner or
/// provisioned process unless the case asks what a pull stops.
struct ConsoleTable {
    service: Service,
}

impl ConsoleTable {
    async fn set(reversible: bool) -> Result<Self, Box<dyn Error>> {
        let (service, _) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            move |config| {
                config.operator_upgrade_file =
                    reversible.then(|| config.log_dir.with_file_name("upgrade.json"));
            },
            |config| {
                if let Some(path) = &config.operator_upgrade_file {
                    std::fs::write(path, "unfinished upgrade")?;
                }
                seed_stop(config)
            },
        )
        .await?;
        Ok(Self { service })
    }

    fn body() -> Result<console_stop::Body, Box<dyn Error>> {
        Ok(console_stop::Body {
            operation: operation()?,
            by: "an unlisted person at this console".to_owned(),
            reason: "end an unsafe run".to_owned(),
            kill: true,
        })
    }

    fn sign(&self, bytes: &[u8]) -> Result<String, Box<dyn Error>> {
        let key = Ed25519Identity::load(&self.service.dir.path().join("service.key"))?;
        Ok(lys_runner::protocol::hex(
            &key.sign(&console_stop::signed_bytes(bytes)),
        ))
    }

    async fn post(
        &self,
        bytes: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post_carrying(console_stop::ROUTE, headers, bytes)
            .await
    }

    async fn pull(&self, body: &console_stop::Body) -> Result<(u16, Value), Box<dyn Error>> {
        let bytes = serde_json::to_vec(body)?;
        let signature = self.sign(&bytes)?;
        self.post(bytes, &[(console_stop::SIGNATURE, &signature)])
            .await
    }

    async fn untouched(&self) -> TestResult {
        let cookie = self.service.sign_in(login(ADMINISTRATOR)).await?;
        let (status, view) = self
            .service
            .get("/runtime/stop-everything", Some(&cookie))
            .await?;
        assert_eq!(status, 200, "{view}");
        assert_eq!(view["pulled"], Value::Null, "{view}");
        assert_eq!(view["last"], Value::Null, "{view}");
        Ok(())
    }
}

#[tokio::test]
async fn console_stop_missing_signature_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    refused(
        &table
            .post(serde_json::to_vec(&ConsoleTable::body()?)?, &[])
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_malformed_signature_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    refused(
        &table
            .post(
                serde_json::to_vec(&ConsoleTable::body()?)?,
                &[(console_stop::SIGNATURE, "not a signature")],
            )
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_duplicate_signature_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let bytes = serde_json::to_vec(&ConsoleTable::body()?)?;
    let signature = table.sign(&bytes)?;
    refused(
        &table
            .post(
                bytes,
                &[
                    (console_stop::SIGNATURE, &signature),
                    (console_stop::SIGNATURE, &signature),
                ],
            )
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_another_key_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let bytes = serde_json::to_vec(&ConsoleTable::body()?)?;
    let signature = lys_runner::protocol::hex(
        &Ed25519Identity::ephemeral().sign(&console_stop::signed_bytes(&bytes)),
    );
    refused(
        &table
            .post(bytes, &[(console_stop::SIGNATURE, &signature)])
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_changed_body_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let mut body = ConsoleTable::body()?;
    let signature = table.sign(&serde_json::to_vec(&body)?)?;
    body.reason = "a changed reason".to_owned();
    refused(
        &table
            .post(
                serde_json::to_vec(&body)?,
                &[(console_stop::SIGNATURE, &signature)],
            )
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_another_domain_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let bytes = serde_json::to_vec(&ConsoleTable::body()?)?;
    let key = Ed25519Identity::load(&table.service.dir.path().join("service.key"))?;
    let mut other = b"lys-identity/another-act/v1\n".to_vec();
    other.extend_from_slice(&bytes);
    let signature = lys_runner::protocol::hex(&key.sign(&other));
    refused(
        &table
            .post(bytes, &[(console_stop::SIGNATURE, &signature)])
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_cookie_beside_signature_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let cookie = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let bytes = serde_json::to_vec(&ConsoleTable::body()?)?;
    let signature = table.sign(&bytes)?;
    refused(
        &table
            .post(
                bytes,
                &[(console_stop::SIGNATURE, &signature), ("cookie", &cookie)],
            )
            .await?,
        401,
        "console_signature_refused",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_unknown_member_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let mut body = serde_json::to_value(ConsoleTable::body()?)?;
    body["nonce"] = json!("not part of this request");
    let bytes = serde_json::to_vec(&body)?;
    let signature = table.sign(&bytes)?;
    let answer = table
        .post(bytes, &[(console_stop::SIGNATURE, &signature)])
        .await?;
    refused(&answer, 400, "RequestMalformed");
    assert!(
        answer.1["reason"]
            .as_str()
            .ok_or("no reason")?
            .contains("unknown field")
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_empty_claim_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let mut body = ConsoleTable::body()?;
    body.by = "   ".to_owned();
    refused(&table.pull(&body).await?, 400, "RequestMalformed");
    table.untouched().await
}

#[tokio::test]
async fn console_stop_empty_reason_is_named_and_pulls_nothing() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let mut body = ConsoleTable::body()?;
    body.reason = "   ".to_owned();
    refused(&table.pull(&body).await?, 400, "RequestMalformed");
    table.untouched().await
}

#[tokio::test]
async fn console_stop_is_admitted_during_a_reversible_upgrade() -> TestResult {
    let table = ConsoleTable::set(true).await?;
    let body = ConsoleTable::body()?;
    let before = lys_identity_server::session::now();
    let (status, pulled) = table.pull(&body).await?;
    assert_eq!(status, 200, "{pulled}");
    assert_eq!(pulled["pulled"]["by"], "console");
    assert_eq!(pulled["pulled"]["by_name"], body.by);
    let at = pulled["pulled"]["at"].as_u64().ok_or("no service time")?;
    assert!(before <= at && at <= lys_identity_server::session::now());
    Ok(())
}

#[tokio::test]
async fn console_stop_replays_the_same_pull_without_new_work() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let body = ConsoleTable::body()?;
    let first = table.pull(&body).await?;
    assert_eq!(first.0, 200, "{}", first.1);
    assert_eq!(table.pull(&body).await?, first);
    Ok(())
}

#[tokio::test]
async fn console_stop_released_operation_cannot_pull_again() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let body = ConsoleTable::body()?;
    assert_eq!(table.pull(&body).await?.0, 200);
    let cookie = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let release = json!({"operation": operation()?});
    assert_eq!(
        table
            .service
            .post("/runtime/stop-everything/release", Some(&cookie), &release)
            .await?
            .0,
        200
    );
    refused(&table.pull(&body).await?, 409, "cord_reused");
    let (status, view) = table
        .service
        .get("/runtime/stop-everything", Some(&cookie))
        .await?;
    assert_eq!(status, 200);
    assert_eq!(view["pulled"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn console_stop_spent_operation_cannot_name_another_console_person() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let mut body = ConsoleTable::body()?;
    assert_eq!(table.pull(&body).await?.0, 200);
    body.by = "another console person".to_owned();
    refused(&table.pull(&body).await?, 409, "cord_reused");
    Ok(())
}

#[tokio::test]
async fn console_stop_cookie_and_run_pass_cannot_bypass_the_unverified_body_limit() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let cookie = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let bytes = vec![b' '; 2 * 1024 * 1024 + 1];
    let signature = table.sign(&bytes)?;
    refused(
        &table
            .post(
                bytes,
                &[
                    (console_stop::SIGNATURE, &signature),
                    ("cookie", &cookie),
                    (lys_identity_server::agent_pass::HEADER, "unverified-pass"),
                ],
            )
            .await?,
        413,
        "BodyTooLarge",
    );
    table.untouched().await
}

#[tokio::test]
async fn console_stop_signature_does_not_admit_an_ordinary_pull() -> TestResult {
    let table = ConsoleTable::set(false).await?;
    let bytes =
        serde_json::to_vec(&json!({"operation": operation()?, "reason": "not a console route"}))?;
    let signature = table.sign(&bytes)?;
    refused(
        &table
            .service
            .post_carrying(
                "/runtime/stop-everything",
                &[(console_stop::SIGNATURE, &signature)],
                bytes,
            )
            .await?,
        401,
        "NotSignedIn",
    );
    table.untouched().await
}

#[test]
fn console_stop_openapi_names_its_signature_and_shared_body() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    let route = &document["paths"][console_stop::ROUTE]["post"];
    assert_eq!(route["security"], json!([{"console_signature": []}]));
    assert_eq!(
        route["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ConsoleStopBody"
    );
    assert_eq!(
        document["components"]["securitySchemes"]["console_signature"]["name"],
        console_stop::SIGNATURE
    );
    assert_eq!(
        document["components"]["schemas"]["ConsoleStopBody"]["additionalProperties"],
        false
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn console_stop_holds_new_starts_and_names_the_console_claim() -> TestResult {
    let table = Table::set().await?;
    let body = ConsoleTable::body()?;
    let bytes = serde_json::to_vec(&body)?;
    let key = Ed25519Identity::load(&table.service.dir.path().join("service.key"))?;
    let signature = lys_runner::protocol::hex(&key.sign(&console_stop::signed_bytes(&bytes)));
    assert_eq!(
        table
            .service
            .post_carrying(
                console_stop::ROUTE,
                &[(console_stop::SIGNATURE, &signature)],
                bytes
            )
            .await?
            .0,
        200
    );
    let answer = table.start(&table.agent(0)).await?;
    refused(&answer, 409, "everything_stopped");
    let reason = answer.1["reason"]
        .as_str()
        .ok_or("no start refusal reason")?;
    assert!(
        reason.contains(&body.by) && reason.contains(&body.reason),
        "{reason}"
    );
    table.close()
}

async fn console_table_pull(
    table: &Table,
    body: &console_stop::Body,
) -> Result<(u16, Value), Box<dyn Error>> {
    let bytes = serde_json::to_vec(body)?;
    let key = Ed25519Identity::load(&table.service.dir.path().join("service.key"))?;
    let signature = lys_runner::protocol::hex(&key.sign(&console_stop::signed_bytes(&bytes)));
    table
        .service
        .post_carrying(
            console_stop::ROUTE,
            &[(console_stop::SIGNATURE, &signature)],
            bytes,
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn console_stop_ends_handles_as_each_directory_owner_and_replay_ends_nothing_twice()
-> TestResult {
    let table = Table::set().await?;
    for person in 0..2 {
        let agent = table.agent(person);
        table.profile(&agent).await?;
        table.started(&agent).await?;
    }
    let body = ConsoleTable::body()?;
    let first = console_table_pull(&table, &body).await?;
    assert_eq!(first.0, 200, "{}", first.1);
    assert_eq!(first.1["handles_refused"], json!([]));
    assert_eq!(first.1["stopped"].as_array().ok_or("no stopped")?.len(), 2);
    assert_eq!(
        first.1["handles_ended"]
            .as_array()
            .ok_or("no ended handles")?
            .len(),
        2
    );
    let before = {
        let probe = table.drops.lock().expect("fixture lock poisoned");
        assert_eq!(probe.callers.len(), 4);
        for person in &table.seeded.people {
            assert!(
                probe
                    .callers
                    .contains(&("/_lys/handles".to_owned(), person.id.to_string()))
            );
            assert!(
                probe
                    .callers
                    .contains(&("/_lys/drop".to_owned(), person.id.to_string()))
            );
        }
        probe.callers.clone()
    };
    assert_eq!(console_table_pull(&table, &body).await?, first);
    assert_eq!(table.dropped().len(), 2);
    assert_eq!(
        table.drops.lock().expect("fixture lock poisoned").callers,
        before
    );
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn console_stop_names_a_broker_refusal_instead_of_claiming_handles_ended() -> TestResult {
    let table = Table::set().await?;
    let agent = table.agent(0);
    table.profile(&agent).await?;
    let session = table.started(&agent).await?;
    table
        .drops
        .lock()
        .expect("fixture lock poisoned")
        .refuse_drop = true;
    let (status, pulled) = console_table_pull(&table, &ConsoleTable::body()?).await?;
    assert_eq!(status, 200, "{pulled}");
    assert_eq!(pulled["handles_ended"], json!([]));
    let refusals = pulled["handles_refused"]
        .as_array()
        .ok_or("no handle refusals")?;
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0]["agent"], agent);
    assert!(
        refusals[0]["refusal"]
            .as_str()
            .ok_or("no refusal")?
            .contains("BrokerOwnerRefused")
    );
    assert_eq!(pulled["stopped"][0]["session"], session);
    assert!(table.dropped().is_empty());
    table.close()
}
