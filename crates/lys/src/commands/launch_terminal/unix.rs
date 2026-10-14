//! Terminal readiness and process signals wake the bridge without clock polling.

use std::fs::{File, OpenOptions};
use std::io::{IsTerminal, Write};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use rustix::termios::{Action, Termios, tcgetattr, tcgetpgrp, tcgetwinsize, tcsetattr};
use serde_json::{Value, json};
use tokio::io::unix::AsyncFd;
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::sync::{mpsc, oneshot};

use super::{Transport, bytes, delivered, end, exit, finish};
use crate::commands::error::{CliError, CliResult};
use crate::commands::seat_client::{Server, refused};

/// A validated controlling terminal whose original attributes are restored after launch.
pub struct Terminal {
    input: AsyncFd<File>,
    original: Termios,
    raw: bool,
    columns: u16,
    rows: u16,
    runtime: Option<tokio::runtime::Runtime>,
    signals: Option<Signals>,
}

impl Terminal {
    /// The dimensions validated before the server starts its process.
    #[must_use]
    pub fn size(&self) -> (u16, u16) {
        (self.columns, self.rows)
    }

    fn activate(&mut self) -> CliResult<()> {
        let mut raw = self.original.clone();
        raw.make_raw();
        tcsetattr(self.input.get_ref(), Action::Now, &raw)
            .map_err(|error| terminal("set raw mode", error))?;
        self.raw = true;
        Ok(())
    }

    pub(super) fn restore(&mut self) -> CliResult<()> {
        if self.raw {
            tcsetattr(self.input.get_ref(), Action::Now, &self.original)
                .map_err(|error| terminal("restore terminal attributes", error))?;
            self.raw = false;
        }
        Ok(())
    }

    fn dimensions(&self) -> CliResult<(u16, u16)> {
        let size = tcgetwinsize(self.input.get_ref())
            .map_err(|error| terminal("read terminal size", error))?;
        dimensions(size.ws_col, size.ws_row)
    }

    pub(super) fn canonical(&self) -> CliResult<()> {
        let attributes = tcgetattr(self.input.get_ref())
            .map_err(|error| terminal("read managed terminal attributes", error))?;
        let required = rustix::termios::LocalModes::ICANON
            | rustix::termios::LocalModes::ECHO
            | rustix::termios::LocalModes::ISIG;
        if !attributes.local_modes.contains(required) {
            return Err(refused(
                "launch_managed_terminal_mode_required",
                "managed input requires canonical lines, echo and interrupt signals; run from a normal shell terminal",
            ));
        }
        Ok(())
    }

    pub(super) fn events(&mut self) -> CliResult<(tokio::runtime::Runtime, Signals)> {
        let runtime = self.runtime.take().ok_or_else(|| {
            refused(
                "launch_terminal_runtime_missing",
                "the prepared terminal has no event runtime",
            )
        })?;
        let signals = self.signals.take().ok_or_else(|| {
            refused(
                "launch_terminal_signals_missing",
                "the prepared terminal has no signal handlers",
            )
        })?;
        Ok((runtime, signals))
    }

    pub(super) async fn read(&self) -> CliResult<Vec<u8>> {
        input_bytes(&self.input).await
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            eprintln!("{error}");
        }
    }
}

/// Validate terminal input, output and dimensions before any server start request.
///
/// # Errors
/// Refuses missing terminals and absent dimensions without inventing defaults.
pub fn prepare() -> CliResult<Terminal> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    if !stdin.is_terminal() || !stdout.is_terminal() {
        return Err(refused(
            "launch_terminal_required",
            "Lys launch requires terminal input and output; run the command in a terminal",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|error| terminal_error("open the controlling terminal", error))?;
    let held = tcgetpgrp(&file)
        .map_err(|error| terminal("identify the terminal's foreground group", error))?;
    if held != rustix::process::getpgrp() {
        return Err(refused(
            "launch_terminal_background",
            "Lys launch must own its terminal's foreground process group",
        ));
    }
    for actual in [tcgetpgrp(&stdin), tcgetpgrp(&stdout)] {
        let actual = actual.map_err(|error| terminal("identify terminal streams", error))?;
        if actual != held {
            return Err(refused(
                "launch_terminal_mismatch",
                "input and output must use this process's controlling terminal",
            ));
        }
    }
    let original = tcgetattr(&file).map_err(|error| terminal("read terminal attributes", error))?;
    let size = tcgetwinsize(&file).map_err(|error| terminal("read terminal size", error))?;
    let (columns, rows) = dimensions(size.ws_col, size.ws_row)?;
    let flags = rustix::fs::fcntl_getfl(&file)
        .map_err(|error| terminal("read terminal descriptor flags", error))?;
    // A separately opened descriptor avoids changing the shell's shared descriptor flags.
    rustix::fs::fcntl_setfl(&file, flags | rustix::fs::OFlags::NONBLOCK)
        .map_err(|error| terminal("make terminal reads nonblocking", error))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| terminal_error("create the terminal event runtime", error))?;
    let (input, signals) = runtime.block_on(async {
        let signals = Signals::new()?;
        let input =
            AsyncFd::new(file).map_err(|error| terminal_error("watch terminal input", error))?;
        Ok::<_, CliError>((input, signals))
    })?;
    Ok(Terminal {
        input,
        original,
        raw: false,
        columns,
        rows,
        runtime: Some(runtime),
        signals: Some(signals),
    })
}

fn dimensions(columns: u16, rows: u16) -> CliResult<(u16, u16)> {
    if columns == 0 || rows == 0 {
        return Err(refused(
            "launch_terminal_size_missing",
            "the terminal has no columns or rows; open a terminal with a known window size",
        ));
    }
    Ok((columns, rows))
}

pub(super) fn attach(server: &Server, session: &str, mut terminal: Terminal) -> CliResult<()> {
    let mut follower = None;
    let outcome = (|| {
        let control = Arc::new(server.separate()?);
        let read = Arc::new(server.separate()?);
        let (runtime, mut signals) = terminal.events()?;
        runtime.block_on(async {
            terminal.activate()?;
            let (pages, handle) = follow(read, session)?;
            follower = Some(handle);
            interact(control, session, &terminal, pages, &mut signals).await
        })
    })();
    let restored = terminal.restore();
    let ended = end(server, session);
    if ended.is_ok() {
        if let Some(handle) = follower {
            if handle.join().is_err() {
                return finish(
                    outcome,
                    Err(refused(
                        "launch_terminal_follow_panicked",
                        "the terminal reader panicked",
                    )),
                    restored,
                );
            }
        }
    }
    finish(outcome, ended, restored)
}

type Pages = mpsc::Receiver<CliResult<lys_runner::terminal_bytes::ByteOutput>>;

fn follow<T: Transport>(transport: Arc<T>, session: &str) -> CliResult<(Pages, JoinHandle<()>)> {
    let (sent, pages) = mpsc::channel(1);
    let session = session.to_owned();
    let route = format!("/runtime/sessions/{session}/read-bytes");
    let handle = thread::Builder::new()
        .name("lys-terminal-output".to_owned())
        .spawn(move || {
            let mut cursor = None;
            loop {
                let output = transport
                    .post(&route, &json!({"cursor": cursor, "follow": true}))
                    .and_then(|value| bytes(value, &session, cursor));
                let last = match &output {
                    Ok(output) => output.ended.is_some(),
                    Err(_) => true,
                };
                if let Ok(output) = &output {
                    cursor = Some(output.cursor);
                }
                if let Err(abandoned) = sent.blocking_send(output) {
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
        .map_err(|error| terminal_error("start the terminal reader", error))?;
    Ok((pages, handle))
}

pub(super) struct Signals {
    pub(super) stop: Stops,
    resize: Signal,
}

pub(super) struct Stops {
    interrupt: Signal,
    terminate: Signal,
    hangup: Signal,
}

impl Signals {
    pub(super) fn new() -> CliResult<Self> {
        let watch =
            |kind| signal(kind).map_err(|error| terminal_error("watch terminal signals", error));
        Ok(Self {
            stop: Stops {
                interrupt: watch(SignalKind::interrupt())?,
                terminate: watch(SignalKind::terminate())?,
                hangup: watch(SignalKind::hangup())?,
            },
            resize: watch(SignalKind::window_change())?,
        })
    }
}

impl Stops {
    pub(super) async fn received(&mut self) -> CliResult<()> {
        let (name, received) = tokio::select! {
            received = self.interrupt.recv() => ("SIGINT", received),
            received = self.terminate.recv() => ("SIGTERM", received),
            received = self.hangup.recv() => ("SIGHUP", received),
        };
        match received {
            Some(()) => Err(refused(
                "launch_terminal_interrupted",
                format!("received {name}; ending the launched session"),
            )),
            None => Err(refused(
                "launch_terminal_signal_closed",
                format!("the {name} signal stream closed"),
            )),
        }
    }
}

async fn interact<T: Transport>(
    transport: Arc<T>,
    session: &str,
    terminal: &Terminal,
    mut pages: Pages,
    signals: &mut Signals,
) -> CliResult<()> {
    let resize = format!("/runtime/sessions/{session}/resize");
    let (columns, rows) = terminal.size();
    let resized = operate(
        Arc::clone(&transport),
        &resize,
        json!({"columns": columns, "rows": rows}),
        session,
        "resize",
        &mut signals.stop,
    )
    .await;
    if finish_resize(resized, &mut pages, &mut signals.stop, print_bytes).await? {
        return Ok(());
    }
    loop {
        tokio::select! {
            stopped = signals.stop.received() => return stopped,
            resized = signals.resize.recv() => {
                resized.ok_or_else(|| refused("launch_terminal_signal_closed", "the resize signal stream closed"))?;
                let (columns, rows) = terminal.dimensions()?;
                let resized = operate(Arc::clone(&transport), &resize, json!({"columns": columns, "rows": rows}), session, "resize", &mut signals.stop).await;
                if finish_resize(resized, &mut pages, &mut signals.stop, print_bytes).await? {
                    return Ok(());
                }
            }
            page = pages.recv() => {
                let output = page.ok_or_else(|| refused("launch_terminal_follow_closed", "the terminal reader stopped without an end"))??;
                print_bytes(&output.data)?;
                if let Some(ended) = output.ended {
                    return exit(&ended);
                }
            }
            data = input_bytes(&terminal.input) => {
                let data = data?;
                if data.is_empty() {
                    return Ok(());
                }
                let route = format!("/runtime/sessions/{session}/input-bytes");
                operate(Arc::clone(&transport), &route, json!({"data": data}), session, "input-bytes", &mut signals.stop).await?;
            }
        }
    }
}

async fn finish_resize<F>(
    resized: CliResult<()>,
    pages: &mut Pages,
    signals: &mut Stops,
    mut write: F,
) -> CliResult<bool>
where
    F: FnMut(&[u8]) -> CliResult<()> + Send,
{
    let Err(error) = resized else {
        return Ok(false);
    };
    if !matches!(&error, CliError::Runner(error) if error.name() == "session_ended") {
        return Err(error);
    }
    eprintln!("{error}; the process ended before resize; inspect with `lys agents`");
    loop {
        tokio::select! {
            stopped = signals.received() => return stopped.map(|()| false),
            next = pages.recv() => {
                let output = next.ok_or_else(|| refused("launch_terminal_follow_closed", "the terminal reader stopped before giving final output and exit evidence"))??;
                write(&output.data)?;
                if let Some(ended) = output.ended {
                    exit(&ended)?;
                    return Ok(true);
                }
            }
        }
    }
}

fn print_bytes(data: &[u8]) -> CliResult<()> {
    let mut out = std::io::stdout().lock();
    out.write_all(data)
        .and_then(|()| out.flush())
        .map_err(|error| terminal_error("write agent terminal output", error))
}

async fn input_bytes(input: &AsyncFd<File>) -> CliResult<Vec<u8>> {
    loop {
        let mut ready = input
            .readable()
            .await
            .map_err(|error| terminal_error("wait for terminal input", error))?;
        let mut buffer = [0_u8; 4096];
        match ready.try_io(|input| {
            rustix::io::read(input.get_ref(), &mut buffer).map_err(std::io::Error::from)
        }) {
            Ok(Ok(length)) => return Ok(buffer[..length].to_vec()),
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Ok(Err(error)) => return Err(terminal_error("read terminal input", error)),
            Err(_) => {}
        }
    }
}

async fn operate<T: Transport>(
    transport: Arc<T>,
    route: &str,
    body: Value,
    session: &str,
    verb: &str,
    signals: &mut Stops,
) -> CliResult<()> {
    let request = request(transport, route.to_owned(), body)?;
    tokio::select! {
        stopped = signals.received() => stopped,
        reply = request => {
            let value = reply.map_err(|error| refused("launch_terminal_control_closed", format!("the {verb} reply channel closed: {error}")))??;
            delivered(value, session, verb)
        }
    }
}

pub(super) fn request<T: Transport>(
    transport: Arc<T>,
    route: String,
    body: Value,
) -> CliResult<oneshot::Receiver<CliResult<Value>>> {
    let (sent, received) = oneshot::channel();
    thread::Builder::new()
        .name("lys-terminal-control".to_owned())
        .spawn(move || {
            if let Err(answer) = sent.send(transport.post(&route, &body)) {
                if let Err(error) = answer {
                    eprintln!("{error}");
                }
            }
        })
        .map_err(|error| terminal_error("start the terminal control request", error))?;
    Ok(received)
}

fn terminal(operation: &str, error: rustix::io::Errno) -> CliError {
    terminal_error(operation, error.into())
}

pub(super) fn terminal_error(operation: &str, error: std::io::Error) -> CliError {
    refused(
        "launch_terminal_io_failed",
        format!("cannot {operation}: {error}"),
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
