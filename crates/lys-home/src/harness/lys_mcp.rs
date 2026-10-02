//! A run pass is added only to the generated, session-owned native config.
//!
//! When Lys gives the run a seat, the runner signs the pass with the seat's
//! key as it writes the config: the `lys-seat` header carries the
//! certificate the seat speaks for, the seat's public key, the delegation the
//! AI's certificate key signed for it, and the seat's signature over the
//! pass. The seat's key itself is never written anywhere.

use std::collections::BTreeMap;
use std::fmt;

use hyper::Uri;
use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, sign_attestation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

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
    /// The run's seat, when Lys gave it one: the runner signs the pass with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<Seat>,
}

/// A key Lys made for one run and delegated to it from the AI's certificate.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    /// The agent the run is for.
    pub agent: String,
    /// The session the seat is for.
    pub session: String,
    /// The serial of the certificate whose key signed the delegation.
    pub serial: String,
    /// The end of the delegation, in seconds since the Unix epoch.
    pub not_after: u64,
    /// The delegation, a `COSE_Sign1` in hex over [`delegation_bytes`].
    pub delegation: String,
    /// The seat's private seed, in hex; held by the runner, never written.
    pub key: String,
}

impl fmt::Debug for Seat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Seat")
            .field("agent", &self.agent)
            .field("session", &self.session)
            .field("serial", &self.serial)
            .field("not_after", &self.not_after)
            .finish_non_exhaustive()
    }
}

/// The header a seated run's calls carry beside the pass.
pub const SEAT_HEADER: &str = "lys-seat";

/// The bytes the AI's certificate key signs to delegate to a seat.
pub fn delegation_bytes(
    agent: &str,
    serial: &str,
    session: &str,
    seat_public: &[u8; 32],
    not_after: u64,
) -> Vec<u8> {
    format!(
        "lys-identity/seat-delegation/v1\n{agent}\n{serial}\n{session}\n{}\n{not_after}",
        hex(seat_public)
    )
    .into_bytes()
}

/// The bytes a seat signs over its run's pass.
pub fn pass_bytes(agent: &str, session: &str, pass: &str) -> Vec<u8> {
    format!(
        "lys-identity/seat-pass/v1\n{agent}\n{session}\n{}",
        hex(&Sha256::digest(pass.as_bytes()))
    )
    .into_bytes()
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The bytes of lowercase or uppercase hex `text`; none when it is not hex.
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

/// The `lys-seat` header value: the runner signs the pass with the seat key.
/// A seat that names no agent, session or certificate, ends at zero, or whose
/// delegation is not a `COSE_Sign1`, is refused, so no runner writes a header
/// Lys would refuse.
///
/// # Errors
///
/// [`Refusal::Invalid`] when the seat cannot be carried.
pub fn seat_header(seat: &Seat, pass: &str) -> Result<String, Refusal> {
    let seed = Zeroizing::new(
        unhex(&seat.key)
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or(Refusal::Invalid)?,
    );
    let key = Ed25519Identity::from_seed(&seed);
    let named = |word: &str| !word.is_empty() && !word.contains(char::is_whitespace);
    if !named(&seat.agent)
        || !named(&seat.session)
        || !named(&seat.serial)
        || seat.not_after == 0
        || unhex(&seat.delegation).is_none_or(|bytes| Attestation::from_cose_bytes(&bytes).is_err())
    {
        return Err(Refusal::Invalid);
    }
    let signature = sign_attestation(&pass_bytes(&seat.agent, &seat.session, pass), &key);
    Ok(format!(
        "{} {} {} {} {}",
        seat.serial,
        seat.not_after,
        hex(&key.public_key_bytes()),
        seat.delegation,
        hex(&signature.to_cose_bytes())
    ))
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
    let mut headers = serde_json::Map::new();
    headers.insert("lys-agent-pass".to_owned(), json!(entry.pass));
    if let Some(seat) = &entry.seat {
        headers.insert(
            SEAT_HEADER.to_owned(),
            json!(seat_header(seat, &entry.pass)?),
        );
    }
    servers.insert(
        "lys".to_owned(),
        json!({"type": "http", "url": entry.url, "headers": headers}),
    );
    serde_json::to_string_pretty(root)
        .map(|encoded| encoded + "\n")
        .map_err(Refusal::Json)
}

fn codex_headers(entry: &LysMcp) -> Result<toml::Table, Refusal> {
    let mut headers = toml::Table::from_iter([(
        "lys-agent-pass".to_owned(),
        toml::Value::String(entry.pass.clone()),
    )]);
    if let Some(seat) = &entry.seat {
        headers.insert(
            SEAT_HEADER.to_owned(),
            toml::Value::String(seat_header(seat, &entry.pass)?),
        );
    }
    Ok(headers)
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
                toml::Value::Table(codex_headers(entry)?),
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
