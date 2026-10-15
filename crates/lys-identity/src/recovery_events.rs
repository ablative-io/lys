//! Durable recovery events shared by local issuance and the identity service.

use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::Path;

/// The recovery step recorded without passwords or bearer codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Local authentication approved a first-run code request.
    FirstRunCodeIssued,
    /// Local authentication approved an administrator recovery code request.
    PasswordCodeIssued,
    /// A valid recovery code reached the password writer.
    PasswordResetStarted,
    /// The issuer accepted the new password.
    PasswordResetCompleted,
}

/// An immutable, versioned security record. It contains no account secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// The stored record shape.
    pub version: u8,
    /// A unique event identity.
    pub id: String,
    /// The recovery step.
    pub kind: Kind,
    /// The pending code's digest, used only to correlate recovery steps.
    pub correlation: String,
    /// Unix seconds at the step.
    pub at: u64,
}

fn invalid(reason: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.into())
}

fn validate(event: &Event) -> io::Result<()> {
    if event.version != 1
        || event.id.parse::<crate::OperationId>().is_err()
        || event.correlation.len() != 64
        || !event
            .correlation
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(invalid("RecoveryEventsUnavailable: invalid recovery event"));
    }
    Ok(())
}

/// Record one immutable recovery event before returning its identity.
///
/// # Errors
/// Returns identity, serialization or durable filesystem failures.
pub fn record(dir: &Path, kind: Kind, correlation: &str, at: u64) -> io::Result<Event> {
    let event = Event {
        version: 1,
        id: crate::OperationId::generate()
            .map_err(io::Error::other)?
            .to_string(),
        kind,
        correlation: correlation.to_owned(),
        at,
    };
    validate(&event)?;
    let bytes = serde_json::to_vec(&event).map_err(io::Error::other)?;
    std::fs::create_dir_all(dir)?;
    let pending = dir.join(format!(".{}.pending", event.id));
    let destination = dir.join(format!("{}.json", event.id));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&pending)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&pending, destination)?;
        std::fs::File::open(dir)?.sync_all()
    })();
    if let Err(error) = result {
        match std::fs::remove_file(&pending) {
            Ok(()) => {}
            Err(cleanup) if cleanup.kind() == io::ErrorKind::NotFound => {}
            Err(cleanup) => {
                return Err(io::Error::other(format!(
                    "{error}; event cleanup: {cleanup}"
                )));
            }
        }
        return Err(error);
    }
    Ok(event)
}

/// Read complete events, newest first. An older install has no events yet.
///
/// # Errors
/// Refuses unreadable or malformed complete records, without hiding them.
pub fn read(dir: &Path) -> io::Result<Vec<Event>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut events = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "pending")
        {
            continue;
        }
        let event: Event = serde_json::from_slice(&std::fs::read(&path)?)
            .map_err(|error| invalid(format!("RecoveryEventsUnavailable: {error}")))?;
        validate(&event)?;
        if path.file_name().and_then(|name| name.to_str()) != Some(&format!("{}.json", event.id)) {
            return Err(invalid(
                "RecoveryEventsUnavailable: event identity differs from its file",
            ));
        }
        events.push(event);
    }
    events.sort_by(|left, right| right.at.cmp(&left.at).then(right.id.cmp(&left.id)));
    Ok(events)
}

#[cfg(test)]
#[path = "recovery_events_tests.rs"]
mod tests;
