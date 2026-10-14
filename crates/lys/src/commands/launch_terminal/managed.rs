//! Managed seats keep their boundary controls while canonical lines target an exact session.

use std::io::Write;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use lys_runner::attach::AttachLine;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::unix::{Signals, Stops, Terminal, request, terminal_error};
use super::{Transport, delivered, ended_session, exit, finish};
use crate::commands::error::{CliError, CliResult};
use crate::commands::seat_client::{Server, refused};

#[derive(Debug, Deserialize)]
struct Page {
    session: String,
    lines: Vec<AttachLine>,
    cursor: u64,
    ended: bool,
}

pub(super) fn attach(
    server: &Server,
    seat: &str,
    session: &str,
    mut terminal: Terminal,
) -> CliResult<()> {
    let mut follower = None;
    let outcome = (|| {
        terminal.canonical()?;
        let control = Arc::new(server.separate()?);
        let read = Arc::new(server.separate()?);
        let (runtime, mut signals) = terminal.events()?;
        runtime.block_on(async {
            let (pages, handle) = follow(read, seat, session)?;
            follower = Some(handle);
            interact(control, session, &terminal, pages, &mut signals).await
        })
    })();
    let observed_end = matches!(outcome, Ok(true));
    let restored = terminal.restore();
    let ended = ended_session(server, session);
    let confirmed = ended.is_ok();
    let ended = if observed_end {
        ended.and_then(|ended| exit(&ended))
    } else {
        ended.map(drop)
    };
    if confirmed {
        if let Some(handle) = follower {
            if handle.join().is_err() {
                return finish(
                    outcome.map(drop),
                    Err(refused(
                        "launch_managed_follow_panicked",
                        "the managed terminal reader panicked",
                    )),
                    finish(Ok(()), ended, restored),
                );
            }
        }
    }
    finish(outcome.map(drop), ended, restored)
}

type Pages = mpsc::Receiver<CliResult<Page>>;

fn follow<T: Transport>(
    transport: Arc<T>,
    seat: &str,
    session: &str,
) -> CliResult<(Pages, JoinHandle<()>)> {
    let (output, pages) = mpsc::channel(1);
    let route = format!("/seats/{seat}/attach");
    let session = session.to_owned();
    let handle = thread::Builder::new()
        .name("lys-managed-terminal-output".to_owned())
        .spawn(move || {
            let mut cursor = None;
            loop {
                let page = transport
                    .post(&route, &json!({"cursor": cursor, "follow": true}))
                    .and_then(|value| page(value, &session, cursor));
                let last = match &page {
                    Ok(page) => page.ended,
                    Err(_) => true,
                };
                if let Ok(page) = &page {
                    cursor = Some(page.cursor);
                }
                if let Err(abandoned) = output.blocking_send(page) {
                    if let Err(error) = abandoned.0 {
                        eprintln!("{error}");
                    }
                    return;
                }
                if last {
                    return;
                }
            }
        })
        .map_err(|error| terminal_error("start the managed terminal reader", &error))?;
    Ok((pages, handle))
}

fn page(value: Value, session: &str, cursor: Option<u64>) -> CliResult<Page> {
    let page: Page = serde_json::from_value(value).map_err(|error| unreadable("attach", &error))?;
    let count = u64::try_from(page.lines.len()).map_err(|error| {
        refused(
            "launch_managed_cursor_invalid",
            format!("the frame count cannot be represented: {error}"),
        )
    })?;
    if page.session != session {
        return Err(refused(
            "launch_managed_session_changed",
            "the seat's attach page names another session; the launched session is being ended",
        ));
    }
    let valid_cursor = match cursor {
        Some(cursor) => page.cursor.checked_sub(cursor) == Some(count),
        None => page.cursor >= count,
    };
    if !valid_cursor {
        return Err(refused(
            "launch_managed_cursor_invalid",
            "the attach page did not preserve the requested frame cursor",
        ));
    }
    if page.lines.is_empty() && !page.ended {
        return Err(refused(
            "launch_managed_follow_empty",
            "the managed follow answered without frames or a session end",
        ));
    }
    for line in &page.lines {
        if !matches!(
            line.kind.as_str(),
            "user" | "assistant" | "tool_call" | "tool_result" | "status" | "system"
        ) {
            return Err(refused(
                "launch_managed_frame_unknown",
                "the attach page contains an unnamed frame kind",
            ));
        }
    }
    Ok(page)
}

#[derive(Default)]
struct Lines {
    pending: Vec<u8>,
}

const LINE_LIMIT: usize = 1_048_576;

impl Lines {
    fn receive(&mut self, data: &[u8]) -> CliResult<Vec<String>> {
        self.pending.extend_from_slice(data);
        let mut lines = Vec::new();
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            if end > LINE_LIMIT {
                return Err(refused(
                    "launch_managed_input_too_long",
                    "a managed line exceeds one mebibyte; submit shorter lines",
                ));
            }
            // The canonical delimiter is not part of the notice given to the harness.
            let complete: Vec<_> = self.pending.drain(..=end).take(end).collect();
            let text = String::from_utf8(complete).map_err(|error| {
                refused(
                    "launch_managed_input_not_utf8",
                    format!(
                        "managed input must be a complete UTF-8 line: {}",
                        error.utf8_error()
                    ),
                )
            })?;
            if let Some(control) = text
                .chars()
                .find(|character| character.is_control() && *character != '\t')
            {
                return Err(refused(
                    "launch_managed_input_control",
                    format!(
                        "managed input contains control character U+{:04X}; enter plain text",
                        u32::from(control)
                    ),
                ));
            }
            if !text.trim().is_empty() {
                lines.push(text);
            }
        }
        if self.pending.len() > LINE_LIMIT {
            return Err(refused(
                "launch_managed_input_too_long",
                "an unfinished managed line exceeds one mebibyte; end it with a newline before adding more text",
            ));
        }
        Ok(lines)
    }

    fn ended(&self) -> CliResult<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        Err(refused(
            "launch_managed_input_incomplete",
            "terminal input ended before the line's newline; the partial line was not submitted",
        ))
    }
}

async fn interact<T: Transport>(
    transport: Arc<T>,
    session: &str,
    terminal: &Terminal,
    mut pages: Pages,
    signals: &mut Signals,
) -> CliResult<bool> {
    let mut lines = Lines::default();
    loop {
        tokio::select! {
            stopped = signals.stop.received() => return stopped.map(|()| false),
            next = pages.recv() => {
                let page = next.ok_or_else(|| refused("launch_managed_follow_closed", "the managed reader stopped without an end"))??;
                render(&page.lines, &mut std::io::stdout().lock())?;
                if page.ended {
                    return Ok(true);
                }
            }
            data = terminal.read() => {
                let data = data?;
                if data.is_empty() {
                    lines.ended()?;
                    return Ok(false);
                }
                for text in lines.receive(&data)? {
                    type_line(Arc::clone(&transport), session, &text, &mut signals.stop, &mut pages).await?;
                }
            }
        }
    }
}

async fn type_line<T: Transport>(
    transport: Arc<T>,
    session: &str,
    text: &str,
    signals: &mut Stops,
    pages: &mut Pages,
) -> CliResult<()> {
    wait_for_admission(transport, session, text, signals, pages, print_frames).await?;
    let mut out = std::io::stdout().lock();
    writeln!(out, "input admitted to session {session}")
        .and_then(|()| out.flush())
        .map_err(|error| terminal_error("write the managed input admission", &error))
}

async fn wait_for_admission<T, F>(
    transport: Arc<T>,
    session: &str,
    text: &str,
    signals: &mut Stops,
    pages: &mut Pages,
    mut write: F,
) -> CliResult<()>
where
    T: Transport,
    F: FnMut(&[AttachLine]) -> CliResult<()> + Send,
{
    let mut received = request(
        transport,
        format!("/runtime/sessions/{session}/input"),
        json!({"text": text, "enter": true}),
    )?;
    loop {
        tokio::select! {
            stopped = signals.received() => return stopped,
            next = pages.recv() => {
                let page = next.ok_or_else(|| refused("launch_managed_follow_closed", "the managed reader stopped without an end"))??;
                write(&page.lines)?;
                if page.ended {
                    return Err(refused("launch_managed_input_unconfirmed", "the session ended before the submitted line's admission was confirmed; no retry was made"));
                }
            }
            reply = &mut received => {
                let value = reply.map_err(|error| refused("launch_managed_type_closed", format!("the input reply channel closed: {error}")))??;
                admitted(value, session)?;
                return Ok(());
            }
        }
    }
}

fn print_frames(lines: &[AttachLine]) -> CliResult<()> {
    render(lines, &mut std::io::stdout().lock())
}

fn admitted(value: Value, session: &str) -> CliResult<()> {
    if value.get("session").and_then(Value::as_str) != Some(session) {
        return Err(refused(
            "launch_managed_type_answer_mismatch",
            "the input reply envelope did not name the submitted session",
        ));
    }
    delivered(value, session, "managed input")
}

fn render(lines: &[AttachLine], out: &mut impl Write) -> CliResult<()> {
    for line in lines {
        let mut text = String::with_capacity(line.text.len());
        for character in line.text.chars() {
            if character.is_control() && !matches!(character, '\n' | '\t') {
                text.extend(character.escape_default());
            } else {
                text.push(character);
            }
        }
        let written = if line.kind == "assistant" {
            writeln!(out, "{text}")
        } else {
            writeln!(out, "{}: {text}", line.kind)
        };
        written.map_err(|error| terminal_error("write managed terminal frames", &error))?;
    }
    out.flush()
        .map_err(|error| terminal_error("flush managed terminal frames", &error))
}

fn unreadable(verb: &str, error: &serde_json::Error) -> CliError {
    refused(
        "launch_managed_answer_unreadable",
        format!(
            "the server's {verb} reply has no managed receipt shape ({:?})",
            error.classify()
        ),
    )
}

#[cfg(test)]
#[path = "managed_tests.rs"]
mod tests;
