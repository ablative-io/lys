//! `lys-home given` and `lys-home given-check` (HOME-003 R5): list a
//! session's given records, and check a document a record lists against a
//! file on disk by hash. Both print one JSON report of ids, paths, lengths,
//! hashes and names, never a byte of any file.
//!
//! `given-check` answers as `diff` does, so it can stand in a script: the
//! process exits 0 when the answer is `matches`, 1 when it is `differs`, and
//! 2 on a refusal (a missing argument, a session or file that cannot be read,
//! an entry id or a listed path that is not in the session). Differs is an
//! answer, not a refusal, and the status tells the two apart; the printed
//! report is the same shape either way.
//!
//! Paths are compared canonical (HOME-010 R3, ADR-029): `given-check`
//! canonicalises each listed path and the `--path` argument by the record's
//! document rule at check time, so a record written before the rule, or a
//! path reached through a symlinked directory or a `..`, compares by the one
//! file it names; a relative or unresolved path compares as written, and
//! `--file` is read at its canonical path. Both reports carry `unresolved`,
//! the paths left as given because they do not exist; every other field
//! echoes what it did before, and nothing is added to the entry's data.

use std::path::{Path, PathBuf};

use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::given_path::{canonical_dir, canonical_document};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::given::GivenRecord;

/// The status `given-check` exits with when the answer is `differs`.
pub const STATUS_DIFFERS: i32 = 1;
/// The status `given-check` exits with when it refuses.
pub const STATUS_REFUSED: i32 = 2;

/// The arguments of `given`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct GivenArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The session whose given records to list.
    #[arg(long)]
    pub session: String,
}

/// The arguments of `given-check`.
#[derive(Clone, Debug, PartialEq, Eq, Args)]
pub struct GivenCheckArgs {
    /// The home directory.
    #[arg(long)]
    pub home: PathBuf,
    /// The session holding the given record.
    #[arg(long)]
    pub session: String,
    /// The given entry's id, as `given` reports it.
    #[arg(long)]
    pub entry: String,
    /// The document's path as the entry lists it, or another path to the
    /// same file; both are compared canonicalised.
    #[arg(long)]
    pub path: PathBuf,
    /// The file on disk to check against it.
    #[arg(long)]
    pub file: PathBuf,
}

/// What a check answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    /// The file's length and SHA-256 equal the listed document's.
    Matches,
    /// They do not.
    Differs,
}

impl Answer {
    /// The status the process exits with for this answer, as `diff` does.
    #[must_use]
    pub fn status(self) -> i32 {
        match self {
            Self::Matches => 0,
            Self::Differs => STATUS_DIFFERS,
        }
    }
}

/// A command's report and the status the process exits with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The one JSON report, printed to stdout.
    pub report: Value,
    /// The exit status; 0 for every command but a `given-check` that differs.
    pub status: i32,
}

impl Outcome {
    /// A report of a command that ended as it should.
    #[must_use]
    pub fn done(report: Value) -> Self {
        Self { report, status: 0 }
    }
}

/// One given record as `given` reports it: the entry id, then the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListedRecord {
    /// The entry's id.
    pub entry: String,
    /// The record.
    #[serde(flatten)]
    pub record: GivenRecord,
    /// The record's config directory and then its absolute document paths,
    /// each as recorded and only when it does not exist at listing time.
    pub unresolved: Vec<PathBuf>,
}

impl ListedRecord {
    /// The record listed under its entry id, with the paths of it that do
    /// not exist now named as unresolved.
    pub fn new(entry: String, record: GivenRecord) -> Result<Self, HomeError> {
        let mut unresolved = Vec::new();
        if canonical_dir(&record.config_dir.path)?.is_unresolved() {
            unresolved.push(record.config_dir.path.clone());
        }
        for document in record.documents.iter().filter(|d| d.path.is_absolute()) {
            if canonical_document(&document.path)?.is_unresolved() {
                unresolved.push(document.path.clone());
            }
        }
        Ok(Self {
            entry,
            record,
            unresolved,
        })
    }
}

/// `given`: every `lys.given` entry of the session, in file order.
pub fn list(args: &GivenArgs) -> Result<Value, HomeError> {
    let home = Home::open(&args.home)?;
    let session = home.open_session(&args.session)?;
    let records = GivenRecord::read_all(&session)?
        .into_iter()
        .map(|(entry, record)| ListedRecord::new(entry, record))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "command": "given",
        "session": args.session,
        "count": records.len(),
        "records": records,
    }))
}

/// `given-check`: hash the file and answer `matches` when its SHA-256 and
/// length equal the listed document's, `differs` otherwise. A refusal names
/// the id, the path or the file concerned.
pub fn check(args: &GivenCheckArgs) -> Result<Outcome, HomeError> {
    let home = Home::open(&args.home)?;
    let session = home.open_session(&args.session)?;
    let entry = session.entry(&args.entry)?;
    let record = GivenRecord::from_entry(&entry)?;
    let wanted = canonical_document(&args.path)?;
    let mut unresolved: Vec<&Path> = Vec::new();
    if wanted.is_unresolved() {
        unresolved.push(&args.path);
    }
    let mut found = None;
    for document in &record.documents {
        let listed = canonical_document(&document.path)?;
        if listed.path() == wanted.path() {
            found = Some((document, listed.is_unresolved()));
            break;
        }
    }
    let (listed, listed_unresolved) = found.ok_or_else(|| HomeError::UnlistedDocument {
        entry: args.entry.clone(),
        path: args.path.clone(),
    })?;
    if listed_unresolved && !unresolved.contains(&listed.path.as_path()) {
        unresolved.push(&listed.path);
    }
    let file = canonical_document(&args.file)?;
    let bytes = std::fs::read(file.path())
        .map_err(|e| HomeError::io("reading the file to check", file.path(), e))?;
    let length = bytes.len() as u64;
    let sha256 = Hash::of(&bytes);
    drop(bytes);
    let answer = if length == listed.length && sha256.as_str() == listed.sha256 {
        Answer::Matches
    } else {
        Answer::Differs
    };
    let report = json!({
        "command": "given-check",
        "entry": args.entry,
        "path": args.path,
        "file": args.file,
        "answer": answer,
        "listed": {"kind": listed.kind, "length": listed.length, "sha256": listed.sha256},
        "on_disk": {"length": length, "sha256": sha256.as_str()},
        "unresolved": unresolved,
    });
    Ok(Outcome {
        report,
        status: answer.status(),
    })
}
