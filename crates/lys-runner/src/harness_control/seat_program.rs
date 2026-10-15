//! File identity is checked without interpreting a seat's contents.
const PROOF_VARIABLE: &str = "LYS_SEAT_PROGRAM_PROOF";
const SPEC_VARIABLE: &str = "LYS_SEAT_PROGRAM";
use crate::session::{Table, append};
use crate::tracking_store::{Body, Coverage};
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

pub(crate) fn record(
    table: &mut Table,
    id: &str,
    state: &str,
    value: &serde_json::Value,
) -> Result<(), RunnerError> {
    let words = serde_json::to_string(value)
        .map_err(|error| RunnerError::refused("seat_program_record_invalid", error.to_string()))?;
    append(
        table,
        id,
        vec![Body::Coverage(Coverage {
            state: state.to_owned(),
            words,
            source: None,
            generation: 0,
            offset: None,
            executable: value
                .get("executable")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            harness_version: None,
            adapter: Some("acp/1".to_owned()),
        })],
        None,
    )
}
