//! Legacy session allowances are written before alarm evaluation uses them.

use super::{Kept, ProvisioningStore, index, journal, unavailable};
use crate::error::ServerError;
use std::sync::Arc;

impl ProvisioningStore {
    pub(crate) fn session_stall_after(
        &mut self,
        agent: &str,
        number: u32,
        session: &str,
        say: &dyn Fn(&str),
    ) -> Result<u64, ServerError> {
        let version = self
            .version(agent, number)
            .ok_or(ServerError::LaunchRecordMissing)?;
        let seconds = if let Some(settings) = &version.settings.session {
            settings.stall_after
        } else {
            let mut next = serde_json::to_value(&self.kept).map_err(unavailable)?;
            let profiles = next["profiles"]
                .as_array_mut()
                .ok_or_else(|| unavailable("provisioning profiles are not an array"))?;
            let profile = profiles
                .iter_mut()
                .find(|profile| profile["agent"] == agent)
                .ok_or(ServerError::LaunchRecordMissing)?;
            let versions = profile["versions"]
                .as_array_mut()
                .ok_or_else(|| unavailable("provisioning versions are not an array"))?;
            let version = versions
                .iter_mut()
                .find(|version| version["number"] == number)
                .ok_or(ServerError::LaunchRecordMissing)?;
            version["settings"]["session"] = serde_json::json!({"stall_after":300});
            let next: Kept = serde_json::from_value(next).map_err(unavailable)?;
            let before = std::fs::read(&self.path).map_err(unavailable)?;
            if let Err(error) = journal::record_stall(&self.path, &next, &before) {
                self.uncertain = true;
                self.settle()?;
                return Err(error);
            }
            self.indexes = Arc::new(index::Indexes::rebuild(&next));
            self.kept = next;
            self.journal = true;
            self.since_snapshot = 0;
            self.stall_upgrades.insert((agent.to_owned(), number));
            300
        };
        if self.stall_upgrades.remove(&(agent.to_owned(), number)) {
            say(&format!(
                "provisioning: session {session} agent {agent} version {number}: wrote session.stall_after={seconds} to its kept profile"
            ));
        }
        Ok(seconds)
    }
}
