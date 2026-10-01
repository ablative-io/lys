//! A run pass is added only to the generated, session-owned native config.

use std::collections::BTreeMap;
use std::fmt;

use hyper::Uri;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::rendering_launch::File;
use crate::record::blocks::Hash;

/// The service's MCP address and the pass issued for this run.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LysMcp {
    /// The complete HTTP MCP endpoint.
    pub url: String,
    /// The opaque run pass, never a template or environment setting.
    pub pass: String,
}

impl fmt::Debug for LysMcp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LysMcp").finish_non_exhaustive()
    }
}

/// A named refusal that never includes config contents or the supplied pass.
#[derive(thiserror::Error)]
pub enum Refusal {
    /// The native template already declares the reserved server.
    #[error("LysMcpDuplicate: the config already names a lys server")]
    Duplicate,
    /// The address or pass cannot be carried as an HTTP request.
    #[error("LysMcpInvalid: invalid MCP endpoint or run pass")]
    Invalid,
    /// The signed root binding does not select exactly one native config.
    #[error("LysMcpConfigMissing: one native config root is required")]
    ConfigMissing,
    /// The native file is missing, repeated or cannot be read or encoded.
    #[error("LysMcpConfigInvalid: the native config cannot carry the entry")]
    ConfigInvalid,
    /// An invalid address, retained without displaying the supplied value.
    #[error("LysMcpInvalid: the endpoint is not an HTTP address")]
    Address(hyper::http::uri::InvalidUri),
    /// A native JSON codec failure, with contents kept out of diagnostics.
    #[error("LysMcpConfigInvalid: the native JSON config cannot be encoded or read")]
    Json(serde_json::Error),
    /// A native TOML parse failure, with contents kept out of diagnostics.
    #[error("LysMcpConfigInvalid: the native TOML config cannot be read")]
    Toml(toml::de::Error),
    /// A native TOML encoding failure, with contents kept out of diagnostics.
    #[error("LysMcpConfigInvalid: the native TOML config cannot be encoded")]
    Encoding(toml::ser::Error),
}

impl fmt::Debug for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Refusal {
    /// The stable refusal name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Duplicate => "LysMcpDuplicate",
            Self::Invalid | Self::Address(_) => "LysMcpInvalid",
            Self::ConfigMissing => "LysMcpConfigMissing",
            Self::ConfigInvalid | Self::Json(_) | Self::Toml(_) | Self::Encoding(_) => {
                "LysMcpConfigInvalid"
            }
        }
    }
}

fn check(entry: &LysMcp) -> Result<(), Refusal> {
    let uri: Uri = entry.url.parse().map_err(Refusal::Address)?;
    if !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.host().is_none_or(str::is_empty)
        || uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'))
        || uri.path() != "/api/mcp"
        || uri.query().is_some()
        || entry.url.contains('#')
        || entry.pass.is_empty()
        || !entry.pass.bytes().all(|byte| (33..=126).contains(&byte))
    {
        return Err(Refusal::Invalid);
    }
    Ok(())
}

fn claude(text: &str, entry: &LysMcp) -> Result<String, Refusal> {
    let mut root: serde_json::Value = serde_json::from_str(text).map_err(Refusal::Json)?;
    let root = root.as_object_mut().ok_or(Refusal::ConfigInvalid)?;
    let servers = root.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = servers.as_object_mut().ok_or(Refusal::ConfigInvalid)?;
    if servers.contains_key("lys") {
        return Err(Refusal::Duplicate);
    }
    servers.insert(
        "lys".to_owned(),
        json!({
            "type": "http", "url": entry.url,
            "headers": {"lys-agent-pass": entry.pass}
        }),
    );
    serde_json::to_string_pretty(root)
        .map(|encoded| encoded + "\n")
        .map_err(Refusal::Json)
}

fn codex(text: &str, entry: &LysMcp) -> Result<String, Refusal> {
    let mut root: toml::Value = toml::from_str(text).map_err(Refusal::Toml)?;
    let root = root.as_table_mut().ok_or(Refusal::ConfigInvalid)?;
    let servers = root
        .entry("mcp_servers")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let servers = servers.as_table_mut().ok_or(Refusal::ConfigInvalid)?;
    if servers.contains_key("lys") {
        return Err(Refusal::Duplicate);
    }
    servers.insert(
        "lys".to_owned(),
        toml::Value::Table(toml::Table::from_iter([
            ("url".to_owned(), toml::Value::String(entry.url.clone())),
            (
                "http_headers".to_owned(),
                toml::Value::Table(toml::Table::from_iter([(
                    "lys-agent-pass".to_owned(),
                    toml::Value::String(entry.pass.clone()),
                )])),
            ),
        ])),
    );
    toml::to_string(root).map_err(Refusal::Encoding)
}

/// Add the reserved entry to the single config selected by its native root.
/// Other files and process inputs remain untouched; errors change no file.
///
/// # Errors
/// Refuses invalid endpoints, passes, roots, config shapes or a duplicate lys.
pub fn render(
    files: &mut [File],
    roots: &BTreeMap<String, String>,
    entry: &LysMcp,
) -> Result<(), Refusal> {
    check(entry)?;
    let (path, claude_code) = match (
        roots.get("CLAUDE_CONFIG_DIR").map(String::as_str),
        roots.get("CODEX_HOME").map(String::as_str),
    ) {
        (Some(""), None) => ("mcp.json", true),
        (None, Some("")) => ("config.toml", false),
        _ => return Err(Refusal::ConfigMissing),
    };
    let mut selected = files.iter_mut().filter(|file| file.path == path);
    let file = selected.next().ok_or(Refusal::ConfigInvalid)?;
    if selected.next().is_some() {
        return Err(Refusal::ConfigInvalid);
    }
    let text = if claude_code {
        claude(&file.text, entry)?
    } else {
        codex(&file.text, entry)?
    };
    Hash::of(text.as_bytes())
        .as_str()
        .clone_into(&mut file.sha256);
    file.text = text;
    Ok(())
}
