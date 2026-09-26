//! Resolving a lantern to the session it was lit in, and cutting that
//! session's chain at the last assistant message at or before the lantern's
//! point (HOME-006 R2).
//!
//! Every session the home lists is read through its index alone, the same
//! lock-free view the read-only reader wraps ([`Index::read`]): no lock is
//! taken and nothing is written. A session holds the lantern when its index
//! has a `lys.lantern` row under that id; an id that names an entry of
//! another kind holds nothing. The lantern entry is read once, from the
//! session the cut is taken from (a second time only when the first holder
//! read turns out, by its `lit_in`, not to be that session), for its
//! `point` and its `lit_in`; then the point's ancestry is read from the root
//! to the point in one pass, the point's entry being its last row. The cut
//! is that chain up to the last `message` entry whose role is `assistant`,
//! in chain order, with a compaction or a custom entry at its own place and
//! no side leaf; nothing is reordered as `context_path` does.
//!
//! A lantern whose data carries `lit_in` is cut from that session and no
//! other, so a copy of its line inside a child (copied lines keep their ids)
//! is a copy and not a second lantern; one whose data carries no `lit_in` is
//! an older record and resolves by the sessions holding it: one holder cuts,
//! several refuse by name until one is named. Every refusal comes before any
//! file is created or written, and none carries a note or an entry's data.

use std::path::PathBuf;

use serde_json::Value;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::entries::{CUSTOM_LANTERN, Entry, EntryBody};
use crate::record::index::{Index, IndexRow};
use crate::record::lantern::session_file;

/// A session whose index holds a `lys.lantern` row under the lantern id.
struct Holder {
    id: String,
    file: PathBuf,
    index: Index,
}

/// What a resolve and cut found: the session cut from, the chain up to the
/// cut entry, and the carried entry when the point is a user message.
#[derive(Clone, Debug)]
pub struct Cut {
    /// The session the cut is taken from: the lantern's lit-in session.
    pub session: String,
    /// That session's file.
    pub file: PathBuf,
    /// The lantern's entry id.
    pub lantern: String,
    /// The lantern's point.
    pub point: String,
    /// The lit-in session the lantern's data records, when it records one.
    pub lit_in: Option<String>,
    /// The index rows of the cut chain, root first.
    pub rows: Vec<IndexRow>,
    /// The entries of the cut chain, root first, one per row.
    pub entries: Vec<Entry>,
    /// The cut entry: the last assistant message at or before the point.
    pub cut_at: String,
    /// The point's entry when it is a user message, which is carried and
    /// not copied; `None` otherwise.
    pub carried: Option<Entry>,
    /// The bytes of the session file read to resolve and cut.
    pub bytes_read: u64,
}

impl Cut {
    /// Whether the point is carried rather than copied.
    #[must_use]
    pub fn coordinate_carried(&self) -> bool {
        self.carried.is_some()
    }

    /// The ids of the cut chain, root first.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(Entry::id).collect()
    }
}

/// Every session of the home whose index holds a `lys.lantern` row under
/// `lantern`, in ascending id order, read without a lock or a write.
fn holders(home: &Home, lantern: &str) -> Result<Vec<Holder>, HomeError> {
    let mut out = Vec::new();
    for id in home.session_ids()? {
        let file = home.session_path(&id)?;
        let (_, index, _) = Index::read(&file)?;
        let holds = index
            .row(lantern)
            .is_some_and(|row| row.custom.as_deref() == Some(CUSTOM_LANTERN));
        if holds {
            out.push(Holder { id, file, index });
        }
    }
    Ok(out)
}

/// The lantern's point and lit-in session, read from its row in `holder`,
/// with the bytes read; data that is not a lantern's shape refuses by name.
fn read_lantern(
    holder: &Holder,
    lantern: &str,
) -> Result<(String, Option<String>, u64), HomeError> {
    let row = holder
        .index
        .row(lantern)
        .ok_or_else(|| HomeError::UnknownEntry {
            session: holder.id.clone(),
            id: lantern.to_owned(),
        })?;
    let (mut entries, read) = holder.index.read_rows_from(&holder.file, &[row])?;
    let entry = entries.pop().ok_or(HomeError::StaleIndex {
        path: holder.file.clone(),
        reason: "a row read no entry",
    })?;
    let shape = || HomeError::EntryShape {
        session: holder.id.clone(),
        id: lantern.to_owned(),
        custom_type: CUSTOM_LANTERN.to_owned(),
        source: None,
    };
    let EntryBody::Custom {
        data: Some(data), ..
    } = &entry.body
    else {
        return Err(shape());
    };
    let point = data
        .get("point")
        .and_then(Value::as_str)
        .ok_or_else(shape)?
        .to_owned();
    let lit_in = match data.get("lit_in") {
        None | Some(Value::Null) => None,
        Some(Value::String(session)) => Some(session.clone()),
        Some(_) => return Err(shape()),
    };
    Ok((point, lit_in, read))
}

/// The role of a message entry; `None` for any other entry.
fn role_of(entry: &Entry) -> Option<&str> {
    match &entry.body {
        EntryBody::Message { message } => message.get("role").and_then(Value::as_str),
        _ => None,
    }
}

/// Resolve `lantern` to the session it was lit in, cutting from `session`
/// when one is named as R2 reads it, and cut that session's chain.
pub fn resolve_and_cut(
    home: &Home,
    lantern: &str,
    session: Option<&str>,
) -> Result<Cut, HomeError> {
    if let Some(named) = session {
        session_file(home, named)?;
    }
    let holders = holders(home, lantern)?;
    let position = |id: &str| holders.iter().position(|holder| holder.id == id);
    let first = match session {
        Some(named) => position(named).ok_or_else(|| HomeError::UnknownLantern {
            session: named.to_owned(),
            id: lantern.to_owned(),
        })?,
        None if holders.is_empty() => {
            return Err(HomeError::NoSuchLantern {
                lantern: lantern.to_owned(),
            });
        }
        None => 0,
    };
    let (mut point, lit_in, mut bytes_read) = read_lantern(&holders[first], lantern)?;
    let chosen = match (&lit_in, session) {
        (Some(lit_in), Some(named)) if lit_in != named => {
            return Err(HomeError::LanternNotLitHere {
                lantern: lantern.to_owned(),
                session: named.to_owned(),
                lit_in: lit_in.clone(),
            });
        }
        (Some(lit_in), _) => position(lit_in).ok_or_else(|| HomeError::UnknownLantern {
            session: lit_in.clone(),
            id: lantern.to_owned(),
        })?,
        (None, Some(_)) => first,
        (None, None) if holders.len() == 1 => first,
        (None, None) => {
            return Err(HomeError::LanternAmbiguous {
                lantern: lantern.to_owned(),
                sessions: holders.iter().map(|holder| holder.id.clone()).collect(),
            });
        }
    };
    if chosen != first {
        let (chosen_point, _, read) = read_lantern(&holders[chosen], lantern)?;
        point = chosen_point;
        bytes_read += read;
    }
    let holder = &holders[chosen];
    if holder.index.row(&point).is_none() {
        return Err(HomeError::UnknownEntry {
            session: holder.id.clone(),
            id: point,
        });
    }
    let rows = holder.index.ancestry(&point)?;
    let (mut entries, read) = holder.index.read_rows_from(&holder.file, &rows)?;
    bytes_read += read;
    let Some(end) = entries
        .iter()
        .rposition(|entry| role_of(entry) == Some("assistant"))
    else {
        return Err(HomeError::NothingToFork {
            lantern: lantern.to_owned(),
        });
    };
    let carried = entries
        .last()
        .filter(|entry| role_of(entry) == Some("user"))
        .cloned();
    entries.truncate(end + 1);
    let rows: Vec<IndexRow> = rows[..=end].iter().map(|row| (*row).clone()).collect();
    let cut_at = entries[end].id().to_owned();
    Ok(Cut {
        session: holder.id.clone(),
        file: holder.file.clone(),
        lantern: lantern.to_owned(),
        point,
        lit_in,
        rows,
        entries,
        cut_at,
        carried,
        bytes_read,
    })
}
