//! The per-leaf directory layout, `lys/log-dir/v1`, read as bytes for the one
//! migration out of it (LYSLOGSTORE-008 R4). Nothing here writes.
//!
//! The layout: `log.json` with the format marker and origin, `state.json`
//! with the pinned tree size and root, one file per leaf under `leaves/`
//! named by its twenty-digit index, and `snapshot.bin` when a snapshot was
//! kept. A leaf file holds the leaf's bytes exactly.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;

use crate::error::{StoreError, StoreResult};
use crate::store::PinnedRoot;

/// The v1 format marker, as `log.json` carried it.
pub(crate) const FORMAT: &str = "lys/log-dir/v1";

/// Width of a v1 leaf filename.
const LEAF_NAME_WIDTH: usize = 20;

/// `log.json`, read for its marker alone.
#[derive(Deserialize)]
struct Marker {
    format: String,
}

/// `log.json` in full.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    format: String,
    origin: String,
}

/// `state.json`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    tree_size: u64,
    root_hash: String,
}

/// What a v1 directory holds, read whole.
pub(crate) struct Contents {
    /// The log's origin.
    pub(crate) origin: String,
    /// The pinned `(tree_size, root)`.
    pub(crate) pinned: PinnedRoot,
    /// Every leaf under the pin, in order.
    pub(crate) leaves: Vec<Vec<u8>>,
    /// Leaves present past the pin: an append the v1 store never pinned.
    /// They are not migrated; the kept directory still holds them.
    pub(crate) beyond_pin: u64,
    /// The snapshot slot's bytes, when one was kept.
    pub(crate) snapshot: Option<Vec<u8>>,
}

/// Whether `dir` holds a v1 store: its `log.json` names the v1 marker. A
/// directory with no `log.json` is not one; the open says `NotInitialized`.
pub(crate) fn present(dir: &Path) -> StoreResult<bool> {
    let path = dir.join("log.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(StoreError::Io {
                context: format!("failed to read {}", path.display()),
                source,
            });
        }
    };
    let marker: Marker = serde_json::from_slice(&bytes).map_err(|error| StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!("log.json does not parse: {error}"),
    })?;
    Ok(marker.format == FORMAT)
}

/// Read the v1 store at `dir` whole: identity, pin, every leaf under the pin
/// in order, and the snapshot. A leaf missing under the pin is `Corrupt`,
/// named by index.
pub(crate) fn read(dir: &Path) -> StoreResult<Contents> {
    let config: Config = parse(dir, "log.json")?;
    if config.format != FORMAT {
        return Err(StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: format!("log.json format is {:?}, not {FORMAT:?}", config.format),
        });
    }
    let state: State = parse(dir, "state.json")?;
    let root = STANDARD
        .decode(&state.root_hash)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: "state.json root_hash is not 32 base64 bytes".to_owned(),
        })?;
    let pinned = PinnedRoot {
        tree_size: state.tree_size,
        root,
    };
    let count = usize::try_from(state.tree_size).map_err(|source| {
        StoreError::LeafCountUnrepresentable {
            count: state.tree_size,
            source,
        }
    })?;
    let mut leaves = Vec::with_capacity(count);
    for index in 0..state.tree_size {
        let Some(bytes) = leaf(dir, index)? else {
            return Err(StoreError::Corrupt {
                path: dir.to_path_buf(),
                reason: format!(
                    "leaf {index} is missing under the pin at {}",
                    state.tree_size
                ),
            });
        };
        leaves.push(bytes);
    }
    let mut beyond_pin = 0;
    while leaf(dir, state.tree_size + beyond_pin)?.is_some() {
        beyond_pin += 1;
    }
    let snapshot_path = dir.join("snapshot.bin");
    let snapshot = match std::fs::read(&snapshot_path) {
        Ok(bytes) => Some(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(StoreError::Io {
                context: format!("failed to read {}", snapshot_path.display()),
                source,
            });
        }
    };
    Ok(Contents {
        origin: config.origin,
        pinned,
        leaves,
        beyond_pin,
        snapshot,
    })
}

fn parse<T: serde::de::DeserializeOwned>(dir: &Path, name: &str) -> StoreResult<T> {
    let path = dir.join(name);
    let bytes = std::fs::read(&path).map_err(|source| StoreError::Io {
        context: format!("failed to read {}", path.display()),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|error| StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!("{name} does not parse: {error}"),
    })
}

fn leaf(dir: &Path, index: u64) -> StoreResult<Option<Vec<u8>>> {
    let path = dir
        .join("leaves")
        .join(format!("{index:0LEAF_NAME_WIDTH$}"));
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StoreError::Io {
            context: format!("failed to read leaf file {}", path.display()),
            source,
        }),
    }
}
