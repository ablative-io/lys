//! Count every admitted credential attempt before contacting the issuer or
//! reading a setup code. Address buckets expire only when another request
//! checks them; blocked attempts do not extend the fixed window. No timer.
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::error::ServerError;

const WINDOW: Duration = Duration::from_secs(60);
const ADDRESSES: usize = 1024;

struct Bucket {
    start: Instant,
    count: u8,
}

#[derive(Default)]
pub(crate) struct Attempts(Mutex<HashMap<IpAddr, Bucket>>);

impl Attempts {
    pub(crate) fn admit(&self, address: IpAddr, limit: u8) -> Result<(), ServerError> {
        self.at(address, limit, Instant::now())
    }

    fn at(&self, address: IpAddr, limit: u8, now: Instant) -> Result<(), ServerError> {
        let mut buckets = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        buckets.retain(|_, bucket| now.saturating_duration_since(bucket.start) < WINDOW);
        if let Some(bucket) = buckets.get_mut(&address) {
            if bucket.count >= limit {
                return Err(ServerError::SignInThrottled);
            }
            bucket.count += 1;
        } else {
            if buckets.len() >= ADDRESSES {
                return Err(ServerError::SignInThrottled);
            }
            buckets.insert(
                address,
                Bucket {
                    start: now,
                    count: 1,
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "sign_in_attempts_tests.rs"]
mod tests;
