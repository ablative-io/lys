//! Adding an epilogue to a lantern (HOME-004 R4).
//!
//! An epilogue is a `lys.lantern_epilogue` custom entry appended as a child
//! of the head of the lantern's own session, naming the lantern's entry id
//! and carrying the further words byte for byte, who added them and when
//! (ADR-013). The lantern entry and every earlier epilogue stay as they are;
//! a lantern's story is its entry followed by its epilogues in file order.
//! Entry ids are unique only within a session, so an epilogue is addressed
//! by the session and the lantern's id together. Every refusal happens
//! before the append and writes nothing; no error carries the words.

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::entries::{CUSTOM_LANTERN, CUSTOM_LANTERN_EPILOGUE, EntryBody, EpilogueData};
use crate::record::lantern::{own, require_words};
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
    pub ordinal: u64,
}

/// The epilogue data of an entry, when it is one.
#[must_use]
pub fn epilogue_of(body: &EntryBody) -> Option<EpilogueData> {
    match body {
        EntryBody::Custom {
            custom_type,
            data: Some(data),
        } if custom_type == CUSTOM_LANTERN_EPILOGUE => EpilogueData::deserialize(data).ok(),
        _ => None,
    }
}

/// How many epilogues of `lantern` the session already holds.
fn epilogues_so_far(owner: &Session, lantern: &str) -> Result<u64, HomeError> {
    let count = owner
        .customs_everywhere(CUSTOM_LANTERN_EPILOGUE)?
        .iter()
        .filter(|entry| epilogue_of(&entry.body).is_some_and(|data| data.lantern == lantern))
        .count();
    Ok(u64::try_from(count).unwrap_or(u64::MAX))
}

/// Add `words` as an epilogue to the lantern `lantern` of `session`, by `by`.
pub fn add_epilogue(
    home: &Home,
    session: &str,
    lantern: &str,
    words: &str,
    by: &str,
) -> Result<Added, HomeError> {
    require_words("epilogue", words)?;
    let mut owner = own(home, session)?;
    let is_lantern = owner.contains(lantern)? && owner.entry(lantern)?.is_custom(CUSTOM_LANTERN);
    if !is_lantern {
        return Err(HomeError::UnknownLantern {
            session: session.to_owned(),
            id: lantern.to_owned(),
        });
    }
    let ordinal = epilogues_so_far(&owner, lantern)? + 1;
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
