//! Every session's lock, held together (HOME-019 R5).
//!
//! Ship takes the lock of every session the home lists before it reads any
//! of them, with the same non-blocking exclusive lock a session's owner
//! takes, and holds them all until its commit is written, so no seat can
//! append between the check and the snapshot. No index is loaded and no
//! head is read here: a lock is only a lock. A session another owner holds
//! is refused by name at once, without waiting, and the locks already taken
//! are released as the partial set is dropped.

use crate::error::HomeError;
use crate::record::Home;
use crate::record::lock::SessionLock;

/// The locks of every session of a home, released when dropped.
#[derive(Debug)]
pub struct Held {
    locks: Vec<SessionLock>,
}

impl Held {
    /// How many session locks are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.locks.len()
    }

    /// Whether no lock is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.locks.is_empty()
    }
}

/// Take the lock of each named session, in the order given; a session a
/// live owner holds refuses as `session_held`, naming it.
pub fn hold_all(home: &Home, sessions: &[String]) -> Result<Held, HomeError> {
    let mut locks = Vec::with_capacity(sessions.len());
    for session in sessions {
        let file = home.session_path(session)?;
        match SessionLock::take(&file) {
            Ok(lock) => locks.push(lock),
            Err(HomeError::SessionHeld { .. }) => {
                return Err(HomeError::HeldByOwner {
                    session: session.clone(),
                });
            }
            Err(e) => return Err(e),
        }
    }
    Ok(Held { locks })
}
