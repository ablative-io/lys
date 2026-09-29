//! Identify the measured Codex package by bytes, never its version banner.
//! Identity is not an enforcement receipt: native config, event-schema and
//! containment probes must still succeed for the admitted session.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::hex;

/// The source commit named by the measured installed package.
pub const SOURCE_COMMIT: &str = "bd3798faee33820aad0044ed732841261d2bbe15";
/// Version of the source-grounded adapter, independent of Codex's banner.
pub const ADAPTER_VERSION: &str = "lys-codex-policy/v1";
/// The target of the artifact whose bytes have been measured.
pub const MEASURED_TARGET: &str = "aarch64-apple-darwin";

const EXECUTABLE_SHA256: &str = "4957a969886dce09ecf07ae03b1235fe4fdc57f16d9bd0ccb592c13a906a154f";
const MANIFEST_SHA256: &str = "919e151c9c378c5a6714ae093d83364248fdde31fdddc65977c4f41a56f61a95";

/// Bytes read from a package matched the adapter's trusted artifact record.
/// This type cannot be deserialized or constructed from caller claims.
/// It does not assert that a process was launched, or any policy enforced.
#[derive(Debug, Clone, Serialize)]
pub struct PackageIdentity {
    executable: PathBuf,
    executable_sha256: String,
    manifest: PathBuf,
    manifest_sha256: String,
    source_commit: &'static str,
    target: &'static str,
    adapter: &'static str,
}

impl PackageIdentity {
    /// The actual canonical executable path whose bytes were read.
    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

fn unsupported(path: &Path, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(
        "codex_policy_contract_unsupported",
        format!("{}: {reason}", path.display()),
    )
}

fn digest(path: &Path) -> Result<String, RunnerError> {
    let file = File::open(path).map_err(|error| unsupported(path, error))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    loop {
        let bytes = reader
            .fill_buf()
            .map_err(|error| unsupported(path, error))?;
        if bytes.is_empty() {
            break;
        }
        digest.update(bytes);
        let length = bytes.len();
        reader.consume(length);
    }
    Ok(hex(&digest.finalize()))
}

/// Verify the package directory's entrypoint and manifest against the
/// measured trusted record. A similarly named binary, changed manifest or
/// another platform refuses by name. This neither executes nor replaces it.
/// The launch owner must keep these files outside agent-writable roots and
/// bind its effective-config and native-capability probes to this identity.
pub fn verify_package(package: &Path) -> Result<PackageIdentity, RunnerError> {
    let canonical = |relative: &str| {
        let path = package.join(relative);
        std::fs::canonicalize(&path).map_err(|error| unsupported(&path, error))
    };
    let manifest = canonical("codex-package.json")?;
    let manifest_sha256 = digest(&manifest)?;
    if manifest_sha256 != MANIFEST_SHA256 {
        return Err(unsupported(
            &manifest,
            "manifest differs from the trusted package record",
        ));
    }
    let executable = canonical("bin/codex")?;
    let executable_sha256 = digest(&executable)?;
    if executable_sha256 != EXECUTABLE_SHA256 {
        return Err(unsupported(
            &executable,
            "executable differs from the trusted package record",
        ));
    }
    Ok(PackageIdentity {
        executable,
        executable_sha256,
        manifest,
        manifest_sha256,
        source_commit: SOURCE_COMMIT,
        target: MEASURED_TARGET,
        adapter: ADAPTER_VERSION,
    })
}
