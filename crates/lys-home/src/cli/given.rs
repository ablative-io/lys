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

use std::path::PathBuf;

use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::HomeError;
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
    /// The document's path exactly as the entry lists it.
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
}

/// `given`: every `lys.given` entry of the session, in file order.
pub fn list(args: &GivenArgs) -> Result<Value, HomeError> {
    let home = Home::open(&args.home)?;
    let session = home.open_session(&args.session)?;
    let records: Vec<ListedRecord> = GivenRecord::read_all(&session)?
        .into_iter()
        .map(|(entry, record)| ListedRecord { entry, record })
        .collect();
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
    let listed = record
        .documents
        .iter()
        .find(|document| document.path == args.path)
        .ok_or_else(|| HomeError::UnlistedDocument {
            entry: args.entry.clone(),
            path: args.path.clone(),
        })?;
    let bytes = std::fs::read(&args.file)
        .map_err(|e| HomeError::io("reading the file to check", &args.file, e))?;
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
    });
    Ok(Outcome {
        report,
        status: answer.status(),
    })
}
