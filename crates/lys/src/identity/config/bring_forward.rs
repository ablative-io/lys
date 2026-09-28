//! Brings a deployment file an earlier build wrote forward to the shape this
//! build reads, in its own text so its comments and order stay as they were.
//!
//! An install keeps its `deployment.toml` across upgrades, so every change to
//! the file's shape is carried here as one step, applied once and written
//! back before anything reads the file.

use std::fs;
use std::io::Write;
use std::path::Path;

use toml::Table;

use super::refuse;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};

/// The compose network range an install is given when its file names none:
/// the range the install template writes.
pub const DEFAULT_NETWORK: &str = "172.29.47.0/24";

const APP_CLIENT_HEADER: &str = "[clients.app]";
const DEPLOYMENT_HEADER: &str = "[deployment]";

/// The text brought forward, or `None` when it is already in this build's
/// shape.
pub fn bring_forward(text: &str) -> IdentityResult<Option<String>> {
    let table: Table = toml::from_str(text)
        .map_err(|error| refuse(ErrorKind::ConfigInvalid, "configuration", error.message()))?;
    // An earlier build named the one app client beside the platform's after
    // the product it was installed for; this build names it `app`.
    let earlier_app_client = table
        .get("clients")
        .and_then(toml::Value::as_table)
        .filter(|clients| !clients.contains_key("app"))
        .and_then(|clients| {
            let mut others = clients.keys().filter(|name| *name != "platform");
            match (others.next(), others.next()) {
                (Some(name), None) => Some(format!("[clients.{name}]")),
                _ => None,
            }
        });
    let names_network = table
        .get("deployment")
        .and_then(toml::Value::as_table)
        .is_none_or(|deployment| deployment.contains_key("network"));
    if earlier_app_client.is_none() && names_network {
        return Ok(None);
    }
    let mut lines = Vec::new();
    for line in text.lines() {
        let header = line.trim();
        if earlier_app_client.as_deref() == Some(header) {
            lines.push(APP_CLIENT_HEADER.to_string());
            continue;
        }
        lines.push(line.to_string());
        if !names_network && header == DEPLOYMENT_HEADER {
            lines.push(format!("network = \"{DEFAULT_NETWORK}\""));
        }
    }
    let mut brought = lines.join("\n");
    if text.ends_with('\n') {
        brought.push('\n');
    }
    if brought == text {
        return Err(refuse(
            ErrorKind::ConfigInvalid,
            "configuration",
            "an earlier build's clients or deployment table is written in a form this build cannot bring forward",
        ));
    }
    Ok(Some(brought))
}

/// Replaces the file at `path` with `text` durably, keeping its permissions:
/// the whole new text is synced beside it and renamed over it, so a crash
/// leaves either the earlier file or the brought-forward one.
pub fn write_back(path: &Path, text: &str) -> IdentityResult<()> {
    let io = |operation: &'static str, at: &Path, error: &std::io::Error| {
        IdentityError::new(
            ErrorKind::PrivateFileIo,
            operation,
            "configuration",
            error.to_string(),
        )
        .at(at)
    };
    let permissions = fs::metadata(path)
        .map_err(|error| io("read permissions", path, &error))?
        .permissions();
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".forward");
    let partial = path.with_file_name(name);
    let mut file = fs::File::create(&partial).map_err(|error| io("create", &partial, &error))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.set_permissions(permissions))
        .and_then(|()| file.sync_all())
        .map_err(|error| io("write", &partial, &error))?;
    drop(file);
    fs::rename(&partial, path).map_err(|error| io("rename", path, &error))?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io("sync directory", parent, &error))?;
    }
    Ok(())
}
