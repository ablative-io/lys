use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use lys_identity::SNAPSHOT_EVERY;

use super::changes::Change;
use super::{Kept, unavailable};
use crate::error::ServerError;

pub(super) const HEADER: &[u8] = b"lys-roles-log-v1\n";

pub(super) struct Persistence {
    path: PathBuf,
    journal: bool,
    since_snapshot: u64,
}

impl Persistence {
    pub(super) fn open(path: &Path) -> Result<(Self, Kept), ServerError> {
        let (kept, journal, since_snapshot) = read(path)?;
        let mut persistence = Self {
            path: path.to_owned(),
            journal,
            since_snapshot,
        };
        if since_snapshot > SNAPSHOT_EVERY.get() {
            persistence.checkpoint_if_due(&kept)?;
        }
        Ok((persistence, kept))
    }

    pub(super) fn read(&mut self) -> Result<Kept, ServerError> {
        let (kept, journal, since_snapshot) = read(&self.path)?;
        if journal {
            OpenOptions::new()
                .write(true)
                .open(&self.path)
                .and_then(|file| file.sync_all())
                .and_then(|()| sync_parent(&self.path))
                .map_err(|error| {
                    unavailable(format!("settling {}: {error}", self.path.display()))
                })?;
        }
        self.journal = journal;
        self.since_snapshot = since_snapshot;
        self.checkpoint_if_due(&kept)?;
        Ok(kept)
    }

    pub(super) fn append(&mut self, kept: &Kept, change: &Change) -> Result<(), ServerError> {
        self.checkpoint_if_due(kept)?;
        let next = self
            .since_snapshot
            .checked_add(1)
            .ok_or_else(|| unavailable("role log record count overflow"))?;
        let mut record = serde_json::to_vec(change).map_err(unavailable)?;
        record.push(b'\n');
        let written = if self.journal {
            append(&self.path, &record)
        } else {
            replace_snapshot(&self.path, kept, Some(&record))
        };
        written
            .map_err(|error| unavailable(format!("writing {}: {error}", self.path.display())))?;
        self.journal = true;
        self.since_snapshot = next;
        Ok(())
    }

    pub(super) fn checkpoint_if_due(&mut self, kept: &Kept) -> Result<(), ServerError> {
        if self.journal && self.since_snapshot >= SNAPSHOT_EVERY.get() {
            replace_snapshot(&self.path, kept, None).map_err(|error| {
                unavailable(format!("checkpointing {}: {error}", self.path.display()))
            })?;
            self.since_snapshot = 0;
        }
        Ok(())
    }
}

fn read(path: &Path) -> Result<(Kept, bool, u64), ServerError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok((Kept::default(), false, 0));
        }
        Err(error) => {
            return Err(unavailable(format!("reading {}: {error}", path.display())));
        }
    };
    let decoded = if let Some(records) = bytes.strip_prefix(HEADER) {
        let complete = records
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|position| position + 1)
            .ok_or_else(|| unavailable("role log has an incomplete initial snapshot"))?;
        let (kept, count) = decode(&records[..complete])?;
        if complete != records.len() {
            recover_tail(path, HEADER.len() + complete, bytes.len())?;
        }
        Ok((kept, true, count))
    } else {
        serde_json::from_slice(&bytes)
            .map(|kept| (kept, false, 0))
            .map_err(unavailable)
    };
    decoded.map_err(|error| unavailable(format!("{} does not read: {error}", path.display())))
}

fn decode(records: &[u8]) -> Result<(Kept, u64), ServerError> {
    let mut records = records[..records.len() - 1].split(|byte| *byte == b'\n');
    let snapshot = records
        .next()
        .ok_or_else(|| unavailable("role log has no snapshot"))?;
    let mut kept = serde_json::from_slice(snapshot).map_err(unavailable)?;
    let mut count = 0_u64;
    for record in records {
        let change: Change = serde_json::from_slice(record).map_err(unavailable)?;
        #[cfg(test)]
        super::REPLAYED_CHANGES.with(|replayed| replayed.set(replayed.get() + 1));
        change.apply(&mut kept)?;
        count = count
            .checked_add(1)
            .ok_or_else(|| unavailable("role log record count overflow"))?;
    }
    Ok((kept, count))
}

fn recover_tail(path: &Path, length: usize, before: usize) -> Result<(), ServerError> {
    let length_u64 = u64::try_from(length).map_err(unavailable)?;
    OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| {
            file.set_len(length_u64)?;
            file.sync_all()
        })
        .map_err(|error| unavailable(format!("recovering {}: {error}", path.display())))?;
    tracing::warn!(
        recovery = "roles_tail_recovered",
        discarded_bytes = before - length,
        "an incomplete final role change was durably removed"
    );
    #[cfg(test)]
    super::RECOVERED_TAILS.with(|recovered| recovered.set(recovered.get() + 1));
    Ok(())
}

fn write(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    #[cfg(test)]
    super::WRITTEN_BYTES.with(|written| written.set(written.get() + bytes.len()));
    Ok(())
}

fn append(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(path)?;
    let before = file.metadata()?.len();
    if let Err(failure) = write(&mut file, bytes) {
        file.set_len(before)
            .and_then(|()| file.sync_all())
            .map_err(|rollback| {
                io::Error::new(
                    rollback.kind(),
                    format!("{failure}; restoring the prior log length: {rollback}"),
                )
            })?;
        return Err(failure);
    }
    file.sync_all()
}

fn replace_snapshot(path: &Path, kept: &Kept, record: Option<&[u8]>) -> io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = BufWriter::with_capacity(8 * 1024, File::create(&beside)?);
    file.write_all(HEADER)?;
    serde_json::to_writer(&mut file, kept).map_err(io::Error::other)?;
    file.write_all(b"\n")?;
    if let Some(record) = record {
        file.write_all(record)?;
    }
    file.flush()?;
    #[cfg(test)]
    {
        let bytes = usize::try_from(file.get_ref().metadata()?.len()).map_err(io::Error::other)?;
        super::WRITTEN_BYTES.with(|written| written.set(written.get() + bytes));
    }
    file.get_ref().sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    sync_parent(path)
}

fn sync_parent(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}
