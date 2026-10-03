//! Forwarding: a request goes to the provider with its method, path, query,
//! headers and body as they came, and the response comes back with its
//! status, headers and body as they came, frame by frame as the upstream
//! sends them.
//!
//! Invariants:
//! - Nothing is buffered on the way: each body frame is handed on as it
//!   arrives, so a streamed response reaches the client event by event.
//!   [`Tee`] lets an [`Observer`] see each frame after it is read and
//!   before it is passed on; it never changes, holds or reorders one.
//! - One header is set rather than copied: `host`, which names the
//!   connection's target, becomes the upstream's authority, since the
//!   provider serves its own name and the client addressed this process.
//!   Every other header passes as it came. No header value is written or
//!   logged here; the one read is a response's `content-type`, to know an
//!   event stream.
//! - The transport is one Hyper client whose cancelled-request setting is
//!   set explicitly to off, so a request whose connection closes under it is
//!   answered with the failure and never sent a second time. The
//!   pass-through example (R7) and `lys proxy serve` (R10) share this transport.
//! - No clock bounds a call: the pool keeps no idle limit and nothing here
//!   waits on a timer.

use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll};

use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes, Frame, Incoming, SizeHint};
use hyper::header::{CONTENT_TYPE, HOST, HeaderValue, InvalidHeaderValue};
use hyper::http::uri::{InvalidUri, PathAndQuery};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use crate::proxy::capture::Call;
use crate::proxy::error::ProxyError;
use crate::proxy::journal::{CallReport, Journal, OpenCall, Sink, recover};
use crate::record::call::Api;
use crate::record::{Home, fresh_id, now};

/// The body type both directions are carried in.
pub type ProxyBody = UnsyncBoxBody<Bytes, hyper::Error>;

#[cfg(test)]
#[path = "admission_tests.rs"]
mod admission_tests;

/// An upstream base: an absolute `http` or `https` URL, with or without a
/// path of its own, onto which a request's path and query are joined.
#[derive(Clone, Debug)]
pub struct Base {
    text: String,
    host: HeaderValue,
}

impl Base {
    /// Check and keep a base as the operator gave it.
    pub fn parse(base: &str) -> Result<Self, ProxyError> {
        let bad = |reason: String| ProxyError::BadUpstream {
            base: base.to_owned(),
            reason,
        };
        let uri: Uri = base.parse().map_err(|e: InvalidUri| bad(e.to_string()))?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) {
            return Err(bad(String::from("the scheme is not http or https")));
        }
        let authority = uri
            .authority()
            .ok_or_else(|| bad(String::from("there is no host")))?;
        let host = HeaderValue::from_str(authority.as_str())
            .map_err(|e: InvalidHeaderValue| bad(e.to_string()))?;
        Ok(Self {
            text: base.trim_end_matches('/').to_owned(),
            host,
        })
    }

    /// The base as given, without a trailing `/`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The full upstream URI for a request path and query.
    pub fn target(&self, path_and_query: &str) -> Result<Uri, ProxyError> {
        format!("{}{path_and_query}", self.text)
            .parse()
            .map_err(|e: InvalidUri| ProxyError::BadTarget {
                base: self.text.clone(),
                reason: e.to_string(),
            })
    }
}

/// The one transport to the providers.
#[derive(Clone, Debug)]
pub struct Upstream {
    client: Client<HttpsConnector<HttpConnector>, ProxyBody>,
}

impl Upstream {
    /// A transport over TLS with the Mozilla roots, or plain HTTP for an
    /// `http` base (a loopback fake), with no request ever sent twice.
    pub fn new() -> Result<Self, ProxyError> {
        let connector = HttpsConnectorBuilder::new()
            .with_provider_and_webpki_roots(rustls::crypto::ring::default_provider())
            .map_err(|source| ProxyError::Tls { source })?
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new())
            .retry_canceled_requests(false)
            .pool_idle_timeout(None::<std::time::Duration>)
            .build(connector);
        Ok(Self { client })
    }

    /// Send a request to `base`, its path and query joined onto the base,
    /// its headers as they came but `host`, and its body as it streams.
    pub async fn send(
        &self,
        base: &Base,
        path_and_query: &str,
        request: Request<ProxyBody>,
    ) -> Result<Response<Incoming>, ProxyError> {
        let (mut parts, body) = request.into_parts();
        parts.uri = base.target(path_and_query)?;
        parts.headers.insert(HOST, base.host.clone());
        let request = Request::from_parts(parts, body);
        Box::pin(self.client.request(request))
            .await
            .map_err(|source| ProxyError::Upstream {
                source: Box::new(source),
            })
    }
}

/// How a body ended, as a [`Tee`] saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// The body ended as its sender meant it to.
    Complete,
    /// Reading the body failed.
    Failed,
    /// The body was let go before it ended: its reader went away.
    Dropped,
}

/// What watches a body pass.
pub trait Observer: Send + Unpin + 'static {
    /// A data frame's bytes, after they were read and before they are
    /// passed on.
    fn data(&mut self, bytes: &Bytes);
    /// The body's end; called once.
    fn ended(&mut self, end: End);
}

/// A body passed on frame by frame, unchanged, with an [`Observer`] shown
/// each data frame and the end.
#[derive(Debug)]
pub struct Tee<O: Observer> {
    inner: Incoming,
    observer: Option<O>,
}

impl<O: Observer> Tee<O> {
    /// Watch `inner` with `observer`.
    pub fn new(inner: Incoming, observer: O) -> Self {
        Self {
            inner,
            observer: Some(observer),
        }
    }

    fn end(&mut self, end: End) {
        if let Some(mut observer) = self.observer.take() {
            observer.ended(end);
        }
    }
}

impl<O: Observer> Body for Tee<O> {
    type Data = Bytes;
    type Error = hyper::Error;

    /// Pass the frame after enqueueing the observer event. Capture runs on its worker.
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, hyper::Error>>> {
        let this = self.get_mut();
        let polled = Pin::new(&mut this.inner).poll_frame(cx);
        match &polled {
            Poll::Ready(Some(Ok(frame))) => {
                if let (Some(bytes), Some(observer)) = (frame.data_ref(), this.observer.as_mut()) {
                    observer.data(bytes);
                }
                // Hyper stops polling a body that says it has ended, so the
                // end is seen here rather than waited for.
                if this.inner.is_end_stream() {
                    this.end(End::Complete);
                }
            }
            Poll::Ready(Some(Err(_))) => this.end(End::Failed),
            Poll::Ready(None) => this.end(End::Complete),
            Poll::Pending => {}
        }
        polled
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl<O: Observer> Drop for Tee<O> {
    fn drop(&mut self) {
        // A body that had ended (an empty one, never polled) ended whole.
        let end = if self.inner.is_end_stream() {
            End::Complete
        } else {
            End::Dropped
        };
        self.end(end);
    }
}

/// A plain-text answer from this process itself, naming why: never a
/// header value or a body byte of the call.
#[must_use]
pub fn refusal(status: StatusCode, reason: &str) -> Response<ProxyBody> {
    let body = Full::new(Bytes::from(format!("{reason}\n")))
        .map_err(|never: Infallible| match never {})
        .boxed_unsync();
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
}

/// Accepts and serves each connection without a proxy admission limit.
/// The connection tasks are reaped when they finish.
pub async fn serve<H, F>(listener: TcpListener, handle: H) -> Result<(), ProxyError>
where
    H: Fn(Request<Incoming>) -> F + Send + Sync + 'static,
    F: Future<Output = Response<ProxyBody>> + Send + 'static,
{
    serve_until(listener, handle, std::future::pending()).await
}

pub(super) async fn serve_until<H, F>(
    listener: TcpListener,
    handle: H,
    shutdown: impl Future<Output = ()>,
) -> Result<(), ProxyError>
where
    H: Fn(Request<Incoming>) -> F + Send + Sync + 'static,
    F: Future<Output = Response<ProxyBody>> + Send + 'static,
{
    tokio::pin!(shutdown);
    let handle = Arc::new(handle);
    let mut connections = JoinSet::new();
    loop {
        let (stream, _) = tokio::select! {
            biased;
            () = &mut shutdown => break,
            finished = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(error)) = finished {
                    eprintln!("lys-proxy: proxy_connection_task_failed: {error}");
                }
                continue;
            }
            accepted = listener.accept() => {
                accepted.map_err(|source| ProxyError::Accept { source })?
            }
        };
        let handle = Arc::clone(&handle);
        connections.spawn(async move {
            let service = service_fn(move |request| {
                let answer = handle(request);
                async move { Ok::<_, Infallible>(answer.await) }
            });
            let connection = http1::Builder::new().serve_connection(TokioIo::new(stream), service);
            if let Err(error) = Box::pin(connection).await {
                eprintln!("lys-proxy: proxy_connection_failed: {error}");
            }
        });
    }
    connections.abort_all();
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result
            && !error.is_cancelled()
        {
            eprintln!("lys-proxy: proxy_connection_task_failed: {error}");
        }
    }
    Ok(())
}

#[path = "pass_through.rs"]
mod passing;
pub use passing::pass_through;

/// What `lys proxy serve` gives the proxy: the home its calls are recorded in, the
/// directory its journal and capture live in, and the two provider bases.
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    /// The home the calls are recorded in.
    pub home: PathBuf,
    /// Where `journal/` and `capture/` live.
    pub state: PathBuf,
    /// The base a `/anthropic` path is forwarded to.
    pub anthropic: Base,
    /// The base an `/openai` path is forwarded to.
    pub openai: Base,
}

/// The little proxy: forwards by path prefix and records each model call
/// under the session its key names.
#[derive(Debug)]
pub struct Proxy {
    upstream: Upstream,
    anthropic: Base,
    openai: Base,
    capture: PathBuf,
    journal: Journal,
    sink: Sink,
}

/// A proxy started: itself, the calls a previous run left open (each now
/// recorded `lost`, once), and the reports of calls as the sink records
/// them.
#[derive(Debug)]
pub struct Started {
    /// The proxy.
    pub proxy: Arc<Proxy>,
    /// The calls a previous run left open.
    pub lost: Vec<CallReport>,
    /// One report per call recorded, or held.
    pub reports: mpsc::Receiver<CallReport>,
}

impl Proxy {
    /// Open the home, the journal and the capture directory, record every
    /// call a previous run left open as `lost`, and start the sink.
    pub fn start(config: ProxyConfig) -> Result<Started, ProxyError> {
        let home = Home::open(&config.home)?;
        let journal = Journal::open(config.state.join("journal"))?;
        let capture = config.state.join("capture");
        std::fs::create_dir_all(&capture)
            .map_err(|e| ProxyError::io("creating the capture directory", &capture, e))?;
        let lost = recover(&home, &journal, &capture)?;
        let upstream = Upstream::new()?;
        let (tx, reports) = mpsc::channel();
        let sink = Sink::start(home, journal.clone(), tx);
        let proxy = Self {
            upstream,
            anthropic: config.anthropic,
            openai: config.openai,
            capture,
            journal,
            sink,
        };
        Ok(Started {
            proxy: Arc::new(proxy),
            lost,
            reports,
        })
    }

    /// The sink, to ask it to look again at calls it holds.
    #[must_use]
    pub const fn sink(&self) -> &Sink {
        &self.sink
    }

    /// Serve on `listener` for as long as it accepts.
    pub async fn serve(self: Arc<Self>, listener: TcpListener) -> Result<(), ProxyError> {
        serve(listener, move |request| {
            let proxy = Arc::clone(&self);
            async move { proxy.handle(request).await }
        })
        .await
    }

    fn route(&self, path_and_query: &str) -> Option<(&'static str, &Base, String)> {
        let providers = [("anthropic", &self.anthropic), ("openai", &self.openai)];
        providers.into_iter().find_map(|(provider, base)| {
            let rest = path_and_query.strip_prefix('/')?.strip_prefix(provider)?;
            match rest.chars().next() {
                None => Some((provider, base, String::from("/"))),
                Some('/') => Some((provider, base, rest.to_owned())),
                Some('?') => Some((provider, base, format!("/{rest}"))),
                Some(_) => None,
            }
        })
    }

    /// Answer one request: forward it, and for a model call journal it
    /// first, capture it as it passes and hand it to the sink at its end.
    pub async fn handle(&self, request: Request<Incoming>) -> Response<ProxyBody> {
        let path_and_query = request
            .uri()
            .path_and_query()
            .map_or("/", PathAndQuery::as_str)
            .to_owned();
        let Some((provider, base, rest)) = self.route(&path_and_query) else {
            return refusal(
                StatusCode::NOT_FOUND,
                "lys-proxy: no provider for this path; a path begins /anthropic or /openai",
            );
        };
        let Some(api) = api_of(request.method(), &rest) else {
            // Not a model call: forwarded as it came, and not recorded.
            let request = request.map(BodyExt::boxed_unsync);
            return match self.upstream.send(base, &rest, request).await {
                Ok(response) => response.map(BodyExt::boxed_unsync),
                Err(error) => refusal(StatusCode::BAD_GATEWAY, &format!("lys-proxy: {error}")),
            };
        };
        let mut open = OpenCall {
            call_id: fresh_id(),
            provider: provider.to_owned(),
            api,
            started_at: now(),
            session: None,
            admission_ns: None,
            completed: None,
        };
        // Admission gate: the request waits for the journal file and directory sync.
        let admission = std::time::Instant::now();
        if let Err(error) = self.journal.write(&open) {
            return refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("lys-proxy refused the call: {error}"),
            );
        }
        open.admission_ns = Some(u64::try_from(admission.elapsed().as_nanos()).unwrap_or(u64::MAX));
        let call = Call::admit(open, &self.capture, self.journal.clone(), self.sink.clone());
        let request = request.map(|body| Tee::new(body, call.request_side()).boxed_unsync());
        match self.upstream.send(base, &rest, request).await {
            Ok(response) => {
                let side = call.response_side(
                    is_event_stream(response.headers()),
                    response.headers().get_all(hyper::header::CONTENT_ENCODING),
                );
                response.map(|body| Tee::new(body, side).boxed_unsync())
            }
            Err(error) => {
                call.finish(End::Failed);
                refusal(StatusCode::BAD_GATEWAY, &format!("lys-proxy: {error}"))
            }
        }
    }
}

/// The api a request follows, by method and path: a `POST` to the
/// Messages, Chat Completions or Responses endpoint. Anything else is not a
/// model call.
#[must_use]
pub fn api_of(method: &Method, path_and_query: &str) -> Option<Api> {
    if method != Method::POST {
        return None;
    }
    let path = path_and_query
        .split_once('?')
        .map_or(path_and_query, |(path, _)| path);
    match path {
        "/v1/messages" => Some(Api::Messages),
        "/v1/chat/completions" => Some(Api::ChatCompletions),
        "/v1/responses" => Some(Api::Responses),
        _ => None,
    }
}

/// Whether a response is an event stream, by its content type.
fn is_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim_start().starts_with("text/event-stream"))
}
