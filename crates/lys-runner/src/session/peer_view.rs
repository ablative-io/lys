//! What the peer proof reads of the sessions: the leader a process id names
//! and whether a connection's proved leader still holds its session.

use super::{Guard, Sessions};
use crate::error::RunnerError;
use crate::peer::Leader;

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
