//! Open the public sign-in page with the platform browser.
use super::errors::source_refused;
use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;
use std::process::{Command, Stdio};

pub(super) fn browser(address: &str) -> CliResult<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(program)
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| {
            source_refused(
                "sign_in_browser_unavailable",
                "the browser could not be opened",
                error,
            )
        })?;
    if !status.success() {
        return Err(refused(
            "sign_in_browser_unavailable",
            "the browser opener refused the sign-in",
        ));
    }
    Ok(())
}
