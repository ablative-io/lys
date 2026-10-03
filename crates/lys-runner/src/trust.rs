//! The trust answer for the folder a Lys-started Claude Code run starts in.
//!
//! Naming the folder on the launch is the trust answer (Tom, 3 October
//! 2026): a run that sits at a trust prompt nobody sees is a silent failure.
//! So before the spawn the runner records the run's folder as trusted in the
//! harness's own per-project configuration, `.claude.json` under the
//! harness's configuration home: one row for that path, nothing else in the
//! file touched, the whole file written to a sibling and renamed over it so
//! the harness's own writes are never torn. A row already there leaves the
//! file byte for byte as it was.
//!
//! The file is the one the harness reads for this run: under
//! `CLAUDE_CONFIG_DIR` when the launch's environment sets it, else under the
//! login's home, which the harness inherits from the runner. A Lys-started
//! seat uses the machine's own Claude Code setup unless the person chose a
//! configuration directory, so for most runs this is the login's
//! `~/.claude.json`, and this one row is the only write Lys makes there.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::RunnerError;

/// The configuration home the harness reads for a run with `environment`:
/// `CLAUDE_CONFIG_DIR` when set there, else the runner's own `HOME`, which
/// the harness inherits. Either must be absolute; neither is invented.
pub fn config_home(environment: &BTreeMap<String, String>) -> Result<PathBuf, RunnerError> {
    let unknown = |words: String| RunnerError::refused("trust_home_unknown", words);
    let (name, home) = match environment.get("CLAUDE_CONFIG_DIR") {
        Some(dir) => ("CLAUDE_CONFIG_DIR", PathBuf::from(dir)),
        None => (
            "HOME",
            std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
                unknown(
                    "the launch sets no CLAUDE_CONFIG_DIR and the runner has no HOME to find the harness's configuration in"
                        .to_owned(),
                )
            })?,
        ),
    };
    if !home.is_absolute() {
        return Err(unknown(format!(
            "{name} is {}, not an absolute path",
            home.display()
        )));
    }
    Ok(home)
}

/// The harness's per-project configuration file, under its configuration
/// home.
pub const FILE: &str = ".claude.json";
/// The row that answers the trust prompt for a project folder.
pub const ROW: &str = "hasTrustDialogAccepted";

/// What recording the trust answer found and did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trust {
    /// The file the row is in.
    pub file: String,
    /// The folder the row names.
    pub directory: String,
    /// Whether the file was written: false when the row was already there.
    pub written: bool,
}

impl Trust {
    /// The record's words.
    pub fn words(&self) -> String {
        if self.written {
            format!(
                "recorded {} as trusted in {} before the spawn",
                self.directory, self.file
            )
        } else {
            format!(
                "{} was already trusted in {}; nothing written",
                self.directory, self.file
            )
        }
    }
}

/// Record `directory` as trusted in the harness's configuration under
/// `config_home`. The file is read whole, the one row set, and the file
/// written to a sibling and renamed into place; a file that is not a JSON
/// object, or whose `projects` or row is not one, is refused
/// `trust_file_invalid` and left as it is.
pub fn record(config_home: &Path, directory: &str) -> Result<Trust, RunnerError> {
    let file = config_home.join(FILE);
    let shown = file.display().to_string();
    let invalid = |words: String| RunnerError::refused("trust_file_invalid", words);
    let mut root = match std::fs::read(&file) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
            .map_err(|error| invalid(format!("{shown} is not JSON: {error}")))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Value::Object(Map::new()),
        Err(error) => {
            return Err(RunnerError::refused(
                "trust_file_unreadable",
                format!("{shown} was not read: {error}"),
            ));
        }
    };
    let Value::Object(top) = &mut root else {
        return Err(invalid(format!("{shown} is not a JSON object")));
    };
    let projects = top
        .entry("projects")
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(projects) = projects else {
        return Err(invalid(format!("{shown}: projects is not a JSON object")));
    };
    let row = projects
        .entry(directory.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(row) = row else {
        return Err(invalid(format!(
            "{shown}: the row for {directory} is not a JSON object"
        )));
    };
    if row.get(ROW) == Some(&Value::Bool(true)) {
        return Ok(Trust {
            file: shown,
            directory: directory.to_owned(),
            written: false,
        });
    }
    row.insert(ROW.to_owned(), Value::Bool(true));
    let mut bytes = serde_json::to_vec_pretty(&root)
        .map_err(|error| invalid(format!("{shown} was not rendered: {error}")))?;
    bytes.push(b'\n');
    let unwritable = |words: String| RunnerError::refused("trust_file_unwritable", words);
    std::fs::create_dir_all(config_home)
        .map_err(|error| unwritable(format!("{} was not made: {error}", config_home.display())))?;
    let sibling = config_home.join(format!("{FILE}.lys-{}", std::process::id()));
    let mut temp = std::fs::File::create(&sibling)
        .map_err(|error| unwritable(format!("{} was not made: {error}", sibling.display())))?;
    temp.write_all(&bytes)
        .and_then(|()| temp.sync_all())
        .map_err(|error| unwritable(format!("{} was not written: {error}", sibling.display())))?;
    drop(temp);
    std::fs::rename(&sibling, &file).map_err(|error| {
        unwritable(format!(
            "{} was not renamed over {shown}: {error}",
            sibling.display()
        ))
    })?;
    Ok(Trust {
        file: shown,
        directory: directory.to_owned(),
        written: true,
    })
}
