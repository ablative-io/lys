//! `lys identity health`: named readiness checks for the declared services.
//!
//! Each declared service is checked on its own and, when it is not ready,
//! named with the reason: `PostgreSQL` by its wire protocol at the address this
//! machine reaches it, Rauthy by its own health answer, which says whether it
//! reaches the shared database, and `SpiceDB` by its readiness endpoint only.
//! No check presents or prints a credential, and `SpiceDB` is asked nothing
//! but whether it is ready.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::rauthy::RauthyApi;
use crate::commands::output::Emitter;

/// The services a deployment declares, in the order they are checked.
pub const DECLARED_SERVICES: [&str; 3] = ["postgres", "rauthy", "spicedb"];

const TIMEOUT: Duration = Duration::from_secs(3);

/// The `PostgreSQL` `SSLRequest` message: any `PostgreSQL` server answers it
/// with one byte before authentication is involved.
const SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f];

/// One service's readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// The declared service.
    pub service: &'static str,
    /// The address or endpoint checked.
    pub target: String,
    /// The named failure and its detail, or `None` when ready.
    pub failure: Option<(&'static str, String)>,
}

fn connect(target: &str) -> Result<TcpStream, String> {
    let address: SocketAddr = target
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .next()
        .ok_or_else(|| format!("{target} resolves to no address"))?;
    let stream =
        TcpStream::connect_timeout(&address, TIMEOUT).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(TIMEOUT)))
        .map_err(|error| error.to_string())?;
    Ok(stream)
}

/// Checks that `PostgreSQL` answers at `target`.
pub fn check_postgres(target: &str) -> Check {
    let failure = match connect(target) {
        Err(detail) => Some(("database_unreachable", detail)),
        Ok(mut stream) => {
            let mut answer = [0_u8; 1];
            match stream
                .write_all(&SSL_REQUEST)
                .and_then(|()| stream.read_exact(&mut answer))
            {
                Ok(()) if answer[0] == b'S' || answer[0] == b'N' => None,
                Ok(()) => Some((
                    "database_not_postgres",
                    "the answer is not PostgreSQL's".to_string(),
                )),
                Err(error) => Some(("database_unreachable", error.to_string())),
            }
        }
    };
    Check {
        service: "postgres",
        target: target.to_string(),
        failure,
    }
}

/// Checks Rauthy's own health answer at `admin_url`.
pub fn check_rauthy(admin_url: &str) -> Check {
    let failure = match RauthyApi::new(admin_url, None).and_then(|api| api.health()) {
        Err(error) if error.kind() == ErrorKind::RauthyUnreachable => {
            Some(("rauthy_unreachable", error.to_string()))
        }
        Err(error) => Some(("rauthy_unready", error.to_string())),
        Ok(health) if !health.db_healthy => Some((
            "rauthy_database_unhealthy",
            "Rauthy does not reach the shared database".to_string(),
        )),
        Ok(health) if !health.cache_healthy => Some((
            "rauthy_cache_unhealthy",
            "Rauthy's cache does not answer".to_string(),
        )),
        Ok(_) => None,
    };
    Check {
        service: "rauthy",
        target: admin_url.to_string(),
        failure,
    }
}

/// Checks `SpiceDB`'s readiness endpoint on the loopback `port`.
pub fn check_spicedb(port: u16) -> Check {
    let target = format!("127.0.0.1:{port}");
    let failure = match connect(&target) {
        Err(detail) => Some(("spicedb_unreachable", detail)),
        Ok(mut stream) => {
            let request =
                format!("GET /healthz HTTP/1.1\r\nHost: {target}\r\nConnection: close\r\n\r\n");
            let mut raw = Vec::new();
            match stream
                .write_all(request.as_bytes())
                .and_then(|()| stream.read_to_end(&mut raw))
            {
                Ok(_) if raw.starts_with(b"HTTP/1.1 200") || raw.starts_with(b"HTTP/1.0 200") => {
                    None
                }
                Ok(_) => Some((
                    "spicedb_unready",
                    String::from_utf8_lossy(raw.split(|b| *b == b'\r').next().unwrap_or_default())
                        .into_owned(),
                )),
                Err(error) => Some(("spicedb_unreachable", error.to_string())),
            }
        }
    };
    Check {
        service: "spicedb",
        target: format!("http://{target}/healthz"),
        failure,
    }
}

/// Checks every declared service of `config`.
pub fn check_all(config: &DeploymentConfig) -> [Check; 3] {
    [
        check_postgres(&config.database.check_address),
        check_rauthy(&config.issuer.admin_url),
        check_spicedb(config.spicedb.http_port),
    ]
}

/// Runs `lys identity health` against the configuration at `config_path`.
pub fn run(config_path: &Path, json: bool) -> IdentityResult<()> {
    let config = DeploymentConfig::load(config_path)?;
    let checks = check_all(&config);
    let mut emitter = Emitter::new(json);
    let mut unready = Vec::new();
    for check in &checks {
        match &check.failure {
            None => emitter.note(&format!("{} ready ({})", check.service, check.target)),
            Some((name, detail)) => {
                eprintln!(
                    "{} unready: {name} ({}): {detail}",
                    check.service, check.target
                );
                unready.push(format!("{} {name}", check.service));
            }
        }
    }
    if !unready.is_empty() {
        return Err(IdentityError::new(
            ErrorKind::Unready,
            "check readiness",
            "deployment",
            format!(
                "{} of {} declared services unready: {}",
                unready.len(),
                DECLARED_SERVICES.len(),
                unready.join(", ")
            ),
        ));
    }
    emitter.field("ready", "ready", DECLARED_SERVICES.len());
    emitter.finish();
    Ok(())
}
