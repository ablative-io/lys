//! The home record rendered as a Claude Code JSONL.
//!
//! The context path is walked and each message entry becomes a Claude Code
//! record with the parent chain intact and the chosen session id. A thinking
//! block renders whole, signature included, only when its provider, api and
//! model equal the target's (Pi's rule, transform-messages.ts:95-109);
//! otherwise readable thinking becomes a text part and an opaque or redacted
//! block is dropped and named by hash in the loss account beside the file. A
//! compaction becomes Claude Code's `summary` record. Custom entries (the lys
//! ones included) and labels do not render. An existing target path is
//! refused by name and nothing is written. For a child forked at a user
//! message the report names the seed file written beside the rendered file
//! ([`super::seed`]); no launch line is printed here, since a launch line
//! comes only from the template's render (`render-launch`).
//!
//! The render never writes a default in place of a value the record did not
//! carry (ADR-055). Every value it copies from a message entry is read
//! through the checked readers of `render_fields`: a missing field, or one
//! of another type than the target takes, refuses the render by the home
//! session's id, the entry id, the field and the expected type, and an
//! assistant stopReason Claude Code has no value for refuses by that value.
//! A message whose role is a string the render does not know is skipped.
//! Every refusal is decided before anything is written, and the writing
//! itself is `render_write`'s, which serialises everything first. The only
//! values written that no record carries are the format constants Claude
//! Code's file requires on every record: `gitBranch` `""`, `usage`
//! `{input_tokens 0, output_tokens 0}` and `stop_sequence` `null`. The only
//! field read from its absence is a thinking part's `redacted`: absent means
//! not redacted.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};

use crate::error::HomeError;
use crate::harness::claude_code::render_fields::{
    array, array_or_string, boolean, field_refusal, object, stop_reason, string,
};
use crate::harness::claude_code::seed::{seed_of, seed_path};
use crate::harness::claude_code::{API, PROVIDER, projects_slug, render_fields, render_write};
use crate::record::blocks::{Hash, hex_of};
use crate::record::entries::{CUSTOM_AUTHORED, Entry, EntryBody};
use crate::record::{Session, safe_component};

/// Where and for whom to render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderTarget {
    /// The session id the rendered file carries and Claude Code resumes by.
    pub session_id: String,
    /// The working directory the records name.
    pub cwd: String,
    /// The model the file is rendered for; decides whether thinking renders whole.
    pub model: String,
    /// The Claude Code version to write into records.
    pub version: String,
    /// Where to write; `None` means Claude Code's own place for `cwd`,
    /// `~/.claude/projects/<slug>/<session_id>.jsonl`.
    pub out: Option<PathBuf>,
    /// The canon to place first, before the session's own entries (R11).
    pub canon: Option<PathBuf>,
}

/// One thing a render could not carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loss {
    /// The hash of the part as stored.
    pub hash: String,
    /// Why, naming no content.
    pub reason: String,
}

/// What a render reported: paths, counts and the loss account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderReport {
    /// The file written.
    pub path: PathBuf,
    /// The loss account written beside it.
    pub loss_path: PathBuf,
    /// Records written.
    pub records: u64,
    /// Thinking blocks rendered whole with their signature.
    pub thinking_kept: u64,
    /// Thinking blocks rendered as text.
    pub thinking_as_text: u64,
    /// Blocks dropped, each in the loss account.
    pub dropped: u64,
    /// Whether the session is a hand-authored demonstration.
    pub authored: bool,
    /// Canon examples placed before the session's own entries.
    pub inherited: u64,
    /// The seed file written beside the rendered file, for a child forked
    /// at a user message; `None` otherwise.
    pub seed: Option<PathBuf>,
}

/// The default place for a session file: Claude Code's own directory for the
/// cwd. The session id must be one safe path component, and the slug is one
/// by construction (every character outside ASCII letters and digits becomes
/// `-`); both are checked, so no id can name a path outside that directory.
pub fn default_path(home_dir: &Path, cwd: &str, session_id: &str) -> Result<PathBuf, HomeError> {
    safe_component("session id", session_id)?;
    let slug = projects_slug(cwd);
    safe_component("projects slug", &slug)?;
    Ok(home_dir
        .join(".claude")
        .join("projects")
        .join(slug)
        .join(format!("{session_id}.jsonl")))
}

/// Render the session's context path for Claude Code. `user_home` is the
/// directory `.claude/projects` sits under, needed only when `target.out` is
/// `None`; asked for then and absent, the render is refused by name and
/// nothing is written.
pub fn render_claude_code(
    session: &Session,
    target: &RenderTarget,
    user_home: Option<&Path>,
) -> Result<RenderReport, HomeError> {
    let path = match (&target.out, user_home) {
        (Some(out), _) => out.clone(),
        (None, Some(home)) => default_path(home, &target.cwd, &target.session_id)?,
        (None, None) => return Err(HomeError::NoRenderPlace),
    };
    if path.exists() {
        return Err(HomeError::Exists { path });
    }
    let mut entries = Vec::new();
    let mut inherited = 0u64;
    if let Some(canon_path) = &target.canon {
        let canon = crate::record::canon::load(canon_path)?;
        inherited = canon.examples().len() as u64;
        entries.extend(canon.entries);
    }
    entries.extend(session.context_path()?);
    let seed = seed_of(session, &entries)?;
    let seed_file = seed.as_ref().map(|_| seed_path(&path));
    if let Some(file) = &seed_file
        && file.exists()
    {
        return Err(HomeError::Exists { path: file.clone() });
    }
    let authored = entries.iter().any(|e| e.is_custom(CUSTOM_AUTHORED));
    let mut walk = Walk {
        session: &session.header().id,
        target,
        records: Vec::new(),
        losses: Vec::new(),
        kept: 0,
        as_text: 0,
        prev: None,
    };
    for entry in &entries {
        walk.entry(entry)?;
    }
    let account = LossAccount {
        authored,
        dropped: &walk.losses,
        model: &target.model,
        session_id: &target.session_id,
    };
    let seed_write = seed.as_ref().zip(seed_file.as_deref());
    let loss_path =
        render_write::write_render(walk.session, &path, &walk.records, &account, seed_write)?;
    Ok(RenderReport {
        path,
        loss_path,
        records: walk.records.len() as u64,
        thinking_kept: walk.kept,
        thinking_as_text: walk.as_text,
        dropped: walk.losses.len() as u64,
        authored,
        inherited,
        seed: seed_file,
    })
}

/// The loss account beside a rendered file. Its fields stand in the order
/// of their names, the order the JSON object written before ADR-055 held
/// them in, so the account's bytes did not move.
#[derive(Serialize)]
struct LossAccount<'a> {
    authored: bool,
    dropped: &'a [Loss],
    model: &'a str,
    session_id: &'a str,
}

/// The walk of a render's entries: the records shaped so far, each beside
/// the id of the entry it came from, and the loss account's rows.
struct Walk<'a> {
    /// The home session's header id, which every refusal names.
    session: &'a str,
    target: &'a RenderTarget,
    /// Each record beside the id of the entry it came from; `None` for a
    /// `summary`.
    records: Vec<(Value, Option<String>)>,
    losses: Vec<Loss>,
    kept: u64,
    as_text: u64,
    /// The uuid of the last record written, the next record's parent.
    prev: Option<String>,
}

impl Walk<'_> {
    /// Shape one entry: a message becomes a record, a compaction a
    /// `summary`, anything else nothing.
    fn entry(&mut self, entry: &Entry) -> Result<(), HomeError> {
        match &entry.body {
            EntryBody::Message { message } => self.message(entry, message),
            EntryBody::Compaction { summary, .. } => {
                let record = json!({"type": "summary", "summary": summary, "leafUuid": self.prev});
                self.records.push((record, None));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn message(&mut self, entry: &Entry, message: &Value) -> Result<(), HomeError> {
        let id = entry.id();
        let uuid = record_uuid(self.session, id);
        let (kind, msg) = match string(self.session, id, message, "role")? {
            "user" => {
                let content = user_parts(self.session, id, message)?;
                ("user", json!({"role": "user", "content": content}))
            }
            "toolResult" => {
                let part = self.tool_result(id, message)?;
                ("user", json!({"role": "user", "content": [part]}))
            }
            "assistant" => ("assistant", self.assistant(id, &uuid, message)?),
            _ => return Ok(()),
        };
        let record = json!({
            "parentUuid": self.prev,
            "isSidechain": false,
            "userType": "external",
            "cwd": self.target.cwd,
            "sessionId": self.target.session_id,
            "version": self.target.version,
            "gitBranch": "",
            "uuid": uuid,
            "timestamp": entry.base.timestamp,
            "type": kind,
            "message": msg,
        });
        self.records.push((record, Some(id.to_owned())));
        self.prev = Some(uuid);
        Ok(())
    }

    fn tool_result(&self, id: &str, message: &Value) -> Result<Value, HomeError> {
        let tool_use_id = string(self.session, id, message, "toolCallId")?;
        let content = array_or_string(self.session, id, message, "content")?;
        let is_error = boolean(self.session, id, message, "isError")?;
        Ok(json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": content,
            "is_error": is_error,
        }))
    }

    fn assistant(&mut self, id: &str, uuid: &str, message: &Value) -> Result<Value, HomeError> {
        let provider = string(self.session, id, message, "provider")?;
        let api = string(self.session, id, message, "api")?;
        let model = string(self.session, id, message, "model")?;
        let parts = array(self.session, id, message, "content")?;
        let stop = stop_reason(self.session, id, message)?;
        let same = provider == PROVIDER && api == API && model == self.target.model;
        let mut content = Vec::new();
        for part in parts {
            match part.get("type").and_then(Value::as_str) {
                Some("text") => {
                    let text = string(self.session, id, part, "text")?;
                    content.push(json!({"type": "text", "text": text}));
                }
                Some("thinking") => self.thinking(id, part, same, &mut content)?,
                Some("toolCall") => {
                    let call_id = string(self.session, id, part, "id")?;
                    let name = string(self.session, id, part, "name")?;
                    let input = object(self.session, id, part, "arguments")?;
                    content.push(json!({
                        "type": "tool_use",
                        "id": call_id,
                        "name": name,
                        "input": input,
                    }));
                }
                _ => content.push(part.clone()),
            }
        }
        Ok(json!({
            "id": format!("msg_{}", &uuid[..8]),
            "type": "message",
            "role": "assistant",
            "model": model,
            "content": content,
            "stop_reason": stop,
            "stop_sequence": Value::Null,
            "usage": {"input_tokens": 0, "output_tokens": 0},
        }))
    }

    /// A thinking part: whole for the same provider, api and model, as text
    /// when readable otherwise, and dropped into the loss account when not.
    fn thinking(
        &mut self,
        id: &str,
        part: &Value,
        same: bool,
        content: &mut Vec<Value>,
    ) -> Result<(), HomeError> {
        const SIGNATURE: &str = "thinkingSignature";
        let redacted = render_fields::redacted(self.session, id, part)?;
        let sig = match part.get(SIGNATURE) {
            Some(_) => Some(string(self.session, id, part, SIGNATURE)?),
            None => None,
        };
        let text = string(self.session, id, part, "thinking")?;
        if redacted && sig.is_none() {
            return Err(field_refusal(self.session, id, SIGNATURE, "string", true));
        }
        match sig {
            Some(data) if same && redacted => {
                content.push(json!({"type": "redacted_thinking", "data": data}));
                self.kept += 1;
            }
            Some(signature) if same => {
                let whole = json!({"type": "thinking", "thinking": text, "signature": signature});
                content.push(whole);
                self.kept += 1;
            }
            _ if !text.trim().is_empty() && !redacted => {
                content.push(json!({"type": "text", "text": text}));
                self.as_text += 1;
                if sig.is_some() {
                    let reason =
                        "signed thinking rendered as text: different provider, api or model";
                    self.losses.push(loss(self.session, id, part, reason)?);
                }
            }
            _ => {
                let reason = if redacted {
                    "redacted thinking dropped: different provider, api or model"
                } else {
                    "empty thinking dropped"
                };
                self.losses.push(loss(self.session, id, part, reason)?);
            }
        }
        Ok(())
    }
}

/// The loss account's row for a part of entry `entry` of session `session`
/// the render could not carry: the part's hash as stored, and why.
fn loss(session: &str, entry: &str, part: &Value, reason: &str) -> Result<Loss, HomeError> {
    let bytes = serde_json::to_vec(part).map_err(|source| HomeError::RenderUnserialisable {
        session: session.to_owned(),
        what: "dropped part",
        entry: Some(entry.to_owned()),
        source,
    })?;
    Ok(Loss {
        hash: Hash::of(&bytes).to_string(),
        reason: reason.to_owned(),
    })
}

/// The lys render namespace, fixed: UUID version 5 of the RFC 9562 URL namespace
/// over `lys/home/claude-code/render-uuid/v1`. Every uuid a render derives
/// sits under a session namespace drawn from it; a change to the scheme is a
/// new namespace alongside, never a change to this one.
pub const RENDER_NAMESPACE: [u8; 16] = [
    0x32, 0xc0, 0x59, 0x04, 0xd1, 0xf1, 0x55, 0x0c, 0x9e, 0xee, 0x2f, 0x6c, 0x8f, 0x98, 0xb6, 0x65,
];

/// The one role a derived uuid plays so far: the rendered record's `uuid`.
/// A role never carries `#`, so the last `#` of a name splits it.
pub const ROLE_RECORD: &str = "record";

/// A UUID version 5 (RFC 9562): the SHA-1 of the namespace bytes then the
/// name, its first 16 bytes with the version nibble set to 5 and the variant
/// bits to 10.
#[must_use]
pub fn uuid_v5(namespace: &[u8; 16], name: &[u8]) -> [u8; 16] {
    let mut hasher = Sha1::new();
    hasher.update(namespace);
    hasher.update(name);
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

/// A uuid's lowercase hyphenated form, 8-4-4-4-12.
#[must_use]
pub fn uuid_string(bytes: &[u8; 16]) -> String {
    let hex = hex_of(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// The namespace of one session's derived uuids: UUID version 5 of the render
/// namespace over the session's own id. The session is the salt, so the same
/// entry id in two sessions never derives one uuid; the target session id is
/// not part of the record and never enters it.
#[must_use]
pub fn session_namespace(session_id: &str) -> [u8; 16] {
    uuid_v5(&RENDER_NAMESPACE, session_id.as_bytes())
}

/// Whether an id is 36 characters of hex with `-` at 8, 13, 18 and 23.
fn is_uuid_shaped(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// The uuid a record carries for the entry with `id` in the session with
/// `session_id`. A uuid-shaped id is kept as it is, so an imported Claude
/// Code session keeps its source's uuids; any other id (the importer's
/// `<uuid>-r<i>` for a split tool result, a hand-authored id, a canon id)
/// derives as UUID version 5 under the session's namespace over `<id>#record`, so a
/// derived uuid carries version nibble 5 where Claude Code's own carry 4.
/// Nothing random and no clock enters a render: the same session head
/// renders the same bytes every time.
#[must_use]
pub fn record_uuid(session_id: &str, id: &str) -> String {
    if is_uuid_shaped(id) {
        id.to_owned()
    } else {
        let name = format!("{id}#{ROLE_RECORD}");
        uuid_string(&uuid_v5(&session_namespace(session_id), name.as_bytes()))
    }
}

/// A user message's content: a string as it stands, one text part as its
/// text, and any other list of parts as it stands.
fn user_parts(session: &str, entry: &str, message: &Value) -> Result<Value, HomeError> {
    let content = array_or_string(session, entry, message, "content")?;
    if let Value::Array(parts) = content
        && let [only] = parts.as_slice()
        && only.get("type").and_then(Value::as_str) == Some("text")
    {
        return Ok(Value::String(string(session, entry, only, "text")?.to_owned()));
    }
    Ok(content.clone())
}
