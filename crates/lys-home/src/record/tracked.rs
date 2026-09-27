//! A home's tracked set (HOME-019 R1): exactly the files a shipped home
//! carries, enumerated by name and never by pattern.
//!
//! For each session the home lists, `sessions/<id>.jsonl`,
//! `sessions/<id>.index.jsonl` and `sessions/<id>.head` where each exists;
//! each regular file `blocks/<hh>/<name>` whose name is 64 lowercase hex
//! digits and whose directory `<hh>` is the name's first two characters;
//! each `templates/<hh>/<name>` by the same rule. Nothing else: not a
//! session's lock file (`sessions/<id>..lock`), not a head or block
//! temporary, not a file at the home root, nothing under `.git`, not a
//! symbolic link. The set is returned as home-relative paths with `/`
//! separators in ascending byte order, so two enumerations of one home
//! agree whatever order the directory listing gave.

use std::path::Path;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::blocks::Hash;
use crate::record::index::Index;

/// The home's tracked set, as home-relative paths in ascending byte order.
pub fn tracked_set(home: &Home) -> Result<Vec<String>, HomeError> {
    let mut paths = Vec::new();
    for id in home.session_ids()? {
        let file = home.session_path(&id)?;
        let beside = [
            (file.clone(), format!("sessions/{id}.jsonl")),
            (
                Index::index_path(&file),
                format!("sessions/{id}.index.jsonl"),
            ),
            (Index::head_path(&file), format!("sessions/{id}.head")),
        ];
        for (path, name) in beside {
            if is_regular_file(&path)? {
                paths.push(name);
            }
        }
    }
    for store in ["blocks", "templates"] {
        by_hash(&home.root().join(store), store, &mut paths)?;
    }
    paths.sort_unstable();
    Ok(paths)
}

/// Push `<store>/<hh>/<hash>` for every regular file under `dir` named by a
/// hash whose first two characters name its directory. An absent `dir`
/// holds nothing.
fn by_hash(dir: &Path, store: &str, paths: &mut Vec<String>) -> Result<(), HomeError> {
    let shards = match std::fs::read_dir(dir) {
        Ok(shards) => shards,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(HomeError::io("listing a store", dir, e)),
    };
    for shard in shards {
        let shard = shard.map_err(|e| HomeError::io("listing a store", dir, e))?;
        let kind = shard
            .file_type()
            .map_err(|e| HomeError::io("reading a store entry's type", shard.path(), e))?;
        let Some(prefix) = shard.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        let files = std::fs::read_dir(shard.path())
            .map_err(|e| HomeError::io("listing a store directory", shard.path(), e))?;
        for file in files {
            let file = file.map_err(|e| HomeError::io("listing a store directory", dir, e))?;
            let kind = file
                .file_type()
                .map_err(|e| HomeError::io("reading a store file's type", file.path(), e))?;
            let Some(name) = file.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if kind.is_file() && Hash::parse(&name).is_ok() && name.get(..2) == Some(&prefix) {
                paths.push(format!("{store}/{prefix}/{name}"));
            }
        }
    }
    Ok(())
}

/// Whether `path` is a regular file, not following a symbolic link; an
/// absent path is not one.
fn is_regular_file(path: &Path) -> Result<bool, HomeError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => Ok(meta.file_type().is_file()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(HomeError::io("reading a session file's type", path, e)),
    }
}
