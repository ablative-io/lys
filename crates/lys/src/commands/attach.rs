//! `lys attach`: show a seat's live session in this terminal (AGENTS-002
//! R4), asked of the installed identity server over loopback as the
//! operator.
//!
//! A seat runs managed, so what is shown is the runner's rendering of its
//! frames: user turns, assistant text, tool calls and results, and status,
//! one line each, followed as the session runs. It is read-only; with
//! `--type` each line typed is sent as a user turn through the seat's
//! input path, never as keys. A session not started managed is shown with
//! `--session <id>`, read-only, as the exact bytes of its pseudo-terminal.
//!
//! Detaching ends only this attach: Ctrl-C ends this process, and with
//! `--type` so does the end of standard input. Nothing here ever asks for
//! the session to end. The server keeps the attach and every typed line as
//! a record naming who.

use std::io::{BufRead, Write};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::cli::AttachArgs;
use crate::commands::error::{CliError, CliResult};
use crate::commands::seat_client::{Server, operation, refused, segment};

/// One page of a seat's session, as `POST /seats/{name}/attach` answers.
#[derive(Deserialize)]
struct Page {
    /// The session shown.
    session: String,
    /// Each line rendered from a frame, in order.
    lines: Vec<AttachLine>,
    /// Where the next page starts.
    cursor: u64,
    /// Whether the session has ended.
    ended: bool,
}

/// One frame of a managed session, rendered as a line.
#[derive(Deserialize)]
struct AttachLine {
    /// When, in milliseconds since the Unix epoch.
    at: u64,
    /// `user`, `assistant`, `tool_call`, `tool_result`, `status` or
    /// `system`.
    kind: String,
    /// The line's text.
    text: String,
}

/// A raw read, as `POST /runtime/sessions/{id}/read-bytes` answers.
#[derive(Deserialize)]
struct BytesReply {
    /// The runner's answer to the read.
    answer: lys_runner::Answer,
}

/// What reaches a typing attach: a line typed, standard input's end or a
/// failure to read it, or the follow's own end.
enum Event {
    /// A line typed.
    Typed(String),
    /// Standard input ended.
    Closed,
    /// Standard input could not be read.
    Unread(std::io::Error),
    /// The follow ended, as it ended.
    Followed(CliResult<()>),
}

/// Runs `lys attach`.
///
/// # Errors
///
/// The server's refusal by its name and words, or a named failure to find
/// the install, reach its server, read an answer or write the terminal.
pub fn run(args: &AttachArgs, json: bool) -> CliResult<()> {
    let server = Server::reach(args.server.as_deref())?;
    match (&args.seat, &args.session) {
        (Some(seat), None) if args.typing => typed(server, seat, json),
        (Some(seat), None) => follow_seat(&server, seat, json),
        (None, Some(session)) => follow_bytes(&server, session, json),
        (Some(_), Some(_)) | (None, None) => Err(refused(
            "attach_target_invalid",
            "name a seat, or a session with --session, and not both",
        )),
    }
}

/// Follows the seat's session until it ends, printing each line.
fn follow_seat(server: &Server, seat: &str, json: bool) -> CliResult<()> {
    let name = segment("seat_name_invalid", "the seat name", seat)?;
    let route = format!("/seats/{name}/attach");
    let mut cursor: Option<u64> = None;
    loop {
        let mut body = json!({ "follow": true });
        if let Some(cursor) = cursor {
            body["cursor"] = json!(cursor);
        } else if !json {
            eprintln!(
                "attaching to seat {name}; Ctrl-C detaches, and the seat's session runs on"
            );
        }
        let answer = server.post(&route, &body)?;
        let page: Page = serde_json::from_value(answer).map_err(|error| {
            refused(
                "attach_answer_unreadable",
                format!("POST {route}: the answer is not a page of the session: {error}"),
            )
        })?;
        print_lines(&page.lines, json)?;
        if page.ended {
            let fields = json!({ "seat": name, "session": page.session, "cursor": page.cursor });
            ended(json, &fields);
            return Ok(());
        }
        cursor = Some(page.cursor);
    }
}

/// Prints each line: assistant text as it is, every other kind after its
/// name; under `--json`, each line as one JSON object.
fn print_lines(lines: &[AttachLine], json: bool) -> CliResult<()> {
    let mut out = std::io::stdout().lock();
    for line in lines {
        let written = if json {
            let object = json!({ "at": line.at, "kind": line.kind, "text": line.text });
            writeln!(out, "{object}")
        } else if line.kind == "assistant" {
            writeln!(out, "{}", line.text)
        } else {
            writeln!(out, "{}: {}", line.kind, line.text)
        };
        written.map_err(terminal)?;
    }
    out.flush().map_err(terminal)
}

/// A failure to write the attach to standard output.
fn terminal(source: std::io::Error) -> CliError {
    CliError::Io {
        context: "writing the attach to standard output".to_owned(),
        source,
    }
}

/// Says the session ended: a line on standard error for a person, the
/// closing object under `--json`.
fn ended(json: bool, fields: &Value) {
    if json {
        let mut closing = fields.clone();
        closing["ok"] = Value::Bool(true);
        closing["ended"] = Value::Bool(true);
        println!("{closing}");
    } else {
        eprintln!("the session ended");
    }
}

/// Follows the seat while each line typed on standard input is sent to it
/// as a user turn. An empty line sends nothing. The end of standard input
/// detaches; the session runs on.
fn typed(server: Server, seat: &str, json: bool) -> CliResult<()> {
    let name = segment("seat_name_invalid", "the seat name", seat)?.to_owned();
    let route = format!("/seats/{name}/type");
    let server = Arc::new(server);
    let (events, received) = mpsc::channel();
    let follower = {
        let server = Arc::clone(&server);
        let events = events.clone();
        let name = name.clone();
        move || {
            let followed = follow_seat(&server, &name, json);
            if events.send(Event::Followed(followed)).is_err() {
                eprintln!("the follow of seat {name} ended after the attach did");
            }
        }
    };
    thread::spawn(follower);
    thread::spawn(move || {
        let lines = std::io::stdin()
            .lock()
            .lines()
            .map(|line| line.map_or_else(Event::Unread, Event::Typed));
        for event in lines.chain(std::iter::once(Event::Closed)) {
            let last = !matches!(event, Event::Typed(_));
            if events.send(event).is_err() || last {
                break;
            }
        }
    });
    for event in received {
        match event {
            Event::Typed(text) if text.trim().is_empty() => {}
            Event::Typed(text) => {
                let body = json!({ "operation": operation()?, "text": text });
                server.post(&route, &body)?;
            }
            Event::Closed => {
                if json {
                    println!("{}", json!({ "ok": true, "seat": name, "detached": true }));
                } else {
                    eprintln!("detached; the seat's session runs on");
                }
                return Ok(());
            }
            Event::Unread(source) => {
                return Err(CliError::Io {
                    context: "reading the lines to type from standard input".to_owned(),
                    source,
                });
            }
            Event::Followed(followed) => return followed,
        }
    }
    Err(refused(
        "attach_ended_unsaid",
        format!("the attach to seat {name} ended without its follow or its input saying why"),
    ))
}

/// Follows a session's pseudo-terminal read-only, writing its exact bytes
/// to standard output, until its process ends. Under `--json`, each read
/// is one JSON object carrying the bytes in base64.
fn follow_bytes(server: &Server, session: &str, json: bool) -> CliResult<()> {
    let id = segment("session_id_invalid", "the session id", session)?;
    let route = format!("/runtime/sessions/{id}/read-bytes");
    if !json {
        eprintln!(
            "attaching read-only to session {id}; Ctrl-C detaches, and the session runs on"
        );
    }
    let mut cursor: Option<u64> = None;
    loop {
        let answer = server.post(&route, &json!({ "cursor": cursor, "follow": true }))?;
        let reply: BytesReply = serde_json::from_value(answer).map_err(|error| {
            refused(
                "read_bytes_answer_unreadable",
                format!("POST {route}: the answer is not a runner's read: {error}"),
            )
        })?;
        let output = match reply.answer {
            lys_runner::Answer::Bytes { output } => output,
            lys_runner::Answer::Refused { refusal, words, .. } => {
                return Err(refused(&refusal, words));
            }
            other => {
                return Err(refused(
                    "read_bytes_answer_unexpected",
                    format!("POST {route}: the runner answered a read with {other:?}"),
                ));
            }
        };
        let mut out = std::io::stdout().lock();
        let written = if json {
            let data = base64::engine::general_purpose::STANDARD.encode(&output.data);
            let object = json!({ "from": output.from, "cursor": output.cursor, "data": data });
            writeln!(out, "{object}")
        } else {
            out.write_all(&output.data)
        };
        written.and_then(|()| out.flush()).map_err(terminal)?;
        drop(out);
        if output.ended.is_some() {
            ended(json, &json!({ "session": id, "cursor": output.cursor }));
            return Ok(());
        }
        cursor = Some(output.cursor);
    }
}
