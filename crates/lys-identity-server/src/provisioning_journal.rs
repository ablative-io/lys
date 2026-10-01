//! A snapshot and checked change frames share one durable file.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{Kept, edits, replace, unavailable};
use crate::error::ServerError;

const MAGIC: &[u8; 8] = b"LYSPR01\n";
const FRAME_HEADER: usize = 40;

#[cfg(test)]
std::thread_local! {
    static FAILED_SETTLE_SYNC: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(super) fn fail_settle_sync(parent: bool) {
    FAILED_SETTLE_SYNC.with(|failure| failure.set(Some(parent)));
}

pub(super) fn read(path: &Path) -> Result<(Kept, bool), ServerError> {
    read_file(path, false)
}

pub(super) fn settle(path: &Path) -> Result<(Kept, bool), ServerError> {
    read_file(path, true)
}

fn read_file(path: &Path, durable: bool) -> Result<(Kept, bool), ServerError> {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if durable {
                sync_parent(path)?;
            }
            return Ok((Kept::default(), false));
        }
        Err(error) => {
            return Err(unavailable(format!("reading {}: {error}", path.display())));
        }
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| unavailable(format!("reading {}: {error}", path.display())))?;
    let kept = decode(path, &bytes)?;
    if durable {
        #[cfg(test)]
        fail_sync(false)?;
        file.sync_all()
            .map_err(|error| unavailable(format!("settling {}: {error}", path.display())))?;
        sync_parent(path)?;
    }
    Ok(kept)
}

#[cfg(test)]
fn fail_sync(parent: bool) -> Result<(), ServerError> {
    FAILED_SETTLE_SYNC.with(|failure| {
        if failure.get() == Some(parent) {
            failure.set(None);
            Err(unavailable("injected provisioning settle sync failure"))
        } else {
            Ok(())
        }
    })
}

fn sync_parent(path: &Path) -> Result<(), ServerError> {
    #[cfg(test)]
    fail_sync(true)?;
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| unavailable(format!("settling parent of {}: {error}", path.display())))
}

fn decode(path: &Path, bytes: &[u8]) -> Result<(Kept, bool), ServerError> {
    if !bytes.starts_with(MAGIC) {
        return serde_json::from_slice(bytes)
            .map(|kept| (kept, false))
            .map_err(|error| unavailable(format!("{} does not read: {error}", path.display())));
    }
    let mut cursor = MAGIC.len();
    let mut kept: Kept = serde_json::from_slice(frame(bytes, &mut cursor)?)
        .map_err(|error| unavailable(format!("provisioning snapshot does not read: {error}")))?;
    while cursor < bytes.len() {
        let edit = serde_json::from_slice(frame(bytes, &mut cursor)?)
            .map_err(|error| unavailable(format!("provisioning change does not read: {error}")))?;
        edits::apply(&mut kept, edit)?;
    }
    Ok((kept, true))
}

fn frame<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], ServerError> {
    let start = *cursor;
    let header_end = start
        .checked_add(FRAME_HEADER)
        .ok_or_else(|| unavailable("provisioning frame offset overflow"))?;
    let header = bytes.get(start..header_end).ok_or_else(|| {
        unavailable(format!(
            "incomplete provisioning frame header at byte {start}"
        ))
    })?;
    let length = u64::from_le_bytes(header[..8].try_into().map_err(unavailable)?);
    let length = usize::try_from(length).map_err(unavailable)?;
    let end = header_end
        .checked_add(length)
        .ok_or_else(|| unavailable("provisioning frame length overflow"))?;
    let payload = bytes.get(header_end..end).ok_or_else(|| {
        unavailable(format!(
            "incomplete provisioning frame body at byte {start}"
        ))
    })?;
    let checksum: [u8; 32] = Sha256::digest(payload).into();
    if header[8..] != checksum {
        return Err(unavailable(format!(
            "provisioning frame checksum mismatch at byte {start}"
        )));
    }
    *cursor = end;
    Ok(payload)
}

fn encode(payload: &[u8]) -> Result<Vec<u8>, ServerError> {
    let length = u64::try_from(payload.len()).map_err(unavailable)?;
    let mut frame = Vec::with_capacity(FRAME_HEADER + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&Sha256::digest(payload));
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub(super) fn snapshot(path: &Path, payload: &[u8]) -> Result<(), ServerError> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&encode(payload)?);
    replace(path, &bytes)
        .map_err(|error| unavailable(format!("writing {}: {error}", path.display())))
}

pub(super) fn append(path: &Path, edit: &edits::Edit) -> Result<(), ServerError> {
    let payload = serde_json::to_vec(edit).map_err(unavailable)?;
    let bytes = encode(&payload)?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| unavailable(format!("opening {} to append: {error}", path.display())))?;
    let before = file.metadata().map_err(unavailable)?.len();
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        let rollback = file.set_len(before).and_then(|()| file.sync_all());
        return Err(unavailable(match rollback {
            Ok(()) => format!("appending {}: {error}; append rolled back", path.display()),
            Err(rollback) => format!(
                "appending {}: {error}; append rollback failed: {rollback}",
                path.display()
            ),
        }));
    }
    Ok(())
}
