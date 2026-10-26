//! Request-owned work shares its caller's leave signal, including blocking
//! transport workers. The request's drop closes every registered transport.

use crate::runner_client::Leave;
use std::cell::RefCell;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

#[derive(Default)]
struct Work {
    closed: std::sync::atomic::AtomicBool,
    changed: tokio::sync::Notify,
    calls: Mutex<Vec<Weak<Leave>>>,
}

tokio::task_local! { static CURRENT: Arc<Work>; }
thread_local! { static BLOCKING: RefCell<Option<Arc<Work>>> = const { RefCell::new(None) }; }

fn current() -> Option<Arc<Work>> {
    CURRENT
        .try_with(Arc::clone)
        .ok()
        .or_else(|| BLOCKING.with(|held| held.borrow().clone()))
}

pub(crate) fn register(close: &Arc<Leave>) -> Result<(), crate::error::ServerError> {
    let Some(work) = current() else {
        return Ok(());
    };
    let mut calls =
        work.calls
            .lock()
            .map_err(|error| crate::error::ServerError::WaitsUnavailable {
                reason: error.to_string(),
            })?;
    calls.retain(|call| call.strong_count() > 0);
    calls.push(Arc::downgrade(close));
    let closed = work.closed.load(std::sync::atomic::Ordering::Acquire);
    drop(calls);
    if closed {
        close.leave()?;
    }
    Ok(())
}

pub(crate) fn left() -> bool {
    current().is_some_and(|work| work.closed.load(std::sync::atomic::Ordering::Acquire))
}

struct Request(Arc<Work>);
impl Drop for Request {
    fn drop(&mut self) {
        self.0
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
        self.0.changed.notify_waiters();
        let calls = match self.0.calls.lock() {
            Ok(mut calls) => std::mem::take(&mut *calls),
            Err(error) => {
                tracing::error!("request_work_close_failed: {error}");
                return;
            }
        };
        for close in calls.into_iter().filter_map(|call| call.upgrade()) {
            if let Err(error) = close.leave() {
                tracing::error!("request_transport_close_failed: {error}");
            }
        }
    }
}

pub(crate) async fn scope<T>(work: impl Future<Output = T>) -> T {
    if current().is_some() {
        return work.await;
    }
    let work_state = Arc::new(Work::default());
    let request = Request(Arc::clone(&work_state));
    let answer = CURRENT.scope(work_state, work).await;
    drop(request);
    answer
}

struct BlockingContext(Option<Arc<Work>>);
impl Drop for BlockingContext {
    fn drop(&mut self) {
        BLOCKING.with(|held| *held.borrow_mut() = self.0.take());
    }
}

struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) async fn blocking<F, T>(work: F) -> Result<T, tokio::task::JoinError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let context = current();
    let task = tokio::task::spawn_blocking(move || {
        let previous = BLOCKING.with(|held| held.replace(context));
        let restore = BlockingContext(previous);
        let answer = work();
        drop(restore);
        answer
    });
    let abort = AbortOnDrop(task.abort_handle());
    let answer = task.await;
    drop(abort);
    answer
}

pub(crate) async fn observe(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let context = Arc::new(Work::default());
    let caller = Request(Arc::clone(&context));
    let runtime = tokio::runtime::Handle::current();
    let running = tokio::task::spawn_blocking(move || {
        let work = Arc::clone(&context);
        runtime.block_on(CURRENT.scope(context, async move {
            let closed = work.changed.notified();
            tokio::pin!(closed);
            closed.as_mut().enable();
            if work.closed.load(std::sync::atomic::Ordering::Acquire) {
                return axum::response::Response::new(axum::body::Body::empty());
            }
            tokio::select! {
                response = next.run(request) => response,
                () = closed => axum::response::Response::new(axum::body::Body::empty()),
            }
        }))
    });
    let queued = AbortOnDrop(running.abort_handle());
    let response = match running.await {
        Ok(response) => response,
        Err(error) => crate::error::ServerError::WaitsUnavailable {
            reason: format!("the request worker ended: {error}"),
        }
        .into_response(),
    };
    drop(queued);
    drop(caller);
    response
}
