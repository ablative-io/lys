//! The installer supplies a restart estimate before sending the service its stop signal.

use super::super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::super::install::services;
use super::super::super::private_files;
use super::super::Unit;
use std::path::Path;

pub(crate) fn announce(units: &[Unit], restarting: bool) -> IdentityResult<()> {
    if !restarting {
        return Ok(());
    }
    for unit in units {
        if unit.binary == "lys-identity-server" && services::alive(&unit.pid) {
            let config = unit.args.first().ok_or_else(|| {
                failed("the running identity service has no configuration argument")
            })?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| failed(error.to_string()))?;
            let now = u64::try_from(now.as_millis()).map_err(|error| failed(error.to_string()))?;
            record(Path::new(config), now.saturating_add(30_000))?;
        }
    }
    Ok(())
}

fn failed(reason: impl Into<String>) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        "announce planned return",
        "identity service",
        reason.into(),
    )
}

fn record(config: &Path, expected_return: u64) -> IdentityResult<()> {
    private_files::write(
        &config.with_extension("expected-return"),
        expected_return.to_string().as_bytes(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planned_return_is_durable_and_owner_only_before_the_stop_signal()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir()?;
        let config = root.path().join("identity.json");
        record(&config, 123_000)?;
        let plan = config.with_extension("expected-return");
        assert_eq!(std::fs::read_to_string(&plan)?, "123000");
        assert_eq!(
            std::fs::metadata(&plan)?.permissions().mode() & 0o777,
            0o600
        );
        Ok(())
    }
}
