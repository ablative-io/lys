//! The Claude Code render's one write stage.
//!
//! Every serialisation the render needs is decided here before anything is
//! created: each record compactly with `serde_json::to_string`, one line per
//! record followed by a newline, and the loss account, alone, pretty-printed.
//! A serialisation that fails refuses the render as
//! [`HomeError::RenderUnserialisable`], naming the session, what would not
//! serialise and the entry the record was rendered from, and no directory,
//! rendered file, loss account or seed exists afterwards. Only then is the
//! render directory created, the rendered file created new and synced, the
//! loss account written beside it and the seed, when there is one, written
//! beside that. Nothing else in the render touches the filesystem.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::error::HomeError;
use crate::harness::claude_code::seed::{Seed, write_seed};

/// A [`HomeError::RenderUnserialisable`] of `what` in `session`.
fn unserialisable(
    session: &str,
    what: &'static str,
    entry: Option<&str>,
    source: serde_json::Error,
) -> HomeError {
    HomeError::RenderUnserialisable {
        session: session.to_owned(),
        what,
        entry: entry.map(str::to_owned),
        source,
    }
}

/// Write a render: `records`, each beside the id of the entry it was
/// rendered from (`None` for a summary record), to `path`; `account` to the
/// loss account beside it; and the seed to its path when there is one.
/// Returns the loss account's path. `session` is the home session's header
/// id, which a refusal names.
pub(crate) fn write_render<A: Serialize>(
    session: &str,
    path: &Path,
    records: &[(Value, Option<String>)],
    account: &A,
    seed: Option<(&Seed, &Path)>,
) -> Result<PathBuf, HomeError> {
    let mut lines = String::new();
    for (record, entry) in records {
        let line = serde_json::to_string(record)
            .map_err(|source| unserialisable(session, "record", entry.as_deref(), source))?;
        lines.push_str(&line);
        lines.push('\n');
    }
    let loss = serde_json::to_vec_pretty(account)
        .map_err(|source| unserialisable(session, "loss account", None, source))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the render directory", dir, e))?;
    }
    {
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|e| HomeError::io("creating the rendered file", path, e))?;
        f.write_all(lines.as_bytes())
            .map_err(|e| HomeError::io("writing the rendered file", path, e))?;
        f.sync_all()
            .map_err(|e| HomeError::io("syncing the rendered file", path, e))?;
    }
    let loss_path = path.with_extension("loss.json");
    std::fs::write(&loss_path, loss)
        .map_err(|e| HomeError::io("writing the loss account", &loss_path, e))?;
    if let Some((seed, file)) = seed {
        write_seed(file, seed)?;
    }
    Ok(loss_path)
}
