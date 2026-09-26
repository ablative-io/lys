//! `lys identity health`: named readiness checks for the declared services.
//!
//! # Invariants
//!
//! - Exactly the [`DECLARED_SERVICES`] are checked, each by its own check,
//!   and every failure is named: `database_unreachable`,
//!   `database_not_postgresql`, `database_not_accepting`, `rauthy_unreachable`, `rauthy_unready`,
//!   `rauthy_database_unhealthy`, `rauthy_cache_unhealthy`,
//!   `spicedb_unreachable`, `spicedb_unready`, `spicedb_not_serving`.
//! - The database is dialled at its configured address and nowhere else. An
//!   unreachable database is a failure, never a reason to look for a local
//!   one (ADR-005).
//! - `SpiceDB` is asked only whether it is serving. Health never asks it a
//!   permission question and never writes to it: in step 1 it enforces
//!   nothing (deploy/identity/README.md).
//! - Output carries service names, addresses and Rauthy's public issuer and
//!   key ids. Health reads no credential, so it cannot print one.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;

use serde_json::{Value, json};

use super::config::{LoadedConfig, RAUTHY_DB_USER};
use super::error::IdentityError;
use super::rauthy::{self, Transport};
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;

/// The services a deployment declares, in the order they are checked.
pub const DECLARED_SERVICES: [&str; 3] = ["database", "rauthy", "spicedb"];

/// `PostgreSQL`'s `SSLRequest`: length 8, request code 80877103. Any server
/// answers it with one byte, `S` or `N`, before any authentication.
const SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f];

/// The frontend/backend protocol version a startup message names, 3.0.
const PROTOCOL_3_0: u32 = 196_608;

/// One service's readiness.
#[derive(Debug)]
pub struct Check {
    /// One of [`DECLARED_SERVICES`].
    pub service: &'static str,
    /// What was dialled.
    pub target: String,
    /// The named failure, or `None` when ready.
    pub failure: Option<String>,
}

/// `lys identity health --config <file>`.
pub fn run(config: &Path, json: bool) -> CliResult<()> {
    let loaded = LoadedConfig::load(config)?;
    let checks = check_all(&loaded);
    let mut emit = Emitter::new(json);
    let mut failures = Vec::new();
    let mut rows = Vec::with_capacity(checks.len());
    for check in &checks {
        match &check.failure {
            None => emit.note(&format!("{} {}: ready", check.service, check.target)),
            Some(failure) => {
                emit.note(&format!(
                    "{} {}: unready ({failure})",
                    check.service, check.target
                ));
                failures.push(format!("{failure} [{} at {}]", check.service, check.target));
            }
        }
        rows.push(json!({
            "service": check.service,
            "target": check.target,
            "ready": check.failure.is_none(),
            "failure": check.failure,
        }));
    }
    if !failures.is_empty() {
        return Err(IdentityError::Unready { failures }.into());
    }
    let (issuer, signing_keys) = rauthy::issuer_identity(&loaded.admin)?;
    emit.field("issuer", "issuer", issuer);
    emit.field("signing keys", "signing_keys", signing_keys.join(","));
    if emit.is_json() {
        emit.field("checks", "checks", Value::Array(rows));
    }
    emit.flag("ready: database, rauthy, spicedb", "ready");
    emit.finish();
    Ok(())
}

/// Runs every declared check, in [`DECLARED_SERVICES`] order.
pub fn check_all(loaded: &LoadedConfig) -> [Check; 3] {
    let database = &loaded.config.database;
    [
        Check {
            service: DECLARED_SERVICES[0],
            target: format!("{}:{}", database.host, database.port),
            failure: probe_postgres(&database.host, database.port, &database.name).err(),
        },
        Check {
            service: DECLARED_SERVICES[1],
            target: loaded.admin.to_string(),
            failure: rauthy_failure(loaded),
        },
        Check {
            service: DECLARED_SERVICES[2],
            target: loaded.spicedb_http.to_string(),
            failure: spicedb_failure(loaded),
        },
    ]
}

/// Asks the configured address whether it speaks `PostgreSQL` and whether it
/// accepts connections, without authenticating.
fn probe_postgres(host: &str, port: u16, database: &str) -> Result<(), String> {
    let unreachable = |error: std::io::Error| format!("database_unreachable: {error}");
    let address = (host, port)
        .to_socket_addrs()
        .map_err(unreachable)?
        .next()
        .ok_or("database_unreachable: the host resolved to no address")?;
    let mut stream = TcpStream::connect(address).map_err(unreachable)?;
    stream.write_all(&SSL_REQUEST).map_err(unreachable)?;
    let mut answer = [0_u8; 1];
    stream.read_exact(&mut answer).map_err(|error| {
        format!("database_not_postgresql: no answer to an SSL request: {error}")
    })?;
    if !matches!(answer[0], b'S' | b'N') {
        return Err(format!(
            "database_not_postgresql: answered {:#04x} to an SSL request",
            answer[0]
        ));
    }
    accepting(address, database)
}

/// A server that is starting, recovering or shutting down still answers an
/// `SSLRequest`, so readiness also sends a startup message, which such a
/// server refuses before any authentication with an SQLSTATE of class 57P.
/// Any other answer, an authentication request or a refusal of this role,
/// means the server accepts connections. No password is sent.
fn accepting(address: std::net::SocketAddr, database: &str) -> Result<(), String> {
    let unreachable = |error: std::io::Error| format!("database_unreachable: {error}");
    let mut body = Vec::new();
    body.extend_from_slice(&PROTOCOL_3_0.to_be_bytes());
    for piece in ["user", RAUTHY_DB_USER, "database", database] {
        body.extend_from_slice(piece.as_bytes());
        body.push(0);
    }
    body.push(0);
    let length = u32::try_from(body.len() + 4)
        .map_err(|error| format!("database_unreachable: startup message: {error}"))?;
    let mut message = length.to_be_bytes().to_vec();
    message.extend_from_slice(&body);
    let mut stream = TcpStream::connect(address).map_err(unreachable)?;
    stream.write_all(&message).map_err(unreachable)?;
    let mut head = [0_u8; 5];
    stream.read_exact(&mut head).map_err(|error| {
        format!("database_not_postgresql: no answer to a startup message: {error}")
    })?;
    if head[0] != b'E' {
        return Ok(());
    }
    let declared = u32::from_be_bytes([head[1], head[2], head[3], head[4]]);
    let remaining = usize::try_from(declared.saturating_sub(4))
        .map_err(|error| format!("database_not_postgresql: error length: {error}"))?;
    let mut fields = vec![0_u8; remaining];
    stream
        .read_exact(&mut fields)
        .map_err(|error| format!("database_not_postgresql: an incomplete error answer: {error}"))?;
    let field = |code: u8| {
        fields
            .split(|byte| *byte == 0)
            .find(|part| part.first() == Some(&code))
            .map(|part| String::from_utf8_lossy(&part[1..]).into_owned())
    };
    match field(b'C') {
        Some(sqlstate) if sqlstate.starts_with("57P") => Err(format!(
            "database_not_accepting: {sqlstate} {}",
            field(b'M').unwrap_or_default()
        )),
        _ => Ok(()),
    }
}

fn rauthy_failure(loaded: &LoadedConfig) -> Option<String> {
    match rauthy::rauthy_health(&loaded.admin) {
        Ok(health) if !health.db_healthy => {
            Some("rauthy_database_unhealthy: Rauthy cannot reach its database".to_string())
        }
        Ok(health) if !health.cache_healthy => {
            Some("rauthy_cache_unhealthy: Rauthy's Hiqlite cache is unhealthy".to_string())
        }
        Ok(_) => None,
        Err(error @ IdentityError::Unreachable { .. }) => Some(error.to_string()),
        Err(error) => Some(format!("rauthy_unready: {error}")),
    }
}

fn spicedb_failure(loaded: &LoadedConfig) -> Option<String> {
    match rauthy::exchange(&loaded.spicedb_http, "GET", "/healthz", &[], None) {
        Err(Transport::NotSent(error)) => Some(format!("spicedb_unreachable: {error}")),
        Err(Transport::Uncertain(reason)) => Some(format!("spicedb_unready: {reason}")),
        Ok(response) => {
            let serving = response.status == 200
                && serde_json::from_slice::<Value>(&response.body)
                    .ok()
                    .and_then(|body| body.get("status").cloned())
                    == Some(Value::from("SERVING"));
            if serving {
                None
            } else {
                Some(format!(
                    "spicedb_not_serving: HTTP {} {}",
                    response.status,
                    String::from_utf8_lossy(&response.body)
                ))
            }
        }
    }
}
