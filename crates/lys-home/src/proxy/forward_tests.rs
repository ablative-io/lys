#![cfg(test)]
//! Gates on forwarding against loopback fakes: the pass-through answers
//! with the upstream's status and chunks in order, a stream reaches the
//! client before its last event is sent, one call is one `lys.call`, and
//! every way a call can end short is recorded by its own status.
//!
//! No test waits on a clock: each fake moves on only when the test tells it
//! the client has what it was sent, so a proxy that buffered would stop
//! here rather than pass.

use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll};

use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes, Frame, Incoming};
use hyper::header::{CONTENT_TYPE, HeaderValue};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc as channel;
use tokio::task::JoinHandle;

use crate::proxy::forward::{Base, Proxy, ProxyBody, ProxyConfig, Upstream, pass_through, serve};
use crate::proxy::journal::CallReport;
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::call::{Api, CallRecord, CallStatus, call_record, request_parts};
use crate::record::entries::{CUSTOM_CALL, EntryBody};

/// What a gate or a fixture returns.
pub(super) type Res<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

/// The session key the keyed fixtures carry.
pub(super) const KEY: &str = "0f1e2d3c-4b5a-4968-8776-a5b4c3d2e1f0";

/// A response body fed from a channel: each message is one frame, and the
/// body ends when the sender goes.
pub(super) struct ChanBody(channel::Receiver<Bytes>);

impl Body for ChanBody {
    type Data = Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, hyper::Error>>> {
        self.0
            .poll_recv(cx)
            .map(|chunk| chunk.map(|bytes| Ok(Frame::data(bytes))))
    }
}

/// A body the test feeds, and the response that carries it.
pub(super) fn fed(
    status: StatusCode,
    content_type: &'static str,
) -> (channel::Sender<Bytes>, Response<ProxyBody>) {
    let (tx, rx) = channel::channel(1);
    let mut response = Response::new(ChanBody(rx).boxed_unsync());
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    (tx, response)
}

/// A whole JSON body.
pub(super) fn whole(status: StatusCode, body: &Value) -> Response<ProxyBody> {
    let mut response = Response::new(
        Full::new(Bytes::from(body.to_string()))
            .map_err(|never: Infallible| match never {})
            .boxed_unsync(),
    );
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

/// A fake upstream on loopback: every request is counted, its body read
/// whole, then answered by `answer`.
pub(super) async fn fake<A, F>(answer: A) -> Res<(SocketAddr, Arc<AtomicUsize>)>
where
    A: Fn() -> F + Send + Sync + 'static,
    F: Future<Output = Response<ProxyBody>> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let count = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&count);
    let answer = Arc::new(answer);
    tokio::spawn(serve(listener, move |request: Request<Incoming>| {
        seen.fetch_add(1, Ordering::SeqCst);
        let answer = Arc::clone(&answer);
        async move {
            match request.into_body().collect().await {
                Ok(_) => answer().await,
                Err(error) => whole(StatusCode::BAD_REQUEST, &json!({"fake": error.to_string()})),
            }
        }
    }));
    Ok((addr, count))
}

/// A proxy over a fresh home, forwarding both prefixes to one upstream.
pub(super) struct Harness {
    pub(super) dir: tempfile::TempDir,
    pub(super) proxy: Arc<Proxy>,
    pub(super) reports: mpsc::Receiver<CallReport>,
    pub(super) addr: SocketAddr,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    server: tokio::task::JoinHandle<Result<(), crate::proxy::error::ProxyError>>,
}

impl Harness {
    pub(super) async fn start(upstream: SocketAddr) -> Res<Self> {
        let dir = tempfile::tempdir()?;
        let base = Base::parse(&format!("http://{upstream}"))?;
        let started = Proxy::start(ProxyConfig {
            home: dir.path().join("home"),
            state: dir.path().join("state"),
            anthropic: base.clone(),
            // The same upstream under a path of its own, so a test can tell
            // which base a call was sent to.
            chatgpt: Base::parse(&format!("http://{upstream}/chatgpt-backend"))?,
            openai: base,
        })?;
        assert!(started.lost.is_empty());
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let proxy = Arc::clone(&started.proxy);
        let server = tokio::spawn(crate::proxy::forward::serve_until(
            listener,
            move |request| {
                let proxy = Arc::clone(&proxy);
                async move { proxy.handle(request).await }
            },
            async move {
                match stopped.await {
                    Ok(()) | Err(_) => {}
                }
            },
        ));
        Ok(Self {
            dir,
            proxy: started.proxy,
            reports: started.reports,
            addr,
            stop: Some(stop),
            server,
        })
    }

    /// The next report the sink sends.
    pub(super) fn report(&self) -> Res<CallReport> {
        Ok(tokio::task::block_in_place(|| self.reports.recv())?)
    }

    pub(super) fn home(&self) -> Res<Home> {
        Ok(Home::read(self.dir.path().join("home"))?)
    }

    pub(super) fn state(&self, sub: &str) -> PathBuf {
        self.dir.path().join("state").join(sub)
    }

    /// The `lys.call` records of a session, in file order.
    pub(super) fn calls(&self, session: &str) -> Res<Vec<CallRecord>> {
        let session = self.home()?.open_session(session)?;
        let mut records = Vec::new();
        for entry in session.customs_everywhere(CUSTOM_CALL)? {
            if let EntryBody::Custom {
                data: Some(data), ..
            } = &entry.body
            {
                records.push(call_record(data)?);
            }
        }
        Ok(records)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            match stop.send(()) {
                Ok(()) | Err(()) => {}
            }
        }
        let server = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(&mut self.server)
        });
        assert!(
            matches!(server, Ok(Ok(()))),
            "proxy fixture server did not shut down: {server:?}"
        );
        let worker = self.proxy.sink().shutdown();
        assert!(
            worker.is_ok(),
            "proxy fixture worker did not shut down: {worker:?}"
        );
    }
}

/// Send one request on its own connection; the connection's task is
/// returned so a test can close it.
pub(super) async fn send(
    addr: SocketAddr,
    request: Request<Full<Bytes>>,
) -> Res<(Response<Incoming>, JoinHandle<()>)> {
    let stream = TcpStream::connect(addr).await?;
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
    let task = tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("test client connection ended: {error}");
        }
    });
    Ok((sender.send_request(request).await?, task))
}

/// A Messages request body, keyed to `session` when one is given.
pub(super) fn messages_body(session: Option<&str>, stream: bool) -> Bytes {
    let mut body = json!({
        "model": "claude-test-model",
        "system": [{"type": "text", "text": "rho"}],
        "messages": [
            {"role": "user", "content": [{"type": "text", "text": "sigma"}]},
            {"role": "assistant", "content": [{"type": "text", "text": "tau"}]},
            {"role": "user", "content": "upsilon"}
        ],
        "stream": stream,
    });
    if let Some(session) = session {
        let user_id = format!("user_{}_account_a1_session_{session}", "0".repeat(64));
        body["metadata"] = json!({ "user_id": user_id });
    }
    Bytes::from(body.to_string())
}

/// A Messages request to the proxy, as Claude Code sends it.
pub(super) fn messages_request(session: Option<&str>, stream: bool) -> Res<Request<Full<Bytes>>> {
    Ok(Request::post("/anthropic/v1/messages?beta=true")
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(messages_body(session, stream)))?)
}

/// A complete JSON Messages response.
pub(super) fn message_response() -> Value {
    json!({"type": "message", "role": "assistant", "content": [{"type": "text", "text": "phi"}]})
}

fn sse_event(value: &Value) -> Bytes {
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("message");
    Bytes::from(format!("event: {kind}\ndata: {value}\n\n"))
}

/// A whole Messages stream with `deltas` text deltas: `deltas + 4` events.
pub(super) fn messages_events(deltas: usize) -> Vec<Bytes> {
    let mut events = vec![
        sse_event(&json!({"type": "message_start", "message": {"content": []}})),
        sse_event(&json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        })),
    ];
    for _ in 0..deltas {
        events.push(sse_event(&json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "x"}
        })));
    }
    events.push(sse_event(
        &json!({"type": "content_block_stop", "index": 0}),
    ));
    events.push(sse_event(&json!({"type": "message_stop"})));
    events
}

/// Read frames until `want` bytes have arrived.
async fn read_exactly(body: &mut Incoming, want: usize) -> Res<Vec<u8>> {
    let mut got = Vec::new();
    while got.len() < want {
        let frame = body.frame().await.ok_or("the body ended early")??;
        if let Some(data) = frame.data_ref() {
            got.extend_from_slice(data);
        }
    }
    Ok(got)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_pass_through_answers_with_the_upstream_status_and_its_three_chunks_in_order() -> Res {
    let chunks: [&'static [u8]; 3] = [b"first chunk;", b"second chunk;", b"third chunk."];
    let (ack_tx, ack_rx) = channel::channel::<()>(1);
    let ack_rx = Arc::new(tokio::sync::Mutex::new(ack_rx));
    let (upstream, count) = fake(move || {
        let ack_rx = Arc::clone(&ack_rx);
        async move {
            let (tx, response) = fed(StatusCode::IM_A_TEAPOT, "text/plain");
            tokio::spawn(async move {
                let mut acks = ack_rx.lock().await;
                for chunk in chunks {
                    if tx.send(Bytes::from_static(chunk)).await.is_err() {
                        return;
                    }
                    acks.recv().await;
                }
            });
            response
        }
    })
    .await?;
    let transport = Arc::new((
        Upstream::new()?,
        Base::parse(&format!("http://{upstream}"))?,
    ));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(serve(listener, move |request| {
        let transport = Arc::clone(&transport);
        async move { pass_through(&transport.0, &transport.1, request).await }
    }));
    let request = Request::get("/").body(Full::new(Bytes::new()))?;
    let (response, _connection) = send(addr, request).await?;
    assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
    let mut body = response.into_body();
    let mut received = Vec::new();
    for chunk in chunks {
        // The next chunk is sent only after this one arrived: order is the fake's.
        let got = read_exactly(&mut body, chunk.len()).await?;
        assert_eq!(got, chunk);
        received.push(got);
        ack_tx.send(()).await?;
    }
    assert!(body.frame().await.is_none());
    assert_eq!(received.len(), 3);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stream_of_200_events_reaches_the_client_before_its_last_event_is_sent() -> Res {
    let events = messages_events(196);
    assert_eq!(events.len(), 200);
    let (first_seen_tx, first_seen_rx) = channel::channel::<()>(1);
    let first_seen_rx = Arc::new(tokio::sync::Mutex::new(first_seen_rx));
    let to_send = events.clone();
    let (upstream, _) = fake(move || {
        let first_seen_rx = Arc::clone(&first_seen_rx);
        let to_send = to_send.clone();
        async move {
            let (tx, response) = fed(StatusCode::OK, "text/event-stream");
            tokio::spawn(async move {
                let mut events = to_send.into_iter();
                if let Some(first) = events.next()
                    && tx.send(first).await.is_ok()
                {
                    // The rest is sent only once the client holds the first.
                    first_seen_rx.lock().await.recv().await;
                    for event in events {
                        if tx.send(event).await.is_err() {
                            return;
                        }
                    }
                }
            });
            response
        }
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let (response, _connection) = send(harness.addr, messages_request(Some(KEY), true)?).await?;
    let mut body = response.into_body();
    let mut got = read_exactly(&mut body, events[0].len()).await?;
    assert_eq!(got, events[0].to_vec());
    first_seen_tx.send(()).await?;
    got.extend_from_slice(&body.collect().await?.to_bytes());
    assert_eq!(got, events.concat());
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Complete);
    assert_eq!(report.session, KEY);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].response.len(), 1);
    assert!(calls[0].stream);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_keyed_call_is_one_lys_call_whose_request_blocks_are_the_request_parts() -> Res {
    let (upstream, _) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream).await?;
    let sent = messages_body(Some(KEY), false);
    let (response, _connection) = send(harness.addr, messages_request(Some(KEY), false)?).await?;
    let answered = response.into_body().collect().await?.to_bytes();
    assert_eq!(answered, Bytes::from(message_response().to_string()));
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Complete);
    assert!(report.linked);
    assert!(report.retired);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    let mut parts = Vec::new();
    for part in request_parts(Api::Messages, &sent)? {
        parts.push(Hash::of(&serde_json::to_vec(&part)?).to_string());
    }
    assert_eq!(parts.len(), 4);
    assert_eq!(calls[0].request, parts);
    assert_eq!(calls[0].raw_request, Some(Hash::of(&sent).to_string()));
    assert_eq!(calls[0].model.as_deref(), Some("claude-test-model"));
    assert_eq!(calls[0].response.len(), 1);
    assert_eq!(std::fs::read_dir(harness.state("journal"))?.count(), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_upstream_that_drops_mid_request_sees_one_request_and_the_call_is_partial() -> Res {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let upstream = listener.local_addr()?;
    let accepted = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&accepted);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            seen.fetch_add(1, Ordering::SeqCst);
            if stream.readable().await.is_ok() {
                let mut some = [0u8; 16];
                if let Err(error) = stream.try_read(&mut some) {
                    eprintln!("the fake read nothing before closing: {error}");
                }
            }
            drop(stream);
        }
    });
    let harness = Harness::start(upstream).await?;
    let (response, _connection) = send(harness.addr, messages_request(Some(KEY), false)?).await?;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Partial);
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_client_that_closes_mid_stream_is_recorded_cancelled_with_no_response_parts() -> Res {
    let (upstream, _) = fake(|| async {
        let (tx, response) = fed(StatusCode::OK, "text/event-stream");
        tokio::spawn(async move {
            // Events go on until the proxy lets the stream go.
            for event in messages_events(1).into_iter().cycle() {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });
        response
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let (response, connection) = send(harness.addr, messages_request(Some(KEY), true)?).await?;
    let mut body = response.into_body();
    let first = body.frame().await.ok_or("no first frame")??;
    assert!(first.is_data());
    drop(body);
    connection.abort();
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Cancelled);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Cancelled);
    assert!(calls[0].response.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_upstream_stream_that_ends_before_message_stop_is_partial() -> Res {
    let (upstream, _) = fake(|| async {
        let (tx, response) = fed(StatusCode::OK, "text/event-stream");
        tokio::spawn(async move {
            for event in messages_events(3).into_iter().take(4) {
                if tx.send(event).await.is_err() {
                    return;
                }
            }
        });
        response
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let (response, _connection) = send(harness.addr, messages_request(Some(KEY), true)?).await?;
    let received = response.into_body().collect().await?.to_bytes();
    assert_eq!(received, messages_events(3)[..4].concat());
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Partial);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert!(calls[0].response.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_response_larger_than_the_old_bound_is_forwarded_and_recorded_whole() -> Res {
    let mut sent = vec![b'\n'; 64 * 1024 * 1024];
    sent.extend_from_slice(super::capture_decode_tests::PLAIN);
    let harness =
        super::capture_decode_tests::capture_whole(None, &sent, CallStatus::Complete, 65536)
            .await?;
    let calls = harness.calls(KEY)?;
    assert_eq!(calls[0].status, CallStatus::Complete);
    let bytes = harness.home()?.blocks()?.get(&Hash::parse(
        calls[0].raw_response.as_ref().ok_or("response absent")?,
    )?)?;
    assert_eq!(bytes.len(), sent.len());
    assert_eq!(bytes, sent);
    Ok(())
}

#[path = "forward_codex_tests.rs"]
mod codex;
