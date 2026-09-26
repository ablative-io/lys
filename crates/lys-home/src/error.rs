//! Errors a home reports. None carries transcript contents: paths, ids, hashes,
//! offsets and counts only.

use std::path::PathBuf;

/// What went wrong, named so a caller can act on it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HomeError {
    /// An I/O operation failed at a named path.
    #[error("{context} at {}: {source}", path.display())]
    Io {
        /// What the operation was doing.
        context: &'static str,
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// A line of a session or index file did not parse.
    #[error("{} line {line} is not a valid {what}: {reason}", path.display())]
    Malformed {
        /// The file.
        path: PathBuf,
        /// The 1-based line.
        line: usize,
        /// What the line was expected to be.
        what: &'static str,
        /// The parser's reason, which names no content.
        reason: String,
    },

    /// The session file's first line is not a `session` header.
    #[error("{} does not begin with a session header: {reason}", path.display())]
    NoHeader {
        /// The file.
        path: PathBuf,
        /// Why, naming no content.
        reason: String,
    },

    /// JSON could not be read or written.
    #[error("{context}: {source}")]
    Json {
        /// What was being read or written.
        context: &'static str,
        /// The parser's or serialiser's own error.
        #[source]
        source: serde_json::Error,
    },

    /// An index row's length does not fit in memory.
    #[error("index row of {len} bytes in {} cannot be read: {source}", path.display())]
    RowTooLong {
        /// The session file.
        path: PathBuf,
        /// The row's length.
        len: u64,
        /// The conversion's error.
        #[source]
        source: std::num::TryFromIntError,
    },

    /// A session file already exists where one would be created.
    #[error("session file already exists: {}", path.display())]
    Exists {
        /// The file.
        path: PathBuf,
    },

    /// An entry id was named that the session does not hold.
    #[error("session {session} holds no entry `{id}`")]
    UnknownEntry {
        /// The session id.
        session: String,
        /// The entry id named.
        id: String,
    },

    /// A parent was named that is not on record.
    #[error("entry `{id}` names parent `{parent}`, which is not on record")]
    UnknownParent {
        /// The entry.
        id: String,
        /// The parent it names.
        parent: String,
    },

    /// A hash is not 64 lowercase hex characters.
    #[error("not a block hash: `{hash}`")]
    NotAHash {
        /// What was given.
        hash: String,
    },

    /// The block store holds no block of that hash.
    #[error("no block `{hash}` in the store")]
    NoBlock {
        /// The hash asked for.
        hash: String,
    },

    /// A request or response body was not the JSON shape its api names.
    #[error("{api} body is not the expected shape: {reason}")]
    BodyShape {
        /// The api the body was said to follow.
        api: &'static str,
        /// What was wrong, naming no content.
        reason: &'static str,
    },

    /// An index does not agree with the file it describes.
    #[error("index for {} is stale: {reason}", path.display())]
    StaleIndex {
        /// The session file.
        path: PathBuf,
        /// What disagreed.
        reason: &'static str,
    },

    /// The session file is held open by another owner, in this process or another.
    #[error("session {} is held by another owner; one owner at a time", path.display())]
    SessionHeld {
        /// The session file.
        path: PathBuf,
    },

    /// An entry id is already on record in this session.
    #[error("session {session} already holds an entry `{id}`")]
    DuplicateEntry {
        /// The session id.
        session: String,
        /// The entry id offered again.
        id: String,
    },

    /// A name that must be one safe path component is not.
    #[error(
        "{what} `{name}` is not a safe name: letters, digits, `.`, `_` and `-`, not beginning with `.`, at most 200 bytes"
    )]
    BadName {
        /// What was being named.
        what: &'static str,
        /// The name offered.
        name: String,
    },

    /// A harness event's data would exceed the size an event may carry.
    #[error("harness event from record `{uuid}` would be {len} bytes of data, over the limit")]
    EventTooLarge {
        /// The source record's uuid, empty when it had none.
        uuid: String,
        /// The serialised size.
        len: usize,
    },

    /// A forked file repeats tool actions of the file it was forked from.
    #[error("{count} tool actions in the fork repeat ones in the rendered file")]
    RepeatedToolActions {
        /// How many `tool_use` ids appear more times in the fork than in the rendered file.
        count: u64,
    },

    /// A launch template's `slots` object holds a member outside the five the
    /// schema names.
    #[error(
        "launch template names a slot `{slot}` that is not one of transcript, mcp, env, secrets, instructions"
    )]
    UnknownSlot {
        /// The member found.
        slot: String,
    },

    /// A launch template's `slots` object lacks one of the five slots.
    #[error("launch template is missing the slot `{slot}`")]
    MissingSlot {
        /// The slot missing.
        slot: String,
    },

    /// A launch template field holds a value this profile does not accept.
    #[error(
        "launch template `{field}` is `{value}`, which the Claude Code profile does not accept"
    )]
    TemplateValue {
        /// The field, dotted from the top of the template.
        field: String,
        /// The value given, cut to 80 characters.
        value: String,
    },

    /// A launch template is not the shape the schema names.
    #[error("launch template `{field}` {reason}")]
    TemplateShape {
        /// The field, dotted from the top of the template.
        field: String,
        /// What is wrong with it, naming no content.
        reason: &'static str,
    },

    /// A launch template marks a secret readable, and no broker reader exists
    /// yet to read it at start.
    #[error(
        "secret_reader_unbuilt: the template marks `{env}` readable, and no broker reader exists until SECRETS-002 lands one; only use-only secrets render"
    )]
    SecretReaderUnbuilt {
        /// The environment variable the readable secret would fill.
        env: String,
    },

    /// Two entries of a launch template name the same environment variable.
    #[error(
        "launch template names the environment variable `{name}` more than once across env, secrets.use_only and secrets.readable"
    )]
    DuplicateVariable {
        /// The variable named twice.
        name: String,
    },

    /// A render was asked for Claude Code's own place with no home directory
    /// to find that place under.
    #[error(
        "no place to render to: --out was not given and the user's home directory is not known; give --out, or run where HOME names the directory that holds .claude/projects"
    )]
    NoRenderPlace,

    /// A file render-launch would write already exists.
    #[error("render-launch target already exists: {}", path.display())]
    LaunchTargetExists {
        /// The file.
        path: PathBuf,
    },

    /// The session's config directory has no name: the template's env slot
    /// sets no `CLAUDE_CONFIG_DIR` and the rendering process has no `HOME`.
    #[error(
        "the session's config directory cannot be named: the template's env slot sets no CLAUDE_CONFIG_DIR and the rendering process has no HOME; set CLAUDE_CONFIG_DIR in the template's env slot"
    )]
    NoConfigDir,

    /// A path that must be absolute is not.
    #[error("{what} `{}` is not an absolute path; give it from the root", path.display())]
    NotAbsolute {
        /// What the path names.
        what: &'static str,
        /// The path given.
        path: PathBuf,
    },
}

impl HomeError {
    pub(crate) fn io(
        context: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::Io {
            context,
            path: path.into(),
            source,
        }
    }
}
