//! The rules of the seat-owner store (AGENTS-004 R4): every shape and bound
//! a record keeps, and what each intent does to the projection, with the
//! named refusals. Nothing here touches a file.

use std::collections::BTreeMap;

use crate::error::RunnerError;
use crate::peer::Leader;

use super::record::{
    Custody, Holder, Intent, Lease, MAX_CREDENTIAL_REFERENCES, MAX_REFERENCE_BYTES, OwnerRecord,
    Projection,
};

pub(super) fn refused(code: &'static str, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(code, reason.to_string())
}

pub(super) fn is_hex_id(id: &str) -> bool {
    id.len() == 32 && crate::protocol::unhex(id).is_some()
}

fn validate_leader(start: &Leader, what: &str) -> Result<(), RunnerError> {
    if start.pid <= 1 || start.start.0.is_empty() {
        return Err(refused(
            "seat_owner_record_invalid",
            format!("{what} names an invalid process-start identity"),
        ));
    }
    Ok(())
}

fn validate_name(name: &str, what: &str) -> Result<(), RunnerError> {
    if name.is_empty()
        || name.trim() != name
        || name.chars().any(char::is_control)
        || name.len() > MAX_REFERENCE_BYTES
    {
        return Err(refused(
            "seat_owner_record_invalid",
            format!("{what} is empty, padded, oversized or holds a control character"),
        ));
    }
    Ok(())
}

fn validate_custody(custody: &Custody) -> Result<(), RunnerError> {
    match custody {
        Custody::Owned => Ok(()),
        Custody::HandingOver {
            intent, successor, ..
        } => {
            if !is_hex_id(intent) {
                return Err(refused(
                    "seat_owner_record_invalid",
                    "a handover intent id is not 32 hexadecimal characters",
                ));
            }
            validate_leader(successor, "a handover successor")
        }
        Custody::Stopping { intent, .. } | Custody::Exited { intent, .. } => {
            if is_hex_id(intent) {
                Ok(())
            } else {
                Err(refused(
                    "seat_owner_record_invalid",
                    "a stop intent id is not 32 hexadecimal characters",
                ))
            }
        }
    }
}

pub(super) fn validate_record(record: &OwnerRecord) -> Result<(), RunnerError> {
    validate_name(&record.seat, "a seat")?;
    validate_name(&record.session, "a session")?;
    validate_name(&record.conversation, "a conversation")?;
    if let Some(start) = &record.harness {
        validate_leader(start, "a harness")?;
    }
    match &record.lease.holder {
        Holder::Owner { start } | Holder::Successor { start } => {
            validate_leader(start, "a lease holder")?;
        }
    }
    if record.lease.generation == 0 {
        return Err(refused(
            "seat_owner_record_invalid",
            "a lease generation is zero",
        ));
    }
    validate_custody(&record.custody)?;
    if record.credential_references.len() > MAX_CREDENTIAL_REFERENCES {
        return Err(refused(
            "seat_owner_record_invalid",
            format!(
                "{} credential references exceed the bound of {MAX_CREDENTIAL_REFERENCES}",
                record.credential_references.len()
            ),
        ));
    }
    for reference in &record.credential_references {
        validate_name(reference, "a credential reference")?;
        if reference.contains('=') || reference.contains(':') {
            return Err(refused(
                "seat_owner_record_invalid",
                "a credential reference looks like a value, not a name",
            ));
        }
    }
    Ok(())
}

impl Projection {
    /// A copy holding only the owner `intent` touches, enough to validate it.
    pub(super) fn clone_for_check(&self, intent: &Intent) -> Self {
        let session = match intent {
            Intent::Establish { record } => record.session.as_str(),
            Intent::Lease { session, .. }
            | Intent::Harness { session, .. }
            | Intent::Custody { session, .. }
            | Intent::Cursors { session, .. }
            | Intent::Retire { session } => session.as_str(),
        };
        let mut owners = BTreeMap::new();
        if let Some(record) = self.owners.get(session) {
            owners.insert(session.to_owned(), record.clone());
        }
        Self {
            format: self.format.clone(),
            checkpoint: self.checkpoint,
            owners,
        }
    }
}

/// Apply `intent` to `projection`, refusing by name what the held record
/// forbids.
pub(super) fn apply(projection: &mut Projection, intent: &Intent) -> Result<(), RunnerError> {
    match intent {
        Intent::Establish { record } => {
            validate_record(record)?;
            if projection.owners.contains_key(&record.session) {
                return Err(refused(
                    "seat_owner_held",
                    format!("session {} already has a live owner", record.session),
                ));
            }
            projection
                .owners
                .insert(record.session.clone(), record.as_ref().clone());
        }
        Intent::Lease {
            session,
            generation,
            holder,
            taken_at,
            build,
        } => {
            let record = held(projection, session)?;
            if *generation <= record.lease.generation {
                return Err(refused(
                    "seat_owner_generation_stale",
                    format!(
                        "generation {generation} does not exceed the held {}",
                        record.lease.generation
                    ),
                ));
            }
            match holder {
                Holder::Owner { start } | Holder::Successor { start } => {
                    validate_leader(start, "a lease holder")?;
                }
            }
            record.lease = Lease {
                generation: *generation,
                holder: holder.clone(),
                taken_at: *taken_at,
                build: build.clone(),
            };
        }
        Intent::Harness { session, start } => {
            validate_leader(start, "a harness")?;
            let record = held(projection, session)?;
            if let Some(held_start) = &record.harness
                && held_start != start
            {
                return Err(refused(
                    "seat_owner_harness_mismatch",
                    format!(
                        "session {session} holds harness pid {} and the intent names pid {}",
                        held_start.pid, start.pid
                    ),
                ));
            }
            record.harness = Some(start.clone());
        }
        Intent::Custody { session, custody } => {
            validate_custody(custody)?;
            let record = held(projection, session)?;
            if let Custody::Exited { .. } = record.custody {
                return Err(refused(
                    "seat_owner_exited",
                    format!("session {session} has exited; its custody does not change"),
                ));
            }
            if let (Custody::Stopping { .. }, Custody::HandingOver { .. }) =
                (&record.custody, custody)
            {
                return Err(refused(
                    "seat_owner_stop_fenced",
                    format!("session {session} is stopping; a handover cannot adopt it"),
                ));
            }
            record.custody = custody.clone();
        }
        Intent::Cursors { session, cursors } => {
            let record = held(projection, session)?;
            if cursors.receipt < record.cursors.receipt
                || cursors.feed < record.cursors.feed
                || cursors.hook < record.cursors.hook
            {
                return Err(refused(
                    "seat_owner_cursor_regression",
                    format!("session {session}'s cursors do not move back"),
                ));
            }
            record.cursors = *cursors;
        }
        Intent::Retire { session } => {
            let record = held(projection, session)?;
            if !matches!(record.custody, Custody::Exited { .. }) {
                return Err(refused(
                    "seat_owner_live",
                    format!("session {session} has not exited; a live owner is not retired"),
                ));
            }
            projection.owners.remove(session);
        }
    }
    Ok(())
}

fn held<'a>(
    projection: &'a mut Projection,
    session: &str,
) -> Result<&'a mut OwnerRecord, RunnerError> {
    projection.owners.get_mut(session).ok_or_else(|| {
        refused(
            "seat_owner_unknown",
            format!("session {session} has no live owner"),
        )
    })
}
