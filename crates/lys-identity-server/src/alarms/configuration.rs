//! Explicit, backed-up upgrade of the organisation's alarm setting.

use super::super::alarms_state::{AlarmError, unavailable};
use std::io::Write;
use std::path::Path;

const INITIAL_STALL_SECONDS: u64 = 300;

fn checked(value: &serde_json::Value) -> Result<u64, AlarmError> {
    value
        .as_u64()
        .filter(|seconds| *seconds > 0 && seconds.checked_mul(1000).is_some())
        .ok_or_else(|| {
            AlarmError::Malformed(
                "alarms.stall_after_seconds must be a positive whole number that fits milliseconds"
                    .to_owned(),
            )
        })
}

/// Read the setting, recording a missing key with a backup and a log line.
///
/// # Errors
/// Refuses malformed settings and any failed durable upgrade.
pub fn load(path: &Path, say: &dyn Fn(&str)) -> Result<u64, AlarmError> {
    let parent = path
        .parent()
        .ok_or_else(|| unavailable("organisation configuration has no parent directory"))?;
    std::fs::create_dir_all(parent).map_err(unavailable)?;
    let old = match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(unavailable(error)),
    };
    let mut value: serde_json::Value = match &old {
        Some(bytes) => serde_json::from_slice(bytes).map_err(unavailable)?,
        None => serde_json::json!({}),
    };
    let object = value.as_object_mut().ok_or_else(|| {
        AlarmError::Malformed("organisation configuration must be an object".to_owned())
    })?;
    let alarms = object
        .entry("alarms")
        .or_insert_with(|| serde_json::json!({}));
    let alarms = alarms
        .as_object_mut()
        .ok_or_else(|| AlarmError::Malformed("alarms must be an object".to_owned()))?;
    if let Some(seconds) = alarms.get("stall_after_seconds") {
        return checked(seconds);
    }
    let backup = if let Some(bytes) = old {
        let mut file = tempfile::Builder::new()
            .prefix("configuration.before-alarms-")
            .suffix(".json")
            .tempfile_in(parent)
            .map_err(unavailable)?;
        file.write_all(&bytes).map_err(unavailable)?;
        file.as_file().sync_all().map_err(unavailable)?;
        let (_, backup) = file.keep().map_err(unavailable)?;
        Some(backup)
    } else {
        None
    };
    alarms.insert(
        "stall_after_seconds".to_owned(),
        INITIAL_STALL_SECONDS.into(),
    );
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(unavailable)?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(unavailable)?;
    file.write_all(b"\n").map_err(unavailable)?;
    file.as_file().sync_all().map_err(unavailable)?;
    file.persist(path).map_err(unavailable)?;
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(unavailable)?;
    say(&format!(
        "organisation configuration: wrote alarms.stall_after_seconds={INITIAL_STALL_SECONDS} to {}; backup {}",
        path.display(),
        backup.as_ref().map_or_else(
            || "not needed: new file".to_owned(),
            |path| path.display().to_string()
        )
    ));
    Ok(INITIAL_STALL_SECONDS)
}
