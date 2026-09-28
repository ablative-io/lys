//! The progress page's server: the screens package's own page at
//! `/install`, its files, the board's phases as a stream of server-sent
//! events at `/install/events`, and "Try again" at `/install/retry`.
//!
//! It listens on a loopback port the app chooses, answers only requests
//! whose `Host` names that loopback address, and serves files only from the
//! screens package. Each connection is served on its own thread; the event
//! stream writes the board's phase each time the board changes, blocked on
//! the board between changes, so nothing is asked again on a schedule.
//!
//! Invariants: no file outside the screens package is ever served (a path
//! with any part that is not a plain name is refused); a request from any
//! other host is refused, so a page elsewhere cannot drive the app; and a
//! missing screens package is shown as a named refusal in plain words,
//! never as an empty page.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::progress::Board;

/// The page's own address.
pub const PAGE: &str = "/install";

/// One request's line and the headers the server reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// `GET`, `POST`, ….
    pub method: String,
    /// The path, without its query.
    pub path: String,
    /// The `Host` header, when there is one.
    pub host: Option<String>,
}

/// Reads a request's line and headers from `reader`.
pub fn read_request(reader: &mut dyn BufRead) -> Option<Request> {
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let path = target
        .split(['?', '#'])
        .next()
        .unwrap_or(target)
        .to_string();
    let mut host = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let named = header
            .split_once(':')
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case("host"));
        if let Some((_, value)) = named {
            host = Some(value.trim().to_string());
        }
    }
    Some(Request { method, path, host })
}

/// Whether `host` names the loopback address the app listens on at `port`.
pub fn host_allowed(host: Option<&str>, port: u16) -> bool {
    host.is_some_and(|host| {
        host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}")
    })
}

/// The file under `root` that `path` names, when every part of it is a
/// plain name.
pub fn file_under(root: &Path, path: &str) -> Option<PathBuf> {
    let relative = Path::new(path.trim_start_matches('/'));
    let plain = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    if !plain || relative.as_os_str().is_empty() {
        return None;
    }
    Some(root.join(relative))
}

/// The content type of a file served, by its extension.
pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// The page shown when this copy of the app carries no screens.
pub const NO_SCREENS: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Lys</title></head><body style=\"font-family:system-ui;margin:3rem;\">\
<h1>Lys cannot show its screens</h1><p>This copy of Lys is missing its screens \
(screens_missing). Download Lys again, drag it to Applications and open it from there.</p>\
</body></html>";

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

/// Streams the board's phases to `stream` until the page goes away, the
/// board's view written each time it changes.
fn stream_events(stream: &mut TcpStream, board: &Board) -> std::io::Result<()> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
          Connection: keep-alive\r\n\r\n",
    )?;
    let mut seen = 0;
    loop {
        let view = board.next_after(seen);
        let data = serde_json::to_string(&view).map_err(std::io::Error::other)?;
        write!(stream, "data: {data}\n\n")?;
        stream.flush()?;
        board.delivered(view.version);
        seen = view.version;
    }
}

/// Answers "Try again": accepted when the work has failed and can run
/// again, refused by name otherwise.
fn retry(stream: &mut TcpStream, board: &Board) -> std::io::Result<()> {
    let json = "application/json";
    if board.ask_retry() {
        respond(stream, "202 Accepted", json, b"{\"retry\":true}")
    } else {
        let body = b"{\"refusal\":\"nothing_to_retry\"}";
        respond(stream, "409 Conflict", json, body)
    }
}

/// Answers the page itself: the screens package's page, or the named
/// refusal when this copy carries no screens.
fn page(stream: &mut TcpStream, screens: &Path) -> std::io::Result<()> {
    let html = "text/html; charset=utf-8";
    match std::fs::read(screens.join("index.html")) {
        Ok(page) => respond(stream, "200 OK", html, &page),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            respond(stream, "200 OK", html, NO_SCREENS.as_bytes())
        }
        Err(error) => {
            let body = format!("screens_unreadable: {error}");
            respond(
                stream,
                "500 Internal Server Error",
                "text/plain",
                body.as_bytes(),
            )
        }
    }
}

/// Answers a file of the screens package, `not_found` when there is none,
/// or `screen_unreadable` naming why a file that is there could not be read.
fn file(stream: &mut TcpStream, screens: &Path, path: &str) -> std::io::Result<()> {
    let Some(file) = file_under(screens, path) else {
        return respond(stream, "404 Not Found", "text/plain", b"not_found");
    };
    match std::fs::read(&file) {
        Ok(body) => respond(stream, "200 OK", content_type(&file), &body),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            respond(stream, "404 Not Found", "text/plain", b"not_found")
        }
        Err(error) => {
            let body = format!("screen_unreadable: {path}: {error}");
            respond(
                stream,
                "500 Internal Server Error",
                "text/plain",
                body.as_bytes(),
            )
        }
    }
}

/// Serves one connection.
fn serve(stream: &mut TcpStream, port: u16, board: &Board, screens: &Path) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let Some(request) = read_request(&mut reader) else {
        return respond(stream, "400 Bad Request", "text/plain", b"bad_request");
    };
    if !host_allowed(request.host.as_deref(), port) {
        return respond(stream, "403 Forbidden", "text/plain", b"host_refused");
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let moved = format!(
                "HTTP/1.1 303 See Other\r\nLocation: {PAGE}\r\nContent-Length: 0\r\n\
                 Connection: close\r\n\r\n"
            );
            stream.write_all(moved.as_bytes())
        }
        ("GET", "/install/events") => stream_events(stream, board),
        ("POST", "/install/retry") => retry(stream, board),
        ("GET", PAGE) => page(stream, screens),
        ("GET", path) => file(stream, screens, path),
        _ => respond(
            stream,
            "405 Method Not Allowed",
            "text/plain",
            b"method_refused",
        ),
    }
}

/// Serves the page from `listener` on its own threads, one per connection.
/// A connection that fails, as a page closed mid-stream does, is written to
/// the app's log.
pub fn spawn(listener: TcpListener, board: Arc<Board>, screens: PathBuf) -> std::io::Result<()> {
    let port = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(stream) => stream,
                Err(error) => {
                    eprintln!("page connection not accepted: {error}");
                    continue;
                }
            };
            let board = Arc::clone(&board);
            let screens = screens.clone();
            std::thread::spawn(move || {
                if let Err(error) = serve(&mut stream, port, &board, &screens) {
                    eprintln!("page connection ended: {error}");
                }
            });
        }
    });
    Ok(())
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
