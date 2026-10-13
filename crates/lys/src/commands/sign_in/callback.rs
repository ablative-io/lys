//! The loopback approval receiver validates the callback before exchanging authority.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
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
    loop {
        let (mut stream, peer) = listener.accept().map_err(|error| {
            source_refused(
                "sign_in_callback_unavailable",
                "the loopback receiver could not accept the browser",
                error,
            )
        })?;
        if !peer.ip().is_loopback() {
            return Err(refused(
                "sign_in_callback_invalid",
                "the browser did not arrive on loopback",
            ));
        }
        let mut request = String::new();
        let mut input = BufReader::new(&mut stream).take(16384);
        input.read_line(&mut request).map_err(|error| {
            source_refused(
                "sign_in_callback_invalid",
                "the browser's request could not be read",
                error,
            )
        })?;
        loop {
            let mut header = String::new();
            let size = input.read_line(&mut header).map_err(|error| {
                source_refused(
                    "sign_in_callback_invalid",
                    "the browser's headers could not be read",
                    error,
                )
            })?;
            if size == 0 {
                return Err(refused(
                    "sign_in_callback_invalid",
                    "the browser's headers are incomplete",
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
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .map_err(|error| {
                    source_refused(
                        "sign_in_callback_unavailable",
                        "the browser could not be answered",
                        error,
                    )
                })?;
            continue;
        }
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
            return Err(refused(
                "sign_in_refused",
                "the person did not approve this terminal",
            ));
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
        let body = b"Signed in to Lys. You can close this tab.";
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len())
            .and_then(|()| stream.write_all(body)).map_err(|error| source_refused("sign_in_callback_unavailable", "the browser could not be answered", error))?;
        return Ok(code);
    }
}
