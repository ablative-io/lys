//! What the broker folds from its audit log: each lease's use count, drop,
//! operations and spend, who has read which sealed record, and a rotation
//! the log shows starting and not finishing.
//!
//! One function applies one line, and it is the only way a line reaches the
//! state, at a start and when a snapshot is written alike. So the state a
//! snapshot carries at a tree size is exactly what folding that many lines
//! gives, and a start from it ends where a start from the whole log ends.
//!
//! The state is the snapshot's payload, in a canonical encoding. It holds
//! handle ids, identities, record names, operation ids and request marks, as
//! the audit lines themselves do, and no secret, token, digest or key.

use std::collections::{BTreeMap, BTreeSet};

use crate::audit::{AuditKind, AuditLine};
use crate::encoding::{Canonical, Reader};
use crate::error::SecretsError;

use super::{HandleRecord, ROTATING, lineage, records};

const STATE_FORMAT: &str = "lys-secrets/broker-folded/v1";
const CONTEXT: &str = "broker snapshot state";

/// The handle records with what the log says of each.
pub(super) type Handles = BTreeMap<String, HandleRecord>;

/// What the log says of one lease.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Lease {
    used: u64,
    dropped: bool,
    settled: u64,
    operations: BTreeMap<String, (String, String)>,
    open: BTreeMap<String, u64>,
}

/// The fold of the audit log's first lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Folded {
    leases: BTreeMap<String, Lease>,
    /// Every (identity, record) pair the log shows read.
    pub(super) readers: BTreeSet<(String, String)>,
    /// The change of a rotation the log shows starting and not finishing.
    pub(super) rotating: Option<String>,
}

fn unreadable(reason: impl Into<String>) -> SecretsError {
    SecretsError::Encoding {
        context: CONTEXT,
        reason: reason.into(),
    }
}

fn text(reader: &mut Reader<'_>) -> Result<String, SecretsError> {
    String::from_utf8(reader.field()?.to_vec()).map_err(|_utf8| unreadable("a text is not UTF-8"))
}

impl Folded {
    /// The state `handles` hold, with `readers` and `rotating`. A lease the
    /// log says nothing of is left out.
    pub(super) fn of(
        handles: &Handles,
        readers: &BTreeSet<(String, String)>,
        rotating: Option<&str>,
    ) -> Self {
        let leases = handles
            .iter()
            .map(|(id, record)| {
                let lease = Lease {
                    used: record.used,
                    dropped: record.dropped,
                    settled: record.settled,
                    operations: record.operations.clone(),
                    open: record.open.clone(),
                };
                (id.clone(), lease)
            })
            .filter(|(_id, lease)| *lease != Lease::default())
            .collect();
        Self {
            leases,
            readers: readers.clone(),
            rotating: rotating.map(str::to_owned),
        }
    }

    /// Sets every record of `handles` to what this state says of it, and to
    /// nothing where it says nothing.
    pub(super) fn lay_over(&self, handles: &mut Handles) {
        for (id, record) in handles.iter_mut() {
            let lease = self.leases.get(id).cloned().unwrap_or_default();
            record.used = lease.used;
            record.dropped = lease.dropped;
            record.settled = lease.settled;
            record.operations = lease.operations;
            record.open = lease.open;
        }
    }

    /// The state after `line`, over the records `handles`.
    pub(super) fn apply(
        handles: &mut Handles,
        readers: &mut BTreeSet<(String, String)>,
        rotating: &mut Option<String>,
        line: AuditLine,
    ) {
        if line.kind == AuditKind::SealedRead && line.outcome == records::READ {
            if let (Some(identity), Some(record)) = (line.identity, line.secret) {
                readers.insert((identity, record));
            }
            return;
        }
        if line.kind == AuditKind::Rotation {
            *rotating = line.outcome.strip_prefix(ROTATING).map(str::to_owned);
            return;
        }
        let Some(id) = line.handle.clone() else {
            return;
        };
        match line.kind {
            AuditKind::Drop => {
                if let Some(record) = handles.get_mut(&id) {
                    record.dropped = true;
                }
            }
            AuditKind::Use if line.outcome == "admitted" => {
                if let (Some(operation), Some(mark)) = (&line.operation, &line.request) {
                    lineage::admitted(handles, &id, (operation, mark), line.spend.unwrap_or(0));
                }
            }
            AuditKind::Settlement => {
                if let Some(operation) = &line.operation {
                    lineage::settled(
                        handles,
                        &id,
                        operation,
                        &line.outcome,
                        line.spend.unwrap_or(0),
                    );
                }
            }
            _ => {}
        }
    }

    /// The state in its canonical encoding.
    pub(super) fn encode(&self) -> Result<Vec<u8>, SecretsError> {
        let count =
            |len: usize| u64::try_from(len).map_err(|_size| unreadable("a count is past 64 bits"));
        let mut state = Canonical::new(STATE_FORMAT)?;
        state.number(count(self.leases.len())?)?;
        for (id, lease) in &self.leases {
            state.field(id.as_bytes())?;
            state.number(lease.used)?;
            state.field(&[u8::from(lease.dropped)])?;
            state.number(lease.settled)?;
            state.number(count(lease.operations.len())?)?;
            for (operation, (outcome, mark)) in &lease.operations {
                state.field(operation.as_bytes())?;
                state.field(outcome.as_bytes())?;
                state.field(mark.as_bytes())?;
            }
            state.number(count(lease.open.len())?)?;
            for (operation, reserved) in &lease.open {
                state.field(operation.as_bytes())?;
                state.number(*reserved)?;
            }
        }
        state.number(count(self.readers.len())?)?;
        for (identity, record) in &self.readers {
            state.field(identity.as_bytes())?;
            state.field(record.as_bytes())?;
        }
        match &self.rotating {
            None => state.field(&[0])?,
            Some(change) => state.field(&[1])?.field(change.as_bytes())?,
        };
        Ok(state.into_bytes())
    }

    /// Reads a state in its canonical encoding.
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, SecretsError> {
        let mut reader = Reader::new(bytes, CONTEXT);
        if reader.field()? != STATE_FORMAT.as_bytes() {
            return Err(unreadable("the state does not open with its format"));
        }
        let mut leases = BTreeMap::new();
        for _lease in 0..reader.number()? {
            let id = text(&mut reader)?;
            let used = reader.number()?;
            let dropped = match reader.field()? {
                [0] => false,
                [1] => true,
                _ => return Err(unreadable("a drop is neither 0 nor 1")),
            };
            let settled = reader.number()?;
            let mut operations = BTreeMap::new();
            for _operation in 0..reader.number()? {
                let operation = text(&mut reader)?;
                let outcome = text(&mut reader)?;
                operations.insert(operation, (outcome, text(&mut reader)?));
            }
            let mut open = BTreeMap::new();
            for _reservation in 0..reader.number()? {
                let operation = text(&mut reader)?;
                open.insert(operation, reader.number()?);
            }
            let lease = Lease {
                used,
                dropped,
                settled,
                operations,
                open,
            };
            leases.insert(id, lease);
        }
        let mut readers = BTreeSet::new();
        for _reader in 0..reader.number()? {
            let identity = text(&mut reader)?;
            readers.insert((identity, text(&mut reader)?));
        }
        let rotating = match reader.field()? {
            [0] => None,
            [1] => Some(text(&mut reader)?),
            _ => return Err(unreadable("a rotation is neither absent nor present")),
        };
        if !reader.is_done() {
            return Err(unreadable("bytes follow the state"));
        }
        Ok(Self {
            leases,
            readers,
            rotating,
        })
    }
}
