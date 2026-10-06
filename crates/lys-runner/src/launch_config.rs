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

pub use lys_home::harness::rendering_launch::File;

/// Config files and the process inputs bound to their locations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Whether this exact reviewed setup requires managed controls.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_controls: bool,
    /// All files supplied by the renderer.
    pub files: Vec<File>,
    /// Argument positions bound to the named files.
    #[serde(deserialize_with = "positions")]
    pub argument_files: BTreeMap<usize, String>,
    /// Variables bound to files, or to the root when the value is empty.
    pub environment_paths: BTreeMap<String, String>,
    /// Whether the process starts in the config directory.
    pub working_directory: bool,
    /// The harness the files are for, when the profile declares one. A Claude
    /// Code run's folder is recorded as trusted in the harness's own
    /// per-project configuration before the spawn, so no run sits at a
    /// trust prompt nobody sees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<crate::tracking::Harness>,
}

/// Argument positions as the wire carries them: JSON object keys are strings,
/// and inside the internally tagged `Act` a string key does not read as a
/// number, so each key is read as a string and then as a position.
fn positions<'de, D>(deserializer: D) -> Result<BTreeMap<usize, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    BTreeMap::<String, String>::deserialize(deserializer)?
        .into_iter()
        .map(|(key, path)| {
            key.parse::<usize>()
                .map(|index| (index, path))
                .map_err(|error| {
                    serde::de::Error::custom(format!(
                        "argument position `{key}` is not a position: {error}"
                    ))
                })
        })
        .collect()
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
    // A Codex run with no kept skills and no instructions file has no files.
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

/// Verify the supplied config before adding the run-only MCP entry.
pub(crate) fn add_lys_mcp(
    launch: &mut Launch,
    entry: &crate::protocol::LysMcp,
) -> Result<(), RunnerError> {
    let Launch {
        config,
        arguments,
        environment,
        ..
    } = launch;
    let config = config.as_mut().ok_or_else(|| {
        RunnerError::refused("LysMcpConfigMissing", "the launch carries no native config")
    })?;
    checked(config, arguments.len())?;
    lys_home::harness::lys_mcp::render(&mut config.files, arguments, environment, entry)
        .map_err(|error| RunnerError::refused(error.name(), error.to_string()))
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
    match fs::symlink_metadata(&staged) {
        Ok(metadata) if metadata.is_dir() => {
            fs::remove_dir_all(&staged)
                .map_err(|error| refused(format!("removing unpublished config: {error}")))?;
        }
        Ok(_) => return Err(refused("the unpublished config is not an owned directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(refused(error)),
    }
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
        let bound = dir
            .join(path)
            .to_str()
            .ok_or_else(|| refused("the config directory is not UTF-8"))?
            .to_owned();
        // A setting of the form key="file" keeps its key and quotes the bound
        // path as a TOML string; a bare file is replaced whole.
        let argument = &mut launch.arguments[*index];
        let quoted = format!("=\"{path}\"");
        *argument = match argument.strip_suffix(quoted.as_str()) {
            Some(key) => format!(
                "{key}={}",
                serde_json::to_string(&bound).map_err(|error| refused(error.to_string()))?
            ),
            None => bound,
        };
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
        dir.to_str()
            .ok_or_else(|| refused("the config directory is not UTF-8"))?
            .clone_into(&mut launch.directory);
    }
    Ok(())
}
