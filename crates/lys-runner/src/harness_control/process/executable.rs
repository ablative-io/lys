//! Executable fingerprints and version probes name every failed read or child exit.
use super::{Executable, RunnerError, failed, identity_cache};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Fingerprint the executable resolved for a launch, with bounded read buffers.
///
/// # Errors
/// Refuses missing, nonregular, nonexecutable or unreadable files.
pub fn executable(path: &Path) -> Result<Executable, RunnerError> {
    let path = path
        .canonicalize()
        .map_err(|error| failed("control_executable_unproved", error))?;
    let mut file =
        File::open(&path).map_err(|error| failed("control_executable_unproved", error))?;
    let metadata = file
        .metadata()
        .map_err(|error| failed("control_executable_unproved", error))?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(failed(
            "control_executable_unproved",
            "selected file is not an executable",
        ));
    }
    // An unchanged file's fingerprint is read back, never recomputed: a
    // harness binary is hundreds of megabytes (`identity_cache`).
    if let Some(sha256) = identity_cache::sha256(&path, &metadata) {
        return Ok(Executable {
            path: path
                .to_str()
                .ok_or_else(|| {
                    failed("control_executable_unproved", "executable path is not text")
                })?
                .to_owned(),
            sha256,
        });
    }
    let began = std::time::Instant::now();
    let mut hash = Sha256::new();
    let mut buffer = [0; 16384];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| failed("control_executable_unproved", error))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    let sha256 = crate::protocol::hex(&hash.finalize());
    crate::error::step("fingerprint_executable", began);
    identity_cache::remember_sha256(&path, &metadata, &sha256);
    Ok(Executable {
        path: path
            .to_str()
            .ok_or_else(|| failed("control_executable_unproved", "executable path is not text"))?
            .to_owned(),
        sha256,
    })
}

pub(super) fn report_version(
    program: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<(std::process::ExitStatus, Vec<u8>), RunnerError> {
    let mut child = Command::new(program)
        .arg("--version")
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| failed("control_adapter_unqualified", error))?;
    let read: Result<Vec<u8>, RunnerError> = (|| {
        let mut bytes = Vec::new();
        let mut stdout = child.stdout.take().ok_or_else(|| {
            failed(
                "control_adapter_unqualified",
                "version probe has no output pipe",
            )
        })?;
        stdout
            .read_to_end(&mut bytes)
            .map_err(|error| failed("control_adapter_unqualified", error))?;
        Ok(bytes)
    })();
    if read.is_err()
        && child
            .try_wait()
            .map_err(|error| failed("control_probe_cleanup_failed", error))?
            .is_none()
    {
        child
            .kill()
            .map_err(|error| failed("control_probe_cleanup_failed", error))?;
    }
    let status = child
        .wait()
        .map_err(|error| failed("control_probe_cleanup_failed", error))?;
    Ok((status, read?))
}
