//! The head and the header of a session file, read and written beside its
//! index.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::HomeError;
use crate::record::blocks::sync_dir;
use crate::record::entries::SessionHeader;

use super::{Index, parent_dir};

/// Read the persisted head: `Some(id)`, `None` when the head is the header
/// (no entries or moved before the first), or the last indexed entry when no
/// head file exists.
pub fn read_head(session_file: &Path, index: &Index) -> Result<Option<String>, HomeError> {
    let path = Index::head_path(session_file);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let id = text.trim();
            Ok(if id.is_empty() {
                None
            } else {
                Some(id.to_owned())
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(index.last().map(|r| r.id.clone()))
        }
        Err(e) => Err(HomeError::io("reading the head", &path, e)),
    }
}

/// Persist the head: written whole to a temporary file and renamed into place.
pub fn write_head(session_file: &Path, head: Option<&str>) -> Result<(), HomeError> {
    let path = Index::head_path(session_file);
    let tmp = path.with_extension("head.tmp");
    {
        let mut file =
            fs::File::create(&tmp).map_err(|e| HomeError::io("creating the head", &tmp, e))?;
        file.write_all(head.unwrap_or("").as_bytes())
            .map_err(|e| HomeError::io("writing the head", &tmp, e))?;
        file.write_all(b"\n")
            .map_err(|e| HomeError::io("writing the head", &tmp, e))?;
        file.sync_all()
            .map_err(|e| HomeError::io("syncing the head", &tmp, e))?;
    }
    fs::rename(&tmp, &path).map_err(|e| HomeError::io("placing the head", &path, e))?;
    sync_dir(parent_dir(&path))
}

/// The header line of a session file and its length in bytes.
pub fn read_header(session_file: &Path) -> Result<(SessionHeader, u64), HomeError> {
    let mut reader = BufReader::new(
        fs::File::open(session_file)
            .map_err(|e| HomeError::io("opening the session file", session_file, e))?,
    );
    let mut line = Vec::new();
    let read = reader
        .read_until(b'\n', &mut line)
        .map_err(|e| HomeError::io("reading the session file", session_file, e))?;
    if read == 0 {
        return Err(HomeError::NoHeader {
            path: session_file.to_path_buf(),
            reason: "the file is empty".to_owned(),
        });
    }
    let header: SessionHeader = serde_json::from_slice(&line).map_err(|e| HomeError::NoHeader {
        path: session_file.to_path_buf(),
        reason: e.to_string(),
    })?;
    Ok((header, read as u64))
}
