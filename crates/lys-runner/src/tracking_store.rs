//! The runner's feed: every record it makes of what its sessions did, kept
//! durably in the order made, and read by the server from an opaque cursor.
//!
//! The feed is one append-only file of JSON lines in the state directory,
//! beside an index replaced whole and atomically after each append. Each
//! append is a unit: its entries, then one commit line carrying what the
//! unit consumed, a stream's new cursor or a refusal's attempt, written and
//! made durable together before anything is answered. So received records
//! and the source cursor that produced them are kept together or not at
//! all: a start folds only the units after the index's committed length,
//! and a unit a crash left without its commit is cut off, by name, and read
//! again from its source's saved cursor.
//!
//! A cursor names the feed's own id and a byte offset in it. A cursor of
//! another feed is refused `cursor_expired`: a named gap, never a silent
//! restart. Nothing here polls: a reader waits on the runner's table, which
//! every append wakes.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RunnerError;
use crate::operations::OperationOutcome;
use crate::refusals::RefusalRecord;
use serde_json::Value;

use crate::tracking::{Figures, Unavailable, UsageRecord, count, note};

/// The feed's format.
pub const FEED_FORMAT: &str = "lys-runner-feed/v1";

/// The most entries one page of the feed carries.
pub const PAGE_MAX: usize = 256;

/// A Claude Code response seen, kept until the next record shows it whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    /// Its message and request id.
    pub key: String,
    /// Its latest usage object, as text.
    pub usage: String,
    /// The model it names.
    pub model: Option<String>,
    /// When it was observed, in milliseconds.
    pub observed_at: Option<u64>,
    /// The byte offset of its first record.
    pub offset: u64,
    /// The turn it belongs to.
    pub turn: Option<String>,
}

/// A Codex usage object's counts, as kept between `token_count` events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Totals {
    /// All input tokens, cached ones among them.
    pub input: u64,
    /// Input tokens read from a cache.
    pub cached: u64,
    /// Output tokens.
    pub output: u64,
}

impl Totals {
    /// The totals a Codex usage object says, when it says all three.
    pub fn of(usage: &Value) -> Option<Self> {
        Some(Self {
            input: count(usage, "input_tokens")?,
            cached: count(usage, "cached_input_tokens").unwrap_or(0),
            output: count(usage, "output_tokens")?,
        })
    }

    /// What `self` adds over `before`; none when any count went down, which
    /// is a reset, not spend.
    pub fn since(&self, before: &Self) -> Option<Self> {
        Some(Self {
            input: self.input.checked_sub(before.input)?,
            cached: self.cached.checked_sub(before.cached)?,
            output: self.output.checked_sub(before.output)?,
        })
    }

    /// The spend figures of a difference.
    pub fn figures(&self, context: Option<u64>) -> (Figures, Vec<Unavailable>) {
        let figures = Figures {
            input_tokens: Some(self.input.saturating_sub(self.cached)),
            output_tokens: Some(self.output),
            cache_creation_tokens: None,
            cache_read_tokens: Some(self.cached),
            context_tokens: context,
            running_ms: None,
            ..Figures::default()
        };
        let mut notes = vec![
            Unavailable {
                figure: "dollars_micros".to_owned(),
                reason:
                    "Codex reports dollars only through its app-server; Lys does not read it yet"
                        .to_owned(),
            },
            Unavailable {
                figure: "plan_windows".to_owned(),
                reason: "codex_rollout_has_no_reported_plan_window".to_owned(),
            },
            Unavailable {
                figure: "cache_creation_tokens".to_owned(),
                reason: "codex_reports_none".to_owned(),
            },
            Unavailable {
                figure: "running_ms".to_owned(),
                reason: "measured_from_the_session_process".to_owned(),
            },
        ];
        note("context_tokens", context, "field_absent", &mut notes);
        (figures, notes)
    }
}

/// A session's stream, as far as it has been read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceState {
    /// The file followed.
    pub path: String,
    /// Its generation: raised when it is truncated or replaced.
    pub generation: u64,
    /// The bytes read: every byte before it is a whole line consumed.
    pub offset: u64,
    /// The file's device and inode, when it has been opened.
    pub identity: Option<String>,
    /// The harness's own session or thread id it is bound to.
    pub bound: String,
    /// The turn under way, when the source named one.
    pub turn: Option<String>,
    /// A response not yet shown whole.
    pub pending: Option<Pending>,
    /// The last totals a Codex rollout gave.
    pub totals: Option<Totals>,
    /// The last status-line snapshot kept.
    pub snapshot: Option<Figures>,
    /// The latest reported cost, retained when a later snapshot omits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_cost_micros: Option<u64>,
    /// The latest account windows observed in a rollout.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_windows: Vec<crate::tracking_budget::PlanWindow>,
    /// The attribution of the last status-line snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_account: Option<String>,
    /// The attribution of the last rollout window report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_account: Option<String>,
}

/// What a unit consumed, kept with it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    /// A stream's state after the unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceState>,
    /// A refusal's attempt, so a repeat reuses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<String>,
}

/// Where a session's coverage stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// The state, by name: `tracking_started`, `source_bound`,
    /// `source_generation`, `record_unreadable`, `source_refused`,
    /// `coverage_incomplete` or `audit_incomplete`.
    pub state: String,
    /// The stream it concerns, when it concerns one.
    pub source: Option<String>,
    /// The stream's generation.
    pub generation: u64,
    /// The byte offset it concerns, when it concerns one.
    pub offset: Option<u64>,
    /// Why, in words.
    pub words: String,
    /// The executable launched, when it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    /// The version the executable reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_version: Option<String>,
    /// The adapter that reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

impl Coverage {
    /// Coverage `state` of `source` at `offset`, in `words`.
    pub fn of(state: &str, source: &SourceState, offset: Option<u64>, words: String) -> Self {
        Self {
            state: state.to_owned(),
            source: Some(source.path.clone()),
            generation: source.generation,
            offset,
            words,
            executable: None,
            harness_version: None,
            adapter: None,
        }
    }
}

/// A turn boundary the harness signalled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    /// `session_start`, `turn_start`, `turn_end`, `compacting` or
    /// `session_end`.
    pub boundary: String,
    /// The turn, when the harness named it.
    pub turn: Option<String>,
}

/// What one entry records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "entry", rename_all = "snake_case")]
pub enum Body {
    /// A record of figures.
    Usage(UsageRecord),
    /// Where coverage stands.
    Coverage(Coverage),
    /// A turn boundary.
    Boundary(Boundary),
    /// A tool call the judge denied.
    Refusal(RefusalRecord),
    /// Where an operation stands.
    Operation(OperationOutcome),
    /// The end of a unit, and what it consumed.
    Commit(Commit),
}

/// One entry of the feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedEntry {
    /// Its number in the feed, from 0.
    pub seq: u64,
    /// When it was made, in milliseconds since the Unix epoch.
    pub at: u64,
    /// The session it concerns.
    pub session: String,
    /// What it records.
    pub body: Body,
}

/// A page of the feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedPage {
    /// The feed's format.
    pub format: String,
    /// The entries after the cursor asked from, commits left out.
    pub entries: Vec<FeedEntry>,
    /// The cursor to read on from.
    pub cursor: String,
}

/// Whole entries read, each with the offset after it; where they end; and
/// why a line after them did not read, when one did not.
type Lines = (Vec<(FeedEntry, u64)>, u64, Option<String>);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    format: String,
    feed: String,
    committed: u64,
    next_seq: u64,
    sources: BTreeMap<String, SourceState>,
    attempts: BTreeMap<String, u64>,
}

/// The feed, open for appending and reading.
pub struct Feed {
    path: PathBuf,
    index_path: PathBuf,
    index: Index,
}

fn failed(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::State {
        reason: format!("the feed: {what}"),
    }
}

/// The key a refusal's attempt is kept under.
pub fn attempt_key(session: &str, attempt: &str) -> String {
    format!("{session}\n{attempt}")
}

impl Feed {
    /// The feed in `dir`, made when there is none; a unit a crash left
    /// without its commit is cut off and said.
    pub fn open(dir: &Path) -> Result<Self, RunnerError> {
        let path = dir.join("feed.jsonl");
        let index_path = dir.join("feed.index.json");
        let index = match fs::read(&index_path) {
            Ok(bytes) => serde_json::from_slice::<Index>(&bytes)
                .map_err(|error| failed(format!("the index does not read: {error}")))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Index {
                format: FEED_FORMAT.to_owned(),
                feed: crate::protocol::hex(&rand::random::<[u8; 16]>()),
                ..Index::default()
            },
            Err(error) => return Err(failed(format!("reading the index: {error}"))),
        };
        if index.format != FEED_FORMAT {
            return Err(failed(format!("the index is in format {}", index.format)));
        }
        let mut feed = Self {
            path,
            index_path,
            index,
        };
        feed.recover()?;
        Ok(feed)
    }

    /// Fold the units after the committed length, and cut off the bytes
    /// after the last commit.
    fn recover(&mut self) -> Result<(), RunnerError> {
        let length = match fs::metadata(&self.path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(failed(error)),
        };
        if length < self.index.committed {
            return Err(failed(format!(
                "the feed holds {length} bytes and its index has folded {}",
                self.index.committed
            )));
        }
        if length == self.index.committed {
            return Ok(());
        }
        let (entries, _, _) = self.lines(self.index.committed, length)?;
        let (mut kept, mut unit) = (self.index.committed, self.index.committed);
        for (entry, after) in entries {
            if let Body::Commit(commit) = entry.body {
                self.index.next_seq = entry.seq + 1;
                fold(&mut self.index, &entry.session, commit, unit);
                kept = after;
                unit = after;
            }
        }
        if kept < length {
            crate::error::said(&format!(
                "the feed held {} bytes after its last commit, from a unit a stop cut short: they are cut off and read again from their source",
                length - kept
            ));
            fs::OpenOptions::new()
                .write(true)
                .open(&self.path)
                .and_then(|file| file.set_len(kept).and_then(|()| file.sync_all()))
                .map_err(failed)?;
        }
        self.index.committed = kept;
        self.write_index()
    }

    /// The whole lines between `from` and `to`, each with the offset after
    /// it; where the last whole line that reads ends; and, when a whole line
    /// does not read, why.
    fn lines(&self, from: u64, to: u64) -> Result<Lines, RunnerError> {
        if from >= to {
            return Ok((Vec::new(), from, None));
        }
        let mut file = fs::File::open(&self.path).map_err(failed)?;
        file.seek(SeekFrom::Start(from)).map_err(failed)?;
        let mut reader = BufReader::new(file.take(to.saturating_sub(from)));
        let (mut entries, mut at) = (Vec::new(), from);
        loop {
            let mut line = Vec::new();
            let read = reader.read_until(b'\n', &mut line).map_err(failed)?;
            if read == 0 || line.last() != Some(&b'\n') {
                return Ok((entries, at, None));
            }
            match serde_json::from_slice::<FeedEntry>(&line) {
                Ok(entry) => {
                    at += read as u64;
                    entries.push((entry, at));
                }
                Err(error) => {
                    return Ok((
                        entries,
                        at,
                        Some(format!("the line at byte {at} does not read: {error}")),
                    ));
                }
            }
        }
    }

    fn write_index(&self) -> Result<(), RunnerError> {
        let bytes = serde_json::to_vec(&self.index).map_err(failed)?;
        crate::state::replace(&self.index_path, &bytes).map_err(failed)
    }

    /// The feed's own id, which every cursor names.
    pub fn id(&self) -> &str {
        &self.index.feed
    }

    /// The cursor after every committed entry.
    pub fn end(&self) -> String {
        format!("{}:{}", self.index.feed, self.index.committed)
    }

    /// Session `session`'s stream as last committed.
    pub fn source(&self, session: &str) -> Option<&SourceState> {
        self.index.sources.get(session)
    }

    /// The refusal kept for `key`, when one was: the first entry of the
    /// unit its attempt was committed in.
    pub fn attempt(&self, key: &str) -> Result<Option<RefusalRecord>, RunnerError> {
        let Some(offset) = self.index.attempts.get(key).copied() else {
            return Ok(None);
        };
        let (entries, _, unread) = self.lines(offset, self.index.committed)?;
        match entries.into_iter().next() {
            Some((
                FeedEntry {
                    body: Body::Refusal(record),
                    ..
                },
                _,
            )) => Ok(Some(record)),
            _ => Err(failed(unread.unwrap_or_else(|| {
                format!("the refusal kept at byte {offset} is not there")
            }))),
        }
    }

    /// Append `bodies` for `session` as one unit ending in `commit`, made
    /// durable before this answers; answers the first entry's number.
    pub fn append(
        &mut self,
        session: &str,
        at: u64,
        bodies: Vec<Body>,
        commit: Commit,
    ) -> Result<u64, RunnerError> {
        let (first, unit) = (self.index.next_seq, self.index.committed);
        let mut text = String::new();
        let mut seq = first;
        for body in bodies.into_iter().chain([Body::Commit(commit.clone())]) {
            let entry = FeedEntry {
                seq,
                at,
                session: session.to_owned(),
                body,
            };
            text.push_str(&serde_json::to_string(&entry).map_err(failed)?);
            text.push('\n');
            seq += 1;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(failed)?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_data())
            .map_err(failed)?;
        self.index.committed += text.len() as u64;
        self.index.next_seq = seq;
        fold(&mut self.index, session, commit, unit);
        if let Err(error) = self.write_index() {
            crate::error::said(&format!(
                "the feed's index was not written, and the next start folds the unit again: {error}"
            ));
        }
        Ok(first)
    }

    /// The entries after `cursor`, the first when none is given, up to
    /// [`PAGE_MAX`]; refused `cursor_expired` for another feed's cursor and
    /// `cursor_ahead` for one past its end.
    pub fn page(&self, cursor: Option<&str>) -> Result<FeedPage, RunnerError> {
        let from = self.offset(cursor)?;
        let (entries, _, unread) = self.lines(from, self.index.committed)?;
        if let (Some(reason), true) = (unread, entries.is_empty()) {
            return Err(failed(reason));
        }
        let mut page = Vec::new();
        let mut next = from;
        for (entry, after) in entries {
            if page.len() == PAGE_MAX {
                break;
            }
            next = after;
            if !matches!(entry.body, Body::Commit(_)) {
                page.push(entry);
            }
        }
        Ok(FeedPage {
            format: FEED_FORMAT.to_owned(),
            entries: page,
            cursor: format!("{}:{next}", self.index.feed),
        })
    }

    /// Whether anything is committed after `cursor`.
    pub fn after(&self, cursor: Option<&str>) -> Result<bool, RunnerError> {
        Ok(self.offset(cursor)? < self.index.committed)
    }

    fn offset(&self, cursor: Option<&str>) -> Result<u64, RunnerError> {
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        let (feed, offset) = cursor.split_once(':').ok_or_else(|| {
            RunnerError::refused(
                "cursor_invalid",
                "a feed cursor is the feed's id and an offset",
            )
        })?;
        if feed != self.index.feed {
            return Err(RunnerError::refused(
                "cursor_expired",
                format!(
                    "the cursor is of feed {feed}, and this runner keeps feed {}: read again from the start of this one",
                    self.index.feed
                ),
            ));
        }
        let offset: u64 = offset.parse().map_err(|error| {
            RunnerError::refused(
                "cursor_invalid",
                format!("a feed cursor's offset is a number: {error}"),
            )
        })?;
        if offset > self.index.committed {
            return Err(RunnerError::refused(
                "cursor_ahead",
                "the cursor is past the end of the feed",
            ));
        }
        Ok(offset)
    }
}

/// Fold a unit's commit, the unit beginning at byte `unit`.
fn fold(index: &mut Index, session: &str, commit: Commit, unit: u64) {
    if let Some(source) = commit.source {
        index.sources.insert(session.to_owned(), source);
    }
    if let Some(attempt) = commit.attempt {
        index
            .attempts
            .entry(attempt_key(session, &attempt))
            .or_insert(unit);
    }
}
