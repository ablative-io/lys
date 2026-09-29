//! A file read once and held. Each ask looks at the file's length and
//! modification time, which reads nothing of it, and reads and parses it
//! again only when either is not what it was at the reading held. So a
//! file another process changes (an `add-route` or `trust-service` while
//! the proxy serves) is seen at the next ask, and an unchanged one is never
//! read twice.
//!
//! Invariants. What is held is always a parse of the file's bytes read no
//! earlier than the length and time it is held under, so it is never older
//! than the file was when those were taken. A file that is not there is
//! empty, as it always was, and nothing of an earlier reading is kept. A
//! write through [`Watched::write`] lets go of what was held, so the next
//! ask reads what was written. A read or parse that fails holds nothing and
//! is answered as the failure, every time it fails.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use lys_secrets::SecretsError;

/// A file's modification time and length.
type Stamp = (Option<SystemTime>, u64);

/// A file read once and held until it changes.
#[derive(Debug)]
pub struct Watched<T> {
    path: PathBuf,
    held: Mutex<Option<(Stamp, Arc<T>)>>,
    /// How many times the file was read.
    pub(crate) reads: AtomicU64,
}

/// The stamp of the file at `path`, when there is one.
fn stamp(path: &Path) -> Option<Stamp> {
    fs::metadata(path)
        .ok()
        .map(|meta| (meta.modified().ok(), meta.len()))
}

impl<T: Default> Watched<T> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            held: Mutex::new(None),
            reads: AtomicU64::new(0),
        }
    }

    /// The file as `parse` reads it, from what is held when the file has
    /// not changed since it was read.
    pub fn get(
        &self,
        parse: impl FnOnce(&[u8]) -> Result<T, SecretsError>,
    ) -> Result<Arc<T>, SecretsError> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(now) = stamp(&self.path) else {
            *held = None;
            return Ok(Arc::new(T::default()));
        };
        if let Some((at, value)) = held.as_ref() {
            if *at == now {
                return Ok(Arc::clone(value));
            }
        }
        *held = None;
        self.reads.fetch_add(1, Ordering::Relaxed);
        let bytes = fs::read(&self.path).map_err(|source| SecretsError::Io {
            context: format!("reading {}", self.path.display()),
            source,
        })?;
        let value = Arc::new(parse(&bytes)?);
        *held = Some((now, Arc::clone(&value)));
        Ok(value)
    }

    /// Writes `bytes` as the file, and lets go of what was held.
    pub fn write(&self, bytes: &[u8]) -> Result<(), SecretsError> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        *held = None;
        let failure = |source| SecretsError::Io {
            context: format!("writing {}", self.path.display()),
            source,
        };
        let directory = self
            .path
            .parent()
            .ok_or_else(|| failure(std::io::Error::other("missing parent directory")))?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(failure)?;
        #[cfg(test)]
        if crate::route_write_fault_tests::partial_write(
            &self.path,
            temporary.as_file_mut(),
            bytes,
        )? {
            return Err(failure(std::io::Error::other(
                "injected partial route write",
            )));
        }
        temporary.write_all(bytes).map_err(failure)?;
        temporary.as_file().sync_all().map_err(failure)?;
        temporary
            .persist(&self.path)
            .map_err(|error| failure(error.error))?;
        fs::File::open(directory)
            .and_then(|file| file.sync_all())
            .map_err(failure)
    }
}
