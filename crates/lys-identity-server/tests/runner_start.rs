#![cfg(test)]
//! DIRECTORY-050 R3: a start on a machine whose record names a runner is
//! run by that runner and is listed running; a machine that names none is
//! answered its command as before, and nothing runs. R1: a runner on a
//! second machine, dialling the server through its bridge with that
//! machine's own key, starts the agent the server asked for. R2: a machine
//! naming a runner that answers another protocol version is refused
//! `runner_protocol_mismatch` at start, by name. DIRECTORY-051 R6: the
//! start carries the agent's policy and the digest it was kept under, and
//! the runner holds that digest. Every wait ends on an
//! answer, never a clock.

#[path = "support/runner_start.rs"]
mod support;
use support::{Table, operation};

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::dial::Dial;
use lys_runner::protocol::hex;
use lys_runner::{Act, Answer, Client, Options, Runner};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn machine_body(table: &Table) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(json!({
        "operation": support::operation()?, "name": "Box", "kind": "laptop", "runtime": "sh",
        "slots": 1, "may_run": [table.agent()], "may_reach": [],
    }))
}

fn start_body(machine: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(json!({ "machine": machine, "operation": support::operation()? }))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_start_on_a_machine_with_the_runner_runs_and_is_listed_running() -> TestResult {
    let table = Table::set().await?;
    let machine = table
        .machine(&machine_body(&table)?, Some(json!({ "kind": "lys" })))
        .await?;
    let (status, started) = table.start(&table.agent(), &start_body(&machine)?).await?;
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
    let Answer::Matched { .. } = client.ask(&Act::Wait {
        session: session.to_owned(),
        cursor: Some(0),
        pattern: "profile-read".to_owned(),
        regex: false,
    })?
    else {
        return Err("the declared program did not read its profile".into());
    };
    let config = table
        .dir
        .path()
        .join("runner-state/sessions")
        .join(session)
        .join("config");
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config.join("settings.json"))?)?;
    assert_eq!(settings["env"]["LYS_AGENT"], table.agent());
    assert_eq!(settings["env"]["LYS_HANDLE_GIT_HOST_TOKEN"], "h-live");
    assert_eq!(std::fs::read(config.join("instructions.txt"))?, b"");
    let (status, provisioning) = table
        .service
        .get(
            &format!("/agents/{}/provisioning", table.agent()),
            Some(&table.ada),
        )
        .await?;
    assert_eq!(status, 200, "{provisioning}");
    assert_eq!(provisioning["enforced"], true);
    let path = format!("/agents/{}/provisioning", table.agent());
    table
        .ok(
            &path,
            &json!({
                "operation": operation()?, "from_version": 1,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [], "mcp_servers": [],
                "instructions": "The next profile has not run.", "note": "",
                "harness": provisioning["profile"]["harness"],
            }),
        )
        .await?;
    table
        .ok(
            &format!("{path}/2/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
    let (status, next) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{next}");
    assert_eq!(next["enforced"], false);
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_start_carries_the_agents_policy_and_the_runner_holds_its_digest() -> TestResult {
    let table = Table::set().await?;
    let rule = json!({ "id": "no-denied-writes", "tool": "Edit", "kind": "path_prefix",
                       "target": "/probe/denied", "authority": "hard" });
    let kept = table
        .ok(
            &format!("/agents/{}/policy", table.agent()),
            &json!({ "version": 0, "rules": [rule] }),
        )
        .await?;
    let machine = table
        .machine(&machine_body(&table)?, Some(json!({ "kind": "lys" })))
        .await?;
    let (status, started) = table.start(&table.agent(), &start_body(&machine)?).await?;
    assert_eq!(status, 200, "{started}");
    let session = started["session"].as_str().ok_or("no session")?;
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
    let held = status.sessions[0]
        .policy
        .as_ref()
        .ok_or("the session holds no policy")?;
    assert_eq!(held.version, 1);
    assert_eq!(
        Some(held.digest.as_str()),
        kept["digest"].as_str(),
        "{kept}"
    );
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_runner_that_does_not_answer_is_named_beside_the_live_list() -> TestResult {
    let mut table = Table::set().await?;
    let machine = table
        .machine(&machine_body(&table)?, Some(json!({ "kind": "lys" })))
        .await?;
    let (status, started) = table.start(&table.agent(), &start_body(&machine)?).await?;
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
    let machine = table.machine(&machine_body(&table)?, None).await?;
    let (status, started) = table.start(&table.agent(), &start_body(&machine)?).await?;
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
        .machine(
            &machine_body(&table)?,
            Some(json!({ "kind": "socket", "path": path })),
        )
        .await?;
    let (status, refused) = table.start(&table.agent(), &start_body(&machine)?).await?;
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
        .machine(
            &machine_body(&table)?,
            Some(json!({ "kind": "dialled", "key": hex(&machine_key.public_key_bytes()) })),
        )
        .await?;
    let dial = Dial {
        server: table.service.base.clone(),
        machine: machine.clone(),
        key: Arc::clone(&machine_key),
        socket: socket.clone(),
        authority: None,
    };
    std::thread::spawn(move || dial.bridge());

    let (status, started) = table.start(&table.agent(), &start_body(&machine)?).await?;
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
        .machine(
            &machine_body(&table)?,
            Some(json!({ "kind": "dialled", "key": hex(&key.public_key_bytes()) })),
        )
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
        .machine(
            &machine_body(&table)?,
            Some(json!({
                "kind": "dialled", "key": hex(&key.public_key_bytes()), "runner": own,
            })),
        )
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
