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
//! carried, nothing is signed or encrypted, and no field is added outside
//! `custom.data`, so the shape can be signed over and encrypted at rest
//! later without changing what is recorded.
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
use crate::record::Session;
use crate::record::entries::{CUSTOM_GIVEN, Entry, EntryBody};

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

fn not_given(entry: &Entry) -> HomeError {
    HomeError::NotOfCustomType {
        id: entry.id().to_owned(),
        custom_type: CUSTOM_GIVEN,
    }
}
