//! The door's handle records, read over HTTP: the one production
//! [`HandleRecords`] a start's credentials check reads through.
//!
//! SECRETS-002 R1 lands the handle-resolving endpoint in the door (its
//! secrets-handle route, in the door repository, door-owned). This client
//! reads it as `GET /secrets/handles?holder=<agent>` and takes from the
//! answer ids and validity only: an answer is
//! `{"holder": <agent>, "handles": [{"id": <id>, "state": <state>}]}`, a
//! handle whose state is `active` is valid and any other state is not, and a
//! `404` says the agent has no handle record.
//!
//! A handle carrying any member beside its id and its state may be carrying
//! a credential value, so the whole answer is refused as
//! `credential_value_in_answer`, naming the handle and the member and never
//! what it held, and no id is taken from it. Nothing this client returns,
//! prints or logs holds a byte of the answer's body beyond ids, states and
//! member names: a failure is named by its status or its step, never by the
//! body.
//!
//! The door is reached over plain HTTP on the address it is given, one
//! request to a connection. The client waits on the door for as long as the
//! door takes; a door that never answers is found by its signal, not a clock.

use std::io::{Read, Write};
use std::net::TcpStream;

use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use serde_json::Value;

/// The path of the door's handle-resolving endpoint, as this client reads it
/// from SECRETS-002 R1.
pub const HANDLES_PATH: &str = "/secrets/handles";

/// A handle's state that makes it valid.
const VALID: &str = "active";

/// Whether `text` is a credential id this client admits onto a command line:
/// one to 128 ASCII letters, digits, `-`, `_` and `.`.
pub fn credential_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 128
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// The door's handle records, at the address the door serves on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorHandles {
    door: Option<(String, u16)>,
}

impl DoorHandles {
    /// A client of the door at `address`, `http://host:port` or `host:port`.
    pub fn at(address: &str) -> Result<Self, String> {
        let authority = address
            .strip_prefix("http://")
            .unwrap_or(address)
            .trim_end_matches('/');
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or_else(|| format!("the door's address {address} names no port"))?;
        let port = port
            .parse::<u16>()
            .map_err(|_unread| format!("the door's address {address} names no port"))?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        if host.is_empty() {
            return Err(format!("the door's address {address} names no host"));
        }
        Ok(Self {
            door: Some((host.to_owned(), port)),
        })
    }

    /// A client no door address is configured for: every agent is answered
    /// as having no handle record, because none can be read here.
    pub fn unconfigured() -> Self {
        Self { door: None }
    }
}

/// The door's answer for `agent`: its status and its body.
fn get(host: &str, port: u16, agent: &str) -> Result<(u16, Vec<u8>), String> {
    let mut stream = TcpStream::connect((host, port))
        .map_err(|error| format!("the door at {host}:{port} could not be reached: {error}"))?;
    let request = format!(
        "GET {HANDLES_PATH}?holder={} HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        query_value(agent)
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|error| format!("the request to the door was not sent: {error}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|error| format!("the door's answer was not read: {error}"))?;
    response(&raw)
}

impl HandleRecords for DoorHandles {
    fn handles(&self, agent: &str) -> HandleAnswer {
        let Some((host, port)) = &self.door else {
            tracing::warn!(
                agent,
                "no door address is configured, so no handle record can be read"
            );
            return HandleAnswer::RecordMissing;
        };
        match get(host, *port, agent) {
            Ok((200, body)) => read_answer(&body),
            Ok((404, _)) => HandleAnswer::RecordMissing,
            Ok((status, _)) => HandleAnswer::Unreadable {
                reason: format!("the door answered status {status}"),
            },
            Err(reason) => HandleAnswer::Unreadable { reason },
        }
    }
}

/// Read ids and states from the door's answer, refusing one that carries a
/// member beside them.
fn read_answer(body: &[u8]) -> HandleAnswer {
    let unreadable = |reason: &str| HandleAnswer::Unreadable {
        reason: format!("the door's handle record {reason}"),
    };
    let Ok(Value::Object(answer)) = serde_json::from_slice::<Value>(body) else {
        return unreadable("is not a JSON object");
    };
    if let Some(member) = answer
        .keys()
        .find(|member| !matches!(member.as_str(), "holder" | "handles"))
    {
        return HandleAnswer::ValueInAnswer {
            record: "the handle record".to_owned(),
            field: member.clone(),
        };
    }
    let Some(Value::Array(handles)) = answer.get("handles") else {
        return unreadable("holds no handles list");
    };
    let mut held = Vec::with_capacity(handles.len());
    for handle in handles {
        let Value::Object(members) = handle else {
            return unreadable("holds a handle that is not an object");
        };
        let Some(Value::String(id)) = members.get("id") else {
            return unreadable("holds a handle with no id");
        };
        if let Some(member) = members
            .keys()
            .find(|member| !matches!(member.as_str(), "id" | "state"))
        {
            return HandleAnswer::ValueInAnswer {
                record: id.clone(),
                field: member.clone(),
            };
        }
        let Some(Value::String(state)) = members.get("state") else {
            return unreadable("holds a handle with no state");
        };
        held.push(HeldCredential {
            id: id.clone(),
            valid: state == VALID,
        });
    }
    HandleAnswer::Held(held)
}

/// `text` percent-encoded for a query value: every byte but an unreserved one.
fn query_value(text: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(DIGITS[usize::from(byte >> 4)]));
            out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
    }
    out
}

/// The status and the body of a whole HTTP/1.x response, de-chunked.
fn response(raw: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let cut = || "the door's answer is not a whole HTTP response".to_owned();
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(cut)?;
    let head = std::str::from_utf8(raw.get(..split).ok_or_else(cut)?).map_err(|_text| cut())?;
    let rest = raw.get(split + 4..).ok_or_else(cut)?;
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(cut)?;
    let mut chunked = false;
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(cut)?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") && value.eq_ignore_ascii_case("chunked") {
            chunked = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            length = Some(value.parse::<usize>().map_err(|_length| cut())?);
        }
    }
    let body = if chunked {
        dechunk(rest).ok_or_else(cut)?
    } else if let Some(length) = length {
        rest.get(..length).ok_or_else(cut)?.to_vec()
    } else {
        rest.to_vec()
    };
    Ok((status, body))
}

fn dechunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = rest.windows(2).position(|window| window == b"\r\n")?;
        let size_text = std::str::from_utf8(rest.get(..line_end)?).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        rest = rest.get(line_end + 2..)?;
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

/// What the example prints for `agent`: the count of credentials and each
/// id with its validity, or the answer's refusal by name. Never a value.
pub fn report(handles: &DoorHandles, agent: &str) -> String {
    match handles.handles(agent) {
        HandleAnswer::Held(held) => {
            let mut lines = vec![format!("credentials: {}", held.len())];
            for credential in held {
                let validity = if credential.valid {
                    "valid"
                } else {
                    "not valid"
                };
                lines.push(format!("{} {validity}", credential.id));
            }
            lines.join("\n")
        }
        HandleAnswer::RecordMissing => format!("no handle record exists for {agent}"),
        answer @ HandleAnswer::ValueInAnswer { .. } => answer
            .refusal()
            .map_or_else(String::new, |refusal| refusal.to_string()),
        HandleAnswer::Unreadable { reason } => format!("unreadable: {reason}"),
    }
}
