//! Adding an epilogue to a lantern (HOME-004 R4).
//!
//! An epilogue is a `lys.lantern_epilogue` custom entry appended as a child
//! of the head of the lantern's own session, naming the lantern's entry id
//! and carrying the further words byte for byte, who added them and when
//! (ADR-014). The lantern entry and every earlier epilogue stay as they are;
//! a lantern's story is its entry followed by its epilogues in file order.
//! Entry ids are unique only within a session, so an epilogue is addressed
//! by the session and the lantern's id together. The session is opened as
//! its one owner first, then the lantern is checked (its type and its data's
//! shape), then the words; every
//! refusal happens before the append and writes nothing; no error carries
//! the words.

use serde::Serialize;

use crate::error::HomeError;
use crate::record::entries::{
    CUSTOM_LANTERN, CUSTOM_LANTERN_EPILOGUE, Entry, EntryBody, EpilogueData,
};
use crate::record::lantern::{data_of, lantern_of, own, require_words};
use crate::record::{Home, Session, now};

/// What adding an epilogue reports: its entry id, the lantern, the session,
/// who added it, when, and its ordinal among the lantern's epilogues in
/// file order, counted from 1. Never the words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Added {
    /// The epilogue's entry id.
    pub id: String,
    /// The lantern it belongs to.
    pub lantern: String,
    /// The session both stand in.
    pub session: String,
    /// Who added it, as they named themselves.
    pub added_by: String,
    /// When.
    pub added_at: String,
    /// Which epilogue of the lantern this is, from 1.
    pub ordinal: usize,
}

/// The epilogue data of a `lys.lantern_epilogue` entry of `session`, refused
/// by name when the entry is not one or its data is not the epilogue shape.
pub fn epilogue_of(session: &str, entry: &Entry) -> Result<EpilogueData, HomeError> {
    data_of(session, entry, CUSTOM_LANTERN_EPILOGUE)
}

/// How many epilogues of `lantern` the session already holds; an epilogue
/// entry whose data is not the epilogue shape refuses by name.
fn epilogues_so_far(owner: &Session, session: &str, lantern: &str) -> Result<usize, HomeError> {
    let mut count = 0;
    for entry in owner.customs_everywhere(CUSTOM_LANTERN_EPILOGUE)? {
        if epilogue_of(session, &entry)?.lantern == lantern {
            count += 1;
        }
    }
    Ok(count)
}

/// Add `words` as an epilogue to the lantern `lantern` of `session`, by `by`.
pub fn add_epilogue(
    home: &Home,
    session: &str,
    lantern: &str,
    words: &str,
    by: &str,
) -> Result<Added, HomeError> {
    let mut owner = own(home, session)?;
    if !owner.contains(lantern)? {
        return Err(HomeError::UnknownLantern {
            session: session.to_owned(),
            id: lantern.to_owned(),
        });
    }
    let target = owner.entry(lantern)?;
    if !target.is_custom(CUSTOM_LANTERN) {
        return Err(HomeError::UnknownLantern {
            session: session.to_owned(),
            id: lantern.to_owned(),
        });
    }
    // A lantern whose data is not the lantern shape is refused by name, not
    // taken on its type alone.
    lantern_of(session, &target)?;
    require_words("epilogue", words)?;
    let ordinal = epilogues_so_far(&owner, session, lantern)? + 1;
    let data = EpilogueData {
        lantern: lantern.to_owned(),
        words: words.to_owned(),
        added_by: by.to_owned(),
        added_at: now(),
    };
    let value = serde_json::to_value(&data).map_err(|source| HomeError::Json {
        context: "the epilogue's data could not be serialised",
        source,
    })?;
    let id = owner.append(EntryBody::Custom {
        custom_type: CUSTOM_LANTERN_EPILOGUE.to_owned(),
        data: Some(value),
    })?;
    Ok(Added {
        id,
        lantern: lantern.to_owned(),
        session: session.to_owned(),
        added_by: data.added_by,
        added_at: data.added_at,
        ordinal,
    })
}
