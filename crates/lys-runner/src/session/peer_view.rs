//! What the peer proof reads of the sessions: the leader a process id names
//! and whether a connection's proved leader still holds its session.

use super::*;
use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::input::Input;
use crate::judge::Policy;
use crate::operations::Operations;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Key, Launch, SessionView, StatusView};
use crate::refusals::Desk;
use crate::rotation::RotationState;
use crate::state::{Kept, KeptSession, StateFile};
use crate::tracking::Tracking;
use crate::tracking_store::Feed;
use control::*;
use portable_pty::MasterPty;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

impl Sessions {
    pub(crate) fn peer_leader(
        &self,
        pid: u32,
    ) -> Result<Option<(String, u64, Leader)>, RunnerError> {
        let table = self.read_lock()?;
        if table.stopping {
            return Ok(None);
        }
        Ok(table.sessions.iter().find_map(|(id, session)| {
            let leader = session.guard.leader.as_ref()?;
            (session.ended.is_none()
                && !session.ending
                && session.live.is_some()
                && leader.pid == pid)
                .then(|| (id.clone(), session.generation, leader.clone()))
        }))
    }

    pub(crate) fn peer_matches(
        &self,
        id: &str,
        generation: u64,
        leader: &Leader,
    ) -> Result<bool, RunnerError> {
        let table = self.read_lock()?;
        Ok(!table.stopping
            && table.sessions.get(id).is_some_and(|session| {
                session.ended.is_none()
                    && !session.ending
                    && session.live.is_some()
                    && session.generation == generation
                    && session.guard.leader.as_ref() == Some(leader)
            }))
    }

    pub(crate) fn peer_guard(
        &self,
        id: &str,
        generation: u64,
        leader: &Leader,
    ) -> Result<Option<Guard>, RunnerError> {
        let table = self.read_lock()?;
        Ok(table.sessions.get(id).and_then(|session| {
            (!table.stopping
                && session.ended.is_none()
                && !session.ending
                && session.live.is_some()
                && session.generation == generation
                && session.guard.leader.as_ref() == Some(leader))
            .then(|| session.guard.clone())
        }))
    }
}
