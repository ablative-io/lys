//! Import uses an independently owned service-account credential, read from
//! an owner-only file. It sends one request, never retries an uncertain
//! mutation, never follows a redirect and never sends a bearer off loopback.

use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use zeroize::Zeroizing;

use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::install::layout::Layout;
use super::loopback_http::{Authority, Failure, Request, exchange};
use super::private_files;

fn refused(kind: ErrorKind, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(kind, "import", "identity directory", detail)
}

/// Where install and upgrade place the loader's independent bearer.
pub fn credential_path(layout: &Layout) -> PathBuf {
    layout.root.join("keys").join("identity-loader.credential")
}

/// Install and upgrade create this private bearer once. An existing file
/// is validated and reused, and exclusive creation never overwrites a key
/// made by another process. An incomplete write is a named refusal.
#[cfg(unix)]
pub fn prepare_credential(layout: &Layout) -> IdentityResult<()> {
    use rand::RngCore;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let path = credential_path(layout);
    if let Some(bytes) = private_files::read(&path)? {
        credential(&bytes)?;
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| refused(ErrorKind::ImportInvalid, "credential path has no parent"))?;
    private_files::ensure_dir(parent)?;
    let operation = lys_identity::OperationId::generate().map_err(|_| {
        refused(
            ErrorKind::ImportUnavailable,
            "secure random source unavailable",
        )
    })?;
    let mut secret = Zeroizing::new([0; 32]);
    rand::rng().fill_bytes(secret.as_mut());
    let mut text = Zeroizing::new(format!("lys-registrar.{operation}."));
    let digits = b"0123456789abcdef";
    for byte in secret.iter() {
        text.push(char::from(digits[usize::from(byte >> 4)]));
        text.push(char::from(digits[usize::from(byte & 15)]));
    }
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let bytes = private_files::read(&path)?.ok_or_else(|| {
                refused(
                    ErrorKind::ImportUnavailable,
                    "concurrent credential creation did not leave a file",
                )
            })?;
            credential(&bytes)?;
            return Ok(());
        }
        Err(_) => {
            return Err(refused(
                ErrorKind::PrivateFileIo,
                "cannot exclusively create the loader credential",
            )
            .at(&path));
        }
    };
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|_| {
            refused(
                ErrorKind::PrivateFileIo,
                "cannot durably write the loader credential",
            )
            .at(&path)
        })?;
    fs::File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|_| {
            refused(
                ErrorKind::PrivateFileIo,
                "cannot sync the credential directory",
            )
            .at(parent)
        })?;
    Ok(())
}

/// Private credential preparation requires filesystem mode enforcement.
#[cfg(not(unix))]
pub fn prepare_credential(layout: &Layout) -> IdentityResult<()> {
    Err(refused(
        ErrorKind::PrivateFileModeOpen,
        "loader credentials require owner-only file modes",
    )
    .at(&credential_path(layout)))
}

fn credential(bytes: &[u8]) -> IdentityResult<&str> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| refused(ErrorKind::ImportInvalid, "credential is not text"))?
        .trim();
    let mut parts = text.split('.');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("lys-registrar"), Some(account), Some(secret), None)
            if lys_identity::ServiceAccountId::from_str(account).is_ok()
                && secret.len() == 64
                && secret.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok(text)
        }
        _ => Err(refused(
            ErrorKind::ImportInvalid,
            "credential is not a service-account registrar bearer",
        )),
    }
}

fn authority(address: &str) -> IdentityResult<Authority> {
    let authority = Authority::parse(address, None).map_err(|_| {
        refused(
            ErrorKind::ImportInvalid,
            "expected a numeric loopback address and port",
        )
    })?;
    if !authority
        .host
        .parse::<IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
    {
        return Err(refused(
            ErrorKind::ImportInvalid,
            "import credentials may be sent only to a numeric loopback address",
        ));
    }
    Ok(authority)
}

/// Use the same installation configuration that starts the server. A surface
/// nests routes under /api; a headless service serves them at the root. Select
/// this before sending anything, never probe by retrying a mutation.
fn endpoint(layout: &Layout, address: Option<&str>) -> IdentityResult<(Authority, &'static str)> {
    let path = layout.service_config();
    let bytes = fs::read(&path).map_err(|_| {
        refused(
            ErrorKind::ConfigUnreadable,
            "cannot read the installed identity service configuration",
        )
        .at(&path)
    })?;
    let config: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
        refused(
            ErrorKind::ConfigInvalid,
            "the installed identity service configuration is not JSON",
        )
        .at(&path)
    })?;
    let listen = address
        .or_else(|| config.get("listen").and_then(serde_json::Value::as_str))
        .ok_or_else(|| {
            refused(
                ErrorKind::ConfigInvalid,
                "the installed service has no listen address",
            )
            .at(&path)
        })?;
    let route = match config.get("surface_dir") {
        None | Some(serde_json::Value::Null) => "/identity/import",
        Some(serde_json::Value::String(_)) => "/api/identity/import",
        Some(_) => {
            return Err(refused(
                ErrorKind::ConfigInvalid,
                "surface_dir is not a directory path",
            )
            .at(&path));
        }
    };
    Ok((authority(listen)?, route))
}

/// Execute the CLI's one import request and preserve a named failure.
pub fn run(
    file: &Path,
    root: Option<PathBuf>,
    credential_file: Option<PathBuf>,
    address: Option<&str>,
    json: bool,
) -> IdentityResult<()> {
    let layout = match root {
        Some(root) => Layout::at(root),
        None => Layout::discover()?,
    };
    let (authority, route) = endpoint(&layout, address)?;
    let path = credential_file.unwrap_or_else(|| credential_path(&layout));
    let bytes = private_files::read(&path)?.ok_or_else(|| {
        refused(
            ErrorKind::ImportInvalid,
            "the service-account credential file is absent",
        )
        .at(&path)
    })?;
    let credential = credential(&bytes)?;
    let account = credential.split('.').nth(1).ok_or_else(|| {
        refused(
            ErrorKind::ImportInvalid,
            "credential has no service account",
        )
    })?;
    let document = fs::read(file).map_err(|error| {
        refused(
            ErrorKind::ImportInvalid,
            format!("cannot read document: {error}"),
        )
        .at(file)
    })?;
    let expected = lys_identity::import_document::parse(&document, account)
        .map_err(|error| refused(ErrorKind::ImportInvalid, error.to_string()).at(file))?;
    let bearer = Zeroizing::new(format!("Bearer {credential}"));
    let headers = [
        ("Authorization", bearer.as_bytes()),
        ("Content-Type", b"application/json".as_slice()),
    ];
    let answer = exchange(&authority, &Request { method: "POST", path: route, headers: &headers, body: &document })
        .map_err(|failure| match failure {
            Failure::Unreachable(_) => refused(ErrorKind::ImportUnavailable, "the loopback service could not be reached; nothing was sent"),
            Failure::Uncertain(_) | Failure::Malformed(_) => refused(ErrorKind::ImportUncertain,
                "no complete answer; no retry was made. Reconcile the content-derived operations before explicitly repeating this document"),
        })?;
    let body: serde_json::Value = serde_json::from_slice(&answer.body).map_err(|_| {
        refused(
            ErrorKind::ImportUncertain,
            "the service answered without an import receipt; no retry was made",
        )
    })?;
    if answer.status != 200 {
        if json {
            println!("{body}");
        }
        let entry = body
            .get("entry")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("document");
        let name = body
            .get("refusal")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unrecognised_refusal");
        let reason = body
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("the service supplied no reason");
        return Err(refused(
            if answer.status >= 500 {
                ErrorKind::ImportUncertain
            } else {
                ErrorKind::ImportRefused
            },
            format!(
                "{entry}: {name}: {reason}; HTTP {}; no retry was made",
                answer.status
            ),
        ));
    }
    verify_receipts(&expected, account, &body)?;
    if json {
        println!("{body}");
    } else if let Some(completed) = body["completed"].as_array() {
        for entry in completed {
            println!(
                "{} {}",
                entry["entry"].as_str().unwrap_or("entry"),
                entry["operation"].as_str().unwrap_or("operation absent")
            );
        }
    }
    Ok(())
}

fn verify_receipts(
    expected: &[lys_identity::import_document::Entry],
    account: &str,
    body: &serde_json::Value,
) -> IdentityResult<()> {
    let completed = body
        .get("completed")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            refused(
                ErrorKind::ImportUncertain,
                "the response has no completed-entry receipts",
            )
        })?;
    let matches = body["by"]["kind"] == "service_account"
        && body["by"]["id"] == account
        && completed.len() == expected.len()
        && completed.iter().zip(expected).all(|(receipt, entry)| {
            receipt["operation"] == entry.operation.to_string()
                && receipt["entry"] == format!("{}/{}", entry.kind.section(), entry.name)
                && receipt
                    .get("result")
                    .is_some_and(serde_json::Value::is_object)
        });
    if !matches {
        return Err(refused(
            ErrorKind::ImportUncertain,
            "the receipts do not name this account and every requested operation",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
