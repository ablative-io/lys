//! Capture: the request and response bodies spooled to files while
//! they are forwarded, the session key read from the request as it passes,
//! the event stream read as it passes, and the call handed to the sink when
//! it ends.
//!
//! Invariants:
//! - A spool that cannot be created or written stops spooling; the bytes go
//!   on to the client as they came, and the call is recorded `unrecorded`.
//! - A call ends once: `complete` only when the upstream response ended
//!   whole, every spool was written and synced, the journal took the call's
//!   link, and, for an event stream, the grammar says the stream ended whole;
//!   `cancelled` when the client went before the response ended; `partial`
//!   when the upstream failed or its stream ended early or malformed. No
//!   other path reports a call complete.
//! - The spools hold body bytes only; no header is written anywhere.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use hyper::body::Bytes;
use hyper::header::{GetAll, HeaderValue};

use crate::proxy::decode::Reader;
use crate::proxy::forward::{End, Observer};
use crate::proxy::journal::{Job, Journal, OpenCall, Sink};
use crate::proxy::link::{KeyScanner, Link};
use crate::record::call::CallStatus;

/// A body spooled to a file.
#[derive(Debug)]
struct Spool {
    path: PathBuf,
    file: Option<File>,
}

impl Spool {
    fn create(path: PathBuf) -> Option<Self> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .ok()?;
        Some(Self {
            path,
            file: Some(file),
        })
    }

    fn write(&mut self, bytes: &[u8], failed: &mut bool) {
        if let Some(file) = &mut self.file
            && file.write_all(bytes).is_err()
        {
            self.file = None;
            *failed = true;
        }
    }

    fn close(&mut self, failed: &mut bool) {
        if let Some(file) = self.file.take()
            && file.sync_all().is_err()
        {
            *failed = true;
        }
    }
}

/// What a call knows while it passes.
#[derive(Debug, Default)]
struct CallState {
    scanner: KeyScanner,
    request: Option<Spool>,
    response: Option<Spool>,
    capture_failed: bool,
    journal_failed: bool,
    reader: Option<Reader>,
    stream: bool,
    finished: bool,
}

/// One call in flight: its journal record, its capture and where it goes
/// when it ends.
#[derive(Debug)]
pub struct Call {
    open: OpenCall,
    started: Instant,
    capture: PathBuf,
    journal: Journal,
    sink: Sink,
    state: Mutex<CallState>,
    poisoned: AtomicBool,
    ended: AtomicBool,
}

impl Call {
    /// Gate: creates the request spool after durable admission.
    #[must_use]
    pub fn admit(open: OpenCall, capture: &Path, journal: Journal, sink: Sink) -> Arc<Self> {
        let request = Spool::create(capture.join(format!("{}.request", open.call_id)));
        let state = CallState {
            capture_failed: request.is_none(),
            request,
            ..CallState::default()
        };
        Arc::new(Self {
            open,
            started: Instant::now(),
            capture: capture.to_path_buf(),
            journal,
            sink,
            state: Mutex::new(state),
            poisoned: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        })
    }

    /// The proxy's id for the call.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.open.call_id
    }

    fn with<R>(&self, f: impl FnOnce(&mut CallState) -> R) -> Option<R> {
        match self.state.lock() {
            Ok(mut state) => Some(f(&mut state)),
            Err(error) => {
                if !self.poisoned.swap(true, Ordering::AcqRel) {
                    eprintln!(
                        "lys-proxy: StatePoisoned: call {}: {error}",
                        self.open.call_id
                    );
                }
                None
            }
        }
    }

    /// The observer of the request body on its way upstream.
    #[must_use]
    pub fn request_side(self: &Arc<Self>) -> RequestSide {
        RequestSide(Arc::clone(self))
    }

    /// The observer of the response body on its way to the client; the
    /// response spool is created now, as its head has arrived.
    #[must_use]
    pub fn response_side(
        self: &Arc<Self>,
        stream: bool,
        encodings: GetAll<'_, HeaderValue>,
    ) -> ResponseSide {
        let api = self.open.api;
        let spool = self.capture.join(format!("{}.response", self.open.call_id));
        self.with(|s| {
            s.stream = stream;
            s.reader = stream.then(|| Reader::for_api(api, encodings));
            if !s.capture_failed {
                s.response = Spool::create(spool);
                s.capture_failed = s.response.is_none();
            }
        });
        ResponseSide(Arc::clone(self))
    }

    fn request_ended(&self) {
        let link = self.with(|s| {
            if let Some(spool) = &mut s.request {
                spool.close(&mut s.capture_failed);
            }
            s.scanner.link()
        });
        if let Some(link) = link.filter(Link::is_linked) {
            let mut open = self.open.clone();
            open.session = link.to_record();
            if self.journal.write(&open).is_err() {
                self.with(|s| s.journal_failed = true);
            }
        }
    }

    /// End the call and hand it to the sink; a second end does nothing.
    pub fn finish(&self, end: End) {
        if self.ended.swap(true, Ordering::AcqRel) {
            return;
        }
        let duration_ms = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let job = self.with(|s| {
            if s.finished {
                return None;
            }
            s.finished = true;
            for spool in [&mut s.request, &mut s.response].into_iter().flatten() {
                spool.close(&mut s.capture_failed);
            }
            let parts = s.reader.take().and_then(|reader| {
                if end != End::Complete {
                    return None;
                }
                match reader.finish() {
                    Ok(parts) => parts,
                    Err(error) => {
                        eprintln!(
                            "lys-proxy: response_decode_failed: call {}: {error}",
                            self.open.call_id
                        );
                        None
                    }
                }
            });
            let unspooled = s.capture_failed || s.journal_failed;
            let status = match end {
                End::Dropped => CallStatus::Cancelled,
                End::Failed => CallStatus::Partial,
                End::Complete if s.stream && parts.is_none() => CallStatus::Partial,
                End::Complete if unspooled => CallStatus::Unrecorded,
                End::Complete => CallStatus::Complete,
            };
            let mut call = self.open.clone();
            call.session = s.scanner.link().to_record();
            Some(Job {
                call,
                status,
                duration_ms,
                stream: s.stream,
                request: s.request.take().map(|spool| spool.path),
                response: s.response.take().map(|spool| spool.path),
                parts,
            })
        });
        // An interrupted capture keeps its durable open journal entry;
        // recovery records the lost call without trusting its partial state.
        if let Some(job) = job.flatten()
            && let Err(error) = self.sink.send(job)
        {
            eprintln!("lys-proxy: {error}");
        }
    }
}

/// Reads the request body as it goes upstream: the session key, and the
/// spool for the call.
#[derive(Debug)]
pub struct RequestSide(Arc<Call>);

impl Observer for RequestSide {
    /// Gate: holds the frame for session scanning and spool write.
    fn data(&mut self, bytes: &Bytes) {
        self.0.with(|s| {
            s.scanner.feed(bytes);
            if let Some(spool) = &mut s.request {
                spool.write(bytes, &mut s.capture_failed);
            }
        });
    }

    fn ended(&mut self, end: End) {
        if end == End::Complete {
            self.0.request_ended();
        }
    }
}

/// Reads the response body as it goes to the client: the stream grammar,
/// and the spool for the call; its end ends the call.
#[derive(Debug)]
pub struct ResponseSide(Arc<Call>);

impl Observer for ResponseSide {
    /// Gate: holds the frame for decode and spool write under the call state lock.
    fn data(&mut self, bytes: &Bytes) {
        self.0.with(|s| {
            if let Some(reader) = &mut s.reader
                && let Err(error) = reader.feed(bytes)
            {
                eprintln!(
                    "lys-proxy: response_decode_failed: call {}: {error}",
                    self.0.open.call_id
                );
                s.reader = None;
            }
            if let Some(spool) = &mut s.response {
                spool.write(bytes, &mut s.capture_failed);
            }
        });
    }

    fn ended(&mut self, end: End) {
        self.0.finish(end);
    }
}

#[cfg(test)]
mod poison_tests {
    use super::*;
    use crate::record::Home;
    use crate::record::call::Api;
    use std::sync::mpsc;

    #[test]
    fn an_interrupted_capture_names_the_fault_and_retains_its_durable_journal()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let home = Home::open(dir.path().join("home"))?;
        let journal = Journal::open(dir.path().join("journal"))?;
        let open = OpenCall {
            call_id: "interrupted".to_owned(),
            provider: "provider".to_owned(),
            api: Api::Messages,
            started_at: "2000-01-01T00:00:00Z".to_owned(),
            session: None,
        };
        journal.write(&open)?;
        let (reports, received) = mpsc::channel();
        let sink = Sink::start(home, journal.clone(), reports);
        let call = Call::admit(open.clone(), dir.path(), journal.clone(), sink);
        let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            call.with(|state| {
                state.capture_failed = true;
                panic!("interrupted capture mutation");
            });
        }));
        assert!(interrupted.is_err());
        assert!(
            call.with(|_| panic!("poisoned state must not be entered"))
                .is_none()
        );
        assert!(call.poisoned.load(Ordering::Acquire));
        call.finish(End::Complete);
        call.finish(End::Complete);
        assert!(call.ended.load(Ordering::Acquire));
        assert_eq!(journal.open_calls()?, vec![open]);
        drop(call);
        assert!(received.recv().is_err());
        Ok(())
    }
}
