use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::changes::Change;
use super::{Kept, unavailable};
use crate::error::ServerError;

pub(super) const HEADER: &[u8] = b"lys-roles-log-v1\n";

pub(super) struct Persistence {
    path: PathBuf,
    journal: bool,
}

impl Persistence {
    pub(super) fn open(path: &Path) -> Result<(Self, Kept), ServerError> {
        let (kept, journal) = read(path)?;
        Ok((
            Self {
                path: path.to_owned(),
                journal,
            },
            kept,
        ))
    }

    pub(super) fn read(&mut self) -> Result<Kept, ServerError> {
        let (kept, journal) = read(&self.path)?;
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
        Ok(kept)
    }

    pub(super) fn append(&mut self, kept: &Kept, change: &Change) -> Result<(), ServerError> {
        let mut record = serde_json::to_vec(change).map_err(unavailable)?;
        record.push(b'\n');
        let written = if self.journal {
            append(&self.path, &record)
        } else {
            let mut snapshot = HEADER.to_vec();
            serde_json::to_writer(&mut snapshot, kept).map_err(unavailable)?;
            snapshot.push(b'\n');
            snapshot.extend_from_slice(&record);
            replace(&self.path, &snapshot)
        };
        written
            .map_err(|error| unavailable(format!("writing {}: {error}", self.path.display())))?;
        self.journal = true;
        Ok(())
    }
}

fn read(path: &Path) -> Result<(Kept, bool), ServerError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok((Kept::default(), false));
        }
        Err(error) => {
            return Err(unavailable(format!("reading {}: {error}", path.display())));
        }
    };
    let decoded = if let Some(records) = bytes.strip_prefix(HEADER) {
        decode(records).map(|kept| (kept, true))
    } else {
        serde_json::from_slice(&bytes)
            .map(|kept| (kept, false))
            .map_err(unavailable)
    };
    decoded.map_err(|error| unavailable(format!("{} does not read: {error}", path.display())))
}

fn decode(records: &[u8]) -> Result<Kept, ServerError> {
    if !records.ends_with(b"\n") {
        return Err(unavailable("role log has an incomplete final record"));
    }
    let mut records = records[..records.len() - 1].split(|byte| *byte == b'\n');
    let snapshot = records
        .next()
        .ok_or_else(|| unavailable("role log has no snapshot"))?;
    let mut kept = serde_json::from_slice(snapshot).map_err(unavailable)?;
    for record in records {
        let change: Change = serde_json::from_slice(record).map_err(unavailable)?;
        change.apply(&mut kept)?;
    }
    Ok(kept)
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

fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = File::create(&beside)?;
    write(&mut file, bytes)?;
    file.sync_all()?;
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
