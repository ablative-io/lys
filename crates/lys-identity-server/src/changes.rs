//! A bounded, coalescing change signal. Subscribers re-read through the
//! ordinary authorised routes; no records or identities travel in the signal.
use crate::error::ServerError;
use crate::routes::{Shared, signed_in};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::Response;
use axum::{Json, Router, routing::get};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::sync::{Semaphore, watch};

pub(crate) struct Changes {
    generation: watch::Sender<OperationId>,
    waiting: Semaphore,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::RuntimeUnavailable {
        reason: reason.into(),
    }
}

impl Changes {
    pub(crate) fn new() -> Result<Self, ServerError> {
        let (generation, _) = watch::channel(OperationId::generate()?);
        Ok(Self {
            generation,
            waiting: Semaphore::new(128),
        })
    }

    fn publish(&self, next: OperationId) {
        self.generation.send_replace(next);
    }

    async fn checked_after(
        &self,
        after: Option<OperationId>,
        authorised: impl Fn() -> Result<(), ServerError>,
    ) -> Result<OperationId, ServerError> {
        let generation = self.after(after).await?;
        authorised()?;
        Ok(generation)
    }

    async fn after(&self, after: Option<OperationId>) -> Result<OperationId, ServerError> {
        let mut current = self.generation.subscribe();
        let generation = *current.borrow_and_update();
        if after == Some(generation) {
            let permit = self.waiting.try_acquire().map_err(|error| {
                unavailable(format!("change subscriptions are at capacity: {error}"))
            })?;
            current
                .changed()
                .await
                .map_err(|error| unavailable(format!("change subscriptions closed: {error}")))?;
            drop(permit);
        }
        let generation = *current.borrow_and_update();
        Ok(generation)
    }
}

#[derive(Deserialize)]
struct After {
    after: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct Changed {
    generation: String,
}

pub(crate) fn routes() -> Router<Shared> {
    Router::new().route("/changes", get(changed))
}

async fn changed(
    State(state): State<Shared>,
    headers: HeaderMap,
    query: Result<Query<After>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Changed>, ServerError> {
    signed_in(&state, &headers)?;
    let Query(query) = query.map_err(|error| ServerError::RequestMalformed {
        reason: error.to_string(),
    })?;
    let after = query
        .after
        .as_deref()
        .map(OperationId::from_str)
        .transpose()
        .map_err(|error| ServerError::RequestMalformed {
            reason: error.to_string(),
        })?;
    let generation = state
        .changes
        .checked_after(after, || signed_in(&state, &headers).map(|_| ()))
        .await?;
    Ok(Json(Changed {
        generation: generation.to_string(),
    }))
}

fn changes_state(method: &Method, path: &str) -> bool {
    if !matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) {
        return false;
    }
    if [
        "/read",
        "/read-bytes",
        "/input",
        "/input-bytes",
        "/resize",
        "/check",
        "/why",
        "/who",
        "/reach",
        "/cannot-give",
        "/ask",
    ]
    .iter()
    .any(|suffix| path.ends_with(suffix))
    {
        return false;
    }
    !matches!(
        path,
        "/grants/check/batch" | "/secrets/recipients" | "/secrets/scope"
    )
}

pub(crate) async fn observe(
    State(state): State<Shared>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, ServerError> {
    let generation = if changes_state(request.method(), request.uri().path()) {
        Some(OperationId::generate()?)
    } else {
        None
    };
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(generation) = generation
    {
        state.changes.publish(generation);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    #[tokio::test]
    async fn a_wait_rechecks_authority_after_the_change_signal()
    -> Result<(), Box<dyn std::error::Error>> {
        let changes = Changes::new()?;
        let first = changes.after(None).await?;
        let admitted = Cell::new(true);
        let checks = Cell::new(0);
        let mut waiting = Box::pin(changes.checked_after(Some(first), || {
            checks.set(checks.get() + 1);
            if admitted.get() {
                Ok(())
            } else {
                Err(ServerError::NotSignedIn)
            }
        }));
        assert!(
            waiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        assert_eq!(checks.get(), 0);
        admitted.set(false);
        changes.publish(OperationId::generate()?);
        assert!(matches!(waiting.await, Err(ServerError::NotSignedIn)));
        assert_eq!(checks.get(), 1);
        assert_eq!(changes.waiting.available_permits(), 128);
        Ok(())
    }

    #[tokio::test]
    async fn changes_wait_for_a_signal_coalesce_and_bound_waiters()
    -> Result<(), Box<dyn std::error::Error>> {
        let changes = Changes::new()?;
        let first = changes.after(None).await?;
        let mut waits = Vec::new();
        for _ in 0..128 {
            let mut wait = Box::pin(changes.after(Some(first)));
            assert!(
                wait.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            waits.push(wait);
        }
        let mut full = Box::pin(changes.after(Some(first)));
        assert!(matches!(
            full.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Err(ServerError::RuntimeUnavailable { .. }))
        ));
        drop(full);
        waits.pop();
        let mut released = Box::pin(changes.after(Some(first)));
        assert!(
            released
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        changes.publish(OperationId::generate()?);
        let last = OperationId::generate()?;
        changes.publish(last);
        assert_eq!(released.await?, last);
        for wait in waits {
            assert_eq!(wait.await?, last);
        }
        assert_eq!(changes.waiting.available_permits(), 128);
        assert!(!changes_state(
            &Method::POST,
            "/runtime/sessions/one/read-bytes"
        ));
        assert!(!changes_state(
            &Method::POST,
            "/runtime/sessions/one/input-bytes"
        ));
        assert!(!changes_state(&Method::GET, "/changes"));
        assert!(changes_state(&Method::POST, "/agents/one/runtime/reports"));
        Ok(())
    }
}
