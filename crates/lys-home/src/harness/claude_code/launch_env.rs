//! The environment file of a launch (HOME-002 R6): a Claude Code settings
//! file whose only member is `env`, holding each of the template's variables
//! with its value and each use-only secret's variable with its handle. No
//! secret's value is read, the process environment is not read, and a
//! readable secret never reaches here (the parser refuses it). Keys are
//! written in sorted order, so one template writes one sequence of bytes.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde_json::json;

use crate::error::HomeError;
use crate::harness::claude_code::template::Template;
use crate::record::blocks::Hash;

/// The settings file's bytes: `{"env": {...}}`, keys sorted, one trailing
/// newline.
pub fn env_settings(template: &Template) -> Result<Vec<u8>, HomeError> {
    let mut env: BTreeMap<&str, &str> = template
        .env
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    for secret in &template.use_only {
        env.insert(secret.env.as_str(), secret.handle.as_str());
    }
    let mut bytes =
        serde_json::to_vec_pretty(&json!({ "env": env })).map_err(|source| HomeError::Json {
            context: "the environment file could not be serialised",
            source,
        })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Write the environment file at `path`, which must not exist.
pub fn write_env_file(template: &Template, path: &Path) -> Result<Hash, HomeError> {
    write_new(path, &env_settings(template)?)
}

/// Write a launch file exclusively (an existing path is refused by name),
/// sync it, and return the hash of the bytes written.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<Hash, HomeError> {
    let mut file = match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(HomeError::LaunchTargetExists {
                path: path.to_path_buf(),
            });
        }
        Err(e) => return Err(HomeError::io("creating a launch file", path, e)),
    };
    file.write_all(bytes)
        .map_err(|e| HomeError::io("writing a launch file", path, e))?;
    file.sync_all()
        .map_err(|e| HomeError::io("syncing a launch file", path, e))?;
    Ok(Hash::of(bytes))
}
