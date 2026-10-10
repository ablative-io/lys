//! `lys seat`: add, list, start, stop, restart and send to the seats the
//! installed identity server holds (AGENTS-002 R1, R6), asked over loopback
//! as the operator. Every act and every refusal is the server's; this file
//! asks, and prints the answer as human lines or one JSON object.

use serde_json::{Value, json};

use crate::cli::{SeatArgs, SeatCommand};
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::commands::seat_client::{Server, operation, segment};

/// Runs `lys seat`.
///
/// # Errors
///
/// The server's refusal by its name and words, or a named failure to find
/// the install or reach its server.
pub fn run(args: SeatArgs, json: bool) -> CliResult<()> {
    let server = Server::reach(args.server.as_deref())?;
    match args.command {
        SeatCommand::Add {
            name,
            agent,
            profile_version,
            machine,
            working_folder,
        } => {
            let mut body = json!({
                "operation": operation()?,
                "name": name,
                "agent": agent,
                "profile_version": profile_version,
                "machine": machine,
            });
            if let Some(folder) = working_folder {
                body["working_folder"] = Value::String(folder);
            }
            let seat = server.post("/seats", &body)?;
            show(json, &seat, |seat| {
                println!("seat {} added", shown(seat, "name"));
                seat_lines(seat);
            });
        }
        SeatCommand::List => {
            let answer = server.get("/seats")?;
            show(json, &answer, list_lines);
        }
        SeatCommand::Start { name } => {
            let answer = act(&server, &name, "start", json!({}))?;
            show(json, &answer, |answer| {
                println!("seat {} started", seat_name(answer));
                println!("session: {}", shown(answer, "session"));
                println!("harness session: {}", shown(answer, "harness_session"));
            });
        }
        SeatCommand::Stop { name, force } => {
            let answer = act(&server, &name, "stop", json!({ "force": force }))?;
            show(json, &answer, |answer| {
                println!("seat {} stopped", seat_name(answer));
                println!("session: {}", shown(answer, "session"));
                println!("ended: {}", shown(answer, "ended"));
            });
        }
        SeatCommand::Restart { name, force } => {
            let answer = act(&server, &name, "restart", json!({ "force": force }))?;
            show(json, &answer, |answer| {
                println!("seat {} restarted", seat_name(answer));
                println!("session: {}", shown(answer, "session"));
            });
        }
        SeatCommand::Send { name, text } => {
            let text = text.join(" ");
            let answer = act(&server, &name, "send", json!({ "text": text }))?;
            show(json, &answer, |answer| {
                println!("delivered to seat {} as a user turn", seat_name(answer));
                println!("session: {}", shown(answer, "session"));
            });
        }
    }
    Ok(())
}

/// `POST /seats/{name}/{verb}` with `body` and a new operation id.
fn act(server: &Server, name: &str, verb: &str, mut body: Value) -> CliResult<Value> {
    let name = segment("seat_name_invalid", "the seat name", name)?;
    body["operation"] = Value::String(operation()?);
    server.post(&format!("/seats/{name}/{verb}"), &body)
}

/// Prints `answer`: every one of its fields beside `ok` under `--json`,
/// else the lines `human` prints.
fn show(json: bool, answer: &Value, human: impl FnOnce(&Value)) {
    let mut emitter = Emitter::new(json);
    if emitter.is_json() {
        match answer {
            Value::Object(fields) => {
                for (key, value) in fields {
                    emitter.field(key, key, value.clone());
                }
            }
            other => emitter.field("answer", "answer", other.clone()),
        }
    } else {
        human(answer);
    }
    emitter.finish();
}

/// One seat's lines: what it runs as, where, and how it stands.
fn seat_lines(seat: &Value) {
    println!("agent: {}", shown(seat, "agent"));
    println!("harness: {}", shown(seat, "harness"));
    println!("profile version: {}", shown(seat, "profile_version"));
    println!("machine: {}", shown(seat, "machine"));
    println!("working folder: {}", shown(seat, "working_folder"));
    println!("responsible: {}", shown(seat, "responsible"));
    println!("state: {}", shown(seat, "state"));
    println!("session: {}", shown(seat, "session"));
}

/// The list's lines: one per seat, then how the runner was read. A runner
/// that could not be read is named with its reason, never shown as no
/// seats running.
fn list_lines(answer: &Value) {
    match answer.get("seats").and_then(Value::as_array) {
        Some(seats) if seats.is_empty() => println!("no seats"),
        Some(seats) => {
            for seat in seats {
                println!(
                    "{}  {}  agent {}  machine {}  session {}  last signal {}",
                    shown(seat, "name"),
                    shown(seat, "state"),
                    shown(seat, "agent"),
                    shown(seat, "machine"),
                    shown(seat, "session"),
                    shown(seat, "last_signal_at"),
                );
            }
        }
        None => println!("seats: the server's answer holds no list: {answer}"),
    }
    match answer.get("runner").and_then(Value::as_str) {
        Some("unknown") => println!("runner: unknown: {}", shown(answer, "reason")),
        Some(runner) => println!("runner: {runner}"),
        None => println!("runner: not stated by the server"),
    }
}

/// The seat an answer names, whether it carries the seat's name or its
/// whole view.
fn seat_name(answer: &Value) -> String {
    match answer.get("seat") {
        Some(seat @ Value::Object(_)) => shown(seat, "name"),
        Some(_) | None => shown(answer, "seat"),
    }
}

/// `value[key]` for a person: text as it is, `none` when absent or null,
/// anything else as JSON.
fn shown(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(text)) => text.clone(),
        None | Some(Value::Null) => "none".to_owned(),
        Some(other) => other.to_string(),
    }
}
