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
use super::validate::network_gateway;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use crate::identity::install::layout::SERVICE_PORT;

/// The compose network range an install is given when its file names none:
/// the range the install template writes.
pub const DEFAULT_NETWORK: &str = "172.29.47.0/24";

const APP_CLIENT_HEADER: &str = "[clients.app]";
const DEPLOYMENT_HEADER: &str = "[deployment]";
const ISSUER_HEADER: &str = "[issuer]";

/// Whether the issuer table is the shape an install wrote before the
/// sign-in service's public address became Lys's own origin: its public
/// origin is the sign-in service's own loopback port. Such a sign-in
/// service names itself, and sends a person back from a provider, on a
/// port Lys does not serve, so a provider sign-in never finishes through Lys.
fn earlier_origin(table: &Table) -> bool {
    let Some(issuer) = table.get("issuer").and_then(toml::Value::as_table) else {
        return false;
    };
    let (Some(origin), Some(port)) = (
        issuer.get("public_origin").and_then(toml::Value::as_str),
        issuer.get("listen_port").and_then(toml::Value::as_integer),
    ) else {
        return false;
    };
    let origin = origin.trim_end_matches('/');
    origin == format!("http://localhost:{port}") || origin == format!("http://127.0.0.1:{port}")
}

/// The key a `key = value` line sets, if it is one.
fn key_of(line: &str) -> Option<&str> {
    line.split_once('=').map(|(key, _)| key.trim())
}

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
    let earlier_origin = earlier_origin(&table);
    if earlier_app_client.is_none() && names_network && !earlier_origin {
        return Ok(None);
    }
    // The sign-in service's gateway on the compose network is the one
    // address the directory service reaches it from, as the template names.
    let network = table
        .get("deployment")
        .and_then(toml::Value::as_table)
        .and_then(|deployment| deployment.get("network"))
        .and_then(toml::Value::as_str)
        .unwrap_or(DEFAULT_NETWORK);
    let gateway = network_gateway(network)?;
    let no_proxies = table
        .get("issuer")
        .and_then(toml::Value::as_table)
        .and_then(|issuer| issuer.get("trusted_proxies"))
        .and_then(toml::Value::as_array)
        .is_some_and(Vec::is_empty);
    let mut lines = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let header = line.trim();
        if header.starts_with('[') {
            header.clone_into(&mut section);
        }
        if earlier_app_client.as_deref() == Some(header) {
            lines.push(APP_CLIENT_HEADER.to_string());
            continue;
        }
        if earlier_origin && section == ISSUER_HEADER {
            match key_of(line) {
                Some("public_origin") => {
                    lines.push(format!(
                        "public_origin = \"http://localhost:{SERVICE_PORT}\""
                    ));
                    continue;
                }
                Some("trusted_proxies") if no_proxies => {
                    lines.push(format!("trusted_proxies = [\"{gateway}/32\"]"));
                    continue;
                }
                _ => {}
            }
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
/// leaves either the earlier file or the brought-forward one. The file it
/// replaces is kept beside it first, named for when it was replaced.
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
    let mut kept = path.file_name().unwrap_or_default().to_os_string();
    let replaced_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    kept.push(format!(".before-{replaced_at}"));
    let kept = path.with_file_name(kept);
    fs::copy(path, &kept).map_err(|error| io("keep", &kept, &error))?;
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
