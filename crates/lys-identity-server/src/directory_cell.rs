//! The directory as the routes share it: one writer at a time, and beside
//! it the projection the last settled directory stood at, which a read
//! answers from without waiting on a write in progress.
//!
//! A write, and any call that needs the log or must settle an uncertain
//! append, holds the writer's lock, and runs where it may wait on the lock
//! and the disk without holding an async worker: on a multi-threaded
//! runtime the worker hands its other tasks on before the section starts.
//!
//! A read of the projection takes the published one when it is held. It is
//! published by the first read after a change, cloned once under the
//! writer's lock, and withdrawn by any change to the leaves folded and by an
//! append whose outcome is not yet known, so a read never answers a state a
//! write has already answered past, and an unsettled directory is read
//! through the writer, which settles it or refuses by name as before. While
//! a write is in progress a read answers the state before it, which is the
//! state every answer so far was given from.

use std::sync::{Arc, Mutex, PoisonError};

use lys_identity::Directory;
use lys_identity::projection::Projection;
use lys_log_store::LeafStore;
use tokio::runtime::{Handle, RuntimeFlavor};

use crate::error::ServerError;

/// The projection a settled directory stood at, with how many leaves it
/// had folded then.
type Published = Option<(u64, Arc<Projection>)>;

/// The directory, one writer at a time, and the projection a read answers
/// from.
pub struct DirectoryCell<S: LeafStore> {
    writer: Mutex<Directory<S>>,
    published: Mutex<Published>,
}

/// Run `work`, which may wait on a lock or on the disk, without holding an
/// async worker: on a multi-threaded runtime the worker hands its other
/// tasks on first; with no runtime, or on one thread, it runs as it is.
fn blocking<T>(work: impl FnOnce() -> T) -> T {
    match Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(work)
        }
        _ => work(),
    }
}

impl<S: LeafStore> DirectoryCell<S> {
    /// The directory `directory`, nothing published yet.
    pub fn new(directory: Directory<S>) -> Self {
        Self {
            writer: Mutex::new(directory),
            published: Mutex::new(None),
        }
    }

    /// Run `act` on the directory under the writer's lock, off the async
    /// worker; the published projection is withdrawn when `act` changed the
    /// leaves folded or left the directory unsettled.
    pub fn write<T>(
        &self,
        act: impl FnOnce(&mut Directory<S>) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        blocking(|| {
            let mut directory = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
            let answer = act(&mut directory);
            let (settled, folded) = (directory.is_settled(), directory.folded());
            let mut published = self
                .published
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if !settled || published.as_ref().is_none_or(|(held, _)| *held != folded) {
                *published = None;
            }
            answer
        })
    }

    /// Run `act` on the projection as the last settled directory stood,
    /// without waiting on a write in progress. With none published, the
    /// directory is settled under the writer's lock, as a write would, its
    /// projection published, and `act` answered from it.
    pub fn read<T>(
        &self,
        act: impl FnOnce(&Projection) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        let held = self
            .published
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|(_, projection)| Arc::clone(projection));
        if let Some(projection) = held {
            return act(&projection);
        }
        let projection = self.write(|directory| {
            let projection = Arc::new(directory.projection()?.clone());
            let mut published = self
                .published
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            *published = Some((directory.folded(), Arc::clone(&projection)));
            Ok(projection)
        })?;
        act(&projection)
    }
}

#[cfg(test)]
#[path = "directory_cell_tests.rs"]
mod tests;
