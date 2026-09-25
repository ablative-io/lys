//! `lys identity health`: name each declared service and the database as
//! ready, or by name why not. The output carries no secret: the `SpiceDB`
//! key travels in a header and is never printed.

use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use super::config::DeployConfig;
use super::credentials::Credentials;
use super::error::{IdentityError, IdentityResult};
use super::rauthy::Rauthy;

/// The declared services, in the order they are checked.
pub const SERVICES: [&str; 3] = ["postgres", "rauthy", "spicedb"];

/// One service's readiness.
#[derive(Debug, Clone)]
pub struct Check {
    /// The service name, one of [`SERVICES`].
    pub service: &'static str,
    /// Where it was dialled, with no credential in it.
    pub address: String,
    /// `None` when ready, otherwise why not.
    pub unready: Option<String>,
}

/// Check every declared service; `Err(Unready)` names each one that is not
/// ready, and the checks are returned beside it for the report.
pub fn run(config: &DeployConfig) -> (Vec<Check>, IdentityResult<()>) {
    let checks: Vec<Check> = SERVICES
        .iter()
        .map(|service| match *service {
            "postgres" => postgres(config),
            "rauthy" => rauthy(config),
            _ => spicedb(config),
        })
        .collect();
    let unready: Vec<String> = checks
        .iter()
        .filter_map(|check| {
            check
                .unready
                .as_ref()
                .map(|why| format!("{} ({why})", check.service))
        })
        .collect();
    let outcome = if unready.is_empty() {
        Ok(())
    } else {
        Err(IdentityError::Unready {
            count: unready.len(),
            names: unready.join(", "),
        })
    };
    (checks, outcome)
}

fn postgres(config: &DeployConfig) -> Check {
    let address = config.database_reach();
    let unready = match address.to_socket_addrs() {
        Err(_) => Some(format!("{address} does not resolve")),
        Ok(mut addrs) => match addrs.next() {
            None => Some(format!("{address} resolves to nothing")),
            Some(addr) => match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
                Ok(_) => None,
                Err(error) => Some(format!(
                    "{address} refused the connection: {}",
                    error.kind()
                )),
            },
        },
    };
    Check {
        service: "postgres",
        address,
        unready,
    }
}

fn rauthy(config: &DeployConfig) -> Check {
    let address = config.rauthy_origin();
    let unready = match Rauthy::new(&address, None).health() {
        Ok(health) if health.db_healthy && health.cache_healthy => None,
        Ok(health) => Some(format!(
            "answered db_healthy {} cache_healthy {}",
            health.db_healthy, health.cache_healthy
        )),
        Err(error) => Some(error.to_string()),
    };
    Check {
        service: "rauthy",
        address,
        unready,
    }
}

fn spicedb(config: &DeployConfig) -> Check {
    let address = config.spicedb_http_origin();
    let unready = match Credentials::load_or_generate(&config.venue) {
        Err(error) => Some(format!(
            "the venue's credentials could not be read: {error}"
        )),
        Ok((credentials, _)) => schema_read(&address, credentials.spicedb_preshared_key.expose()),
    };
    Check {
        service: "spicedb",
        address,
        unready,
    }
}

/// A schema read is readiness: an empty schema answers a named error, which
/// still proves the datastore was reached with the right key.
fn schema_read(origin: &str, key: &str) -> Option<String> {
    let agent_config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(5)))
        .build();
    let agent = ureq::Agent::new_with_config(agent_config);
    let answer = agent
        .post(format!("{origin}/v1/schema/read"))
        .header("Authorization", &format!("Bearer {key}"))
        .send_json(serde_json::json!({}));
    match answer {
        Err(error) => Some(format!(
            "{origin} could not be reached: {}",
            transport_word(&error)
        )),
        Ok(mut answer) => {
            let status = answer.status().as_u16();
            let body = answer
                .body_mut()
                .read_json::<serde_json::Value>()
                .unwrap_or(serde_json::Value::Null);
            match status {
                200 => None,
                404 => {
                    let code = body.get("code").and_then(serde_json::Value::as_i64);
                    if code == Some(5) {
                        None
                    } else {
                        Some(format!("answered {status} for the schema read"))
                    }
                }
                401 | 403 => Some("rejected the preshared key".to_string()),
                _ => Some(format!("answered {status} for the schema read")),
            }
        }
    }
}

fn transport_word(error: &ureq::Error) -> &'static str {
    match error {
        ureq::Error::Timeout(_) => "timed out",
        ureq::Error::ConnectionFailed | ureq::Error::Io(_) => "connection refused",
        ureq::Error::HostNotFound => "host not found",
        _ => "transport failed",
    }
}
