//! Sign-in state is bounded globally and per client. Expired state is pruned
//! on insertion and refused on use; no task, polling loop or timer runs.
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use crate::error::ServerError;

pub(crate) const PER_ADDRESS: usize = 16;
const MAX_AGE: Duration = Duration::from_secs(600);

/// IPv6 privacy addresses share their /64 allowance. Mapped IPv4 keys are
/// identical to native IPv4, so spelling does not buy another allowance.
pub(crate) fn address_key(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or_else(
            || {
                let segments = ip.segments();
                IpAddr::V6(std::net::Ipv6Addr::new(
                    segments[0],
                    segments[1],
                    segments[2],
                    segments[3],
                    0,
                    0,
                    0,
                    0,
                ))
            },
            IpAddr::V4,
        ),
    }
}

struct Flight<T> {
    address: IpAddr,
    began: Instant,
    value: T,
}

pub(crate) struct Flights<T>(HashMap<String, Flight<T>>);

impl<T> Default for Flights<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T> Flights<T> {
    pub(crate) fn insert(
        &mut self,
        key: String,
        value: T,
        address: IpAddr,
        now: Instant,
    ) -> Result<(), ServerError> {
        let address = address_key(address);
        self.0
            .retain(|_, flight| now.saturating_duration_since(flight.began) < MAX_AGE);
        if self.0.len() >= crate::oidc::IN_FLIGHT_MAX
            || self
                .0
                .values()
                .filter(|flight| flight.address == address)
                .count()
                >= PER_ADDRESS
        {
            return Err(ServerError::SignInThrottled);
        }
        self.0.insert(
            key,
            Flight {
                address,
                began: now,
                value,
            },
        );
        Ok(())
    }

    pub(crate) fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub(crate) fn take(&mut self, key: &str, now: Instant) -> Result<T, ServerError> {
        self.0
            .remove(key)
            .filter(|flight| now.saturating_duration_since(flight.began) < MAX_AGE)
            .map(|flight| flight.value)
            .ok_or(ServerError::SignInStateUnknown)
    }
}

#[cfg(test)]
#[path = "sign_in_flights_tests.rs"]
mod tests;
