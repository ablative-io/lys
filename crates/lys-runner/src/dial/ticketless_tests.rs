//! A long poll answered without a ticket is a lost connection, never the end of the dial.

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

/// Serve `answers` connections: the epoch on a GET, and on a POST a page
/// with no ticket, as a restarting server or a proxy in front of Lys answers.
fn ticketless(tcp: TcpListener, answers: usize) -> std::thread::JoinHandle<Result<(), String>> {
    std::thread::spawn(move || {
        for _ in 0..answers {
            let (stream, _) = tcp.accept().map_err(|error| error.to_string())?;
            let mut stream = BufReader::new(stream);
            let mut first = String::new();
            stream
                .read_line(&mut first)
                .map_err(|error| error.to_string())?;
            let mut length = 0;
            loop {
                let mut line = String::new();
                stream
                    .read_line(&mut line)
                    .map_err(|error| error.to_string())?;
                if line == "\r\n" {
                    break;
                }
                if line.is_empty() {
                    return Err("fixture request ended before its head".to_owned());
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value
                        .trim()
                        .parse::<usize>()
                        .map_err(|error| error.to_string())?;
                }
            }
            let mut body = vec![0; length];
            stream
                .read_exact(&mut body)
                .map_err(|error| error.to_string())?;
            let (kind, body) = if first.starts_with("GET ") {
                ("text/plain", "fixture-epoch")
            } else {
                ("text/html", "<html>the origin is restarting</html>")
            };
            write!(
                stream.get_mut(),
                "HTTP/1.1 200 Fixture\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .map_err(|error| error.to_string())?;
            stream
                .get_mut()
                .flush()
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
}

#[test]
fn a_ticketless_answer_is_a_lost_connection_that_names_what_answered() -> Result<(), Box<dyn Error>>
{
    let tcp = TcpListener::bind("127.0.0.1:0")?;
    let server = format!("http://{}", tcp.local_addr()?);
    let serving = ticketless(tcp, 2);
    let dial = super::Dial {
        server,
        machine: "machine".to_owned(),
        socket: std::path::PathBuf::from("unused.sock"),
        key: Arc::new(lys_core::Ed25519Identity::ephemeral()),
        authority: None,
    };
    let answer = dial.next("greeting");
    serving
        .join()
        .map_err(|panic| format!("fixture panicked: {panic:?}"))??;
    match answer {
        Err(crate::RunnerError::Unreachable { reason }) => {
            assert!(reason.contains("HTTP 200 without a ticket"), "{reason}");
            assert!(reason.contains("text/html"), "{reason}");
            Ok(())
        }
        Err(other) => Err(format!("a ticketless answer ended the dial: {other}").into()),
        Ok((ticket, _)) => Err(format!("a ticketless answer was read as ticket {ticket}").into()),
    }
}
