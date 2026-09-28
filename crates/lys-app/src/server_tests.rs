#![cfg(test)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::progress::{Phase, Work, steps};
use crate::refusal::Refusal;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_request_is_read_to_its_method_path_and_host() {
    let text = "GET /install?x=1 HTTP/1.1\r\nhost: 127.0.0.1:5000\r\nAccept: */*\r\n\r\n";
    let request = read_request(&mut text.as_bytes());
    assert_eq!(
        request,
        Some(Request {
            method: "GET".to_string(),
            path: "/install".to_string(),
            host: Some("127.0.0.1:5000".to_string()),
        })
    );
    assert_eq!(read_request(&mut &b""[..]), None);
}

#[test]
fn only_the_apps_own_loopback_host_is_answered() {
    assert!(host_allowed(Some("127.0.0.1:5000"), 5000));
    assert!(host_allowed(Some("localhost:5000"), 5000));
    assert!(!host_allowed(Some("127.0.0.1:5001"), 5000));
    assert!(!host_allowed(Some("attacker.example:5000"), 5000));
    assert!(!host_allowed(None, 5000));
}

#[test]
fn no_file_outside_the_screens_is_named() {
    let root = Path::new("/screens");
    assert_eq!(
        file_under(root, "/assets/index.js"),
        Some(root.join("assets/index.js"))
    );
    for refused in [
        "/../etc/passwd",
        "/assets/../../x",
        "/",
        "",
        "/./index.html",
    ] {
        assert_eq!(file_under(root, refused), None, "{refused}");
    }
}

#[test]
fn files_are_served_with_their_type() {
    assert_eq!(
        content_type(Path::new("a.js")),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(content_type(Path::new("a.css")), "text/css; charset=utf-8");
    assert_eq!(
        content_type(Path::new("a.html")),
        "text/html; charset=utf-8"
    );
    assert_eq!(content_type(Path::new("a.bin")), "application/octet-stream");
}

/// A running server on its own loopback port, over `screens`.
fn running(screens: &Path) -> Result<(u16, Arc<Board>), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    let board = Arc::new(Board::new());
    spawn(listener, Arc::clone(&board), screens.to_path_buf())?;
    Ok((port, board))
}

/// Sends `method path` with `host` and reads the whole answer.
fn ask(port: u16, method: &str, path: &str, host: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: {host}\r\n\r\n")?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer)?;
    Ok(answer)
}

#[test]
fn the_page_is_the_screens_page_and_another_host_is_refused() -> TestResult {
    let screens = tempfile::tempdir()?;
    std::fs::write(screens.path().join("index.html"), "<div id=root></div>")?;
    let (port, _board) = running(screens.path())?;
    let host = format!("127.0.0.1:{port}");
    let page = ask(port, "GET", "/install", &host)?;
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.ends_with("<div id=root></div>"), "{page}");
    let moved = ask(port, "GET", "/", &host)?;
    assert!(moved.contains("Location: /install"), "{moved}");
    let refused = ask(port, "GET", "/install", "evil.example")?;
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
    assert!(refused.ends_with("host_refused"), "{refused}");
    let missing = ask(port, "GET", "/assets/none.js", &host)?;
    assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");
    Ok(())
}

#[test]
fn a_copy_without_screens_says_so_by_name() -> TestResult {
    let empty = tempfile::tempdir()?;
    let (port, _board) = running(empty.path())?;
    let page = ask(port, "GET", "/install", &format!("127.0.0.1:{port}"))?;
    assert!(page.contains("screens_missing"), "{page}");
    assert!(page.contains("Download Lys again"), "{page}");
    Ok(())
}

/// Reads one event's data line from an event stream.
fn next_event(lines: &mut dyn BufRead) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    loop {
        let mut line = String::new();
        if lines.read_line(&mut line)? == 0 {
            return Err("the stream ended".into());
        }
        if let Some(data) = line.strip_prefix("data: ") {
            return Ok(serde_json::from_str(data.trim_end())?);
        }
    }
}

/// The stream sends the phase now and then each change as it is shown;
/// "Try again" is refused until the work has failed, and accepted then.
#[test]
fn the_page_hears_each_phase_as_it_is_shown_and_retries_only_a_failure() -> TestResult {
    let screens = tempfile::tempdir()?;
    let (port, board) = running(screens.path())?;
    let host = format!("127.0.0.1:{port}");
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    write!(
        stream,
        "GET /install/events HTTP/1.1\r\nHost: {host}\r\n\r\n"
    )?;
    let mut events = BufReader::new(stream);
    assert_eq!(next_event(&mut events)?["phase"], "opening");
    board.show(Phase::Working {
        work: Work::Install,
        title: Work::Install.title(),
        steps: steps(None),
        said: Vec::new(),
    });
    let working = next_event(&mut events)?;
    assert_eq!(working["phase"], "working");
    assert_eq!(working["title"], "Installing Lys");
    let early = ask(port, "POST", "/install/retry", &host)?;
    assert!(early.contains("nothing_to_retry"), "{early}");
    let refusal = Refusal::new(
        "install_step_failed",
        "Starting sign-in did not finish.",
        "Press Try again.",
        "detail",
    );
    board.show(Phase::failed(&refusal, true, steps(None)));
    let failed = next_event(&mut events)?;
    assert_eq!(failed["phase"], "failed");
    assert_eq!(failed["words"], "Starting sign-in did not finish.");
    assert!(failed.get("detail").is_none(), "{failed}");
    let accepted = ask(port, "POST", "/install/retry", &host)?;
    assert!(accepted.starts_with("HTTP/1.1 202"), "{accepted}");
    board.wait_retry();
    Ok(())
}
