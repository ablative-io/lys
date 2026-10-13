//! Managed envelopes exercise lifecycle persistence without a provider call.
use super::{Table, TestResult};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::Arc;

const PROGRAM: &str = r"#!/usr/bin/env python3
import json
import socket
import sys

gate = __CONFIRMATION_SOCKET__
received = __RECEIVED_TURNS__

version = '2.1.296'
if sys.argv[1:] == ['--version']:
    print(version + ' (Claude Code)')
    sys.exit(0)

arguments = sys.argv[1:]
session_flag = '--resume' if '--resume' in arguments else '--session-id'
conversation = arguments[arguments.index(session_flag) + 1]
initialized = False
for flag in ['--settings', '--mcp-config', '--append-system-prompt-file', '--system-prompt-file']:
    if flag in arguments:
        with open(arguments[arguments.index(flag) + 1], encoding='utf-8') as source:
            source.read()

def emit(frame):
    print(json.dumps(frame), flush=True)

for line in sys.stdin:
    frame = json.loads(line)
    if frame['type'] == 'control_request':
        if frame['request']['subtype'] != 'initialize':
            raise ValueError('unsupported fixture control request')
        emit({'type': 'control_response', 'response': {
            'subtype': 'success', 'request_id': frame['request_id'], 'response': {}}})
    elif frame['type'] == 'user':
        if frame['session_id'] != conversation or frame['parent_tool_use_id'] is not None:
            raise ValueError('fixture conversation mismatch')
        if 'still here' in frame['message']['content']:
            with open(received, 'a', encoding='utf-8') as record:
                record.write(json.dumps(frame) + '\n')
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as confirmation:
                confirmation.connect(gate)
                confirmation.sendall((json.dumps(frame) + '\n').encode())
                if confirmation.recv(1) != b'1':
                    raise ValueError('fixture confirmation refused')
        emit(frame)
        if not initialized:
            emit({'type': 'system', 'subtype': 'init', 'session_id': conversation,
                  'claude_code_version': version, 'slash_commands': ['compact']})
            initialized = True
        emit({'type': 'assistant', 'session_id': conversation, 'message': {
            'role': 'assistant', 'content': [{'type': 'text', 'text': 'still here'}]}})
        emit({'type': 'result', 'subtype': 'success', 'session_id': conversation,
              'uuid': 'result-' + frame['uuid'], 'is_error': False})
    else:
        raise ValueError('unsupported fixture frame')
";

pub(super) fn install(table: &Table) -> Result<UnixListener, Box<dyn std::error::Error>> {
    let socket = table.dir.path().join("confirm.sock");
    let listener = UnixListener::bind(&socket)?;
    let program = PROGRAM
        .replace("__CONFIRMATION_SOCKET__", &serde_json::to_string(&socket)?)
        .replace(
            "__RECEIVED_TURNS__",
            &serde_json::to_string(&table.dir.path().join("received-turns.jsonl"))?,
        );
    std::fs::write(table.dir.path().join("declared-harness"), program)?;
    Ok(listener)
}

pub(super) fn confirm(listener: &UnixListener, table: &Table, operation: &str) -> TestResult {
    let (mut socket, _) = listener.accept()?;
    let mut frame = String::new();
    BufReader::new(&socket).read_line(&mut frame)?;
    let received = std::fs::read_to_string(table.dir.path().join("received-turns.jsonl"))?;
    assert_eq!(received, frame, "the replay delivered another user turn");
    socket.write_all(b"1")?;
    let client = owner_client(table)?;
    let mut cursor = None;
    loop {
        let lys_runner::Answer::Feed { page } = client.ask(&lys_runner::Act::Feed {
            cursor,
            follow: true,
        })?
        else {
            return Err("confirmation wait did not answer a feed page".into());
        };
        if page.entries.iter().any(|entry| {
            matches!(&entry.body, lys_runner::tracking_store::Body::Operation(outcome)
                if outcome.operation == operation
                    && outcome.state == lys_runner::operations::OperationState::Confirmed)
        }) {
            return Ok(());
        }
        cursor = Some(page.cursor);
    }
}

fn owner_client(table: &Table) -> Result<lys_runner::Client, Box<dyn std::error::Error>> {
    let machine = lys_runner::Client::new(
        table.dir.path().join("runner.sock"),
        Arc::clone(&table.server_key),
    );
    let lys_runner::Answer::Owned { owners, .. } = machine.ask(&lys_runner::Act::Owned)? else {
        return Err("owner inventory did not answer Owned".into());
    };
    let owner = owners.first().ok_or("no owner")?;
    Ok(lys_runner::Client::new(
        owner.endpoint.socket.clone(),
        Arc::clone(&table.server_key),
    ))
}

pub(super) fn status(table: &Table) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = owner_client(table)?;
    let answer = client.ask(&lys_runner::Act::Status { session: None })?;
    let lys_runner::Answer::Status { status } = &answer else {
        return Err("owner status did not answer Status".into());
    };
    let session = &status.sessions.first().ok_or("no owner session")?.session;
    let dir = lys_runner::seat_owner::protocol::owner_dir(
        &table.dir.path().join("runner-state"),
        session,
    );
    let log = std::fs::read_to_string(dir.join(lys_runner::seat_owner::protocol::LOG))?;
    let diagnostics: Vec<_> = log
        .lines()
        .filter(|line| {
            line.contains("managed reader ended")
                || line.contains("control_")
                || line.contains("start_step")
        })
        .collect();
    Ok(serde_json::json!({"status": answer, "diagnostics": diagnostics}))
}
