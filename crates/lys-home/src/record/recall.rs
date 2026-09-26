//! Recalling lanterns by note and by point (HOME-004 R5).
//!
//! Recall reads through the read-only reader only: no lock is taken and no
//! file is written. By note, every session of the home is read and every
//! lantern is listed whose note, or one of whose epilogues' words, contains
//! the given words as one contiguous phrase, case-folded with
//! `str::to_lowercase` on both sides; a phrase is never matched across the
//! note and an epilogue or across two epilogues, and nothing is stemmed,
//! ranked or scored. By point, the lanterns of one session marking one entry
//! are listed, and an entry the session holds with no lantern on it lists
//! nothing rather than refusing. Rows are ordered by session id, then by
//! the lantern's position in its file; each carries the lantern's own
//! epilogues in file order. A session that cannot be read during recall by
//! note is skipped and named with its reason, and every other session is
//! still listed. A row carries the lantern's note and epilogues, the one
//! text the crate prints (ADR-014), and never a line of the transcript.

use serde::{Deserialize, Serialize};

use crate::error::HomeError;
use crate::record::Home;
use crate::record::entries::{CUSTOM_LANTERN, CUSTOM_LANTERN_EPILOGUE, EntryBody, LanternData};
use crate::record::epilogue::epilogue_of;
use crate::record::lantern::{require_words, session_file};
use crate::record::reader::SessionReader;

/// One epilogue of a lantern, as recall lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Epilogue {
    /// Who added it, as they named themselves.
    pub added_by: String,
    /// When.
    pub added_at: String,
    /// The words, as written.
    pub words: String,
}

/// One lantern, as recall lists it, with its epilogues in file order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LanternRow {
    /// The lantern's entry id.
    pub id: String,
    /// The session it stands in.
    pub session: String,
    /// The entry it marks.
    pub point: String,
    /// Who lit it, as they named themselves.
    pub lit_by: String,
    /// When.
    pub lit_at: String,
    /// The note, as written.
    pub note: String,
    /// Its epilogues, in file order.
    pub epilogues: Vec<Epilogue>,
}

/// A session recall by note could not read, with the reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The session id.
    pub session: String,
    /// The refusal's display text, which names no content.
    pub reason: String,
}

/// What a recall lists: the rows, and the sessions it could not read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecallReport {
    /// The lanterns found, by session id then file position.
    pub lanterns: Vec<LanternRow>,
    /// The sessions skipped, each with its reason.
    pub skipped: Vec<Skipped>,
}

/// The lantern data of an entry, when it is one.
fn lantern_of(body: &EntryBody) -> Option<LanternData> {
    match body {
        EntryBody::Custom {
            custom_type,
            data: Some(data),
        } if custom_type == CUSTOM_LANTERN => LanternData::deserialize(data).ok(),
        _ => None,
    }
}

/// Every lantern of one session with its epilogues, in file order, read by
/// seeking to the lantern and epilogue rows only.
fn rows_of(reader: &SessionReader) -> Result<Vec<LanternRow>, HomeError> {
    let session = reader.header().id.clone();
    let epilogues = reader.customs_everywhere(CUSTOM_LANTERN_EPILOGUE)?;
    let mut rows = Vec::new();
    for entry in reader.customs_everywhere(CUSTOM_LANTERN)? {
        let Some(data) = lantern_of(&entry.body) else {
            continue;
        };
        let own: Vec<Epilogue> = epilogues
            .iter()
            .filter_map(|e| epilogue_of(&e.body))
            .filter(|e| e.lantern == entry.id())
            .map(|e| Epilogue {
                added_by: e.added_by,
                added_at: e.added_at,
                words: e.words,
            })
            .collect();
        rows.push(LanternRow {
            id: entry.id().to_owned(),
            session: session.clone(),
            point: data.point,
            lit_by: data.lit_by,
            lit_at: data.lit_at,
            note: data.note,
            epilogues: own,
        });
    }
    Ok(rows)
}

/// Whether the folded phrase occurs inside the note or inside one epilogue.
fn mentions(row: &LanternRow, folded: &str) -> bool {
    row.note.to_lowercase().contains(folded)
        || row
            .epilogues
            .iter()
            .any(|e| e.words.to_lowercase().contains(folded))
}

/// Every lantern of the home whose note or one of whose epilogues contains
/// `words` as one contiguous phrase, case-folded. Blank words are refused
/// by name. A session that cannot be read is skipped and named.
pub fn recall_by_note(home: &Home, words: &str) -> Result<RecallReport, HomeError> {
    require_words("words", words)?;
    let folded = words.to_lowercase();
    let mut lanterns = Vec::new();
    let mut skipped = Vec::new();
    for session in home.session_ids()? {
        let rows = home
            .read_session(&session)
            .and_then(|reader| rows_of(&reader));
        match rows {
            Ok(rows) => lanterns.extend(rows.into_iter().filter(|row| mentions(row, &folded))),
            Err(e) => skipped.push(Skipped {
                session,
                reason: e.to_string(),
            }),
        }
    }
    Ok(RecallReport { lanterns, skipped })
}

/// Every lantern of `session` whose point is `point`. A session with no
/// file is refused by name, as is an entry the session does not hold; an
/// entry with no lantern on it lists nothing.
pub fn recall_by_point(home: &Home, session: &str, point: &str) -> Result<RecallReport, HomeError> {
    let reader = SessionReader::open(session_file(home, session)?)?;
    if !reader.contains(point) {
        return Err(HomeError::UnknownEntry {
            session: session.to_owned(),
            id: point.to_owned(),
        });
    }
    let lanterns = rows_of(&reader)?
        .into_iter()
        .filter(|row| row.point == point)
        .collect();
    Ok(RecallReport {
        lanterns,
        skipped: Vec::new(),
    })
}
