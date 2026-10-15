//! A real socket stays held until a signed long poll is released.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, mpsc, watch};

use crate::protocol::{read_greeting, read_reply, sign_request};
use crate::{Act, Answer, Client, Options, Runner};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplyMode {
    Accepted,
    Refused,
    Expired,
    Duplicate,
    Unknown,
    Unavailable,
}

struct Remote {
    key: Arc<lys_core::Ed25519Identity>,
    polls: AtomicUsize,
    release: Semaphore,
    arrived: mpsc::UnboundedSender<()>,
    replies: mpsc::UnboundedSender<Answer>,
    posted: Mutex<BTreeMap<String, usize>>,
    finished: watch::Sender<bool>,
    mode: ReplyMode,
    settled: tokio::sync::Barrier,
}

async fn next(State(remote): State<Arc<Remote>>, body: Bytes) -> Response {
    let number = remote.polls.fetch_add(1, Ordering::AcqRel);
    let requests = if remote.mode == ReplyMode::Refused {
        2
    } else {
        4
    };
    if number >= requests {
        let mut finished = remote.finished.subscribe();
        if finished.wait_for(|done| *done).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        return (StatusCode::FORBIDDEN, "{\"refusal\":\"fixture_finished\"}").into_response();
    }
    let result = async {
        let greeting = read_greeting(std::str::from_utf8(&body)?)?;
        remote.arrived.send(())?;
        remote.release.acquire().await?.forget();
        let request = sign_request(&remote.key, &greeting, &Act::Liveness { session: None })?;
        Ok::<_, Box<dyn Error + Send + Sync>>((
            [(super::super::TICKET_HEADER, number.to_string())],
            request,
        ))
    }
    .await;
    match result {
        Ok(answer) => answer.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn reply(
    State(remote): State<Arc<Remote>>,
    Path(ticket): Path<String>,
    body: Bytes,
) -> Response {
    let initial = matches!(ticket.as_str(), "0" | "1");
    let result = (|| -> Result<(), Box<dyn Error + Send + Sync>> {
        *remote
            .posted
            .lock()
            .map_err(|error| error.to_string())?
            .entry(ticket)
            .or_default() += 1;
        remote
            .replies
            .send(read_reply(std::str::from_utf8(&body)?)?)?;
        Ok(())
    })();
    if let Err(error) = result {
        return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
    }
    if !initial {
        return "{}".into_response();
    }
    match remote.mode {
        ReplyMode::Expired => (StatusCode::GONE, "{\"refusal\":\"runner_reply_expired\",\"reason\":\"the caller left; do not send this reply again\"}").into_response(),
        ReplyMode::Duplicate => (StatusCode::CONFLICT, "{\"refusal\":\"runner_reply_duplicate\",\"reason\":\"the reply was already delivered; do not send this reply again\"}").into_response(),
        ReplyMode::Unknown => (StatusCode::NOT_FOUND, "{\"refusal\":\"runner_reply_unknown\",\"reason\":\"the ticket is unknown; do not send this reply again\"}").into_response(),
        ReplyMode::Refused => (
            StatusCode::FORBIDDEN,
            "{\"refusal\":\"fixture_reply_refused\"}",
        ).into_response(),
        ReplyMode::Unavailable => {
            remote.settled.wait().await;
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        ReplyMode::Accepted => "{}".into_response(),
    }
}

async fn answer(
    answers: &mut mpsc::UnboundedReceiver<Answer>,
    bridge: &mut tokio::task::JoinHandle<Result<(), crate::RunnerError>>,
) -> Result<Answer, Box<dyn Error>> {
    tokio::select! {
        biased;
        reply = answers.recv() => reply.ok_or_else(|| "the bridge did not post its answer".into()),
        result = bridge => match result? {
            Err(error) => Err(error.into()),
            Ok(()) => Err("the bridge ended before its answer".into()),
        },
    }
}

async fn bridge(mode: ReplyMode) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(lys_core::Ed25519Identity::ephemeral());
    let socket = dir.path().join("runner.sock");
    let serving = Runner::open(&Options {
        socket: socket.clone(),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 4096,
    })?
    .spawn();
    let (arrived, mut arrivals) = mpsc::unbounded_channel();
    let (replies, mut answers) = mpsc::unbounded_channel();
    let (finished, _) = watch::channel(false);
    let remote = Arc::new(Remote {
        key: Arc::clone(&key),
        polls: AtomicUsize::new(0),
        release: Semaphore::new(0),
        arrived,
        replies,
        posted: Mutex::default(),
        finished,
        mode,
        settled: tokio::sync::Barrier::new(2),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let server = format!("http://{}", listener.local_addr()?);
    let routes = axum::Router::new()
        .route(super::super::EPOCH_ROUTE, get(|| async { "fixture-epoch" }))
        .route("/runner/dial/machine/next", post(next))
        .route("/runner/dial/machine/replies/{ticket}", post(reply))
        .with_state(Arc::clone(&remote));
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let http = tokio::spawn(async move {
        axum::serve(listener, routes)
            .with_graceful_shutdown(async {
                drop(stopped.await);
            })
            .await
    });
    let dial = super::Dial {
        server,
        machine: "machine".to_owned(),
        socket: socket.clone(),
        key: Arc::new(lys_core::Ed25519Identity::ephemeral()),
        authority: None,
    };
    let mut bridge = tokio::task::spawn_blocking(move || dial.bridge());
    arrivals
        .recv()
        .await
        .ok_or("first long poll did not arrive")?;
    arrivals
        .recv()
        .await
        .ok_or("second long poll did not arrive")?;
    // The native runner serves another caller while both bridge greetings remain outstanding.
    let native = tokio::task::spawn_blocking(move || {
        Client::new(socket, key).ask(&Act::Liveness { session: None })
    })
    .await??;
    assert!(matches!(native, Answer::Liveness { .. }));
    // Later tickets are accepted so a finished reply cannot disable unrelated requests.
    for _ in 0..2 {
        remote.release.add_permits(2);
        let first = answer(&mut answers, &mut bridge).await?;
        assert!(matches!(first, Answer::Liveness { .. }));
        if mode == ReplyMode::Refused {
            break;
        }
        let second = answer(&mut answers, &mut bridge).await?;
        assert!(matches!(second, Answer::Liveness { .. }));
    }
    if mode != ReplyMode::Refused {
        remote.finished.send(true)?;
    }
    let error = bridge
        .await?
        .err()
        .ok_or("a refused bridge stayed running")?;
    remote.finished.send_replace(true);
    stop.send(())
        .map_err(|()| "HTTP fixture ended before shutdown")?;
    http.await??;
    serving.stop()?;
    let posted = remote.posted.lock().map_err(|error| error.to_string())?;
    assert!(
        posted.values().all(|count| *count == 1),
        "a reply was posted twice"
    );
    if mode == ReplyMode::Refused {
        assert!(
            error.to_string().contains("fixture_reply_refused"),
            "{error}"
        );
    } else {
        assert_eq!(posted.len(), 4);
        assert!(error.to_string().contains("fixture_finished"), "{error}");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_local_connections_survive_the_server_long_polls() -> Result<(), Box<dyn Error>> {
    bridge(ReplyMode::Accepted).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_reply_ends_the_bridge_without_posting_it_twice() -> Result<(), Box<dyn Error>> {
    bridge(ReplyMode::Refused).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_replies_are_retired_once_while_other_requests_remain_usable()
-> Result<(), Box<dyn Error>> {
    bridge(ReplyMode::Expired).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_replies_are_retired_once_while_other_requests_remain_usable()
-> Result<(), Box<dyn Error>> {
    bridge(ReplyMode::Duplicate).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_replies_are_retired_once_while_other_requests_remain_usable()
-> Result<(), Box<dyn Error>> {
    bridge(ReplyMode::Unknown).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unavailable_reply_is_not_replayed_and_does_not_end_the_bridge()
-> Result<(), Box<dyn Error>> {
    bridge(ReplyMode::Unavailable).await
}
