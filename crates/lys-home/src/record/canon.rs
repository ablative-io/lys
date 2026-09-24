//! The canon (HOME-001 R11): one curated, versioned series of examples every
//! new session starts from.
//!
//! The canon is a session file in Pi's grammar, `canon/canon.jsonl` in the
//! lys repository, changed only by appending and only through review. Each
//! example is a `lys.inherited` custom entry naming its source and the rule
//! it shows, followed by the example's message entries copied whole: text,
//! tool calls and thinking blocks with their signatures, byte for byte. A
//! clearly authored example carries provider, api and model `authored`
//! instead of a source, and may hold no thinking block, because thinking is
//! never authored.
//!
//! The canon is read and appended as a plain file: it has no index, head or
//! lock beside it, since it lives in a repository and is small. Curation is a
//! person's act: the tool copies what it is told to and records who told it.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::entries::{CUSTOM_INHERITED, Entry, EntryBase, EntryBody, SessionHeader};
use crate::record::{PI_FORMAT_VERSION, Session, now};

/// The session id the canon file carries.
pub const CANON_ID: &str = "canon";
/// The value of provider, api and model on an authored example.
pub const AUTHORED: &str = "authored";

/// What a `lys.inherited` entry's data says about its example.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inherited {
    /// `true` for a hand-written example, which names no source session.
    #[serde(default)]
    pub authored: bool,
    /// The session the example was copied from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_session: Option<String>,
    /// The entries copied, in order, by their ids in that session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from_entries: Vec<String>,
    /// The provider, api and model of the example's assistant turns.
    pub provider: String,
    /// See `provider`.
    pub api: String,
    /// See `provider`.
    pub model: String,
    /// When it was curated, RFC 3339.
    pub curated_at: String,
    /// Who told the tool to add it.
    pub curated_by: String,
    /// The rule this example shows, stated short.
    pub rule: String,
}

/// The canon as loaded: its header and every entry in file order.
#[derive(Clone, Debug)]
pub struct Canon {
    /// The header line.
    pub header: SessionHeader,
    /// Every entry, in file order.
    pub entries: Vec<Entry>,
}

impl Canon {
    /// The `lys.inherited` entries: one per example.
    #[must_use]
    pub fn examples(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.is_custom(CUSTOM_INHERITED))
            .collect()
    }

    /// The id of the last entry, where the next example chains on.
    #[must_use]
    pub fn leaf(&self) -> Option<&str> {
        self.entries.last().map(Entry::id)
    }
}

/// What one `canon add` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddReport {
    /// The canon file.
    pub canon: std::path::PathBuf,
    /// The `lys.inherited` entry's id.
    pub inherited_id: String,
    /// Message entries copied or written after it.
    pub messages: u64,
    /// Thinking blocks copied whole, with their signatures.
    pub thinking_copied: u64,
    /// Examples in the canon after this one.
    pub examples: u64,
}

/// Create the canon file with its header line; refuses an existing file.
pub fn create(path: &Path) -> Result<(), HomeError> {
    if path.exists() {
        return Err(HomeError::Exists {
            path: path.to_path_buf(),
        });
    }
    let header = SessionHeader {
        version: Some(PI_FORMAT_VERSION),
        id: CANON_ID.to_owned(),
        timestamp: now(),
        cwd: "/canon".to_owned(),
        parent_session: None,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the canon directory", dir, e))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| HomeError::io("creating the canon", path, e))?;
    write_line(&mut file, &header, path)?;
    file.sync_all()
        .map_err(|e| HomeError::io("syncing the canon", path, e))
}

/// Load the canon: the header, then every entry, each parent on record
/// before its child and no id twice, as Pi's reader expects.
pub fn load(path: &Path) -> Result<Canon, HomeError> {
    let text =
        std::fs::read_to_string(path).map_err(|e| HomeError::io("reading the canon", path, e))?;
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    let Some((_, first)) = lines.next() else {
        return Err(HomeError::NoHeader {
            path: path.to_path_buf(),
            reason: "the file is empty".to_owned(),
        });
    };
    let header: SessionHeader = serde_json::from_str(first).map_err(|e| HomeError::NoHeader {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (n, line) in lines {
        let entry: Entry = serde_json::from_str(line).map_err(|e| HomeError::Malformed {
            path: path.to_path_buf(),
            line: n + 1,
            what: "canon entry",
            reason: e.to_string(),
        })?;
        if !seen.insert(entry.id().to_owned()) {
            return Err(HomeError::Malformed {
                path: path.to_path_buf(),
                line: n + 1,
                what: "canon entry",
                reason: "its id is already on record".to_owned(),
            });
        }
        if let Some(parent) = entry.parent_id()
            && !seen.contains(parent)
        {
            return Err(HomeError::Malformed {
                path: path.to_path_buf(),
                line: n + 1,
                what: "canon entry",
                reason: "its parent is not on record before it".to_owned(),
            });
        }
        entries.push(entry);
    }
    Ok(Canon { header, entries })
}

/// Copy entries of a session into the canon, whole, after a `lys.inherited`
/// entry naming where they came from and the rule they show. The entries
/// keep their ids, so the same example cannot be added twice; only their
/// parent links are rewritten to chain onto the canon.
pub fn add_from(
    canon_path: &Path,
    session: &Session,
    entry_ids: &[String],
    rule: &str,
    by: &str,
) -> Result<AddReport, HomeError> {
    if entry_ids.is_empty() {
        return Err(HomeError::BodyShape {
            api: "canon",
            reason: "an example needs at least one entry",
        });
    }
    let mut copied = Vec::with_capacity(entry_ids.len());
    for id in entry_ids {
        let entry = session.entry(id)?;
        if !matches!(entry.body, EntryBody::Message { .. }) {
            return Err(HomeError::BodyShape {
                api: "canon",
                reason: "only message entries are copied into the canon",
            });
        }
        copied.push(entry);
    }
    let (provider, api, model) = assistant_identity(&copied);
    let inherited = Inherited {
        authored: false,
        from_session: Some(session.header().id.clone()),
        from_entries: entry_ids.to_vec(),
        provider,
        api,
        model,
        curated_at: now(),
        curated_by: by.to_owned(),
        rule: rule.to_owned(),
    };
    append_example(canon_path, &inherited, copied)
}

/// Add a hand-written example: user and assistant turns from a turns file
/// (`user: text` or `assistant: text`, one per line), with provider, api and
/// model `authored`. A turns file holding a thinking block is refused by line.
pub fn add_authored(
    canon_path: &Path,
    turns: &Path,
    rule: &str,
    by: &str,
) -> Result<AddReport, HomeError> {
    let turns = parse_turns(turns)?;
    let ms = millis_now();
    let entries = turns
        .into_iter()
        .map(|(role, text)| {
            let message = match role {
                Role::User => json!({"role": "user", "content": [{"type": "text", "text": text}], "timestamp": ms}),
                Role::Assistant => json!({
                    "role": "assistant",
                    "content": [{"type": "text", "text": text}],
                    "api": AUTHORED, "provider": AUTHORED, "model": AUTHORED,
                    "usage": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 0, "cost": {"input": 0.0, "output": 0.0, "cacheRead": 0.0, "cacheWrite": 0.0, "total": 0.0}},
                    "stopReason": "stop",
                    "timestamp": ms,
                }),
            };
            Entry {
                base: EntryBase {
                    id: crate::record::fresh_id(),
                    parent_id: None,
                    timestamp: now(),
                },
                body: EntryBody::Message { message },
            }
        })
        .collect();
    let inherited = Inherited {
        authored: true,
        from_session: None,
        from_entries: Vec::new(),
        provider: AUTHORED.to_owned(),
        api: AUTHORED.to_owned(),
        model: AUTHORED.to_owned(),
        curated_at: now(),
        curated_by: by.to_owned(),
        rule: rule.to_owned(),
    };
    append_example(canon_path, &inherited, entries)
}

/// A turn's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The person's turn.
    User,
    /// The assistant's turn.
    Assistant,
}

/// Parse a turns file: `user: text` or `assistant: text` per line, blank
/// lines skipped. A line that names another role, has no role, or holds a
/// thinking block is refused by line number: thinking is never authored.
pub fn parse_turns(path: &Path) -> Result<Vec<(Role, String)>, HomeError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| HomeError::io("reading the turns file", path, e))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let malformed = |what: &'static str, reason: &str| HomeError::Malformed {
            path: path.to_path_buf(),
            line: n + 1,
            what,
            reason: reason.to_owned(),
        };
        let (role, body) = line.split_once(": ").ok_or_else(|| {
            malformed(
                "turn (`user: text` or `assistant: text`)",
                "no `role: ` prefix",
            )
        })?;
        if body.contains("\"type\":\"thinking\"") || body.contains("\"type\": \"thinking\"") {
            return Err(malformed("turn", "a thinking block is never authored"));
        }
        let role = match role {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            _ => return Err(malformed("turn", "role must be user or assistant")),
        };
        out.push((role, body.to_owned()));
    }
    if out.is_empty() {
        return Err(HomeError::Malformed {
            path: path.to_path_buf(),
            line: 0,
            what: "turns file",
            reason: "no turns".to_owned(),
        });
    }
    Ok(out)
}

fn append_example(
    canon_path: &Path,
    inherited: &Inherited,
    mut entries: Vec<Entry>,
) -> Result<AddReport, HomeError> {
    let canon = load(canon_path)?;
    for entry in &entries {
        if canon.entries.iter().any(|e| e.id() == entry.id()) {
            return Err(HomeError::DuplicateEntry {
                session: CANON_ID.to_owned(),
                id: entry.id().to_owned(),
            });
        }
    }
    let data = serde_json::to_value(inherited).map_err(|source| HomeError::Json {
        context: "the inherited entry could not be serialised",
        source,
    })?;
    let mark = Entry {
        base: EntryBase {
            id: crate::record::fresh_id(),
            parent_id: canon.leaf().map(str::to_owned),
            timestamp: now(),
        },
        body: EntryBody::Custom {
            custom_type: CUSTOM_INHERITED.to_owned(),
            data: Some(data),
        },
    };
    let mut prev = mark.base.id.clone();
    let mut thinking_copied = 0u64;
    for entry in &mut entries {
        entry.base.parent_id = Some(prev.clone());
        prev.clone_from(&entry.base.id);
        thinking_copied += thinking_blocks(entry);
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(canon_path)
        .map_err(|e| HomeError::io("opening the canon", canon_path, e))?;
    write_line(&mut file, &mark, canon_path)?;
    for entry in &entries {
        write_line(&mut file, entry, canon_path)?;
    }
    file.sync_all()
        .map_err(|e| HomeError::io("syncing the canon", canon_path, e))?;
    Ok(AddReport {
        canon: canon_path.to_path_buf(),
        inherited_id: mark.base.id,
        messages: entries.len() as u64,
        thinking_copied,
        examples: canon.examples().len() as u64 + 1,
    })
}

/// The provider, api and model of the example's assistant turns: those of the
/// first assistant message, or `authored` when there is none.
fn assistant_identity(entries: &[Entry]) -> (String, String, String) {
    for entry in entries {
        if let EntryBody::Message { message } = &entry.body
            && message.get("role").and_then(Value::as_str) == Some("assistant")
        {
            let s = |k: &str| {
                message
                    .get(k)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            };
            return (s("provider"), s("api"), s("model"));
        }
    }
    (
        AUTHORED.to_owned(),
        AUTHORED.to_owned(),
        AUTHORED.to_owned(),
    )
}

fn thinking_blocks(entry: &Entry) -> u64 {
    let EntryBody::Message { message } = &entry.body else {
        return 0;
    };
    message
        .get("content")
        .and_then(Value::as_array)
        .map_or(0, |parts| {
            parts
                .iter()
                .filter(|p| p.get("type").and_then(Value::as_str) == Some("thinking"))
                .count() as u64
        })
}

fn write_line<T: Serialize>(
    file: &mut std::fs::File,
    value: &T,
    path: &Path,
) -> Result<(), HomeError> {
    let mut line = serde_json::to_string(value).map_err(|source| HomeError::Json {
        context: "a canon line could not be serialised",
        source,
    })?;
    line.push('\n');
    file.write_all(line.as_bytes())
        .map_err(|e| HomeError::io("writing the canon", path, e))
}

fn millis_now() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
    )
    .unwrap_or(0)
}
