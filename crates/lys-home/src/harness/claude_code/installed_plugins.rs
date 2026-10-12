//! The plugins a Claude Code config directory has installed, read from the
//! `plugins/installed_plugins.json` the program itself keeps there, so a run
//! that turns on a plugin the directory lacks is refused by name before it
//! starts rather than starting without it.

use std::collections::BTreeSet;
use std::path::Path;

use crate::error::HomeError;

/// The file under a config directory naming its installed plugins.
pub const INSTALLED_FILE: &str = "plugins/installed_plugins.json";

/// The `name@marketplace` keys `config_dir` has installed. A directory with no
/// such file has none installed.
///
/// # Errors
///
/// [`HomeError::Io`] when the file exists but cannot be read, and
/// [`HomeError::Json`] when it is not JSON, and
/// [`HomeError::PluginsFileMalformed`] when it carries no `plugins` object.
pub fn installed(config_dir: &Path) -> Result<BTreeSet<String>, HomeError> {
    let path = config_dir.join(INSTALLED_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeSet::new());
        }
        Err(error) => {
            return Err(HomeError::io("reading the installed plugins", &path, error));
        }
    };
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|source| HomeError::Json {
            context: "the installed plugins file is not JSON",
            source,
        })?;
    value
        .get("plugins")
        .and_then(serde_json::Value::as_object)
        .map(|plugins| plugins.keys().cloned().collect())
        .ok_or(HomeError::PluginsFileMalformed { path })
}

/// Refuses the first of `named` that `config_dir` has not installed.
///
/// # Errors
///
/// [`HomeError::PluginNotInstalled`] naming the plugin and the directory, and
/// [`installed`]'s refusals.
pub fn require(config_dir: &Path, named: &[String]) -> Result<(), HomeError> {
    if named.is_empty() {
        return Ok(());
    }
    let have = installed(config_dir)?;
    match named.iter().find(|name| !have.contains(*name)) {
        Some(name) => Err(HomeError::PluginNotInstalled {
            name: name.clone(),
            config_dir: config_dir.to_path_buf(),
        }),
        None => Ok(()),
    }
}
