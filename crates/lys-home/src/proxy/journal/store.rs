//! Appending, settling and reading the open-call journal.

use std::path::{Path, PathBuf};

use super::{Journal, OpenCall};
use crate::proxy::error::ProxyError;

impl Journal {
    /// The journal in `dir`, created when absent.
    ///
    /// # Errors
    ///
    /// Refuses by path when the journal directory cannot be created.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, ProxyError> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)
            .map_err(|e| ProxyError::io("creating the open-call journal", &dir, e))?;
        Ok(Self { dir })
    }

    /// Where the journal lives.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Where the per-run usage files live: `usage`, beside the journal in
    /// the proxy's state ([`crate::proxy::usage`]).
    #[must_use]
    pub fn usage_dir(&self) -> PathBuf {
        self.dir.parent().unwrap_or(&self.dir).join("usage")
    }

    fn path_of(&self, call_id: &str) -> PathBuf {
        self.dir.join(format!("{call_id}.json"))
    }

    /// Write (or rewrite) a call's record durably.
    ///
    /// # Errors
    ///
    /// Refuses a record that cannot be encoded or appended whole, or a
    /// journal file or newly created directory entry that cannot be synced.
    pub fn write(&self, call: &OpenCall) -> Result<(), ProxyError> {
        #[cfg(test)]
        let started = std::time::Instant::now();
        let result = self.put(call).and_then(|()| self.settle(&call.call_id));
        #[cfg(test)]
        crate::proxy::timing::add(&crate::proxy::timing::JOURNAL_WRITE, started);
        result
    }

    /// Put a call's record in place whole, without waiting for the disk.
    ///
    /// This is all a call waits for on the forwarding path. From here the
    /// record is there for the proxy's next start, so a call in flight when
    /// the proxy ends is still found and recorded lost. It holds through a
    /// loss of power once [`Journal::settle`] has returned for it.
    /// Append the call's record as one line of its journal file (AGENTS-005
    /// P01/P04b): an established record takes one append and no rename, so
    /// its durability is the one file sync `settle` makes; a line a dying
    /// proxy left unfinished is closed first, and the last whole line is the
    /// record. Only the cold creation of the file syncs the directory.
    ///
    /// # Errors
    ///
    /// Refuses record serialisation, a file that cannot be opened, measured,
    /// sought, read or appended, or a new journal entry whose directory cannot
    /// be synced.
    pub fn put(&self, call: &OpenCall) -> Result<(), ProxyError> {
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut bytes = serde_json::to_vec(call).map_err(|source| ProxyError::JournalEncode {
            call_id: call.call_id.clone(),
            source,
        })?;
        let path = self.path_of(&call.call_id);
        let unwritable = |source| ProxyError::JournalUnwritable {
            path: path.clone(),
            source,
        };
        let created = !path.is_file();
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .map_err(unwritable)?;
        let mut line = Vec::with_capacity(bytes.len() + 2);
        if file.metadata().map_err(unwritable)?.len() > 0 {
            file.seek(SeekFrom::End(-1)).map_err(unwritable)?;
            let mut last = [0_u8; 1];
            file.read_exact(&mut last).map_err(unwritable)?;
            if last != *b"\n" {
                line.push(b'\n');
            }
        }
        line.append(&mut bytes);
        line.push(b'\n');
        file.write_all(&line).map_err(unwritable)?;
        if created {
            sync_dir(&self.dir).map_err(unwritable)?;
        }
        Ok(())
    }

    /// Make a put record hold through a loss of power: its file synced,
    /// then the journal's directory. A record retired meanwhile has nothing
    /// left to settle.
    ///
    /// # Errors
    ///
    /// Refuses when an existing journal record cannot be opened or synced.
    /// A record already retired has nothing left to settle.
    pub fn settle(&self, call_id: &str) -> Result<(), ProxyError> {
        let path = self.path_of(call_id);
        let unwritable = |source| ProxyError::JournalUnwritable {
            path: path.clone(),
            source,
        };
        // One sync: the record is appended in place, so its directory entry
        // was made durable when the file was created, never again here.
        match std::fs::File::open(&path) {
            Ok(file) => file.sync_all().map_err(unwritable),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(unwritable(error)),
        }
    }

    /// Settle a put record beside the call that put it, so the call does
    /// not wait for the disk: on one of the runtime's blocking threads when
    /// this is called inside a runtime, and here otherwise. A settle that
    /// fails is said; the call's own record is written durably at its end.
    pub fn settle_beside(&self, call_id: &str) {
        let (journal, call_id) = (self.clone(), call_id.to_owned());
        let settle = move || {
            if let Err(error) = journal.settle(&call_id) {
                eprintln!("lys-proxy: {error}");
            }
        };
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => drop(runtime.spawn_blocking(settle)),
            Err(_) => settle(),
        }
    }

    /// Remove a call's record once its outcome is recorded.
    ///
    /// # Errors
    ///
    /// Refuses when the record cannot be removed or the journal directory
    /// cannot be synced. An already absent record is retired.
    pub fn retire(&self, call_id: &str) -> Result<(), ProxyError> {
        let path = self.path_of(call_id);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(ProxyError::io("retiring a journal record", &path, e)),
        }
        sync_dir(&self.dir).map_err(|e| ProxyError::io("syncing the journal", &self.dir, e))
    }

    /// Every open call's record, in file-name order.
    ///
    /// # Errors
    ///
    /// Refuses a journal that cannot be listed, a record that cannot be read,
    /// or a file with no whole line that reads as an open-call record.
    pub fn open_calls(&self) -> Result<Vec<OpenCall>, ProxyError> {
        let listing = std::fs::read_dir(&self.dir)
            .map_err(|e| ProxyError::io("listing the open-call journal", &self.dir, e))?;
        let mut paths = Vec::new();
        for item in listing {
            let item =
                item.map_err(|e| ProxyError::io("listing the open-call journal", &self.dir, e))?;
            let path = item.path();
            let name = item.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with('.') && name.ends_with(".json") {
                paths.push(path);
            }
        }
        paths.sort();
        let mut calls = Vec::with_capacity(paths.len());
        for path in paths {
            let bytes = std::fs::read(&path)
                .map_err(|e| ProxyError::io("reading a journal record", &path, e))?;
            let call = last_record(&bytes, path)?;
            calls.push(call);
        }
        Ok(calls)
    }

    /// Whether the journal in `dir` holds an open call that carries the run
    /// key `run`. This is how another process than the proxy asks whether a
    /// run still has a call whose usage line is yet to be written: the line
    /// is appended before the call's record is retired, so a run with no
    /// record here has every finished call's line in its usage file. A record
    /// retired between the listing and its read is no longer open.
    ///
    /// # Errors
    ///
    /// Refuses a directory that cannot be listed, a record that cannot be
    /// read, or one with no whole open-call record. A record retired between
    /// listing and reading is skipped.
    pub fn holds_run(dir: &Path, run: &str) -> Result<bool, ProxyError> {
        let listing = std::fs::read_dir(dir)
            .map_err(|e| ProxyError::io("listing the open-call journal", dir, e))?;
        for item in listing {
            let item = item.map_err(|e| ProxyError::io("listing the open-call journal", dir, e))?;
            let name = item.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }
            let path = item.path();
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(ProxyError::io("reading a journal record", &path, e)),
            };
            let call = last_record(&bytes, path)?;
            if call.run.as_deref() == Some(run) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// The last whole line of a journal file as its record: a line a dying
/// proxy left unfinished at the tail is not the record, the whole line
/// before it is; a file with no whole record is refused by name.
fn last_record(bytes: &[u8], path: PathBuf) -> Result<OpenCall, ProxyError> {
    let mut lines = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    let last = lines.next_back().unwrap_or_default();
    match serde_json::from_slice(last) {
        Ok(call) => Ok(call),
        Err(source) => match lines.next_back() {
            Some(whole) if !bytes.ends_with(b"\n") => serde_json::from_slice(whole)
                .map_err(|_torn_tail| ProxyError::JournalRecord { path, source }),
            _ => Err(ProxyError::JournalRecord { path, source }),
        },
    }
}

fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}
