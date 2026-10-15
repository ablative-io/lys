//! File identity is checked without interpreting a seat's contents.
use super::{PROOF_VARIABLE, SPEC_VARIABLE};
use crate::{Launch, RunnerError};
use lys_home::harness::seat_program::{FILE, SeatProgram};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

fn refused(name: &str, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(name, reason.to_string())
}

fn private(path: &str, name: &str) -> Result<File, RunnerError> {
    let file = File::open(path).map_err(|error| refused(name, format!("{path}: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| refused(name, format!("{path}: {error}")))?;
    let mode = metadata.permissions().mode() & 0o7777;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || mode & 0o077 != 0
    {
        return Err(refused(
            name,
            format!("{path}: mode {mode:04o}; the seat file must be 0600 and owned by this login"),
        ));
    }
    Ok(file)
}

fn digest(mut file: File, path: &str) -> Result<String, RunnerError> {
    let mut hash = Sha256::new();
    let mut buffer = [0; 16384];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| refused("seat_file_unreadable", format!("{path}: {error}")))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(crate::protocol::hex(&hash.finalize()))
}

pub(crate) fn prepare(launch: &mut Launch) -> Result<Option<Value>, RunnerError> {
    let metadata = launch
        .config
        .as_ref()
        .and_then(|config| config.files.iter().find(|file| file.path == FILE));
    let Some(metadata) = metadata else {
        if launch.environment.contains_key(SPEC_VARIABLE)
            || launch.environment.contains_key(PROOF_VARIABLE)
        {
            return Err(refused(
                "seat_program_metadata_unbound",
                "seat metadata requires the signed config file",
            ));
        }
        return Ok(None);
    };
    if lys_home::record::blocks::Hash::of(metadata.text.as_bytes()).as_str() != metadata.sha256 {
        return Err(refused(
            "seat_program_metadata_changed",
            "seat-program.json differs from its signed digest",
        ));
    }
    let spec: SeatProgram = serde_json::from_str(&metadata.text)
        .map_err(|error| refused("seat_program_invalid", error))?;
    if launch.program != spec.executable || launch.arguments != spec.argv() {
        return Err(refused(
            "seat_program_arguments_changed",
            "the process inputs differ from the reviewed seat_program",
        ));
    }
    if launch.rotation.is_some()
        || launch
            .config
            .as_ref()
            .is_some_and(|config| config.requires_controls)
    {
        return Err(refused(
            "seat_program_controls_unavailable",
            "seat-program/v1 owns model routing and does not provide native account rotation or context controls",
        ));
    }
    let seat_file = private(&spec.seat_file, "seat_file_not_private")?;
    let pass_file = launch.environment.get("LYS_AGENT_PASS").ok_or_else(|| {
        refused(
            "seat_pass_missing",
            "LYS_AGENT_PASS must name the run's owner-only proof file",
        )
    })?;
    drop(private(pass_file, "seat_pass_not_private")?);
    for name in [
        "LYS_RUN",
        "LYS_AGENT",
        "LYS_MACHINE",
        "LYS_RUNNER_SOCKET",
        "LYS_PROVISIONING_VERSION",
    ] {
        if launch.environment.get(name).is_none_or(String::is_empty) {
            return Err(refused(
                "seat_identity_missing",
                format!("the run has no {name}"),
            ));
        }
    }
    let run = launch
        .environment
        .get("LYS_RUN")
        .ok_or_else(|| refused("seat_identity_missing", "LYS_RUN is missing"))?;
    if !lys_home::proxy::usage::is_run_key(run) {
        return Err(refused("seat_identity_invalid", "LYS_RUN is not a run key"));
    }
    for name in ["ANTHROPIC_BASE_URL", "OPENAI_BASE_URL"] {
        if !launch.environment.get(name).is_some_and(|url| {
            (url.starts_with("http://") || url.starts_with("https://"))
                && url.contains(&format!("/{run}/"))
        }) {
            return Err(refused(
                "seat_proxy_unbound",
                format!("{name} does not carry this run's proxy key"),
            ));
        }
    }
    let executable =
        crate::harness_control::process::executable(std::path::Path::new(&spec.executable))?;
    let found_seat = digest(seat_file, &spec.seat_file)?;
    let proof = json!({"contract":"seat-program/v1", "executable":spec.executable, "seat_file":spec.seat_file,
        "reviewed_executable_sha256":spec.executable_sha256, "found_executable_sha256":executable.sha256,
        "reviewed_seat_file_sha256":spec.seat_file_sha256, "found_seat_file_sha256":found_seat});
    let metadata = metadata.text.clone();
    launch
        .environment
        .insert(SPEC_VARIABLE.to_owned(), metadata);
    launch.environment.insert(
        PROOF_VARIABLE.to_owned(),
        serde_json::to_string(&proof)
            .map_err(|error| refused("seat_program_record_invalid", error))?,
    );
    launch
        .environment
        .insert("LYS_SESSION".to_owned(), launch.session.clone());
    Ok(Some(proof))
}
