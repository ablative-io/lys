//! The standing links the service keeps to each runner: the follow of its
//! feed, which carries every use and refusal, and its grant channel, on
//! which the runner's judge asks what an agent holds.
//!
//! Each is begun when the service starts, and begun again when the service
//! next reaches that runner after the link lost it: a runner that came up
//! after the service, or was started again, is linked by the first request
//! it answers. A link that ended on a refusal stays ended until the service
//! starts again; begun again it would end the same way. Every end is said.
//! Nothing here waits on a clock.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use crate::routes::AppState;
use crate::runner_client::RunnerRecord;

/// Which standing link to a runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Link {
    /// The follow of the runner's feed.
    Feed,
    /// The runner's grant channel.
    Grants,
}

/// How a link stands while it is not free to begin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    /// It is held now.
    Live,
    /// It ended on a refusal.
    Refused,
}

/// Which links stand, by link and machine.
#[derive(Debug, Default)]
pub struct Links {
    held: Mutex<BTreeMap<(Link, String), Standing>>,
}

impl Links {
    fn with<T>(&self, act: impl FnOnce(&mut BTreeMap<(Link, String), Standing>) -> T) -> T {
        // The table holds no half-made state: one entry is put or taken
        // whole, so it is as true after another holder's panic as before.
        act(&mut self.held.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Take `link` to `machine`'s runner as begun. False when it is live
    /// already, or ended on a refusal; the caller then begins nothing.
    pub fn begin(&self, link: Link, machine: &str) -> bool {
        self.with(|held| {
            let key = (link, machine.to_owned());
            if held.contains_key(&key) {
                return false;
            }
            held.insert(key, Standing::Live);
            true
        })
    }

    /// `link` to `machine`'s runner ended. `lost` says the runner was not
    /// there or closed it, so the link is begun again when the runner next
    /// answers; otherwise it ended on a refusal and stays ended.
    pub fn ended(&self, link: Link, machine: &str, lost: bool) {
        self.with(|held| {
            let key = (link, machine.to_owned());
            if lost {
                held.remove(&key);
            } else {
                held.insert(key, Standing::Refused);
            }
        });
    }
}

/// Whether a link's end by this refusal name says its runner was not there,
/// rather than that something was refused.
#[must_use]
pub fn lost(refusal: &str) -> bool {
    matches!(refusal, "runner_unreachable" | "runner_socket_unavailable")
}

/// What is said after a link's end when it will be begun again.
pub const AGAIN: &str = "; it is begun again when that runner next answers a request";

/// Begin whichever standing links to `machine`'s runner are free to begin.
/// Called for every runner when the service starts, and after every
/// request a runner answers.
pub fn ensure(state: &Arc<AppState>, machine: &str, runner: &RunnerRecord) {
    crate::refusals_follow::ensure(state, machine, runner.clone());
    crate::grants_refusals::ensure(state, machine, runner.clone());
}

#[cfg(test)]
#[path = "runner_links_tests.rs"]
mod tests;
