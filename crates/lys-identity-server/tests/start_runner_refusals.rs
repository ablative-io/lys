//! A runner's named start refusal survives the HTTP contract unchanged.

#[path = "support/runner_start.rs"]
mod support;

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer, RunnerError};
use serde_json::json;
use support::Table;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_start_keeps_the_runners_refusal_name() -> TestResult {
    let table = Table::set().await?;
    let socket = table.dir.path().join("refusing.sock");
    let listener = UnixListener::bind(&socket)?;
    let machine = table
        .machine(Some(json!({
            "kind": "socket", "path": socket,
        })))
        .await?;
    let server_key = table.server_key.public_key_bytes();
    let answering = std::thread::spawn(move || -> Result<(), String> {
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let greeting = Greeting::fresh("00");
        let mut writer = &stream;
        writeln!(writer, "{}", greeting.line()).map_err(|error| error.to_string())?;
        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        let act =
            verify_request(&line, &server_key, &greeting).map_err(|error| error.to_string())?;
        if !matches!(act, Act::Start { .. }) {
            return Err("the service did not ask the runner to start".to_owned());
        }
        let refused = RunnerError::Malformed {
            reason: "the launch does not read".to_owned(),
        };
        writeln!(writer, "{}", reply_line(Answer::refusal(&refused)))
            .map_err(|error| error.to_string())
    });
    let sent = table.start(&machine).await;
    answering
        .join()
        .map_err(|panic| format!("the refusing runner panicked: {panic:?}"))??;
    table.close()?;
    let (status, refused) = sent?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "runner_request_malformed", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|words| words.contains("the launch does not read")),
        "{refused}"
    );
    Ok(())
}

#[test]
fn the_start_contract_names_transport_policy_and_launch_refusals() -> TestResult {
    let api = lys_identity_server::openapi::api();
    let route = api
        .routes()
        .iter()
        .find(|route| {
            route.path == "/agents/{id}/start-command" && route.method == lys_openapi::Method::Post
        })
        .ok_or("no start-command route")?;
    let expected = [
        "runner_request_unsigned",
        "runner_request_malformed",
        "runner_protocol_mismatch",
        "runner_request_replayed",
        "runner_request_misaddressed",
        "runner_unreachable",
        "runner_reply_malformed",
        "runner_state_unavailable",
        "session_invalid",
        "runner_stopping",
        "spawn_failed",
        "size_invalid",
        "rotation_invalid",
        "launch_config_refused",
        "policy_invalid",
        "policy_rule_duplicate",
        "policy_target_ambiguous",
        "policy_target_uninspectable",
        "policy_digest_mismatch",
        "session_unknown",
    ];
    let missing: Vec<_> = expected
        .into_iter()
        .filter(|name| !route.refusals.contains(name))
        .collect();
    assert!(missing.is_empty(), "start-command omits {missing:?}");
    Ok(())
}
