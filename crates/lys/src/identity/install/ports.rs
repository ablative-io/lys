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
}

impl Default for Ports {
    fn default() -> Self {
        Self {
            service: SERVICE_PORT,
            broker: BROKER_PORT,
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
        let mut ports = Self::default();
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
        Ok(server_config::carried(layout)?.map_or_else(Self::default, |carried| carried.ports))
    }

    /// Apply explicit choices after refusing zero or shared listener ports.
    pub fn chosen(self, service: Option<u16>, broker: Option<u16>) -> IdentityResult<Self> {
        let ports = Self {
            service: service.unwrap_or(self.service),
            broker: broker.unwrap_or(self.broker),
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
