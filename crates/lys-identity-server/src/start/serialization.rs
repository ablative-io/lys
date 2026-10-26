//! Starts sharing an agent and computer cannot race their launch records.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};

use crate::error::ServerError;

type Key = (String, String);
type Lock = crate::wait_turns::Turns;

#[derive(Default)]
pub(super) struct Starts(Mutex<BTreeMap<Key, Weak<Lock>>>);

impl Starts {
    pub(super) fn for_computer(
        &self,
        agent: String,
        computer: String,
    ) -> Result<Arc<Lock>, ServerError> {
        let mut held = self.0.lock().map_err(|error| ServerError::ConfigInvalid {
            reason: format!("start locks are unavailable: {error}"),
        })?;
        held.retain(|_, lock| lock.strong_count() > 0);
        let key = (agent, computer);
        if let Some(lock) = held.get(&key).and_then(Weak::upgrade) {
            return Ok(lock);
        }
        let lock = Arc::new(Lock::default());
        held.insert(key, Arc::downgrade(&lock));
        Ok(lock)
    }
}
