//! The loopback approval receiver validates the callback before exchanging authority.

use std::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::time::{Duration, Instant};
use zeroize::Zeroizing;

use super::errors::source_refused;
use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;

fn decoded(value: &str) -> CliResult<String> {
    let mut bytes = Vec::new();
    let mut input = value.bytes();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let digits: String = input.by_ref().take(2).map(char::from).collect();
            if digits.len() != 2 {
                return Err(refused(
                    "sign_in_callback_invalid",
                    "a callback field is incomplete",
                ));
            }
            bytes.push(u8::from_str_radix(&digits, 16).map_err(|error| {
                source_refused(
                    "sign_in_callback_invalid",
                    "a callback field is malformed",
                    error,
                )
            })?);
        } else {
            bytes.push(if byte == b'+' { b' ' } else { byte });
        }
    }
    String::from_utf8(bytes).map_err(|error| {
        source_refused(
            "sign_in_callback_invalid",
            "a callback field is not UTF-8",
            error,
        )
    })
}

pub(super) fn callback(listener: &TcpListener, expected: &str) -> CliResult<Zeroizing<String>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|error| {
            source_refused(
                "sign_in_callback_unavailable",
                "the approval receiver could not start",
                error,
            )
        })?;
    let socket = listener.try_clone().map_err(|error| {
        source_refused(
            "sign_in_callback_unavailable",
            "the loopback receiver could not be retained",
            error,
        )
    })?;
    socket.set_nonblocking(true).map_err(|error| {
        source_refused(
            "sign_in_callback_unavailable",
            "the loopback receiver could not accept independently",
            error,
        )
    })?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::from_std(socket).map_err(|error| {
            source_refused(
                "sign_in_callback_unavailable",
                "the loopback receiver could not be followed",
                error,
            )
        })?;
        wait(
            listener,
            expected,
            Instant::now() + Duration::from_secs(600),
        )
        .await
    })
}

enum Received {
    Approved(Zeroizing<String>),
    Unrelated,
    Incomplete,
}

async fn wait(
    listener: tokio::net::TcpListener,
    expected: &str,
    until: Instant,
) -> CliResult<Zeroizing<String>> {
    let mut reading = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            () = tokio::time::sleep_until(until) => return Err(refused("sign_in_approval_expired", "the browser approval expired after ten minutes; run `lys sign-in` again and approve its new page")),
            accepted = listener.accept(), if reading.len() < 16 => {
                let (stream, peer) = accepted.map_err(|error| source_refused("sign_in_callback_unavailable", "the loopback receiver could not accept the browser", error))?;
                if !peer.ip().is_loopback() { return Err(refused("sign_in_callback_invalid", "the browser did not arrive on loopback")); }
                let expected = expected.to_owned();
                reading.spawn(async move {
                    match tokio::time::timeout(Duration::from_secs(5), receive(stream, &expected)).await {
                        Ok(result) => result,
                        Err(_) => Ok(Received::Incomplete),
                    }
                });
            }
            Some(answer) = reading.join_next(), if !reading.is_empty() => {
                match answer.map_err(|error| source_refused("sign_in_callback_unavailable", "the browser receiver did not finish", error))?? {
                    Received::Approved(code) => return Ok(code),
                    Received::Unrelated => {},
                    Received::Incomplete => eprintln!("Lys: sign_in_callback_incomplete: a loopback connection sent no whole callback; waiting for browser approval"),
                }
            }
        }
    }
}

async fn receive(mut stream: tokio::net::TcpStream, expected: &str) -> CliResult<Received> {
    let mut request = String::new();
    let mut input = BufReader::new(&mut stream).take(16384);
    input.read_line(&mut request).await.map_err(|error| {
        source_refused(
            "sign_in_callback_invalid",
            "the browser's request could not be read",
            error,
        )
    })?;
    loop {
        let mut header = String::new();
        let size = input.read_line(&mut header).await.map_err(|error| {
            source_refused(
                "sign_in_callback_invalid",
                "the browser's headers could not be read",
                error,
            )
        })?;
        if size == 0 {
            return Err(refused(
                "sign_in_callback_invalid",
                "the browser's headers are incomplete or exceed the callback limit",
            ));
        }
        if header == "\r\n" {
            break;
        }
    }
    drop(input);
    let target = request
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| refused("sign_in_callback_invalid", "the browser sent no callback"))?;
    if !request.starts_with("GET ") || !target.starts_with("/lys/sign-in?") {
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .map_err(|error| {
                source_refused(
                    "sign_in_callback_unavailable",
                    "the browser could not be answered",
                    error,
                )
            })?;
        return Ok(Received::Unrelated);
    }
    let outcome = (|| -> CliResult<Zeroizing<String>> {
        let mut values = std::collections::BTreeMap::new();
        for pair in target.trim_start_matches("/lys/sign-in?").split('&') {
            let (key, value) = pair.split_once('=').ok_or_else(|| {
                refused(
                    "sign_in_callback_invalid",
                    "the callback fields are incomplete",
                )
            })?;
            if values.insert(decoded(key)?, decoded(value)?).is_some() {
                return Err(refused(
                    "sign_in_callback_invalid",
                    "a callback field is repeated",
                ));
            }
        }
        if values.get("state").map(String::as_str) != Some(expected) {
            return Err(refused(
                "sign_in_state_wrong",
                "the callback does not answer this sign-in",
            ));
        }
        if values.contains_key("error") {
            let reason = values
                .get("error_description")
                .map_or("the person did not approve this terminal", String::as_str);
            return Err(refused("sign_in_refused", reason));
        }
        let code = Zeroizing::new(
            values
                .remove("code")
                .filter(|code| !code.is_empty())
                .ok_or_else(|| {
                    refused(
                        "sign_in_callback_invalid",
                        "the callback supplied no authorization answer",
                    )
                })?,
        );
        Ok(code)
    })();
    let code = match outcome {
        Ok(code) => code,
        Err(error) => {
            let words = error.to_string();
            let name = words
                .split(':')
                .next()
                .filter(|name| name.starts_with("sign_in_"))
                .ok_or_else(|| {
                    refused(
                        "sign_in_callback_invalid",
                        "the receiver refusal has no name",
                    )
                })?;
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{name}",
                name.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .map_err(|error| {
                    source_refused(
                        "sign_in_callback_unavailable",
                        "the receiver refusal could not be answered",
                        error,
                    )
                })?;
            return Err(error);
        }
    };
    let body = "Approval received. Return to your terminal to see whether sign-in completed.";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|error| {
            source_refused(
                "sign_in_callback_unavailable",
                "the browser could not be answered",
                error,
            )
        })?;
    Ok(Received::Approved(code))
}

#[cfg(test)]
#[path = "callback_tests.rs"]
mod tests;
