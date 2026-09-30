//! Signed config contents are published together in a session-owned directory.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::{Launch, hex};

/// One signed config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// Its relative location.
    pub path: String,
    /// Its exact contents.
    pub text: String,
    /// Its lowercase content digest.
    pub sha256: String,
}

/// Config files and the process inputs bound to their locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// All files supplied by the renderer.
    pub files: Vec<File>,
    /// Argument positions bound to the named files.
    pub argument_files: BTreeMap<usize, String>,
    /// Variables bound to files, or to the root when the value is empty.
    pub environment_paths: BTreeMap<String, String>,
    /// Whether the process starts in the config directory.
    pub working_directory: bool,
}

fn refused(reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("launch_config_refused", reason.to_string())
}

fn relative(path: &str) -> bool {
    !path.is_empty()
        && !path.chars().any(char::is_control)
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn checked(config: &Config, arguments: usize) -> Result<(), RunnerError> {
    if config.files.is_empty() {
        return Err(refused("the config carries no files"));
    }
    let mut paths = BTreeSet::new();
    for file in &config.files {
        if !relative(&file.path) || !paths.insert(file.path.as_str()) {
            return Err(refused(format!(
                "invalid or repeated config path `{}`",
                file.path
            )));
        }
        if hex(&Sha256::digest(file.text.as_bytes())) != file.sha256 {
            return Err(refused(format!(
                "config file `{}` does not hash to its digest",
                file.path
            )));
        }
    }
    for path in &paths {
        if Path::new(path)
            .ancestors()
            .skip(1)
            .any(|parent| parent.to_str().is_some_and(|p| paths.contains(p)))
        {
            return Err(refused(format!(
                "config file `{path}` stands above another file"
            )));
        }
    }
    for (index, path) in &config.argument_files {
        if *index >= arguments || !paths.contains(path.as_str()) {
            return Err(refused(format!(
                "argument {index} names no supplied config file `{path}`"
            )));
        }
    }
    for (name, path) in &config.environment_paths {
        if name.is_empty()
            || name.starts_with(|c: char| c.is_ascii_digit())
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || !(path.is_empty() || paths.contains(path.as_str()))
        {
            return Err(refused(format!(
                "invalid config environment binding `{name}`"
            )));
        }
    }
    Ok(())
}

fn directory(path: &Path) -> Result<(), RunnerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(refused(format!(
            "{} is not an owned directory",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(refused)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(refused)
        }
        Err(error) => Err(refused(error)),
    }
}

fn write(staged: &Path, config: &Config) -> Result<(), RunnerError> {
    for file in &config.files {
        let path = staged.join(&file.path);
        let parent = path
            .parent()
            .ok_or_else(|| refused("a config file has no parent"))?;
        let relative_parent = parent.strip_prefix(staged).map_err(refused)?;
        let mut dir = staged.to_owned();
        for part in relative_parent.components() {
            dir.push(part);
            directory(&dir)?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(refused)?;
        output.write_all(file.text.as_bytes()).map_err(refused)?;
        output.sync_all().map_err(refused)?;
    }
    Ok(())
}

fn identical(target: &Path, config: &Config) -> Result<(), RunnerError> {
    if !fs::symlink_metadata(target).map_err(refused)?.is_dir() {
        return Err(refused("the kept config is not an owned directory"));
    }
    for file in &config.files {
        let mut path = target.to_owned();
        let parts: Vec<_> = Path::new(&file.path).components().collect();
        for (index, part) in parts.iter().enumerate() {
            path.push(part);
            let metadata = fs::symlink_metadata(&path).map_err(refused)?;
            let last = index + 1 == parts.len();
            if (last && !metadata.is_file()) || (!last && !metadata.is_dir()) {
                return Err(refused(format!(
                    "{} is not an owned config entry",
                    path.display()
                )));
            }
        }
        if fs::read(&path).map_err(refused)? != file.text.as_bytes() {
            return Err(refused(format!(
                "{} differs from its signed contents",
                path.display()
            )));
        }
    }
    Ok(())
}

fn publish(state: &Path, session: &str, config: &Config) -> Result<PathBuf, RunnerError> {
    let sessions = state.join("sessions");
    directory(&sessions)?;
    let own = sessions.join(session);
    directory(&own)?;
    let target = own.join("config");
    match fs::symlink_metadata(&target) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(refused(error)),
        Ok(_) => {
            identical(&target, config)?;
            return Ok(target);
        }
    }
    let staged = own.join("config-writing");
    fs::create_dir(&staged).map_err(refused)?;
    let written = fs::set_permissions(&staged, fs::Permissions::from_mode(0o700))
        .map_err(refused)
        .and_then(|()| write(&staged, config));
    if let Err(error) = written {
        fs::remove_dir_all(&staged).map_err(|cleanup| {
            refused(format!("{error}; removing incomplete config: {cleanup}"))
        })?;
        return Err(error);
    }
    if let Err(error) = fs::rename(&staged, &target) {
        fs::remove_dir_all(&staged).map_err(|cleanup| {
            refused(format!("{error}; removing unpublished config: {cleanup}"))
        })?;
        return Err(refused(error));
    }
    fs::File::open(&own)
        .and_then(|dir| dir.sync_all())
        .map_err(refused)?;
    Ok(target)
}

/// Validate every input, publish all files, then bind process locations.
///
/// # Errors
/// Refuses invalid paths, digests, bindings, existing targets and write failures.
pub(crate) fn prepare(state: &Path, launch: &mut Launch) -> Result<(), RunnerError> {
    let Some(config) = launch.config.as_ref() else {
        return Ok(());
    };
    checked(config, launch.arguments.len())?;
    let dir = publish(state, &launch.session, config)?;
    for (index, path) in &config.argument_files {
        launch.arguments[*index] = dir
            .join(path)
            .to_str()
            .ok_or_else(|| refused("the config directory is not UTF-8"))?
            .to_owned();
    }
    for (name, path) in &config.environment_paths {
        launch.environment.insert(
            name.clone(),
            dir.join(path)
                .to_str()
                .ok_or_else(|| refused("the config directory is not UTF-8"))?
                .to_owned(),
        );
    }
    if config.working_directory {
        launch.directory = dir
            .to_str()
            .ok_or_else(|| refused("the config directory is not UTF-8"))?
            .to_owned();
    }
    Ok(())
}
