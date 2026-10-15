//! Announce a planned service return before draining live browser and runner requests.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use lys_identity_server::ServerError;
use serde_json::json;
use std::path::{Path, PathBuf};
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::sync::watch;

const RETURN_HEADER: &str = "x-lys-going-away-until";

#[derive(Clone)]
pub(super) struct PlannedStop(watch::Sender<Option<u64>>);

impl PlannedStop {
    pub(super) fn new() -> Self {
        Self(watch::channel(None).0)
    }

    fn announce(&self, expected_return: u64) {
        self.0.send_replace(Some(expected_return));
    }
}

pub(super) fn router(app: Router, stop: PlannedStop) -> Router {
    app.layer(axum::middleware::from_fn_with_state(stop, announce_request))
}

async fn announce_request(
    State(stop): State<PlannedStop>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let runner = (path.starts_with("/runner/dial/") || path.starts_with("/api/runner/dial/"))
        && (path.ends_with("/next") || path.ends_with("/control"));
    let browser = request.method() == axum::http::Method::GET;
    if !runner && !browser {
        return next.run(request).await;
    }
    let epoch = path.ends_with("/runner/dial/epoch");
    let mut changed = stop.0.subscribe();
    let announce = async {
        loop {
            if let Some(expected_return) = *changed.borrow_and_update() {
                return Ok::<u64, tokio::sync::watch::error::RecvError>(expected_return);
            }
            changed.changed().await?;
        }
    };
    tokio::pin!(announce);
    tokio::select! {
        biased;
        response = next.run(request) => {
            let mut response = response;
            if epoch { response.headers_mut().insert("x-lys-runner-control", axum::http::HeaderValue::from_static("v1")); }
            response
        },
        expected = &mut announce => match expected {
            Ok(expected_return) => frame(expected_return, runner),
            Err(error) => (axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"refusal":"LysStopSignalUnavailable","reason":error.to_string()}))).into_response(),
        },
    }
}

fn frame(expected_return: u64, runner: bool) -> Response {
    let mut response = Json(json!({
        "kind":"going_away", "expected_return":expected_return,
        "refusal":"LysGoingAway", "reason":"Lys is stopping for planned maintenance. It will reconnect at the announced return."
    })).into_response();
    if !runner {
        *response.status_mut() = axum::http::StatusCode::SERVICE_UNAVAILABLE;
    }
    match axum::http::HeaderValue::from_str(&expected_return.to_string()) {
        Ok(value) => {
            response.headers_mut().insert(RETURN_HEADER, value);
        }
        Err(error) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"refusal":"LysReturnInvalid","reason":error.to_string()})),
            )
                .into_response();
        }
    }
    response
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::ConfigInvalid {
        reason: reason.into(),
    }
}

pub(super) struct Signals {
    terminate: Signal,
    interrupt: Signal,
    hangup: Signal,
}

impl Signals {
    pub(super) fn open() -> Result<Self, ServerError> {
        Ok(Self {
            terminate: signal(SignalKind::terminate())
                .map_err(|error| unavailable(format!("planned_stop_signal_failed: {error}")))?,
            interrupt: signal(SignalKind::interrupt())
                .map_err(|error| unavailable(format!("planned_stop_signal_failed: {error}")))?,
            hangup: signal(SignalKind::hangup())
                .map_err(|error| unavailable(format!("planned_stop_signal_failed: {error}")))?,
        })
    }

    pub(super) async fn wait(
        mut self,
        stop: PlannedStop,
        config: PathBuf,
    ) -> Result<(), ServerError> {
        let received = tokio::select! {
            received = self.terminate.recv() => received,
            received = self.interrupt.recv() => received,
            received = self.hangup.recv() => received,
        };
        received.ok_or_else(|| {
            unavailable("planned_stop_signal_closed: the service signal stream ended")
        })?;
        let expected_return = expected_return(&config)?;
        eprintln!("lys-identity-server going away; expected return at {expected_return}");
        stop.announce(expected_return);
        Ok(())
    }
}

pub(super) fn retire_previous_plan(config: &Path) -> Result<(), ServerError> {
    let path = config.with_extension("expected-return");
    match std::fs::remove_file(&path) {
        Ok(()) => {
            eprintln!("lys-identity-server previous planned return completed");
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(unavailable(format!(
            "planned_return_cleanup_failed: {}: {error}",
            path.display()
        ))),
    }
}

fn expected_return(config: &Path) -> Result<u64, ServerError> {
    let path = config.with_extension("expected-return");
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let expected = text.trim().parse::<u64>().map_err(|error| {
                unavailable(format!(
                    "planned_return_invalid: {}: {error}",
                    path.display()
                ))
            })?;
            std::fs::remove_file(&path).map_err(|error| {
                unavailable(format!(
                    "planned_return_cleanup_failed: {}: {error}",
                    path.display()
                ))
            })?;
            Ok(expected)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| unavailable(format!("planned_return_clock_failed: {error}")))?;
            let now = u64::try_from(now.as_millis())
                .map_err(|error| unavailable(format!("planned_return_clock_failed: {error}")))?;
            eprintln!(
                "lys-identity-server planned return was not specified; announcing the normal restart estimate of ten seconds"
            );
            Ok(now.saturating_add(10_000))
        }
        Err(error) => Err(unavailable(format!(
            "planned_return_unreadable: {}: {error}",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    use tower::ServiceExt;

    #[tokio::test]
    async fn planned_stop_announces_its_return_to_every_waiting_browser_and_runner()
    -> Result<(), Box<dyn std::error::Error>> {
        let stop = PlannedStop::new();
        let app = Router::new().fallback(|| std::future::pending::<Response>());
        let app = router(app, stop.clone());
        let browser = app
            .clone()
            .oneshot(Request::get("/api/changes").body(axum::body::Body::empty())?);
        let runner = app
            .oneshot(Request::post("/runner/dial/machine/next").body(axum::body::Body::empty())?);
        tokio::pin!(browser, runner);
        let mut context = Context::from_waker(Waker::noop());
        assert!(browser.as_mut().poll(&mut context).is_pending());
        assert!(runner.as_mut().poll(&mut context).is_pending());
        stop.announce(123_000);
        for (future, status) in [(browser.as_mut(), 503), (runner.as_mut(), 200)] {
            let Poll::Ready(response) = future.poll(&mut context) else {
                return Err(
                    "the planned stop left a live stream waiting without its going-away frame"
                        .into(),
                );
            };
            let response = response?;
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[RETURN_HEADER], "123000");
            let body = axum::body::to_bytes(response.into_body(), 4096).await?;
            let body: serde_json::Value = serde_json::from_slice(&body)?;
            assert_eq!(body["expected_return"], 123_000);
        }
        Ok(())
    }
}
