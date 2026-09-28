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
//! note, or that holds a lantern or epilogue entry whose data is not its
//! shape, is skipped and named with its reason, and every other session is
//! still listed; recall by point refuses such an entry by name. A row carries the lantern's note and epilogues, the one
//! text the crate prints (ADR-015), and never a line of the transcript.
//! Each row carries the lantern's `lit_in`: null for a lantern lit before
//! the light act recorded it, which is listed as any other, and otherwise
//! the session the check in `lit_in_session` returns; a recorded `lit_in`
//! that is not a session id is skipped by note and refused by point as
//! `lit_in_not_a_session`, and never listed as a row.

use serde::Serialize;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::entries::{CUSTOM_LANTERN, CUSTOM_LANTERN_EPILOGUE};
use crate::record::epilogue::epilogue_of;
use crate::record::lantern::{lantern_of, lit_in_session, require_words, session_file};
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
    /// The session its data records it was lit in; `None`, printed as
    /// null, for a lantern whose data records none.
    pub lit_in: Option<String>,
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

/// Every lantern of one session with its epilogues, in file order, read by
/// seeking to the lantern and epilogue rows only. `session` is the id the
/// home lists the file under, and is what every row carries. A lantern or
/// epilogue entry whose data is not its shape refuses by name, as does a
/// lantern whose recorded `lit_in` is not a session of `home`.
pub(crate) fn rows_of(
    home: &Home,
    reader: &SessionReader,
    session: &str,
) -> Result<Vec<LanternRow>, HomeError> {
    let epilogues = reader
        .customs_everywhere(CUSTOM_LANTERN_EPILOGUE)?
        .iter()
        .map(|entry| epilogue_of(session, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    for entry in reader.customs_everywhere(CUSTOM_LANTERN)? {
        let data = lantern_of(session, &entry)?;
        let lit_in = data
            .lit_in
            .map(|lit_in| lit_in_session(home, entry.id(), &lit_in))
            .transpose()?;
        let own: Vec<Epilogue> = epilogues
            .iter()
            .filter(|e| e.lantern == entry.id())
            .map(|e| Epilogue {
                added_by: e.added_by.clone(),
                added_at: e.added_at.clone(),
                words: e.words.clone(),
            })
            .collect();
        rows.push(LanternRow {
            id: entry.id().to_owned(),
            session: session.to_owned(),
            point: data.point,
            lit_by: data.lit_by,
            lit_at: data.lit_at,
            lit_in,
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

/// Every lantern of the home, by session id then file position. A session
/// that cannot be read is skipped and named.
pub fn recall_all(home: &Home) -> Result<RecallReport, HomeError> {
    recall_where(home, |_row| true)
}

/// Every lantern of the home whose note or one of whose epilogues contains
/// `words` as one contiguous phrase, case-folded. Blank words are refused
/// by name. A session that cannot be read is skipped and named.
pub fn recall_by_note(home: &Home, words: &str) -> Result<RecallReport, HomeError> {
    require_words("words", words)?;
    let folded = words.to_lowercase();
    recall_where(home, |row| mentions(row, &folded))
}

/// Every lantern of the home that `keep` keeps.
fn recall_where(
    home: &Home,
    keep: impl Fn(&LanternRow) -> bool,
) -> Result<RecallReport, HomeError> {
    let mut lanterns = Vec::new();
    let mut skipped = Vec::new();
    for session in home.session_ids()? {
        let rows = home
            .read_session(&session)
            .and_then(|reader| rows_of(home, &reader, &session));
        match rows {
            Ok(rows) => lanterns.extend(rows.into_iter().filter(|row| keep(row))),
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
    let lanterns = rows_of(home, &reader, session)?
        .into_iter()
        .filter(|row| row.point == point)
        .collect();
    Ok(RecallReport {
        lanterns,
        skipped: Vec::new(),
    })
}
