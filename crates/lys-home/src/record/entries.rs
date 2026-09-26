//! The entries of a session file, in Pi's grammar.
//!
//! Read from the Pi checkout at `3d5cbe98`
//! (`packages/coding-agent/src/core/session-manager.ts`): a `session` header
//! line, then entries that each carry `type`, `id`, `parentId` and `timestamp`.
//! Only the fields this crate acts on are typed; every other field an entry
//! carries is kept verbatim in `rest`, so a file round-trips without loss and
//! nothing is added to Pi's grammar. lys's own data rides in [`EntryBody::Custom`]
//! under the `lys.*` custom types named below.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The custom type of a harness-local event (a hook outcome, a permission
/// mode, a tool completion) attached under the message it followed.
pub const CUSTOM_HARNESS_EVENT: &str = "lys.harness_event";
/// The custom type of a proxy call record (see [`crate::record::call`]).
pub const CUSTOM_CALL: &str = "lys.call";
/// The custom type marking a session as authored by hand: a demonstration,
/// never a history of tools that ran.
pub const CUSTOM_AUTHORED: &str = "lys.authored";
/// The custom type marking an entry inherited from another session.
pub const CUSTOM_INHERITED: &str = "lys.inherited";
/// The custom type of the context record: what a session was given at
/// render, as paths, lengths and hashes (see [`crate::record::given`]).
pub const CUSTOM_GIVEN: &str = "lys.given";

/// The first line of a session file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "session")]
pub struct SessionHeader {
    /// Pi's file format version; 2 is the tree format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// The session id.
    pub id: String,
    /// When the session began, RFC 3339.
    pub timestamp: String,
    /// The working directory the session was started in.
    pub cwd: String,
    /// The session this one was branched from, when it was.
    #[serde(
        rename = "parentSession",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_session: Option<String>,
}

/// What every entry carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryBase {
    /// The entry's id, unique within the session.
    pub id: String,
    /// The entry this one follows; `None` only for the first entry.
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    /// When it was appended, RFC 3339.
    pub timestamp: String,
}

/// The kinds of entry, tagged by Pi's `type` values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntryBody {
    /// A message in the conversation: Pi's `AgentMessage`, kept as JSON.
    Message {
        /// The message.
        message: Value,
    },
    /// The model changed.
    ModelChange {
        /// The provider.
        provider: String,
        /// The model.
        #[serde(rename = "modelId")]
        model_id: String,
    },
    /// The thinking level changed.
    ThinkingLevelChange {
        /// The level.
        #[serde(rename = "thinkingLevel")]
        thinking_level: Value,
    },
    /// Usage accounting.
    Usage {
        /// Pi's fields, verbatim.
        #[serde(flatten)]
        rest: Map<String, Value>,
    },
    /// A compaction: a summary replacing the entries before `first_kept_entry_id`;
    /// the originals stay in the file.
    Compaction {
        /// The summary.
        summary: String,
        /// The first entry the compaction keeps.
        #[serde(rename = "firstKeptEntryId")]
        first_kept_entry_id: String,
        /// Tokens before compaction.
        #[serde(rename = "tokensBefore")]
        tokens_before: u64,
        /// Pi's other fields, verbatim.
        #[serde(flatten)]
        rest: Map<String, Value>,
    },
    /// The work of an abandoned branch carried into this one.
    BranchSummary {
        /// The entry the branch left from.
        #[serde(rename = "fromId")]
        from_id: String,
        /// The summary.
        summary: String,
        /// Pi's other fields, verbatim.
        #[serde(flatten)]
        rest: Map<String, Value>,
    },
    /// A label on an entry.
    Label {
        /// The entry labelled.
        #[serde(rename = "targetId")]
        target_id: String,
        /// The label, or `None` to clear it.
        #[serde(default)]
        label: Option<String>,
    },
    /// Session metadata.
    SessionInfo {
        /// A display name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Other fields, verbatim.
        #[serde(flatten)]
        rest: Map<String, Value>,
    },
    /// Extension data that is not part of the model's context.
    Custom {
        /// Which extension's entry this is.
        #[serde(rename = "customType")]
        custom_type: String,
        /// Its data.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
    },
    /// Extension data that becomes a user message in the model's context.
    CustomMessage {
        /// Which extension's entry this is.
        #[serde(rename = "customType")]
        custom_type: String,
        /// The content.
        content: Value,
        /// Pi's other fields, verbatim.
        #[serde(flatten)]
        rest: Map<String, Value>,
    },
    /// A context edit.
    ContextEdit {
        /// Pi's fields, verbatim.
        #[serde(flatten)]
        rest: Map<String, Value>,
    },
}

/// One line of a session file after the header.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Id, parent and timestamp.
    #[serde(flatten)]
    pub base: EntryBase,
    /// The typed body.
    #[serde(flatten)]
    pub body: EntryBody,
}

impl Entry {
    /// The entry's id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.base.id
    }

    /// The parent's id, if any.
    #[must_use]
    pub fn parent_id(&self) -> Option<&str> {
        self.base.parent_id.as_deref()
    }

    /// Whether this is a custom entry of the given custom type.
    #[must_use]
    pub fn is_custom(&self, custom_type: &str) -> bool {
        matches!(&self.body, EntryBody::Custom { custom_type: t, .. } if t == custom_type)
    }
}
