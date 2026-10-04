//! The open-call journal and the sink that records calls.
//!
//! Before a call is sent upstream its id is written to the journal, one
//! durable file per open call; after the sink has recorded the call's
//! `lys.call` entry, the file is removed. On a start, every file still in
//! the journal is recovered once: its prepared outcome when present, otherwise
//! `lost` because capture did not finish ([`recover`]).
//!
//! Invariants:
//! - A journal record is durable (written, synced, renamed into place and its
//!   directory synced) before the call is sent upstream; one that cannot be
//!   written refuses the call with nothing sent.
//! - A lost call is recorded only when no terminal outcome for it is
//!   durable: the ingest is idempotent on the call id, so a call whose entry
//!   was written before the process died is answered as already recorded,
//!   and nothing is written twice.
//! - The sink rewrites the call's record, naming its session, before it
//!   ingests; a call whose record cannot be rewritten is recorded
//!   `unrecorded`, and is held, with nothing ingested, until the journal can
//!   be written again ([`Sink::settle`] asks the sink to look again).
//! - A call whose path carried a run key has its usage line appended to the
//!   run's file after its entry is durable and before its journal file is
//!   removed ([`super::usage`]); a start that finds the journal file writes
//!   the line, so a death between the entry and the line loses no line.
//! - The sink writes no body byte anywhere but the home's block store, and
//!   of the headers only their names and the kept values (`proxy::headers`),
//!   on the call's record;
//!   its reports carry ids, a status and counts only.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use super::capture::Work;
use crate::proxy::error::ProxyError;
use crate::proxy::link::{Link, day_of};
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::call::captured::PreparedCall;
use crate::record::call::{Api, CallStatus};

/// What the journal keeps of an open call: ids and names only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCall {
    /// The proxy's id for the call.
    pub call_id: String,
    /// The provider the call went to.
    pub provider: String,
    /// The api its bodies follow.
    pub api: Api,
    /// When it started, RFC 3339.
    pub started_at: String,
    /// The session it is linked to, once its key was read.
    pub session: Option<String>,
    /// The run key the call's path opened with: the first path part a Lys
    /// launch puts on the base address it gives a run, taken off before the
    /// call is forwarded. None for a call whose path carried no key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// Admission hold, absent only until the measured gate has returned.
    pub admission_ns: Option<u64>,
    /// Prepared references, absent while capture has not completed.
    pub(crate) completed: Option<PreparedCall>,
}

/// The open-call journal: a directory of one record per open call.
#[derive(Clone, Debug)]
pub struct Journal {
    dir: PathBuf,
}

impl Journal {
    /// The journal in `dir`, created when absent.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, ProxyError> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)
            .map_err(|e| ProxyError::io("creating the open-call journal", &dir, e))?;
        Ok(Self { dir })
    }

    /// Where the journal lives.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Where the per-run usage files live: `usage`, beside the journal in
    /// the proxy's state ([`super::usage`]).
    #[must_use]
    pub fn usage_dir(&self) -> PathBuf {
        self.dir.parent().unwrap_or(&self.dir).join("usage")
    }

    fn path_of(&self, call_id: &str) -> PathBuf {
        self.dir.join(format!("{call_id}.json"))
    }

    /// Write (or rewrite) a call's record durably.
    pub fn write(&self, call: &OpenCall) -> Result<(), ProxyError> {
        use std::io::Write;
        #[cfg(test)]
        let started = std::time::Instant::now();
        let bytes = serde_json::to_vec(call).map_err(|source| ProxyError::JournalEncode {
            call_id: call.call_id.clone(),
            source,
        })?;
        let path = self.path_of(&call.call_id);
        let tmp = self.dir.join(format!(".{}.json.tmp", call.call_id));
        let unwritable = |source| ProxyError::JournalUnwritable {
            path: path.clone(),
            source,
        };
        let mut file = std::fs::File::create(&tmp).map_err(unwritable)?;
        file.write_all(&bytes).map_err(unwritable)?;
        file.sync_all().map_err(unwritable)?;
        drop(file);
        std::fs::rename(&tmp, &path).map_err(unwritable)?;
        let result = sync_dir(&self.dir).map_err(unwritable);
        #[cfg(test)]
        super::timing::add(&super::timing::JOURNAL_WRITE, started);
        result
    }

    /// Remove a call's record once its outcome is recorded.
    pub fn retire(&self, call_id: &str) -> Result<(), ProxyError> {
        let path = self.path_of(call_id);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(ProxyError::io("retiring a journal record", &path, e)),
        }
        sync_dir(&self.dir).map_err(|e| ProxyError::io("syncing the journal", &self.dir, e))
    }

    /// Every open call's record, in file-name order.
    pub fn open_calls(&self) -> Result<Vec<OpenCall>, ProxyError> {
        let listing = std::fs::read_dir(&self.dir)
            .map_err(|e| ProxyError::io("listing the open-call journal", &self.dir, e))?;
        let mut paths = Vec::new();
        for item in listing {
            let item =
                item.map_err(|e| ProxyError::io("listing the open-call journal", &self.dir, e))?;
            let path = item.path();
            let name = item.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with('.') && name.ends_with(".json") {
                paths.push(path);
            }
        }
        paths.sort();
        let mut calls = Vec::with_capacity(paths.len());
        for path in paths {
            let bytes = std::fs::read(&path)
                .map_err(|e| ProxyError::io("reading a journal record", &path, e))?;
            let call = serde_json::from_slice(&bytes)
                .map_err(|source| ProxyError::JournalRecord { path, source })?;
            calls.push(call);
        }
        Ok(calls)
    }

    /// Whether the journal in `dir` holds an open call that carries the run
    /// key `run`. This is how another process than the proxy asks whether a
    /// run still has a call whose usage line is yet to be written: the line
    /// is appended before the call's record is retired, so a run with no
    /// record here has every finished call's line in its usage file. A record
    /// retired between the listing and its read is no longer open.
    pub fn holds_run(dir: &Path, run: &str) -> Result<bool, ProxyError> {
        let listing = std::fs::read_dir(dir)
            .map_err(|e| ProxyError::io("listing the open-call journal", dir, e))?;
        for item in listing {
            let item = item.map_err(|e| ProxyError::io("listing the open-call journal", dir, e))?;
            let name = item.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }
            let path = item.path();
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(ProxyError::io("reading a journal record", &path, e)),
            };
            let call: OpenCall = serde_json::from_slice(&bytes)
                .map_err(|source| ProxyError::JournalRecord { path, source })?;
            if call.run.as_deref() == Some(run) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

/// One call handed to the sink at its end.
#[derive(Debug)]
pub struct Job {
    /// The call's journal record; its `session` is the link.
    pub call: OpenCall,
    /// How the call ended, as the proxy saw it.
    pub status: CallStatus,
    /// How long it took.
    pub duration_ms: u64,
    /// Whether the response was an event stream.
    pub stream: bool,
    /// The spooled request body, when one was written.
    pub request: Option<PathBuf>,
    /// The spooled response body, when one was written.
    pub response: Option<PathBuf>,
    /// The response parts the proxy assembled from an event stream.
    pub parts: Option<Vec<serde_json::Value>>,
    /// Raw-body hashes computed during spooling.
    pub request_hash: Option<Hash>,
    /// Raw response hash computed during spooling.
    pub response_hash: Option<Hash>,
    /// Last frame arrival, absent for a recovered call.
    pub last_arrival: Option<std::time::Instant>,
    /// Per-call worker measurements.
    pub timing: crate::record::call::captured::CaptureTiming,
    /// What the proxy read of the call beside its bodies.
    pub seen: crate::record::call::captured::Seen,
}

/// What the sink reports of each call: ids, a status and counts only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallReport {
    /// The proxy's id for the call.
    pub call_id: String,
    /// The home session the call is recorded under.
    pub session: String,
    /// Whether the call carried a session key; `false` means the session
    /// is the day's `unlinked` one.
    pub linked: bool,
    /// The status recorded.
    pub status: CallStatus,
    /// The `lys.call` entry, once recorded.
    pub entry_id: Option<String>,
    /// Whether the call id was already recorded, so nothing was written.
    pub already_recorded: bool,
    /// Whether the journal record was removed.
    pub retired: bool,
    /// Spool files that could not be removed.
    pub spool_kept: u64,
    /// Why the call is held rather than recorded, when it is.
    pub held: Option<String>,
    /// Nanoseconds from the last response frame until the durable entry append returned; absent on recovery.
    pub time_to_record_ns: Option<u64>,
}

#[derive(Debug)]
enum Message {
    Capture(Work),
    Settle,
    #[cfg(test)]
    Stop,
    #[cfg(test)]
    Pause(mpsc::Sender<()>, mpsc::Receiver<()>),
}

/// The handle through which calls reach the sink thread.
#[derive(Clone, Debug)]
pub struct Sink {
    tx: mpsc::Sender<Message>,
    #[cfg(test)]
    worker: std::sync::Arc<std::sync::Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl Sink {
    /// Start the sink thread over a home and a journal; every report is sent
    /// to `reports`.
    pub fn start(home: Home, journal: Journal, reports: mpsc::Sender<CallReport>) -> Self {
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || run(&home, &journal, &rx, &reports));
        #[cfg(not(test))]
        drop(worker);
        Self {
            tx,
            #[cfg(test)]
            worker: std::sync::Arc::new(std::sync::Mutex::new(Some(worker))),
        }
    }

    #[cfg(test)]
    pub(super) fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let worker = self
            .worker
            .lock()
            .map_err(|error| std::io::Error::other(error.to_string()))?
            .take();
        if let Some(worker) = worker {
            let sent = self
                .tx
                .send(Message::Stop)
                .map_err(|error| std::io::Error::other(error.to_string()));
            let joined = worker.join();
            sent?;
            if let Err(panic) = joined {
                std::panic::resume_unwind(panic);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn pause(
        &self,
    ) -> Result<mpsc::Sender<()>, Box<dyn std::error::Error + Send + Sync>> {
        let (ready, arrived) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        self.tx
            .send(Message::Pause(ready, resume))
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        arrived.recv()?;
        Ok(release)
    }

    pub(super) fn capture(&self, work: Work) -> Result<(), ProxyError> {
        self.tx
            .send(Message::Capture(work))
            .map_err(|unsent| ProxyError::SinkStopped {
                what: unsent.0.name(),
            })
    }

    /// Ask the sink to look again at what it holds.
    pub fn settle(&self) -> Result<(), ProxyError> {
        self.tx
            .send(Message::Settle)
            .map_err(|unsent| ProxyError::SinkStopped {
                what: unsent.0.name(),
            })
    }
}

impl Message {
    fn name(&self) -> String {
        match self {
            Self::Capture(..) => String::from("a capture event"),
            Self::Settle => String::from("a settle request"),
            #[cfg(test)]
            Self::Stop => String::from("a worker shutdown"),
            #[cfg(test)]
            Self::Pause(..) => String::from("a worker barrier"),
        }
    }
}

fn run(
    home: &Home,
    journal: &Journal,
    rx: &mpsc::Receiver<Message>,
    reports: &mpsc::Sender<CallReport>,
) {
    let mut held: Vec<Job> = Vec::new();
    let mut unretired: Vec<String> = Vec::new();
    for message in rx {
        #[cfg(test)]
        if let Message::Pause(ready, resume) = message {
            if ready.send(()).is_ok()
                && let Err(mpsc::RecvError) = resume.recv()
            {
                continue;
            }
            continue;
        }
        match message {
            Message::Capture(work) => match work.run() {
                Some(job) => held.push(job),
                None => continue,
            },
            Message::Settle => {}
            #[cfg(test)]
            Message::Stop => return,
            #[cfg(test)]
            Message::Pause(..) => continue,
        }
        unretired.retain(|call_id| journal.retire(call_id).is_err());
        for mut job in std::mem::take(&mut held) {
            let report = record(home, journal, &mut job);
            if report.held.is_some() {
                held.push(job);
            } else {
                if !report.retired && report.spool_kept == 0 {
                    unretired.push(job.call.call_id.clone());
                }
                drop(job);
            }
            #[cfg(test)]
            super::timing::mark(&super::timing::REPORT_SENT);
            if let Err(unread) = reports.send(report) {
                // The sink goes on recording; the report is named on stderr.
                eprintln!(
                    "lys-proxy: no reader for the report of call {}",
                    unread.0.call_id
                );
            }
        }
    }
}

/// Record one call: rewrite its journal record naming its session, ingest
/// it, remove its spool files and retire its record.
fn record(home: &Home, journal: &Journal, job: &mut Job) -> CallReport {
    let link = Link::from_record(job.call.session.as_deref());
    let session_id = link.session_id(day_of(&job.call.started_at));
    let mut report = CallReport {
        call_id: job.call.call_id.clone(),
        session: session_id.clone(),
        linked: link.is_linked(),
        status: job.status,
        entry_id: None,
        already_recorded: false,
        retired: false,
        spool_kept: 0,
        held: None,
        time_to_record_ns: None,
    };
    if let Err(error) = journal.write(&job.call) {
        // The journal cannot be written after the call was admitted: the call
        // is recorded unrecorded, once the journal can be written again.
        job.status = CallStatus::Unrecorded;
        if job.seen.unrecorded.is_none() {
            job.seen.unrecorded = Some(format!("the journal could not be written: {error}"));
        }
        report.status = CallStatus::Unrecorded;
        report.held = Some(error.to_string());
        return report;
    }
    #[cfg(test)]
    let started = std::time::Instant::now();
    let ingested = super::persist::ingest(home, journal, &session_id, job);
    #[cfg(test)]
    {
        super::timing::add(&super::timing::INGEST, started);
        super::timing::mark(&super::timing::DURABLE);
    }
    match ingested {
        Ok((status, ingested)) => {
            report.time_to_record_ns = job
                .last_arrival
                .map(|at| u64::try_from(at.elapsed().as_nanos()).unwrap_or(u64::MAX));
            report.status = status;
            report.entry_id = Some(ingested.entry_id);
            report.already_recorded = ingested.already_recorded;
        }
        Err(error) => {
            report.held = Some(error.to_string());
            return report;
        }
    }
    // The record is durable and the journal still holds the call: a death
    // before this line is written is a line written by the next start.
    if let Err(error) = super::usage::append(&journal.usage_dir(), &job.call) {
        report.held = Some(format!(
            "the call's usage line could not be written: {error}"
        ));
        return report;
    }
    for path in [job.request.take(), job.response.take()]
        .into_iter()
        .flatten()
    {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                eprintln!(
                    "lys-proxy: spool_remove_failed: {}: {error}",
                    path.display()
                );
                report.spool_kept += 1;
            }
        }
    }
    report.retired = report.spool_kept == 0 && journal.retire(&job.call.call_id).is_ok();
    report
}

/// Recover a prepared outcome, or an unfinished call as `lost`, once: a call whose
/// outcome was already recorded is answered as such and nothing is written.
/// Spool files the call left in `capture` are the bodies that exist.
pub fn recover(
    home: &Home,
    journal: &Journal,
    capture: &Path,
) -> Result<Vec<CallReport>, ProxyError> {
    let mut reports = Vec::new();
    for call in journal.open_calls()? {
        let spooled = |suffix: &str| {
            let path = capture.join(format!("{}.{suffix}", call.call_id));
            (path.is_file() || call.completed.is_some()).then_some(path)
        };
        let mut job = Job {
            status: call
                .completed
                .as_ref()
                .map_or(CallStatus::Lost, |ready| ready.record.status),
            duration_ms: 0,
            stream: false,
            request: spooled("request"),
            response: spooled("response"),
            parts: None,
            request_hash: None,
            response_hash: None,
            last_arrival: None,
            timing: crate::record::call::captured::CaptureTiming::interrupted(call.admission_ns),
            // Nothing read in flight survives the process but the journalled
            // run key; a prepared record keeps its own.
            seen: crate::record::call::captured::Seen {
                run: call.run.clone(),
                ..Default::default()
            },
            call,
        };
        let report = record(home, journal, &mut job);
        if let Some(reason) = &report.held {
            return Err(ProxyError::io(
                "recording a lost call",
                journal.dir(),
                std::io::Error::other(reason.clone()),
            ));
        }
        reports.push(report);
    }
    Ok(reports)
}
