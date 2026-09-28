#![cfg(test)]
//! DIRECTORY-050 R3: a start on a machine whose record names a runner is
//! run by that runner and is listed running; a machine that names none is
//! answered its command as before, and nothing runs. R1: a runner on a
//! second machine, dialling the server through its bridge with that
//! machine's own key, starts the agent the server asked for. R2: a machine
//! naming a runner that answers another protocol version is refused
//! `runner_protocol_mismatch` at start, by name. Every wait ends on an
//! answer, never a clock.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::dial::Dial;
use lys_runner::protocol::hex;
use lys_runner::{Act, Answer, Client, Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

async fn broker(request: Request) -> Response {
    if request.uri().path() != "/_lys/handles" {
        return axum::Json(json!({})).into_response();
    }
    axum::Json(json!({ "holder": "any", "handles": [
        { "id": "h-live", "secret": "git-host token", "max_uses": 10, "used": 1,
          "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0, "parent": null },
    ]}))
    .into_response()
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    dir: tempfile::TempDir,
    serving: Option<Serving>,
    server_key: Arc<Ed25519Identity>,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        tokio::spawn(async move { axum::serve(listener, Router::new().fallback(broker)).await });
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
        let (service, (seeded, serving, server_key)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| config.runner_socket = Some(adjusted),
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 1 << 16,
                })?;
                Ok((seeded, runner.spawn(), key))
            },
        )
        .await?;
        let ada = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "ada@example.test".to_owned(),
            })
            .await?;
        let table = Self {
            service,
            seeded,
            ada,
            dir,
            serving: Some(serving),
            server_key,
        };
        table.profile().await?;
        Ok(table)
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    async fn profile(&self) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let body = json!({
            "operation": operation()?, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "",
        });
        self.ok(&path, &body).await?;
        self.ok(
            &format!("{path}/1/review"),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(())
    }

    /// Name a machine that runs the agent's shell, with `runner` as its
    /// runner when one is given.
    async fn machine(&self, runner: Option<Value>) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        let body = json!({
            "operation": id, "name": "Box", "kind": "laptop", "runtime": "sh",
            "slots": 1, "may_run": [self.agent()], "may_reach": [],
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

    async fn start(&self, machine: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": machine, "operation": operation()? });
        self.service.post(&path, Some(&self.ada), &body).await
    }

    fn close(mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_start_on_a_machine_with_the_runner_runs_and_is_listed_running() -> TestResult {
    let table = Table::set().await?;
    let machine = table.machine(Some(json!({ "kind": "lys" }))).await?;
    let (status, started) = table.start(&machine).await?;
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["executed"], false, "the service itself ran nothing");
    assert_eq!(started["runner"]["state"], "running", "{started}");
    let session = started["session"].as_str().ok_or("no session")?;

    let (_, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    let listed = live["sessions"]
        .as_array()
        .and_then(|all| all.iter().find(|s| s["session"] == session))
        .ok_or("the session is not listed live")?;
    assert_eq!(listed["shown"], "running", "{listed}");
    assert_eq!(live["unanswered"], json!([]), "{live}");

    let client = Client::new(
        table.dir.path().join("runner.sock"),
        Arc::clone(&table.server_key),
    );
    let Answer::Status { status } = client.ask(&Act::Status {
        session: Some(session.to_owned()),
    })?
    else {
        return Err("the runner answered no status".into());
    };
    assert!(status.sessions[0].ended.is_none(), "{status:?}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runner_that_does_not_answer_is_named_beside_the_live_list() -> TestResult {
    let mut table = Table::set().await?;
    let machine = table.machine(Some(json!({ "kind": "lys" }))).await?;
    let (status, started) = table.start(&machine).await?;
    assert_eq!(status, 200, "{started}");
    let session = started["session"].as_str().ok_or("no session")?.to_owned();
    if let Some(serving) = table.serving.take() {
        serving.stop()?;
    }
    let (status, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    assert_eq!(status, 200, "{live}");
    let unanswered = live["unanswered"]
        .as_array()
        .ok_or("the live list names no unanswered runners")?;
    assert_eq!(unanswered.len(), 1, "{live}");
    assert_eq!(unanswered[0]["session"], session.as_str(), "{live}");
    assert_eq!(unanswered[0]["machine"], machine.as_str(), "{live}");
    assert_eq!(unanswered[0]["refusal"], "runner_unreachable", "{live}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_machine_without_a_runner_answers_the_command_as_before() -> TestResult {
    let table = Table::set().await?;
    let machine = table.machine(None).await?;
    let (status, started) = table.start(&machine).await?;
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["executed"], false);
    assert!(started.get("runner").is_none(), "{started}");
    assert!(
        started["command"]
            .as_str()
            .is_some_and(|command| command.starts_with("env "))
    );
    let (_, sessions) = table
        .service
        .get("/runtime/sessions", Some(&table.ada))
        .await?;
    assert_eq!(
        sessions["sessions"][0]["shown"], "unconfirmed",
        "{sessions}"
    );
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runner_on_another_protocol_version_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let socket = table.dir.path().join("other.sock");
    let listener = UnixListener::bind(&socket)?;
    let answering = std::thread::spawn(move || -> std::io::Result<()> {
        let (stream, _) = listener.accept()?;
        let mut writer = &stream;
        writer.write_all(b"{\"version\":2,\"runner\":\"00\",\"challenge\":\"00\"}\n")?;
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line)?;
        assert!(
            line.is_empty(),
            "nothing is asked of a runner in another version: {line}"
        );
        Ok(())
    });
    let path = socket.display().to_string();
    let machine = table
        .machine(Some(json!({ "kind": "socket", "path": path })))
        .await?;
    let (status, refused) = table.start(&machine).await?;
    assert_eq!(status, 502, "{refused}");
    assert_eq!(refused["refusal"], "runner_protocol_mismatch", "{refused}");
    answering
        .join()
        .map_err(|_panicked| "the other runner panicked")??;
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runner_on_a_second_machine_dials_in_and_starts_the_agent() -> TestResult {
    let table = Table::set().await?;
    let second = tempfile::tempdir()?;
    let machine_key = Arc::new(Ed25519Identity::load_or_generate(
        &second.path().join("machine.key"),
    )?);
    let socket = second.path().join("runner.sock");
    let runner = Runner::open(&Options {
        socket: socket.clone(),
        state: second.path().join("state"),
        server_key: table.server_key.public_key_bytes(),
        scrollback: 1 << 16,
    })?
    .spawn();
    let machine = table
        .machine(Some(
            json!({ "kind": "dialled", "key": hex(&machine_key.public_key_bytes()) }),
        ))
        .await?;
    let dial = Dial {
        server: table.service.base.clone(),
        machine: machine.clone(),
        key: Arc::clone(&machine_key),
        socket: socket.clone(),
        authority: None,
    };
    std::thread::spawn(move || dial.bridge());

    let (status, started) = table.start(&machine).await?;
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["runner"]["state"], "running", "{started}");
    let session = started["session"].as_str().ok_or("no session")?;
    let local = Client::new(socket.clone(), Arc::clone(&table.server_key));
    let Answer::Status { status } = local.ask(&Act::Status {
        session: Some(session.to_owned()),
    })?
    else {
        return Err("the second machine's runner answered no status".into());
    };
    assert_eq!(
        status.sessions[0].session, session,
        "it runs on the second machine"
    );
    let second_runner = lys_runner::connect(&socket)?.greeting()?.runner;
    let (_, named) = table
        .service
        .get(
            &format!("/network/machines/{machine}/runner"),
            Some(&table.ada),
        )
        .await?;
    assert_eq!(
        named["runner"]["runner"], second_runner,
        "the first greeting pinned the machine to its runner: {named}"
    );

    let stranger = Ed25519Identity::load_or_generate(&second.path().join("stranger.key"))?;
    let forged = Dial {
        server: table.service.base.clone(),
        machine,
        key: Arc::new(stranger),
        socket: second.path().join("unused.sock"),
        authority: None,
    };
    let refused = tokio::task::spawn_blocking(move || forged.next("a greeting")).await?;
    assert!(
        refused.is_err_and(|error| error.to_string().contains("runner_dial_refused")),
        "a dial not signed by the machine's key is refused by name"
    );
    runner.stop()?;
    table.close()
}

/// Send one HTTP request to the service at `base` and read its status and
/// body; the answer ends when the service closes the connection.
fn raw_http(
    base: &str,
    method: &str,
    route: &str,
    headers: &[(&str, String)],
    body: &[u8],
) -> Result<(u16, String), Box<dyn Error + Send + Sync>> {
    use std::io::Read;
    let authority = base
        .strip_prefix("http://")
        .ok_or("the service is not http")?;
    let mut head = format!(
        "{method} {route} HTTP/1.1\r\nHost: {authority}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    let mut stream = std::net::TcpStream::connect(authority)?;
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let status = raw.split(' ').nth(1).ok_or("no status")?.parse()?;
    let (_, answer) = raw.split_once("\r\n\r\n").ok_or("no body")?;
    Ok((status, answer.to_owned()))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_captured_dial_is_admitted_once_and_never_under_another_epoch() -> TestResult {
    use lys_runner::dial::{
        EPOCH_HEADER, EPOCH_ROUTE, NONCE_HEADER, SIGNATURE_HEADER, dial_signed_bytes, reply_route,
    };
    let table = Table::set().await?;
    let key = Ed25519Identity::load_or_generate(&table.dir.path().join("machine.key"))?;
    let machine = table
        .machine(Some(
            json!({ "kind": "dialled", "key": hex(&key.public_key_bytes()) }),
        ))
        .await?;
    let base = table.service.base.clone();
    let asked = tokio::task::spawn_blocking(move || {
        let (status, epoch) = raw_http(&base, "GET", EPOCH_ROUTE, &[], b"")?;
        assert_eq!(status, 200, "{epoch}");
        let route = reply_route(&machine, "no-such-ticket");
        let signed = |epoch: &str, nonce: &str| {
            vec![
                (EPOCH_HEADER, epoch.to_owned()),
                (NONCE_HEADER, nonce.to_owned()),
                (
                    SIGNATURE_HEADER,
                    hex(&key.sign(&dial_signed_bytes("POST", &route, epoch, nonce, b"reply"))),
                ),
            ]
        };
        let captured = signed(&epoch, &lys_runner::protocol::nonce());
        let mut answers = Vec::new();
        for headers in [
            captured.clone(),
            captured,
            signed("0f0f", &lys_runner::protocol::nonce()),
        ] {
            answers.push(raw_http(&base, "POST", &route, &headers, b"reply")?);
        }
        Ok::<_, Box<dyn Error + Send + Sync>>(answers)
    })
    .await?
    .map_err(|error| error.to_string())?;
    let [admitted, replayed, stale] = asked.as_slice() else {
        return Err("three dials were not answered".into());
    };
    assert!(
        admitted.1.contains("no request of machine"),
        "the first dial is admitted and finds no request: {admitted:?}"
    );
    assert_eq!(replayed.0, 401, "{replayed:?}");
    assert!(replayed.1.contains("was already used"), "{replayed:?}");
    assert_eq!(stale.0, 409, "{stale:?}");
    assert!(stale.1.contains("runner_dial_stale"), "{stale:?}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bridge_carrying_another_runners_greeting_is_refused_by_name() -> TestResult {
    use lys_runner::dial::{
        EPOCH_HEADER, EPOCH_ROUTE, NONCE_HEADER, SIGNATURE_HEADER, dial_signed_bytes, next_route,
    };
    let table = Table::set().await?;
    let key = Ed25519Identity::load_or_generate(&table.dir.path().join("machine.key"))?;
    // The machine's own runner is pinned when its runner is named; the
    // greeting the bridge carries is that of another runner it can reach.
    let own = hex(&[0x11; 16]);
    let machine = table
        .machine(Some(json!({
            "kind": "dialled", "key": hex(&key.public_key_bytes()), "runner": own,
        })))
        .await?;
    let other_greeting =
        lys_runner::connect(&table.dir.path().join("runner.sock"))?.greeting_line()?;
    let base = table.service.base.clone();
    let route = next_route(&machine);
    let asked = tokio::task::spawn_blocking(move || {
        let (_, epoch) = raw_http(&base, "GET", EPOCH_ROUTE, &[], b"")?;
        let nonce = lys_runner::protocol::nonce();
        let body = other_greeting.as_bytes();
        let headers = vec![
            (EPOCH_HEADER, epoch.clone()),
            (NONCE_HEADER, nonce.clone()),
            (
                SIGNATURE_HEADER,
                hex(&key.sign(&dial_signed_bytes("POST", &route, &epoch, &nonce, body))),
            ),
        ];
        raw_http(&base, "POST", &route, &headers, body)
    })
    .await?
    .map_err(|error| error.to_string())?;
    assert_eq!(asked.0, 401, "{asked:?}");
    assert!(asked.1.contains("runner_dial_refused"), "{asked:?}");
    assert!(asked.1.contains("pins runner"), "{asked:?}");
    table.close()
}
