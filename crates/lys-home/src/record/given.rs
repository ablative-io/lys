//! The `lys.given` entry (HOME-003 R3): the context record of what a session
//! was given at render, as paths, lengths and hashes only.
//!
//! One custom entry rides inside Pi's `custom` type under `lys.given`, with
//! data exactly `{harness, harness_version, kinds, config_dir, documents,
//! environment}`: the harness name; the version its load order was measured
//! on; the kinds this entry resolves and the kinds it leaves unlisted
//! (`claude_md_imports` and `claude_rules`, which reach the request only by
//! the harness reading a document, and which a later entry at the first
//! request records from the request as the harness resolved it); the config
//! directory with its source; the documents in the measured order, each as
//! kind, path, byte length and SHA-256; and the names of the environment
//! variables the template set, sorted. No document content and no variable value is
//! carried, nothing is encrypted, and no field is added outside
//! `custom.data`. The entry itself is never signed: a render given a key
//! signs the record's canonical bytes as a separate `lys.given_statement`
//! (see [`crate::record::given_statement`]).
//!
//! The canonical bytes are RFC 8785 applied to the data: `serde_json`'s map
//! is ordered by key (the `preserve_order` feature is off in this
//! workspace), its compact writer puts no whitespace between tokens, escapes
//! only the quotation mark, the reverse solidus and control characters (the
//! five short forms, otherwise `\u00` and two lowercase hex digits), and
//! writes every other character as UTF-8. The data holds only strings,
//! unsigned integers, arrays and objects whose keys are fixed ASCII names,
//! so key order by byte equals RFC 8785's order by UTF-16 unit. A literal
//! vector in the tests pins these bytes.
//!
//! The entry is appended as the child of the render event it follows. That
//! event stands beside the context path, so the given entry stands beside it
//! too and the head does not move: a second render of the same session
//! records the same session head hash. Given entries are read back with
//! `customs_everywhere`, in file order.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::HomeError;
use crate::harness::claude_code::HARNESS;
use crate::harness::claude_code::given::{ConfigDir, GivenDocument, MEASURED_VERSION, Resolution};
use crate::record::Home;
use crate::record::Session;
use crate::record::blocks::Hash;
use crate::record::entries::{CUSTOM_GIVEN, Entry, EntryBody};
use crate::record::reader::SessionReader;
use crate::record::recall::{RecallReport, Skipped, rows_of};

/// The kinds a given entry resolves, in the order the record names them.
pub const RESOLVED_KINDS: [&str; 6] = [
    "claude_md_chain",
    "user_claude_md",
    "memory_index",
    "appended_instructions",
    "mcp_config",
    "environment_names",
];
/// The kinds a given entry leaves unlisted, recorded by a later entry at the
/// first request.
pub const UNLISTED_KINDS: [&str; 2] = ["claude_md_imports", "claude_rules"];

/// Which kinds an entry resolves and which it leaves unlisted, so a reader
/// can tell a kind missing from the entry from one absent from the session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kinds {
    /// The kinds this entry looked for.
    pub resolved: Vec<String>,
    /// The kinds this entry does not list.
    pub unlisted: Vec<String>,
}

impl Kinds {
    /// The six resolved and two unlisted kinds of this measurement.
    #[must_use]
    pub fn measured() -> Self {
        Self {
            resolved: RESOLVED_KINDS.iter().map(|k| (*k).to_owned()).collect(),
            unlisted: UNLISTED_KINDS.iter().map(|k| (*k).to_owned()).collect(),
        }
    }
}

/// The RFC 8785 bytes of a given record's data, made only from a record, so
/// nothing else can be handed to a signer in their place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalGiven(Vec<u8>);

impl CanonicalGiven {
    /// The bytes, with no trailing newline.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The given hash: the SHA-256 of the bytes.
    #[must_use]
    pub fn hash(&self) -> Hash {
        Hash::of(&self.0)
    }
}

/// The data of a `lys.given` entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GivenRecord {
    /// The harness, `claude-code`.
    pub harness: String,
    /// The harness version the load order was measured on.
    pub harness_version: String,
    /// The kinds resolved and unlisted.
    pub kinds: Kinds,
    /// The config directory and where it came from.
    pub config_dir: ConfigDir,
    /// The documents, in the measured order.
    pub documents: Vec<GivenDocument>,
    /// The names of the environment variables the template set, never a value.
    pub environment: Vec<String>,
}

impl GivenRecord {
    /// The record of a Claude Code render: the resolution's config directory
    /// and documents, and the variable names the template set, sorted.
    #[must_use]
    pub fn claude_code(resolution: Resolution, mut environment: Vec<String>) -> Self {
        environment.sort_unstable();
        Self {
            harness: HARNESS.to_owned(),
            harness_version: MEASURED_VERSION.to_owned(),
            kinds: Kinds::measured(),
            config_dir: resolution.config_dir,
            documents: resolution.documents,
            environment,
        }
    }

    /// The data as it rides in the entry.
    pub fn data(&self) -> Result<Value, HomeError> {
        serde_json::to_value(self).map_err(|source| HomeError::Json {
            context: "a given record could not be serialised",
            source,
        })
    }

    /// The canonical bytes of the data, the payload a given statement signs.
    pub fn canonical_bytes(&self) -> Result<CanonicalGiven, HomeError> {
        serde_json::to_vec(&self.data()?)
            .map(CanonicalGiven)
            .map_err(|source| HomeError::Json {
                context: "a given record's canonical bytes could not be written",
                source,
            })
    }

    /// The given hash: the SHA-256 of the canonical bytes.
    pub fn given_hash(&self) -> Result<Hash, HomeError> {
        Ok(self.canonical_bytes()?.hash())
    }

    /// Append the record as a `lys.given` entry under `parent`, the render
    /// event it follows, leaving the head where it stands. Returns the id.
    pub fn append_under(&self, session: &mut Session, parent: &str) -> Result<String, HomeError> {
        session.append_under(
            parent,
            EntryBody::Custom {
                custom_type: CUSTOM_GIVEN.to_owned(),
                data: Some(self.data()?),
            },
        )
    }

    /// The record a `lys.given` entry carries; an entry of any other kind is
    /// refused by id.
    pub fn from_entry(entry: &Entry) -> Result<Self, HomeError> {
        let EntryBody::Custom {
            custom_type,
            data: Some(data),
        } = &entry.body
        else {
            return Err(not_given(entry));
        };
        if custom_type != CUSTOM_GIVEN {
            return Err(not_given(entry));
        }
        serde_json::from_value(data.clone()).map_err(|source| HomeError::Json {
            context: "a lys.given entry's data is not the record's shape",
            source,
        })
    }

    /// Every `lys.given` entry of the session in file order, each with its id.
    pub fn read_all(session: &Session) -> Result<Vec<(String, Self)>, HomeError> {
        session
            .customs_everywhere(CUSTOM_GIVEN)?
            .iter()
            .map(|entry| Ok((entry.id().to_owned(), Self::from_entry(entry)?)))
            .collect()
    }
}

/// A `lys.given` entry as it was found: where it stands and when it was
/// appended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GivenSeen {
    /// The session it stands in.
    pub session: String,
    /// The entry's id.
    pub entry: String,
    /// When it was appended, RFC 3339 as the home wrote it.
    pub given_at: String,
    /// What was given.
    pub record: GivenRecord,
}

/// The `lys.given` entry of the home appended last, and the sessions that
/// could not be read for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GivenLast {
    /// The entry appended last, none while nothing was given.
    pub last: Option<GivenSeen>,
    /// Each session skipped, with the reason.
    pub skipped: Vec<Skipped>,
}

fn given_of(reader: &SessionReader, session: &str) -> Result<Vec<GivenSeen>, HomeError> {
    reader
        .customs_everywhere(CUSTOM_GIVEN)?
        .iter()
        .map(|entry| {
            Ok(GivenSeen {
                session: session.to_owned(),
                entry: entry.id().to_owned(),
                given_at: entry.base.timestamp.clone(),
                record: GivenRecord::from_entry(entry)?,
            })
        })
        .collect()
}

/// The `lys.given` entry of the home appended last, read by seeking to the
/// given rows only. The home writes every timestamp in one RFC 3339 form in
/// UTC, so the last is the greatest as text. A session that cannot be read
/// is skipped and named.
pub fn last_given(home: &Home) -> Result<GivenLast, HomeError> {
    let mut last: Option<GivenSeen> = None;
    let mut skipped = Vec::new();
    for session in home.session_ids()? {
        let given = home
            .read_session(&session)
            .and_then(|reader| given_of(&reader, &session));
        match given {
            Ok(given) => keep_last(&mut last, given),
            Err(e) => skipped.push(Skipped {
                session,
                reason: e.to_string(),
            }),
        }
    }
    Ok(GivenLast { last, skipped })
}

/// Keep in `last` the entry of `given` appended last, the later of two
/// appended at the same moment.
fn keep_last(last: &mut Option<GivenSeen>, given: Vec<GivenSeen>) {
    for seen in given {
        if last
            .as_ref()
            .is_none_or(|kept| kept.given_at <= seen.given_at)
        {
            *last = Some(seen);
        }
    }
}

/// Every lantern of the home and its `lys.given` entry appended last, read
/// in one pass: each session is opened once and both are read from that one
/// reader. The answer is exactly what [`recall_all`](crate::recall_all) and
/// [`last_given`] answer, each with the sessions it could not read.
pub fn recall_and_last_given(home: &Home) -> Result<(RecallReport, GivenLast), HomeError> {
    recall_and_last_given_through(home, |session| home.read_session(session))
}

/// As [`recall_and_last_given`], each session opened through `open`, which
/// is asked once for each session the home lists, and never again.
pub fn recall_and_last_given_through(
    home: &Home,
    mut open: impl FnMut(&str) -> Result<SessionReader, HomeError>,
) -> Result<(RecallReport, GivenLast), HomeError> {
    let mut recalled = RecallReport {
        lanterns: Vec::new(),
        skipped: Vec::new(),
    };
    let mut given = GivenLast {
        last: None,
        skipped: Vec::new(),
    };
    for session in home.session_ids()? {
        let reader = match open(&session) {
            Ok(reader) => reader,
            Err(e) => {
                let reason = e.to_string();
                recalled.skipped.push(Skipped {
                    session: session.clone(),
                    reason: reason.clone(),
                });
                given.skipped.push(Skipped { session, reason });
                continue;
            }
        };
        match rows_of(home, &reader, &session) {
            Ok(rows) => recalled.lanterns.extend(rows),
            Err(e) => recalled.skipped.push(Skipped {
                session: session.clone(),
                reason: e.to_string(),
            }),
        }
        match given_of(&reader, &session) {
            Ok(seen) => keep_last(&mut given.last, seen),
            Err(e) => given.skipped.push(Skipped {
                session,
                reason: e.to_string(),
            }),
        }
    }
    Ok((recalled, given))
}

fn not_given(entry: &Entry) -> HomeError {
    HomeError::NotOfCustomType {
        id: entry.id().to_owned(),
        custom_type: CUSTOM_GIVEN,
    }
}
