//! How a `SpiceDB` call ends when the request that asked for it leaves, and
//! the one way a request runs a grants section (ADR-127).
//!
//! A [`Cancel`] is a flag and a wake pipe. Cancelling sets the flag and, on
//! the first cancel only, writes one byte to the pipe's non-blocking write
//! end, so no cancel blocks or fails, however many come from however many
//! threads. A [`scope`] sets the current thread's Cancel for the length of a
//! closure and puts back the one before it when the closure returns or
//! unwinds. `post_json` waits in poll(2) on its socket and on the current
//! scope's wake pipe, given no time bound, so the pipe becoming readable ends the
//! call and nothing ends it on a clock.
//!
//! Every grants section a request runs goes through [`judged`]: on a blocking
//! thread, off the async workers, inside that request's own scope. The
//! request's future holds a guard whose drop cancels, so a request that
//! leaves ends the connect, write or read it is waiting in and the grants
//! lock is let go. `with_grants` reads the flag as soon as it holds the
//! grants lock, so a section that takes the lock after its request left
//! calls no `SpiceDB` at all.

use std::cell::RefCell;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use lys_identity::grants::GrantError;
use rustix::io::{Errno, FdFlags};

use crate::error::ServerError;
use crate::grants::{Judged, with_grants};
use crate::routes::AppState;

/// What a `SpiceDB` call answers when its request left before the answer.
pub(crate) const LEFT: &str = "the request that asked left before SpiceDB answered";

thread_local! {
    /// The Cancel of the request this thread is running a section for.
    static CURRENT: RefCell<Option<Arc<Cancel>>> = const { RefCell::new(None) };
}

fn unmade(error: Errno) -> String {
    format!("a request's wake pipe could not be made: {error}")
}

/// A request's cancel: a flag, and a pipe a wait polls to see it set.
#[derive(Debug)]
pub(crate) struct Cancel {
    left: AtomicBool,
    wake_read: OwnedFd,
    wake_write: OwnedFd,
}

impl Cancel {
    /// A Cancel not yet cancelled, its pipe's write end non-blocking.
    pub(crate) fn new() -> Result<Arc<Self>, String> {
        let (wake_read, wake_write) = rustix::pipe::pipe().map_err(unmade)?;
        rustix::io::fcntl_setfd(&wake_read, FdFlags::CLOEXEC).map_err(unmade)?;
        rustix::io::fcntl_setfd(&wake_write, FdFlags::CLOEXEC).map_err(unmade)?;
        rustix::io::ioctl_fionbio(&wake_write, true).map_err(unmade)?;
        Ok(Arc::new(Self {
            left: AtomicBool::new(false),
            wake_read,
            wake_write,
        }))
    }

    /// Say the request left. Only the first cancel writes to the pipe, and
    /// that write is one byte into an empty non-blocking pipe, so it never
    /// blocks; a cancel returns nothing, so it never fails its caller.
    pub(crate) fn cancel(&self) {
        if self.left.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Err(error) = rustix::io::write(&self.wake_write, &[1]) {
            tracing::warn!("a request that left could not wake its SpiceDB wait: {error}");
        }
    }

    /// Whether the request left.
    pub(crate) fn cancelled(&self) -> bool {
        self.left.load(Ordering::SeqCst)
    }

    /// The pipe end that becomes readable when the request leaves.
    pub(crate) fn wake(&self) -> BorrowedFd<'_> {
        self.wake_read.as_fd()
    }
}

/// Cancels its Cancel when it is dropped, as a request's future is when the
/// request leaves.
#[derive(Debug)]
pub(crate) struct Leaving(pub(crate) Arc<Cancel>);

impl Drop for Leaving {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// The thread's Cancel before a scope, put back when the scope ends.
struct Restore(Option<Arc<Cancel>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let earlier = self.0.take();
        CURRENT.with(|current| *current.borrow_mut() = earlier);
    }
}

/// Run `act` with `cancel` as this thread's Cancel, putting back the one
/// before it when `act` returns and when it unwinds.
pub(crate) fn scope<T>(cancel: &Arc<Cancel>, act: impl FnOnce() -> T) -> T {
    let earlier = CURRENT.with(|current| current.replace(Some(Arc::clone(cancel))));
    let restore = Restore(earlier);
    let answer = act();
    drop(restore);
    answer
}

/// This thread's Cancel, when it is running inside a scope.
pub(crate) fn current() -> Option<Arc<Cancel>> {
    CURRENT.with(|current| current.borrow().as_ref().map(Arc::clone))
}

/// Refuse when this thread's request has left, so a section that holds the
/// grants lock after its request left calls no `SpiceDB`.
pub(crate) fn still_asked() -> Result<(), GrantError> {
    if current().as_deref().is_some_and(Cancel::cancelled) {
        return Err(GrantError::PermissionEngineUnavailable {
            reason: LEFT.to_owned(),
        });
    }
    Ok(())
}

/// Run `act` as a grants section: under `with_grants`, on a blocking
/// thread, inside a Cancel's scope that is cancelled when this future is
/// dropped, so a request that leaves ends its `SpiceDB` wait.
pub(crate) async fn judged<T, A>(state: Arc<AppState>, act: A) -> Result<T, ServerError>
where
    T: Send + 'static,
    A: FnOnce(Judged<'_>) -> Result<T, ServerError> + Send + 'static,
{
    let cancel = Cancel::new().map_err(|reason| ServerError::DirectoryUnavailable { reason })?;
    let leaving = Leaving(Arc::clone(&cancel));
    let section = tokio::task::spawn_blocking(move || scope(&cancel, || with_grants(&state, act)));
    let answered = section
        .await
        .map_err(|error| ServerError::DirectoryUnavailable {
            reason: format!("a grants section ended without its answer: {error}"),
        })?;
    drop(leaving);
    answered
}

#[cfg(test)]
#[path = "spicedb_cancel_tests.rs"]
mod tests;
