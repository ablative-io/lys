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

fn machine_body(table: &Table) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(json!({
        "operation": support::operation()?, "name": "Box", "kind": "laptop", "runtime": "sh",
        "slots": 1, "may_run": [table.agent()], "may_reach": [],
    }))
}

fn start_body(machine: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(json!({ "machine": machine, "operation": support::operation()? }))
}

/// Starts through the HTTP surface against a runner that answers `refused`,
/// and checks the answer carries the runner's refusal `name` and `words`
/// unchanged: the server neither renames nor rewords what the runner said.
async fn the_runners_refusal_survives(refused: RunnerError, name: &str, words: &str) -> TestResult {
    let table = Table::set().await?;
    let socket = table.dir.path().join("refusing.sock");
    let listener = UnixListener::bind(&socket)?;
    let machine = table
        .machine(
            &machine_body(&table)?,
            Some(json!({
                "kind": "socket", "path": socket,
            })),
        )
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
        if !matches!(act, Act::AsCaller { ref done, .. } if matches!(**done, Act::Start { .. })) {
            return Err("the service did not ask the runner to start".to_owned());
        }
        writeln!(writer, "{}", reply_line(Answer::refusal(&refused)))
            .map_err(|error| error.to_string())
    });
    let sent = table.start(&table.agent(), &start_body(&machine)?).await;
    answering
        .join()
        .map_err(|panic| format!("the refusing runner panicked: {panic:?}"))??;
    table.close()?;
    let (status, refused) = sent?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], name, "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains(words)),
        "{refused}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_start_keeps_the_runners_refusal_name() -> TestResult {
    let words = "the launch does not read";
    let refused = RunnerError::Malformed {
        reason: words.to_owned(),
    };
    the_runners_refusal_survives(refused, "runner_request_malformed", words).await
}

/// The runner refuses a launch that names no folder to start the run in
/// (`pty.rs`), never starting it in its own folder or the login's home.
#[tokio::test(flavor = "multi_thread")]
async fn a_launch_without_a_directory_keeps_the_runners_refusal_name() -> TestResult {
    let words = "the launch names no working directory to start the run in";
    let refused = RunnerError::refused("launch_without_directory", words);
    the_runners_refusal_survives(refused, "launch_without_directory", words).await
}

/// The runner cannot find the harness's configuration home: the launch sets
/// no `CLAUDE_CONFIG_DIR` and the runner has no `HOME` (`trust.rs`).
#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_trust_home_keeps_the_runners_refusal_name() -> TestResult {
    let words = "the launch sets no CLAUDE_CONFIG_DIR and the runner has no HOME to find the harness's configuration in";
    let refused = RunnerError::refused("trust_home_unknown", words);
    the_runners_refusal_survives(refused, "trust_home_unknown", words).await
}

/// The harness's configuration file is there but is not the JSON object the
/// trust row goes into; the runner leaves it as it is (`trust.rs`).
#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_trust_file_keeps_the_runners_refusal_name() -> TestResult {
    let words = "/home/seat/.claude.json is not a JSON object";
    let refused = RunnerError::refused("trust_file_invalid", words);
    the_runners_refusal_survives(refused, "trust_file_invalid", words).await
}

/// The harness's configuration file cannot be read (`trust.rs`).
#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_trust_file_keeps_the_runners_refusal_name() -> TestResult {
    let words = "/home/seat/.claude.json could not be read: Permission denied";
    let refused = RunnerError::refused("trust_file_unreadable", words);
    the_runners_refusal_survives(refused, "trust_file_unreadable", words).await
}

/// The trust row cannot be written beside the harness's configuration file
/// (`trust.rs`).
#[tokio::test(flavor = "multi_thread")]
async fn an_unwritable_trust_file_keeps_the_runners_refusal_name() -> TestResult {
    let words = "/home/seat/.claude.json.lys-4242 could not be written: Read-only file system";
    let refused = RunnerError::refused("trust_file_unwritable", words);
    the_runners_refusal_survives(refused, "trust_file_unwritable", words).await
}

/// The thread that watches the run's first screen for the trust dialog could
/// not be started (`session/trust_dialog.rs`).
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_trust_watch_keeps_the_runners_refusal_name() -> TestResult {
    let words = "Resource temporarily unavailable";
    let refused = RunnerError::refused("trust_watch_failed", words);
    the_runners_refusal_survives(refused, "trust_watch_failed", words).await
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
        "launch_without_directory",
        "trust_file_invalid",
        "trust_file_unreadable",
        "trust_file_unwritable",
        "trust_home_unknown",
        "trust_watch_failed",
    ];
    let missing: Vec<_> = expected
        .into_iter()
        .filter(|name| !route.refusals.contains(name))
        .collect();
    assert!(missing.is_empty(), "start-command omits {missing:?}");
    Ok(())
}
