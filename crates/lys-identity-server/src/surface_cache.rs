//! Installed screens are read once and shared without copying their bodies.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use axum::body::Bytes;
use axum::http::HeaderValue;
use sha2::{Digest, Sha256};

use crate::error::ServerError;

const FILE_BYTES: u64 = 16 * 1024 * 1024;
const TOTAL_BYTES: usize = 64 * 1024 * 1024;
const ENTRIES: usize = 4096;

pub(super) struct Asset {
    pub(super) bytes: Bytes,
    pub(super) etag: HeaderValue,
}

pub(super) struct Screens {
    files: BTreeMap<PathBuf, Asset>,
}

fn refused(reason: impl std::fmt::Display) -> ServerError {
    ServerError::ConfigInvalid {
        reason: format!("the installed screens could not be loaded: {reason}"),
    }
}

impl Screens {
    pub(super) fn load(dir: &Path) -> Result<Self, ServerError> {
        let mut files = BTreeMap::new();
        let mut pending = vec![PathBuf::new()];
        let mut entries = 0;
        let mut total = 0;
        while let Some(relative) = pending.pop() {
            let folder = dir.join(&relative);
            let children = match std::fs::read_dir(&folder) {
                Ok(children) => children,
                Err(error)
                    if relative.as_os_str().is_empty()
                        && error.kind() == std::io::ErrorKind::NotFound =>
                {
                    continue;
                }
                Err(error) => return Err(refused(error)),
            };
            for child in children {
                let child = child.map_err(refused)?;
                entries += 1;
                if entries > ENTRIES {
                    return Err(refused("the bundle has more than 4096 entries"));
                }
                let path = relative.join(child.file_name());
                let kind = child.file_type().map_err(refused)?;
                if kind.is_dir() {
                    pending.push(path);
                    continue;
                }
                if !kind.is_file() {
                    return Err(refused("a bundle entry is not a regular file or directory"));
                }
                let file = std::fs::File::open(child.path()).map_err(refused)?;
                let mut bytes = Vec::new();
                file.take(FILE_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(refused)?;
                total += bytes.len();
                if bytes.len() as u64 > FILE_BYTES || total > TOTAL_BYTES {
                    return Err(refused(
                        "a file exceeds 16 MiB or the bundle exceeds 64 MiB",
                    ));
                }
                let etag = HeaderValue::from_str(&format!("\"{:x}\"", Sha256::digest(&bytes)))
                    .map_err(refused)?;
                files.insert(
                    path,
                    Asset {
                        bytes: Bytes::from(bytes),
                        etag,
                    },
                );
            }
        }
        Ok(Self { files })
    }

    pub(super) fn get(&self, path: &Path) -> Option<&Asset> {
        self.files.get(path)
    }
}
