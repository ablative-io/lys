//! Claude Code JSONL into the home record.
//!
//! Each `user` or `assistant` record becomes a message entry in Pi's grammar
//! with the record's `uuid` as the entry id and its `parentUuid` as the
//! parent, so the file's tree is the home's tree. Tool results inside a user
//! record become Pi `toolResult` messages, one each. Claude Code's
//! compaction records (the `compact_boundary` and `isCompactSummary` pair,
//! and the `summary` record) become one Pi compaction entry each, with a
//! `lys.loss` entry directly after it (the `compaction` module beside this
//! one, and [`crate::record::loss`]). A sidechain
//! (a subagent's records) hangs under the last main-path message before it
//! with a label naming the agent. Every content part is also stored as a
//! block, so a proxy call that resends the conversation adds no new blocks,
//! and each stored part gets one row in `<id>.blocks.jsonl` naming the entry
//! it went into, its index in the source record and the hash the store
//! returned ([`crate::record::block_rows`]). `attachment`, `system` and
//! `permission-mode` records become `lys.harness_event` entries (R8, see
//! [`super::events`]): the first two sit on the file's chain under their own
//! uuid, so a message whose parent is one of them still finds it; the last is
//! a side leaf. Every other record type is counted and left in the
//! byte-for-byte original.
//!
//! An assistant record whose model is `authored` marks a hand-written
//! demonstration: it imports with provider, api and model `authored` behind
//! one `lys.authored` entry, so it is never mistaken for a model's turn.

use std::collections::{BTreeMap, HashMap};
use std::io::BufRead;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::compaction::Compactions;
use crate::harness::claude_code::events::{HarnessEvent, event_of, tool_completed};
use crate::harness::claude_code::{API, AUTHORED, PROVIDER};
use crate::record::block_rows::BlockRowWriter;
use crate::record::blocks::BlockStore;
use crate::record::entries::{CUSTOM_AUTHORED, CUSTOM_HARNESS_EVENT, Entry, EntryBase, EntryBody};
use crate::record::{Session, fresh_id};

mod content;

use content::{Stored, UserContent, assistant_content, stop_reason, usage, user_content};

/// What an import reported: counts only, no text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    /// Lines in the source file.
    pub records: u64,
    /// Entries written.
    pub entries: u64,
    /// Content parts stored as blocks (new plus reused).
    pub blocks: u64,
    /// Blocks that were new to the store.
    pub blocks_new: u64,
    /// Source bytes read.
    pub bytes_in: u64,
    /// Bytes written to the session file.
    pub bytes_out: u64,
    /// Records by type that were counted and left in the original.
    pub counted_types: BTreeMap<String, u64>,
    /// Whether an authored turn was found.
    pub authored: bool,
    /// Sidechains attached, by agent id count.
    pub sidechains: u64,
    /// Harness events written, by kind (R8).
    pub events: BTreeMap<String, u64>,
    /// Compaction entries written (HOME-030 R3).
    #[serde(default)]
    pub compactions: u64,
    /// The source records those compaction entries came from.
    #[serde(default)]
    pub compaction_sources: u64,
}

/// The state of one import: the session and store written to, the block
/// rows, the report, and where the file's chain stands.
pub(super) struct Importer<'a> {
    pub(super) session: &'a mut Session,
    pub(super) blocks: &'a BlockStore,
    pub(super) rows: BlockRowWriter,
    pub(super) report: ImportReport,
    /// The last entry on the file's chain: where the head goes when the import ends.
    pub(super) chain_leaf: Option<String>,
    /// The last main-path entry: where a sidechain's first record hangs.
    pub(super) last_main: Option<String>,
    /// The first entry each source record produced, by the record's uuid.
    pub(super) first_entry: HashMap<String, String>,
    /// The compaction records read and not yet complete.
    pub(super) compactions: Compactions,
    tool_names: HashMap<String, String>,
    authored_marked: bool,
}

/// Import one Claude Code JSONL into an open, empty session.
pub fn import_claude_code(
    source: &Path,
    session: &mut Session,
    blocks: &BlockStore,
) -> Result<ImportReport, HomeError> {
    let file = std::fs::File::open(source)
        .map_err(|e| HomeError::io("opening the transcript", source, e))?;
    let reader = std::io::BufReader::new(file);
    let start_len = session_len(session)?;
    let mut st = Importer {
        rows: BlockRowWriter::open(session.published_file())?,
        chain_leaf: session.head()?.map(str::to_owned),
        session,
        blocks,
        report: ImportReport::default(),
        last_main: None,
        first_entry: HashMap::new(),
        compactions: Compactions::default(),
        tool_names: HashMap::new(),
        authored_marked: false,
    };
    for (n, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| HomeError::io("reading the transcript", source, e))?;
        st.report.records += 1;
        st.report.bytes_in += line.len() as u64 + 1;
        if line.trim().is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(&line).map_err(|e| HomeError::Malformed {
            path: source.to_path_buf(),
            line: n + 1,
            what: "Claude Code record",
            reason: e.to_string(),
        })?;
        if st.compaction_record(&record, source, n)? {
            continue;
        }
        let kind = record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_owned();
        match kind.as_str() {
            "user" | "assistant" => st.message(&record, &kind, source, n)?,
            other => st.harness_record(&record, other)?,
        }
    }
    st.finish_compactions()?;
    st.rows.sync()?;
    st.session.move_head(st.chain_leaf.as_deref())?;
    st.report.bytes_out = session_len(st.session)?.saturating_sub(start_len);
    Ok(st.report)
}

impl Importer<'_> {
    /// A `user` or `assistant` record: its message entries, its block rows
    /// and its `tool_completed` events.
    fn message(
        &mut self,
        record: &Value,
        kind: &str,
        source: &Path,
        n: usize,
    ) -> Result<(), HomeError> {
        let uuid = field(record, "uuid", source, n)?;
        let parent = record
            .get("parentUuid")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let timestamp = str_of(record, "timestamp").to_owned();
        let sidechain = record
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let parent_id = match parent {
            Some(p) => {
                if !self.session.contains(&p)? {
                    return Err(HomeError::UnknownParent {
                        id: uuid,
                        parent: p,
                    });
                }
                Some(p)
            }
            None if sidechain => {
                // A subagent's first record: hang it under the last main-path message.
                let under = self.last_main.clone();
                if let Some(agent) = record.get("agentId").and_then(Value::as_str) {
                    let label = Entry {
                        base: EntryBase {
                            id: fresh_id(),
                            parent_id: under.clone(),
                            timestamp: timestamp.clone(),
                        },
                        body: EntryBody::Label {
                            target_id: uuid.clone(),
                            label: Some(format!("agent {agent}")),
                        },
                    };
                    self.session.append_entry(&label)?;
                    self.report.entries += 1;
                    self.report.sidechains += 1;
                }
                under
            }
            None => None,
        };
        let Some(message) = record.get("message") else {
            count(&mut self.report, kind);
            return Ok(());
        };
        let ms = millis(&timestamp);
        let first = if kind == "assistant" {
            self.assistant(message, &uuid, parent_id, &timestamp, ms)?
        } else {
            self.user(message, &uuid, parent_id, &timestamp, ms)?
        };
        self.first_entry.insert(uuid.clone(), first);
        if !sidechain {
            self.last_main = Some(uuid.clone());
        }
        self.chain_leaf = Some(uuid);
        Ok(())
    }

    /// An assistant record's entry, behind one `lys.authored` mark when it
    /// is the first authored turn; returns the first entry written.
    fn assistant(
        &mut self,
        message: &Value,
        uuid: &str,
        parent_id: Option<String>,
        timestamp: &str,
        ms: i64,
    ) -> Result<String, HomeError> {
        let model = str_of(message, "model").to_owned();
        let authored = model == AUTHORED;
        let mut parent_for_next = parent_id;
        let mut first = uuid.to_owned();
        if authored {
            self.report.authored = true;
            if !self.authored_marked {
                let mark = Entry {
                    base: EntryBase {
                        id: fresh_id(),
                        parent_id: parent_for_next,
                        timestamp: timestamp.to_owned(),
                    },
                    body: EntryBody::Custom {
                        custom_type: CUSTOM_AUTHORED.to_owned(),
                        data: None,
                    },
                };
                self.session.append_entry(&mark)?;
                self.report.entries += 1;
                first.clone_from(&mark.base.id);
                parent_for_next = Some(mark.base.id);
                self.authored_marked = true;
            }
        }
        let names = &mut self.tool_names;
        let (content, stored) = assistant_content(message, self.blocks, names, &mut self.report)?;
        let pi = json!({
            "role": "assistant",
            "content": content,
            "api": if authored { AUTHORED } else { API },
            "provider": if authored { AUTHORED } else { PROVIDER },
            "model": model,
            "usage": usage(message),
            "stopReason": stop_reason(message),
            "timestamp": ms,
        });
        let entry = Entry {
            base: EntryBase {
                id: uuid.to_owned(),
                parent_id: parent_for_next,
                timestamp: timestamp.to_owned(),
            },
            body: EntryBody::Message { message: pi },
        };
        self.session.append_entry(&entry)?;
        self.report.entries += 1;
        self.rows_of(&stored, |_| uuid.to_owned())?;
        Ok(first)
    }

    /// A user record: tool results first (each its own message), then the
    /// user's own parts. Whichever entry comes last for the record carries
    /// the record's uuid, so the next record's parentUuid lands on the leaf.
    /// Returns the first entry written.
    fn user(
        &mut self,
        message: &Value,
        uuid: &str,
        parent_id: Option<String>,
        timestamp: &str,
        ms: i64,
    ) -> Result<String, HomeError> {
        let UserContent {
            own,
            results,
            stored,
        } = user_content(message, self.blocks, &self.tool_names, &mut self.report)?;
        let mut chain = parent_id;
        let last_result_is_leaf = own.is_empty();
        let result_count = results.len();
        let result_id = |i: usize| {
            if last_result_is_leaf && i + 1 == result_count {
                uuid.to_owned()
            } else {
                format!("{uuid}-r{i}")
            }
        };
        let mut completed = Vec::new();
        for (i, result) in results.into_iter().enumerate() {
            let id = result_id(i);
            let tool_id = result["toolCallId"].as_str().unwrap_or("").to_owned();
            let tool_name = result["toolName"].as_str().unwrap_or("").to_owned();
            let is_error = result["isError"].as_bool().unwrap_or(false);
            let entry = Entry {
                base: EntryBase {
                    id: id.clone(),
                    parent_id: chain.clone(),
                    timestamp: timestamp.to_owned(),
                },
                body: EntryBody::Message { message: result },
            };
            self.session.append_entry(&entry)?;
            self.report.entries += 1;
            completed.push((id.clone(), tool_id, tool_name, is_error));
            chain = Some(id);
        }
        if !last_result_is_leaf {
            let pi = json!({"role": "user", "content": own, "timestamp": ms});
            let entry = Entry {
                base: EntryBase {
                    id: uuid.to_owned(),
                    parent_id: chain,
                    timestamp: timestamp.to_owned(),
                },
                body: EntryBody::Message { message: pi },
            };
            self.session.append_entry(&entry)?;
            self.report.entries += 1;
        }
        let entry_of = |result: Option<usize>| result.map_or_else(|| uuid.to_owned(), result_id);
        self.rows_of(&stored, entry_of)?;
        let first = completed
            .first()
            .map_or_else(|| uuid.to_owned(), |(id, ..)| id.clone());
        // One tool_completed event under each tool result message, as side leaves.
        for (under, tool_id, tool_name, is_error) in completed {
            let event = tool_completed(&tool_id, &tool_name, is_error, uuid);
            self.event(&event, &fresh_id(), Some(under), timestamp)?;
        }
        Ok(first)
    }

    /// One block row per stored part, under the entry `entry_of` names for
    /// the part's tool result (`None` for the record's own entry).
    fn rows_of(
        &mut self,
        stored: &[Stored],
        entry_of: impl Fn(Option<usize>) -> String,
    ) -> Result<(), HomeError> {
        for part in stored {
            let entry = entry_of(part.result);
            self.rows.append(&entry, part.part, &part.hash, part.len)?;
        }
        Ok(())
    }

    /// Store one part of a record as a block with its row under `entry`.
    pub(super) fn store_row(
        &mut self,
        entry: &str,
        part: u64,
        value: &Value,
    ) -> Result<(), HomeError> {
        let (hash, len) = content::store_part(value, self.blocks, &mut self.report)?;
        self.rows.append(entry, part, &hash, len)
    }

    /// A record of any other type: a harness event when R8 maps it,
    /// otherwise counted and left in the original.
    fn harness_record(&mut self, record: &Value, kind: &str) -> Result<(), HomeError> {
        let Some(event) = event_of(record, self.blocks)? else {
            count(&mut self.report, kind);
            return Ok(());
        };
        let timestamp = str_of(record, "timestamp").to_owned();
        let Some(uuid) = event.source_uuid.clone() else {
            // A record without a uuid: a side leaf under the chain's leaf.
            let under = self.chain_leaf.clone();
            return self.event(&event, &fresh_id(), under, &timestamp);
        };
        // A record on the file's chain: at its exact place, under its own uuid.
        let parent = record
            .get("parentUuid")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if let Some(p) = &parent
            && !self.session.contains(p)?
        {
            return Err(HomeError::UnknownParent {
                id: uuid,
                parent: p.clone(),
            });
        }
        let parent = parent.or_else(|| self.chain_leaf.clone());
        self.event(&event, &uuid, parent, &timestamp)?;
        self.first_entry.insert(uuid.clone(), uuid.clone());
        self.chain_leaf = Some(uuid);
        Ok(())
    }

    /// Append one `lys.harness_event` entry and count it by kind.
    fn event(
        &mut self,
        event: &HarnessEvent,
        id: &str,
        parent_id: Option<String>,
        timestamp: &str,
    ) -> Result<(), HomeError> {
        let entry = Entry {
            base: EntryBase {
                id: id.to_owned(),
                parent_id,
                timestamp: timestamp.to_owned(),
            },
            body: EntryBody::Custom {
                custom_type: CUSTOM_HARNESS_EVENT.to_owned(),
                data: Some(event.data()),
            },
        };
        self.session.append_entry(&entry)?;
        self.report.entries += 1;
        *self.report.events.entry(event.kind.clone()).or_insert(0) += 1;
        Ok(())
    }
}

fn count(report: &mut ImportReport, kind: &str) {
    *report.counted_types.entry(kind.to_owned()).or_insert(0) += 1;
}

/// A record's string field, or `""` when it has none.
pub(super) fn str_of<'v>(record: &'v Value, name: &str) -> &'v str {
    record.get(name).and_then(Value::as_str).unwrap_or("")
}

/// A record's required string field, refused by line when absent.
pub(super) fn field(
    record: &Value,
    name: &str,
    source: &Path,
    n: usize,
) -> Result<String, HomeError> {
    record
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| HomeError::Malformed {
            path: source.to_path_buf(),
            line: n + 1,
            what: "Claude Code record",
            reason: format!("no {name}"),
        })
}

fn session_len(session: &Session) -> Result<u64, HomeError> {
    std::fs::metadata(session.file())
        .map(|m| m.len())
        .map_err(|e| HomeError::io("measuring the session file", session.file(), e))
}

fn millis(timestamp: &str) -> i64 {
    time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
        .map_or(0, |t| {
            i64::try_from(t.unix_timestamp_nanos() / 1_000_000).unwrap_or(0)
        })
}
