//! Every outbound call is held until its answer, transport close or explicit cancel.
//! Entries and cancellation hooks are changed under a short record lock; no I/O
//! runs under it. Dropping the caller withdraws the entry and closes its transport.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use tokio::sync::watch;

use crate::error::ServerError;
use crate::runner_client::Leave;

/// One outbound call, with its start in Unix seconds.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct Wait {
    /// The opaque cancellation identity.
    pub id: String,
    /// The service being asked.
    pub service: String,
    /// Its configured transport address, without credentials.
    pub peer: String,
    /// The act being served.
    pub act: String,
    /// The account, operation, machine or session being served.
    pub subject: String,
    /// When the call began; elapsed time is presentation, never a decision.
    pub started_at: u64,
}

struct Entry {
    view: Wait,
    cancelled: watch::Sender<Option<String>>,
    close: Arc<Leave>,
    order: u64,
}

#[derive(Default)]
struct Table {
    entries: BTreeMap<String, Entry>,
    order: u64,
}

fn table() -> &'static Mutex<Table> {
    static TABLE: OnceLock<Mutex<Table>> = OnceLock::new();
    TABLE.get_or_init(Mutex::default)
}

fn changes() -> &'static watch::Sender<u64> {
    static CHANGES: OnceLock<watch::Sender<u64>> = OnceLock::new();
    CHANGES.get_or_init(|| watch::channel(0).0)
}

fn changed(held: &mut Table) {
    held.order = held.order.wrapping_add(1);
    changes().send_replace(held.order);
}

pub(crate) async fn after(after: Option<u64>) -> Result<(u64, Vec<Wait>), ServerError> {
    let mut next = changes().subscribe();
    let generation = *next.borrow_and_update();
    if after == Some(generation) {
        next.changed().await.map_err(|error| unavailable(&error))?;
    }
    let held = table().lock().map_err(|error| unavailable(&error))?;
    Ok((held.order, listed(&held)))
}

fn unavailable(error: &dyn std::fmt::Display) -> ServerError {
    ServerError::WaitsUnavailable { reason: error.to_string() }
}

/// An entry held by the call itself, never by a detached worker.
pub struct Call {
    id: String,
    cancelled: watch::Receiver<Option<String>>,
    close: Arc<Leave>,
}

impl Call {
    /// Record an outbound call before opening its socket.
    ///
    /// # Errors
    /// Refuses an unavailable registry or random identity source.
    pub fn begin(service: &str, peer: &str, act: &str, subject: &str) -> Result<Self, ServerError> {
        let id = lys_identity::OperationId::generate()?.to_string();
        let close = Arc::new(Leave::default());
        crate::wait_request::register(&close)?;
        let (cancelled, receiving) = watch::channel(None);
        let view = Wait {
            id: id.clone(), service: service.to_owned(), peer: peer.to_owned(),
            act: act.to_owned(), subject: subject.to_owned(), started_at: crate::session::now(),
        };
        let mut held = table().lock().map_err(|error| unavailable(&error))?;
        changed(&mut held);
        let order = held.order;
        held.entries.insert(id.clone(), Entry { view, cancelled, close: Arc::clone(&close), order });
        Ok(Self { id, cancelled: receiving, close })
    }

    /// Its cancellation identity.
    #[must_use]
    pub fn id(&self) -> &str { &self.id }

    /// The same leave signal that a blocking transport checks and closes on.
    #[must_use]
    pub fn closer(&self) -> Arc<Leave> { Arc::clone(&self.close) }

    /// A person's cancellation reason, also readable by synchronous transports.
    #[must_use]
    pub fn reason(&self) -> Option<String> { self.cancelled.borrow().clone() }

    /// Wait on work or a person's cancel. The losing future is dropped here.
    ///
    /// # Errors
    /// Returns the cancelling person's recorded words.
    pub async fn until<F: Future>(&mut self, work: F) -> Result<F::Output, String> {
        if let Some(reason) = self.cancelled.borrow().clone() { return Err(reason); }
        tokio::select! {
            biased;
            changed = self.cancelled.changed() => {
                changed.map_err(|error| format!("the call's cancellation record closed: {error}"))?;
                Err(self.cancelled.borrow().clone().unwrap_or_else(|| "the caller closed".to_owned()))
            }
            answer = work => Ok(answer),
        }
    }

    /// Run a call whose own errors are words, removing its entry on every exit.
    ///
    /// # Errors
    /// Returns the peer's error or a person's cancel.
    pub async fn run<T>(mut self, work: impl Future<Output = Result<T, String>>) -> Result<T, String> {
        self.until(work).await?
    }
}

impl Drop for Call {
    fn drop(&mut self) {
        if let Err(error) = self.close.leave() { tracing::error!("outbound_close_failed: {error}"); }
        match table().lock() {
            Ok(mut held) => { held.entries.remove(&self.id); changed(&mut held); }
            Err(error) => tracing::error!("wait_withdrawal_failed: {error}"),
        }
    }
}

/// Read every current wait in begin order, even when starts share a second.
///
/// # Errors
/// Refuses an unavailable registry.
pub fn snapshot() -> Result<Vec<Wait>, ServerError> {
    let held = table().lock().map_err(|error| unavailable(&error))?;
    Ok(listed(&held))
}

fn listed(held: &Table) -> Vec<Wait> {
    let mut entries: Vec<_> = held.entries.values().collect();
    entries.sort_by_key(|entry| entry.order);
    entries.into_iter().map(|entry| entry.view.clone()).collect()
}

/// Cancel by a person's explicit word; the call answers under its own service name.
///
/// # Errors
/// Refuses an unknown wait, an unavailable registry or a transport close failure.
pub fn cancel(id: &str, by: &str) -> Result<(), ServerError> {
    let close = {
        let held = table().lock().map_err(|error| unavailable(&error))?;
        let entry = held.entries.get(id).ok_or(ServerError::WaitUnknown)?;
        entry.cancelled.send_replace(Some(format!("cancelled by {by}")));
        Arc::clone(&entry.close)
    };
    close.leave().map_err(|error| unavailable(&error))
}
