//! A strict verification of a home (HOME-019 R2): what ship refuses before
//! it snapshots and what fetch refuses before it records an arrival.
//!
//! Each session, in the order the home lists them, gets at most one reason,
//! the first that applies: `index_missing` (no `sessions/<id>.index.jsonl`),
//! `index_not_this_file` (a row does not parse, or the rows are not the
//! session file's), `head_missing` (no `sessions/<id>.head`),
//! `head_not_indexed` (the head names a non-empty id the index does not
//! hold). The index is taken only as it stands beside the file: it is never
//! rebuilt by scanning the session file, in memory or on disk, so an index
//! that lags its file is named and not quietly replaced. Blocks and
//! templates, when asked for, are each hashed and every file of the
//! tracked set whose bytes do not hash to its name is named by that name.
//!
//! Nothing is written, renamed or removed, and the result holds session
//! ids, reasons and hashes only: never a line, an entry's data or a block's
//! or template's bytes.

use serde::Serialize;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::index::Index;
use crate::record::tracked::tracked_set;

/// Why a session is not fit to ship or to arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// `sessions/<id>.index.jsonl` does not exist.
    IndexMissing,
    /// The index does not parse, or its rows are not the session file's.
    IndexNotThisFile,
    /// `sessions/<id>.head` does not exist.
    HeadMissing,
    /// The head names an id the index does not hold.
    HeadNotIndexed,
}

impl Reason {
    /// The reason's name as reports print it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IndexMissing => "index_missing",
            Self::IndexNotThisFile => "index_not_this_file",
            Self::HeadMissing => "head_missing",
            Self::HeadNotIndexed => "head_not_indexed",
        }
    }
}

/// One session and the first reason that applies to it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionReason {
    /// The session id.
    pub session: String,
    /// The reason.
    pub reason: Reason,
}

/// What a verification found: sessions with a reason, and the names of the
/// blocks and templates whose bytes do not hash to their names, each in
/// ascending byte order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Verification {
    /// Each session with a reason, in the order the home lists them.
    pub sessions: Vec<SessionReason>,
    /// Each block whose bytes do not hash to its name.
    pub bad_blocks: Vec<String>,
    /// Each template whose bytes do not hash to its name.
    pub bad_templates: Vec<String>,
}

impl Verification {
    /// Whether nothing was found.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.sessions.is_empty() && self.bad_blocks.is_empty() && self.bad_templates.is_empty()
    }
}

/// Verify every session of the home, and nothing of its stores.
pub fn verify_sessions(home: &Home) -> Result<Vec<SessionReason>, HomeError> {
    let mut out = Vec::new();
    for session in home.session_ids()? {
        if let Some(reason) = session_reason(home, &session)? {
            out.push(SessionReason { session, reason });
        }
    }
    Ok(out)
}

/// Verify every session of the home, then every block and template of its
/// tracked set.
pub fn verify_home(home: &Home) -> Result<Verification, HomeError> {
    let sessions = verify_sessions(home)?;
    let mut bad_blocks = Vec::new();
    let mut bad_templates = Vec::new();
    for path in tracked_set(home)? {
        let (bad, name) = if let Some(rest) = path.strip_prefix("blocks/") {
            (&mut bad_blocks, rest)
        } else if let Some(rest) = path.strip_prefix("templates/") {
            (&mut bad_templates, rest)
        } else {
            continue;
        };
        let Some((_, name)) = name.split_once('/') else {
            continue;
        };
        let file = home.root().join(&path);
        let bytes =
            std::fs::read(&file).map_err(|e| HomeError::io("reading a stored object", &file, e))?;
        if Hash::of(&bytes).as_str() != name {
            bad.push(name.to_owned());
        }
    }
    Ok(Verification {
        sessions,
        bad_blocks,
        bad_templates,
    })
}

/// The first reason that applies to one session, or `None`.
fn session_reason(home: &Home, session: &str) -> Result<Option<Reason>, HomeError> {
    let file = home.session_path(session)?;
    if !Index::index_path(&file).is_file() {
        return Ok(Some(Reason::IndexMissing));
    }
    let Some(index) = Index::cached_only(&file)? else {
        return Ok(Some(Reason::IndexNotThisFile));
    };
    let head_file = Index::head_path(&file);
    let bytes = match std::fs::read(&head_file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(Reason::HeadMissing));
        }
        Err(e) => return Err(HomeError::io("reading the head", &head_file, e)),
    };
    let text = String::from_utf8_lossy(&bytes);
    let head = text.trim();
    if !head.is_empty() && index.row(head).is_none() {
        return Ok(Some(Reason::HeadNotIndexed));
    }
    Ok(None)
}
