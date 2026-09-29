//! DIRECTORY-051 R6: a judge client's question reaches the judge on the
//! runner's own socket, and a caller the runner cannot prove is one of its
//! sessions is denied `peer_unproved` with nothing written in any session's
//! name.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::peer::{PeerAct, PeerRequest};
use lys_runner::protocol::PROTOCOL_VERSION;
use lys_runner::refusals::JudgeAsk;
use lys_runner::{Options, Runner};
use serde_json::{Value, json};

#[test]
fn an_unproved_judge_client_is_denied_on_the_runner_socket() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let socket = dir.path().join("runner.sock");
    let runner = Runner::open(&Options {
        socket: socket.clone(),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 4096,
    })?;
    let serving = runner.spawn();
    let stream = UnixStream::connect(&socket)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut greeting = String::new();
    reader.read_line(&mut greeting)?;
    let request = PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Judge(JudgeAsk {
            attempt: Some("toolu-1".to_owned()),
            tool_name: "Read".to_owned(),
            tool_input: json!({"file_path": "/etc/hosts"}),
            claimed_session: Some("s1".to_owned()),
            subagent: false,
        }),
    };
    let mut writer = &stream;
    writer.write_all(serde_json::to_string(&request)?.as_bytes())?;
    writer.write_all(b"\n")?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let answer: Value = serde_json::from_str(&line)?;
    let verdict = answer
        .pointer("/verdict")
        .or_else(|| answer.pointer("/answer/verdict"))
        .ok_or_else(|| format!("the runner answered no verdict: {line}"))?;
    assert_eq!(verdict["deny"], true, "{line}");
    assert_eq!(verdict["refusal"], "peer_unproved", "{line}");
    assert_eq!(verdict["audit"], "not_attributed", "{line}");
    serving.stop()?;
    Ok(())
}
