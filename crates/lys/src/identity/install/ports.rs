//! Recorded local listeners and explicit choices for new installations.

use serde_json::Value;

use super::layout::{BROKER_PORT, Layout, SERVICE_PORT};
use super::server_config;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};

/// The two local listeners, retained in the service's existing configuration.
#[derive(Debug, Clone, Copy)]
pub struct Ports {
    /// The directory service listener.
    pub service: u16,
    /// The local secrets broker listener.
    pub broker: u16,
    service_recorded: bool,
    broker_recorded: bool,
}

impl Default for Ports {
    fn default() -> Self {
        Self {
            service: SERVICE_PORT,
            broker: BROKER_PORT,
            service_recorded: false,
            broker_recorded: false,
        }
    }
}

fn refused(field: &str) -> IdentityError {
    IdentityError::new(
        ErrorKind::ConfigInvalid,
        "read listener",
        field,
        "the address must name a nonzero port",
    )
}

fn port(value: &Value, field: &str) -> IdentityResult<u16> {
    let address = value.as_str().ok_or_else(|| refused(field))?;
    let authority = if field == "secrets.broker" {
        address
            .strip_prefix("http://")
            .ok_or_else(|| refused(field))?
    } else {
        address
    };
    let socket: std::net::SocketAddr =
        authority.trim_end_matches('/').parse().map_err(|error| {
            IdentityError::new(
                ErrorKind::ConfigInvalid,
                "read listener",
                field,
                format!("invalid listener address: {error}"),
            )
        })?;
    let port = socket.port();
    if port == 0 {
        return Err(refused(field));
    }
    Ok(port)
}

impl Ports {
    /// Read listener choices from the existing service configuration.
    pub fn from_value(value: &Value) -> IdentityResult<Self> {
        let mut ports = Self {
            service_recorded: true,
            broker_recorded: true,
            ..Self::default()
        };
        if let Some(listen) = value.get("listen") {
            ports.service = port(listen, "listen")?;
        }
        if let Some(broker) = value.pointer("/secrets/broker") {
            ports.broker = port(broker, "secrets.broker")?;
        }
        ports.chosen(None, None)
    }

    /// Read recorded listeners, retaining defaults for a new install.
    pub fn load(layout: &Layout) -> IdentityResult<Self> {
        if let Some(carried) = server_config::carried(layout)? {
            return Ok(carried.ports);
        }
        let path = layout.deployment_config();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(IdentityError::new(
                    ErrorKind::ConfigInvalid,
                    "read listeners",
                    "deployment.toml",
                    error.to_string(),
                )
                .at(&path));
            }
        };
        let deployment: toml::Table = toml::from_str(&text).map_err(|error| {
            IdentityError::new(
                ErrorKind::ConfigInvalid,
                "read listeners",
                "deployment.toml",
                error.to_string(),
            )
            .at(&path)
        })?;
        let origin = deployment
            .get("issuer")
            .and_then(|issuer| issuer.get("public_origin"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| refused("issuer.public_origin"))?;
        // A proxy's public origin does not record the local listener.
        let direct = origin.strip_prefix("http://").and_then(|authority| {
            let (host, number) = authority.trim_end_matches('/').rsplit_once(':')?;
            ["localhost", "127.0.0.1", "[::1]"]
                .contains(&host)
                .then_some(number)
        });
        let service = match direct {
            Some(number) => number.parse::<u16>().map_err(|error| {
                IdentityError::new(
                    ErrorKind::ConfigInvalid,
                    "read listeners",
                    "issuer.public_origin",
                    error.to_string(),
                )
            })?,
            None => SERVICE_PORT,
        };
        Self {
            service,
            service_recorded: true,
            ..Self::default()
        }
        .chosen(None, None)
    }

    /// Apply explicit choices after refusing zero or shared listener ports.
    pub fn chosen(self, service: Option<u16>, broker: Option<u16>) -> IdentityResult<Self> {
        for (selected, held, recorded, field) in [
            (service, self.service, self.service_recorded, "service-port"),
            (broker, self.broker, self.broker_recorded, "broker-port"),
        ] {
            if recorded && selected.is_some_and(|selected| selected != held) {
                return Err(IdentityError::new(
                    ErrorKind::ConfigInvalid,
                    "choose listeners",
                    field,
                    "an existing install keeps its recorded port; changing it needs a coordinated migration",
                ));
            }
        }
        let ports = Self {
            service: service.unwrap_or(self.service),
            broker: broker.unwrap_or(self.broker),
            ..self
        };
        if ports.service == 0 || ports.broker == 0 || ports.service == ports.broker {
            return Err(IdentityError::new(
                ErrorKind::ConfigInvalid,
                "choose listeners",
                "ports",
                "service and broker need distinct nonzero ports",
            ));
        }
        Ok(ports)
    }

    /// The browser origin for this listener.
    pub fn service_url(self) -> String {
        format!("http://localhost:{}", self.service)
    }

    /// The setup page on this listener.
    pub fn setup_url(self) -> String {
        format!("{}/setup", self.service_url())
    }
}

#[cfg(test)]
#[path = "ports_tests.rs"]
mod tests;
