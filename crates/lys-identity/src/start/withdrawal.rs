//! The withdrawal: one signed directory event saying a launch record's
//! request no longer stands, who withdrew it and when.
//!
//! A withdrawal never records that the agent did not start: a command once
//! given may have been run, and only the agent's signed report says it was.
//! The person who gave a start, or anyone holding the same right, withdraws
//! it; anyone else is refused as `start_right_missing` and nothing is kept.
//! A record withdrawn once is answered as it stands, with no second
//! withdrawal. A report arriving after a withdrawal shows the record running,
//! with the withdrawal beside it.

use lys_log_store::LeafStore;

use crate::encoding::{map, text, uint};
use crate::start::authority::admit;
use crate::start::error::StartError;
use crate::start::give::Owners;
use crate::start::launch_record::{VERSION, as_text, as_uint, cbor, fields};
use crate::start::request::resolve;
use crate::start::state::{LaunchRecords, LaunchState, state_of};

/// A kept withdrawal: the request no longer stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdrawal {
    /// The launch record withdrawn.
    pub launch_record: String,
    /// Who withdrew it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

impl Withdrawal {
    /// The canonical body: a map of keys 1 to 4, as IDENTITY-EVENTS.md gives them.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        map(&mut out, 4);
        uint(&mut out, 1);
        uint(&mut out, VERSION);
        uint(&mut out, 2);
        text(&mut out, &self.launch_record);
        uint(&mut out, 3);
        text(&mut out, &self.by);
        uint(&mut out, 4);
        uint(&mut out, self.at);
        out
    }

    pub(crate) fn decode(body: &[u8]) -> Result<Self, String> {
        let refused = || "the message is not a launch event this service signed".to_owned();
        let values = fields(cbor(body)?, 4)?;
        let [version, launch_record, by, at] =
            <[ciborium::Value; 4]>::try_from(values).map_err(|_short| refused())?;
        if as_uint(&version)? != VERSION {
            return Err(refused());
        }
        let withdrawal = Self {
            launch_record: as_text(launch_record)?,
            by: as_text(by)?,
            at: as_uint(&at)?,
        };
        if withdrawal.encode() != body {
            return Err(refused());
        }
        Ok(withdrawal)
    }
}

/// Withdraw the launch record `launch_record` as `caller` at `now`, and
/// answer its state after: withdrawn, or running with the withdrawal beside
/// it when its report has arrived.
pub fn withdraw<S: LeafStore>(
    store: &mut LaunchRecords<S>,
    owners: &Owners<'_>,
    caller: &str,
    launch_record: &str,
    now: u64,
) -> Result<LaunchState, StartError> {
    store.settle()?;
    let record =
        store
            .record(launch_record)
            .cloned()
            .ok_or_else(|| StartError::LaunchRecordUnknown {
                launch_record: launch_record.to_owned(),
            })?;
    let agent = resolve(owners.agents, &record.agent)?;
    let admitted = admit(owners.admission, caller, &agent)?;
    if store.withdrawal(launch_record).is_none() {
        store.keep_withdrawal(&Withdrawal {
            launch_record: record.id,
            by: admitted.caller,
            at: now,
        })?;
    }
    state_of(store, launch_record, owners.sessions)
}
