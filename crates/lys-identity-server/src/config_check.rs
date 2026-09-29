//! Structural configuration check; never opens stores or starts the service.

use std::ffi::OsString;
use std::io::Read;
use std::process::ExitCode;

use lys_identity_server::Config;
use lys_identity_server::read_api::BUILD;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

fn check(mut args: impl Iterator<Item = OsString>) -> Result<serde_json::Value, &'static str> {
    let path = args.next().ok_or("usage: --check-config <config.json|->")?;
    if args.next().is_some() {
        return Err("usage: --check-config <config.json|->");
    }
    let mut bytes = Zeroizing::new(Vec::new());
    if path == "-" {
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|_error| "config_check_read_failed")?;
    } else {
        *bytes = std::fs::read(path).map_err(|_error| "config_check_read_failed")?;
    }
    // Serde errors can quote submitted values. Keep diagnostics independent
    // of the input, including unknown keys, paths and credentials.
    let config: Config = serde_json::from_slice(&bytes).map_err(
        |_error| "config_check_schema_refused: unsupported, missing or mistyped configuration",
    )?;
    config.validate().map_err(
        |_error| "config_check_validation_refused: configuration constraints were not met",
    )?;
    Ok(serde_json::json!({
        "format": "lys-config-check/1",
        "build": BUILD,
        "config_sha256": format!("{:x}", Sha256::digest(bytes.as_slice())),
    }))
}

pub(super) fn run(args: impl Iterator<Item = OsString>) -> ExitCode {
    match check(args) {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::FAILURE
        }
    }
}
