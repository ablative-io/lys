//! Lighting a lantern at an entry of a session (HOME-004 R3).
//!
//! A lantern is a `lys.lantern` custom entry appended as a child of the
//! session's head, carrying the point it marks, the note byte for byte as
//! given, who lit it and when (ADR-014). The point may be any entry of the
//! session that is not itself a lantern or an epilogue, the head or one the
//! head has moved past. Lighting takes the session as its one owner, then
//! checks the point, then the note, in that order, and appends through the
//! record's own path, so the line is durable before its index row and the
//! head; every refusal happens before the append, and nothing of the session
//! file changes on one. No error carries the note.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::entries::{
    CUSTOM_LANTERN, CUSTOM_LANTERN_EPILOGUE, Entry, EntryBody, LanternData,
};
use crate::record::{Home, Session, now};

/// What lighting reports: the lantern's entry id, its session, its point and
/// when it was lit. Never the note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lit {
    /// The lantern's entry id.
    pub id: String,
    /// The session it stands in.
    pub session: String,
    /// The entry it marks.
    pub point: String,
    /// When it was lit.
    pub lit_at: String,
}

/// The data of a custom entry of `custom_type`, as `T`; an entry of another
/// type, or one whose data is not that shape, is refused by name.
pub(crate) fn data_of<T: for<'de> Deserialize<'de>>(
    session: &str,
    entry: &Entry,
    custom_type: &str,
) -> Result<T, HomeError> {
    let shape = |source| HomeError::EntryShape {
        session: session.to_owned(),
        id: entry.id().to_owned(),
        custom_type: custom_type.to_owned(),
        source,
    };
    match &entry.body {
        EntryBody::Custom {
            custom_type: found,
            data: Some(data),
        } if found == custom_type => T::deserialize(data).map_err(|e| shape(Some(e))),
        _ => Err(shape(None)),
    }
}

/// The lantern data of a `lys.lantern` entry of `session`, refused by name
/// when the entry is not one or its data is not the lantern shape.
pub fn lantern_of(session: &str, entry: &Entry) -> Result<LanternData, HomeError> {
    data_of(session, entry, CUSTOM_LANTERN)
}

/// The session file of `id` in the home, refused by name when no such file
/// stands under `sessions/`, before any lock is taken or file created.
pub(crate) fn session_file(home: &Home, id: &str) -> Result<PathBuf, HomeError> {
    let file = home.session_path(id)?;
    if file.is_file() {
        Ok(file)
    } else {
        Err(HomeError::UnknownSession {
            session: id.to_owned(),
        })
    }
}

/// Open the session as its one owner, refusing an absent session by name.
pub(crate) fn own(home: &Home, id: &str) -> Result<Session, HomeError> {
    Session::open(session_file(home, id)?)
}

/// Refuse text that is empty or only whitespace, naming what it was.
pub(crate) fn require_words(what: &'static str, text: &str) -> Result<(), HomeError> {
    if text.trim().is_empty() {
        Err(HomeError::EmptyNote { what })
    } else {
        Ok(())
    }
}

/// Light a lantern at `point` of `session` with `note`, lit by `by`.
pub fn light(
    home: &Home,
    session: &str,
    point: &str,
    note: &str,
    by: &str,
) -> Result<Lit, HomeError> {
    let mut owner = own(home, session)?;
    let target = owner.entry(point)?;
    if target.is_custom(CUSTOM_LANTERN) || target.is_custom(CUSTOM_LANTERN_EPILOGUE) {
        return Err(HomeError::PointIsLantern {
            session: session.to_owned(),
            id: point.to_owned(),
        });
    }
    require_words("note", note)?;
    let data = LanternData {
        point: point.to_owned(),
        note: note.to_owned(),
        lit_by: by.to_owned(),
        lit_at: now(),
    };
    let value = serde_json::to_value(&data).map_err(|source| HomeError::Json {
        context: "the lantern's data could not be serialised",
        source,
    })?;
    let id = owner.append(EntryBody::Custom {
        custom_type: CUSTOM_LANTERN.to_owned(),
        data: Some(value),
    })?;
    Ok(Lit {
        id,
        session: session.to_owned(),
        point: point.to_owned(),
        lit_at: data.lit_at,
    })
}
