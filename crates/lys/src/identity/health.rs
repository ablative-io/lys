//! `lys identity health`: named readiness checks for the declared services.
//!
//! Invariants:
//!
//! - The declared services are exactly [`DECLARED_SERVICES`]: the `PostgreSQL`
//!   database, Rauthy and `SpiceDB`. Each is checked every run, and each
//!   unready one is named with a stable reason; health passes only when all
//!   three are ready.
//! - The database is probed at the address the configuration names
//!   ([`DeploymentConfig::database_probe`]); an unreachable external database
//!   is reported as such, never replaced by a local one.
//! - `SpiceDB` is asked for its readiness only (`/healthz`); health sends it no
//!   permission check and writes nothing to it (R4).
//! - Health reads no credential, so its output cannot carry one: it prints
//!   service names, configured addresses and named reasons.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::error::CliResult;
use crate::commands::output::Emitter;
use crate::identity::config::DeploymentConfig;
use crate::identity::error::IdentityError;
use crate::identity::rauthy::{self, TransportFailure};

/// The services health checks, in order.
pub const DECLARED_SERVICES: [&str; 3] = ["database", "rauthy", "spicedb"];

/// One service's readiness.
#[derive(Debug)]
pub struct Check {
    /// The service name, one of [`DECLARED_SERVICES`].
    pub service: &'static str,
    /// The address it was checked at.
    pub address: String,
    /// `None` when ready; otherwise the named reason it is not.
    pub failure: Option<String>,
}

#[derive(Deserialize)]
struct RauthyHealth {
    db_healthy: bool,
    cache_healthy: bool,
}

/// Run `lys identity health --config <config>`.
pub fn run(config_path: &Path, timeout_secs: u64, json: bool) -> CliResult<()> {
    let config = DeploymentConfig::load(config_path)?;
    let checks = check_all(&config, Duration::from_secs(timeout_secs.max(1)));
    let mut out = Emitter::new(json);
    if out.is_json() {
        let services: Vec<Value> = checks
            .iter()
            .map(|check| {
                json!({
                    "service": check.service,
                    "address": check.address,
                    "ready": check.failure.is_none(),
                    "failure": check.failure,
                })
            })
            .collect();
        out.field("services", "services", Value::Array(services));
    } else {
        for check in &checks {
            match &check.failure {
                None => out.note(&format!("service {}: ready at {}", check.service, check.address)),
                Some(reason) => out.note(&format!(
                    "service {}: unready at {} ({reason})",
                    check.service, check.address
                )),
            }
        }
    }
    let unready: Vec<String> = checks
        .iter()
        .filter_map(|check| {
            check
                .failure
                .as_ref()
                .map(|reason| format!("{} ({})", check.service, first_word(reason)))
        })
        .collect();
    if unready.is_empty() {
        out.finish();
        Ok(())
    } else {
        Err(IdentityError::ServicesUnready {
            names: unready.join(", "),
        }
        .into())
    }
}

/// Check every declared service.
pub fn check_all(config: &DeploymentConfig, timeout: Duration) -> Vec<Check> {
    let database = config.database_probe();
    let rauthy_address = config.rauthy.publish.clone();
    let spicedb_address = config.spicedb.http_publish.clone();
    vec![
        Check {
            service: DECLARED_SERVICES[0],
            failure: check_database(&database, timeout),
            address: database,
        },
        Check {
            service: DECLARED_SERVICES[1],
            failure: check_rauthy(&rauthy_address, timeout),
            address: rauthy_address,
        },
        Check {
            service: DECLARED_SERVICES[2],
            failure: check_spicedb(&spicedb_address, timeout),
            address: spicedb_address,
        },
    ]
}

fn check_database(address: &str, timeout: Duration) -> Option<String> {
    let resolved = match address.to_socket_addrs() {
        Ok(resolved) => resolved,
        Err(error) => return Some(format!("database_unreachable: resolving failed: {error}")),
    };
    let mut last = String::from("the address resolved to nothing");
    for candidate in resolved {
        match TcpStream::connect_timeout(&candidate, timeout) {
            Ok(_) => return None,
            Err(error) => last = format!("connecting to {candidate} failed: {error}"),
        }
    }
    Some(format!("database_unreachable: {last}"))
}

fn check_rauthy(address: &str, timeout: Duration) -> Option<String> {
    match rauthy::send(address, "GET", "/auth/v1/health", None, None, timeout) {
        Err(TransportFailure::NotSent(reason) | TransportFailure::Uncertain(reason)) => {
            Some(format!("rauthy_unreachable: {reason}"))
        }
        Ok(response) => match serde_json::from_slice::<RauthyHealth>(&response.body) {
            Ok(health) if response.status == 200 && health.db_healthy && health.cache_healthy => None,
            Ok(health) => Some(format!(
                "rauthy_unhealthy: status {}, db_healthy={}, cache_healthy={}",
                response.status, health.db_healthy, health.cache_healthy
            )),
            Err(error) => Some(format!(
                "rauthy_unhealthy: status {}, unreadable health body: {error}",
                response.status
            )),
        },
    }
}

fn check_spicedb(address: &str, timeout: Duration) -> Option<String> {
    match rauthy::send(address, "GET", "/healthz", None, None, timeout) {
        Err(TransportFailure::NotSent(reason) | TransportFailure::Uncertain(reason)) => {
            Some(format!("spicedb_unreachable: {reason}"))
        }
        Ok(response) => {
            let serving = String::from_utf8_lossy(&response.body).contains("\"SERVING\"");
            if response.status == 200 && serving {
                None
            } else {
                Some(format!(
                    "spicedb_unready: status {} without SERVING",
                    response.status
                ))
            }
        }
    }
}

fn first_word(reason: &str) -> &str {
    reason.split(':').next().unwrap_or(reason)
}
