//! Check original and rendered bytes with the incoming server before a new
//! upgrade stops anything. An old binary without this protocol refuses.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::render::RenderedFile;
use super::{ErrorKind, IdentityError, IdentityResult, Layout, refuse};

const SERVER: &str = "lys-identity-server";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format: String,
    build: String,
    config_sha256: String,
}

fn refused(stage: &str, reason: &'static str) -> IdentityError {
    refuse(
        ErrorKind::ConfigInvalid,
        "preflight configuration",
        stage,
        reason,
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> IdentityResult<Zeroizing<Vec<u8>>> {
    std::fs::read(path)
        .map(Zeroizing::new)
        .map_err(|_error| refused("installed", "configuration could not be read"))
}

fn check(program: &Path, build: &str, stage: &str, bytes: &[u8]) -> IdentityResult<()> {
    let mut child = Command::new(program)
        .args(["--check-config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // An older executable may echo inputs in errors. Never relay them.
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_error| {
            refused(
                stage,
                "incoming server configuration checker could not start",
            )
        })?;
    let sent = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(bytes).is_ok());
    // Reap even when the checker exits before consuming stdin.
    let output = child.wait_with_output().map_err(|_error| {
        refused(
            stage,
            "incoming server configuration checker did not finish",
        )
    })?;
    if !sent || !output.status.success() {
        return Err(refused(
            stage,
            "incoming server refused configuration or does not support --check-config; no new upgrade was started",
        ));
    }
    let receipt: Receipt = serde_json::from_slice(&output.stdout).map_err(|_error| {
        refused(
            stage,
            "incoming server returned no valid configuration receipt",
        )
    })?;
    if receipt.format != "lys-config-check/1"
        || receipt.build != build
        || receipt.config_sha256 != digest(bytes)
    {
        return Err(refused(
            stage,
            "configuration receipt does not match the incoming build and checked bytes",
        ));
    }
    Ok(())
}

pub(super) struct Original {
    digest: String,
    build: String,
}

impl Original {
    pub(super) fn check(
        layout: &Layout,
        from: &Path,
        builds: &BTreeMap<String, String>,
    ) -> IdentityResult<Self> {
        let build = builds
            .get(SERVER)
            .ok_or_else(|| refused("installed", "incoming server build is missing"))?;
        let bytes = read(&layout.service_config())?;
        check(&from.join(SERVER), build, "installed", &bytes)?;
        Ok(Self {
            digest: digest(&bytes),
            build: build.clone(),
        })
    }

    pub(super) fn candidate(
        &self,
        layout: &Layout,
        from: &Path,
        files: &[RenderedFile],
    ) -> IdentityResult<()> {
        let target = layout.service_config();
        let mut candidates = files.iter().filter(|file| file.target == target);
        let candidate = candidates
            .next()
            .ok_or_else(|| refused("rendered", "service configuration is missing"))?;
        if candidates.next().is_some() {
            return Err(refused("rendered", "service configuration is repeated"));
        }
        check(
            &from.join(SERVER),
            &self.build,
            "rendered",
            &candidate.bytes,
        )?;
        if digest(&read(&target)?) != self.digest {
            return Err(refused(
                "installed",
                "configuration changed during preflight; rerun against the current configuration",
            ));
        }
        Ok(())
    }
}
