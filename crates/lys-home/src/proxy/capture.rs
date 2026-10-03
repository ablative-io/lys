//! Capture: the request and response bodies spooled to files while
//! the sink worker receives shared byte handles, the session key read once,
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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::spool::Spool;
use hyper::HeaderMap;
use hyper::body::Bytes;
use hyper::header::{GetAll, HeaderValue};

use crate::proxy::decode::Reader;
use crate::proxy::forward::{End, Observer};
use crate::proxy::journal::{Job, Journal, OpenCall, Sink};
use crate::proxy::link::{KeyScanner, Link};
use crate::record::call::CallStatus;
use crate::record::call::captured::CaptureTiming;

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
    last_arrival: Option<Instant>,
    pending_bytes: u64,
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
    processed: AtomicU64,
}

impl Call {
    /// Queue capture after the measured durable admission gate.
    #[must_use]
    pub fn admit(open: OpenCall, capture: &Path, journal: Journal, sink: Sink) -> Arc<Self> {
        let call = Arc::new(Self {
            open,
            started: Instant::now(),
            capture: capture.to_path_buf(),
            journal,
            sink,
            state: Mutex::new(CallState::default()),
            poisoned: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            processed: AtomicU64::new(0),
        });
        call.enqueue(Event::Start);
        call
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
    /// response head queues decoder and spool creation on the worker.
    #[must_use]
    pub fn response_side(
        self: &Arc<Self>,
        stream: bool,
        encodings: GetAll<'_, HeaderValue>,
    ) -> ResponseSide {
        let mut headers = HeaderMap::new();
        for value in encodings {
            headers.append(hyper::header::CONTENT_ENCODING, value.clone());
        }
        self.enqueue(Event::ResponseHead { stream, headers });
        ResponseSide {
            call: Arc::clone(self),
            offered: 0,
        }
    }

    fn request_ended(&self) {
        let link = self
            .with(|s| {
                if s.finished {
                    return None;
                }
                if let Some(spool) = &mut s.request {
                    spool.close(&mut s.capture_failed);
                }
                Some(s.scanner.link())
            })
            .flatten();
        if let Some(link) = link.filter(Link::is_linked) {
            let mut open = self.open.clone();
            open.session = link.to_record();
            if let Err(error) = self.journal.write(&open) {
                eprintln!("lys-proxy: {error}");
                self.with(|s| s.journal_failed = true);
            }
        }
    }

    /// End the call and hand it to the sink; a second end does nothing.
    pub fn finish(self: &Arc<Self>, end: End) {
        if self.ended.swap(true, Ordering::AcqRel) {
            return;
        }
        self.enqueue(Event::End(end, Instant::now()));
    }

    fn complete(&self, end: End, ended_at: Instant) -> Option<Job> {
        let duration_ms =
            u64::try_from(ended_at.duration_since(self.started).as_millis()).unwrap_or(u64::MAX);
        let job = self.with(|s| {
            if s.finished {
                return None;
            }
            s.finished = true;
            let mut timing = CaptureTiming::interrupted(self.open.admission_ns);
            timing.pending_bytes = s.pending_bytes;
            timing.drain_ns = s.last_arrival.map_or(0, |at| {
                u64::try_from(at.elapsed().as_nanos()).unwrap_or(u64::MAX)
            });
            #[cfg(test)]
            if let Some(arrived) = s.last_arrival {
                super::timing::drained(arrived);
            }
            for spool in [&mut s.request, &mut s.response].into_iter().flatten() {
                spool.close(&mut s.capture_failed);
                timing.spool_writes += spool.writes;
                timing.spool_bytes += spool.bytes;
                timing.spool_syncs += spool.syncs;
                timing.write_ns += spool.write_ns;
                timing.hash_ns += spool.hash_ns;
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
                request_hash: s.request.as_ref().and_then(|spool| spool.hash.clone()),
                response_hash: s.response.as_ref().and_then(|spool| spool.hash.clone()),
                request: s.request.take().map(|spool| spool.path),
                response: s.response.take().map(|spool| spool.path),
                last_arrival: Some(s.last_arrival.unwrap_or(ended_at)),
                parts,
                timing,
            })
        });
        // An interrupted capture keeps its durable open journal entry;
        // recovery records the lost call without trusting its partial state.
        job.flatten()
    }

    fn enqueue(self: &Arc<Self>, event: Event) {
        if let Err(error) = self.sink.capture(Work {
            call: Arc::clone(self),
            event,
        }) {
            eprintln!("lys-proxy: {error}");
        }
    }
}

#[derive(Debug)]
pub(super) struct Work {
    call: Arc<Call>,
    event: Event,
}

#[derive(Debug)]
enum Event {
    Start,
    Request(Bytes),
    RequestEnd,
    ResponseHead { stream: bool, headers: HeaderMap },
    Response(Bytes, Instant, u64),
    End(End, Instant),
}

impl Work {
    pub(super) fn run(self) -> Option<Job> {
        let call = self.call;
        match self.event {
            Event::Start => {
                call.with(|s| {
                    s.request =
                        Spool::create(call.capture.join(format!("{}.request", call.open.call_id)));
                    s.capture_failed = s.request.is_none();
                });
            }
            Event::Request(bytes) => {
                call.with(|s| {
                    s.scanner.feed(&bytes);
                    if let Some(spool) = &mut s.request {
                        spool.write(&bytes, &mut s.capture_failed);
                    }
                });
            }
            Event::RequestEnd => call.request_ended(),
            Event::ResponseHead { stream, headers } => {
                call.with(|s| {
                    s.stream = stream;
                    s.reader = stream.then(|| {
                        Reader::for_api(
                            call.open.api,
                            headers.get_all(hyper::header::CONTENT_ENCODING),
                        )
                    });
                    if !s.capture_failed {
                        s.response = Spool::create(
                            call.capture.join(format!("{}.response", call.open.call_id)),
                        );
                        s.capture_failed = s.response.is_none();
                    }
                });
            }
            Event::Response(bytes, arrived, pending) => {
                call.with(|s| {
                    s.last_arrival = Some(arrived);
                    s.pending_bytes = pending;
                    #[cfg(test)]
                    let started = Instant::now();
                    if let Some(reader) = &mut s.reader
                        && let Err(error) = reader.feed(&bytes)
                    {
                        eprintln!(
                            "lys-proxy: response_decode_failed: call {}: {error}",
                            call.open.call_id
                        );
                        s.reader = None;
                    }
                    #[cfg(test)]
                    super::timing::add(&super::timing::DECODE, started);
                    if let Some(spool) = &mut s.response {
                        spool.write(&bytes, &mut s.capture_failed);
                    }
                    #[cfg(test)]
                    super::timing::spooled(bytes.len());
                    call.processed.fetch_add(
                        u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                        Ordering::Relaxed,
                    );
                });
            }
            Event::End(end, at) => return call.complete(end, at),
        }
        None
    }
}

/// Reads the request body as it goes upstream: the session key, and the
/// spool for the call.
#[derive(Debug)]
pub struct RequestSide(Arc<Call>);

impl Observer for RequestSide {
    /// Enqueue a shared byte handle without waiting for capture.
    fn data(&mut self, bytes: &Bytes) {
        self.0.enqueue(Event::Request(bytes.clone()));
    }

    fn ended(&mut self, end: End) {
        if end == End::Complete {
            self.0.enqueue(Event::RequestEnd);
        }
    }
}

/// Reads the response body as it goes to the client: the stream grammar,
/// and the spool for the call; its end ends the call.
#[derive(Debug)]
pub struct ResponseSide {
    call: Arc<Call>,
    offered: u64,
}

impl Observer for ResponseSide {
    /// Enqueue a shared byte handle and its arrival time; no decode, lock or disk work.
    fn data(&mut self, bytes: &Bytes) {
        #[cfg(test)]
        super::timing::arriving();
        #[cfg(test)]
        super::timing::offered(bytes.len());
        self.offered += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let pending = self
            .offered
            .saturating_sub(self.call.processed.load(Ordering::Relaxed));
        self.call
            .enqueue(Event::Response(bytes.clone(), Instant::now(), pending));
    }

    fn ended(&mut self, end: End) {
        self.call.finish(end);
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
            admission_ns: None,
            completed: None,
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
