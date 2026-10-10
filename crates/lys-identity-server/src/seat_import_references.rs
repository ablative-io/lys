//! A seat's credentials, kept as references (AGENTS-003 R3).
//!
//! The seat-identity file the manifest declares is recorded by its path
//! alone. It is never opened, parsed, copied or hashed: whether it is usable
//! is read from its metadata only (a regular file, not a link, mode 0600,
//! owned by this service's own user). Its public identity is taken from
//! Lys's own records, never from the file or its name.
//!
//! A declared broker handle is accepted only when the approved broker's
//! handle record already holds it as active for the seat's agent; moving a
//! key into the broker is a separate, separately authorised act. The
//! person answering for the seat is the agent's responsible person in the
//! directory, never inferred from a path. Every reference that cannot be
//! used refuses the plan by name before confirmation, and every answer
//! carries paths, handle ids and public identities only.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use lys_identity::start::credentials::{HandleAnswer, HandleRecords};

use crate::error::ServerError;
use crate::routes::AppState;
use crate::routes::door_handles::credential_id;
use crate::seat_import_plan::{CredentialReference, Fragment, Manifest, Refusal};
use crate::seat_import_profiles::{empty_fragment, refusal};

/// The reference kind of a declared seat-identity file.
pub const SEAT_FILE_KIND: &str = "seat_identity_file";
/// The reference kind of a declared broker handle.
pub const HANDLE_KIND: &str = "broker_handle";

/// The seat has no responsible person in the directory.
pub const RESPONSIBLE_MISSING: &str = "import_responsible_missing";
/// A seat-identity file is missing, a link, not mode 0600 or owned by
/// another user.
pub const REFERENCE_UNUSABLE: &str = "import_reference_unusable";
/// A broker handle the approved broker does not hold as active.
pub const REFERENCE_UNRESOLVED: &str = "import_reference_unresolved";
/// The approved broker's handle record could not be read.
pub const REFERENCE_UNREADABLE: &str = "import_reference_unreadable";
/// The broker's answer carried a member beside a handle's id and state.
pub const CREDENTIAL_IN_ANSWER: &str = "credential_value_in_answer";
/// A reference's public identity differs from the one Lys records.
pub const IDENTITY_MISMATCH: &str = "import_reference_identity_mismatch";

const NO_DOOR: &str = "this service is configured with no door address, so the handle cannot be resolved through the approved broker";

/// The mode a seat-identity file must have.
const FILE_MODE: u32 = 0o600;

/// What Lys holds about the seat a reference belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatFacts {
    /// The agent identity the seat runs as.
    pub agent: String,
    /// The agent's responsible person in the directory.
    pub responsible: Option<String>,
    /// The seat's public identity as Lys's own records hold it.
    pub recorded_identity: Option<String>,
    /// The public identity the door answers for the declared seat file,
    /// when the door was asked.
    pub door_identity: Option<String>,
}

/// The seat's credential references, read through `state`: the seat and
/// its agent from the seats record, the responsible person from the
/// directory, the file's usability by its metadata under this service's own
/// user. This service is configured with no door address, so a declared
/// broker handle cannot be resolved here and is refused by name.
pub fn read_references(manifest: &Manifest, state: &AppState) -> Fragment {
    let facts = match facts(state, &manifest.seat) {
        Ok(facts) => facts,
        Err(refused) => {
            let mut fragment = empty_fragment();
            fragment.refusals.push(refused);
            return fragment;
        }
    };
    read_references_from(manifest, &facts, None, lys_runner::peer::own_user())
}

/// The seat's agent and responsible person; a seat Lys does not hold is
/// refused by the seats record's own name, `seat_unknown`.
fn facts(state: &AppState, seat: &str) -> Result<SeatFacts, Refusal> {
    let named = |error: &ServerError| refusal(&error.name(), seat, &error.to_string());
    let found = crate::seats_api::seat(state, seat).map_err(|error| named(&error))?;
    let agent = found.added.agent;
    let people = crate::seats_api::responsible(state, &BTreeSet::from([agent.clone()]))
        .map_err(|error| named(&error))?;
    let responsible = people.get(&agent).cloned().flatten();
    Ok(SeatFacts {
        recorded_identity: Some(agent.clone()),
        agent,
        responsible,
        door_identity: None,
    })
}

/// The seat's credential references from `facts`: `handles` is the approved
/// broker's handle record (none when no broker can be asked) and `owner` is
/// the user a seat-identity file must belong to.
pub fn read_references_from(
    manifest: &Manifest,
    facts: &SeatFacts,
    handles: Option<&dyn HandleRecords>,
    owner: u32,
) -> Fragment {
    let mut fragment = empty_fragment();
    let person = facts.responsible.clone().unwrap_or_default();
    if facts.responsible.is_none() {
        fragment.refusals.push(refusal(
            RESPONSIBLE_MISSING,
            &manifest.seat,
            &format!(
                "agent {} has no responsible person in the directory, so no person answers for its credentials",
                facts.agent
            ),
        ));
    }
    if let Some(path) = &manifest.seat_identity_file {
        let reference = seat_file(path, facts, &person, owner, &mut fragment);
        fragment.references.push(reference);
    }
    if !manifest.secret_handles.is_empty() {
        broker_handles(manifest, facts, &person, handles, &mut fragment);
    }
    fragment
}

/// The declared seat-identity file at `path`, by its metadata alone.
fn seat_file(
    path: &Path,
    facts: &SeatFacts,
    person: &str,
    owner: u32,
    fragment: &mut Fragment,
) -> CredentialReference {
    let locator = path.display().to_string();
    let mut reference = CredentialReference {
        kind: SEAT_FILE_KIND.to_owned(),
        locator: locator.clone(),
        owner: person.to_owned(),
        public_identity: facts.recorded_identity.clone(),
        usable: false,
        reason: None,
    };
    let unusable = match usable(path, owner) {
        Ok(()) => None,
        Err(reason) => Some((REFERENCE_UNUSABLE, reason)),
    };
    let mismatched = match (&facts.recorded_identity, &facts.door_identity) {
        (Some(recorded), Some(answered)) if recorded != answered => Some((
            IDENTITY_MISMATCH,
            format!(
                "the door answers public identity {answered} for this file, and Lys records {recorded} for the seat"
            ),
        )),
        (Some(_), Some(_)) => None,
        (None, _) | (_, None) => {
            fragment.prerequisites.push(format!(
                "{locator}: the file's public identity is not confirmed against the door; Lys records {} for the seat",
                facts.recorded_identity.as_deref().unwrap_or("no public identity")
            ));
            None
        }
    };
    match unusable.or(mismatched) {
        Some((name, reason)) => {
            fragment.refusals.push(refusal(name, &locator, &reason));
            reference.reason = Some(reason);
        }
        None => reference.usable = true,
    }
    reference
}

/// Whether the file at `path` is usable, read from its metadata alone: a
/// regular file, not a link, mode 0600, owned by `owner`. The file is never
/// opened.
pub fn usable(path: &Path, owner: u32) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        ErrorKind::NotFound => "the file is missing".to_owned(),
        _ => format!("the file's metadata cannot be read: {error}"),
    })?;
    if metadata.file_type().is_symlink() {
        return Err("the path is a symbolic link; the reference names the file itself".to_owned());
    }
    if !metadata.is_file() {
        return Err("the path is not a regular file".to_owned());
    }
    let mode = metadata.permissions().mode() & 0o7777;
    if mode != FILE_MODE {
        return Err(format!("the file's mode is {mode:04o}, not 0600"));
    }
    if metadata.uid() != owner {
        return Err(format!(
            "the file is owned by user {}, not by this service's user {owner}",
            metadata.uid()
        ));
    }
    Ok(())
}

/// Each declared broker handle, resolved through the approved broker's
/// handle record in one read for the seat's agent.
fn broker_handles(
    manifest: &Manifest,
    facts: &SeatFacts,
    person: &str,
    handles: Option<&dyn HandleRecords>,
    fragment: &mut Fragment,
) {
    let answer = handles.map(|records| records.handles(&facts.agent));
    for handle in &manifest.secret_handles {
        let verdict = if credential_id(handle) {
            verdict(handle, &facts.agent, answer.as_ref())
        } else {
            Err((
                REFERENCE_UNRESOLVED,
                "a handle id is ASCII letters, digits, `-`, `_` and `.`".to_owned(),
            ))
        };
        let mut reference = CredentialReference {
            kind: HANDLE_KIND.to_owned(),
            locator: handle.clone(),
            owner: person.to_owned(),
            public_identity: Some(facts.agent.clone()),
            usable: false,
            reason: None,
        };
        match verdict {
            Ok(()) => reference.usable = true,
            Err((name, reason)) => {
                fragment.refusals.push(refusal(name, handle, &reason));
                reference.reason = Some(reason);
            }
        }
        fragment.references.push(reference);
    }
}

/// Whether `handle` is held active for `agent` in the broker's `answer`.
fn verdict(
    handle: &str,
    agent: &str,
    answer: Option<&HandleAnswer>,
) -> Result<(), (&'static str, String)> {
    match answer {
        None => Err((REFERENCE_UNRESOLVED, NO_DOOR.to_owned())),
        Some(HandleAnswer::Held(held)) => match held.iter().find(|one| one.id == handle) {
            Some(one) if one.valid => Ok(()),
            Some(_) => Err((
                REFERENCE_UNRESOLVED,
                format!("the broker holds this handle for agent {agent}, and it is not active"),
            )),
            None => Err((
                REFERENCE_UNRESOLVED,
                format!("the broker holds no handle with this id for agent {agent}"),
            )),
        },
        Some(HandleAnswer::RecordMissing) => Err((
            REFERENCE_UNRESOLVED,
            format!("the broker has no handle record for agent {agent}"),
        )),
        Some(HandleAnswer::ValueInAnswer { record, field }) => Err((
            CREDENTIAL_IN_ANSWER,
            format!(
                "the broker's answer for {record} carried the member `{field}`; nothing is taken from it"
            ),
        )),
        Some(HandleAnswer::Unreadable { reason }) => Err((
            REFERENCE_UNREADABLE,
            format!("the broker's handle record could not be read: {reason}"),
        )),
    }
}
