//! Lys's model proxy beside the service: `lys proxy serve` on loopback, the
//! address every model call of a Claude Code run Lys starts is sent to. It
//! forwards each call unchanged and records it in its own home under the
//! session the call names. It runs like the runner: its own Unit with a pid
//! file, an exit lock and a log, from the `lys` the install runs. Install
//! starts it when it is not running; upgrade stops it with the runner before
//! anything is placed and starts it again after the services, and a call it
//! held at the stop is recorded `lost` when it next starts.
//!
//! Its Anthropic upstream is recorded, never read from its environment: the
//! install and each upgrade ask the login shell for the login's own
//! `ANTHROPIC_BASE_URL`, as they ask it for `PATH`, and write what it answered
//! and where it came from to `upstream.json` beside the proxy's state. A
//! machine pointed at a gateway keeps it; a login changed later takes effect
//! at the next install or upgrade.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::private_files::{self, Outcome};
use super::super::upgrade::{Ready, Unit};
use super::layout::Layout;
use super::log_wait::{self, LogCursor};
use super::ports::Ports;
use super::services::{self, Environment};

/// The words the proxy's start line carries once it listens.
const LISTENING: &str = r#""proxy":"listening""#;

/// Anthropic's API, the upstream when the login names none.
pub const ANTHROPIC: &str = "https://api.anthropic.com";

/// What the login shell prints around the login's `ANTHROPIC_BASE_URL`, so
/// that anything a profile or a logout file prints is never taken for it.
const BASE_MARKER: &str = "LYS_LOGIN_ANTHROPIC_BASE_URL:";
const BASE_END: &str = ":LYS_LOGIN_ANTHROPIC_BASE_URL_END";

/// The proxy's Anthropic upstream as the install recorded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upstream {
    /// The base a call to `/anthropic` is forwarded to.
    pub anthropic: String,
    /// `login` when the login's own `ANTHROPIC_BASE_URL` named it, `default`
    /// when the login named none.
    pub from: String,
}

/// Where the recorded upstream is kept.
pub fn upstream_file(layout: &Layout) -> PathBuf {
    layout.data_dir().join("proxy").join("upstream.json")
}

fn unread(detail: impl Into<String>) -> IdentityError {
    IdentityError::new(
        ErrorKind::Unready,
        "read login",
        "ANTHROPIC_BASE_URL",
        detail,
    )
}

/// The login's own `ANTHROPIC_BASE_URL`, as its login shell answers it when
/// started with `environment` and nothing else; `None` when it names none
/// or names it empty.
pub fn login_base(environment: &Environment) -> IdentityResult<Option<String>> {
    let variables: Vec<(&str, &str)> = environment.variables().collect();
    let shell = variables
        .iter()
        .find(|(name, _)| *name == "SHELL")
        .map(|(_, value)| *value)
        .filter(|shell| !shell.is_empty())
        .ok_or_else(|| unread("SHELL is not set; start the install from a login"))?;
    let output = Command::new(shell)
        .env_clear()
        .envs(variables.iter().copied())
        .args([
            "-l",
            "-c",
            &format!(
                r#"printf '\n%s%s%s' '{BASE_MARKER}' "${{ANTHROPIC_BASE_URL-}}" '{BASE_END}'"#
            ),
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| unread(format!("login shell {shell} could not run: {error}")))?;
    if !output.status.success() {
        return Err(unread(format!(
            "login shell {shell} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let answer = String::from_utf8(output.stdout)
        .map_err(|error| unread(format!("login shell {shell} answered no text: {error}")))?;
    let base = answer
        .rsplit_once(BASE_MARKER)
        .and_then(|(_, after)| after.split_once(BASE_END))
        .map(|(base, _)| base)
        .ok_or_else(|| unread(format!("login shell {shell} did not answer it")))?;
    Ok((!base.is_empty()).then(|| base.to_owned()))
}

/// Whether the service's configuration in place names a model proxy.
pub fn configured(layout: &Layout) -> IdentityResult<bool> {
    let path = layout.service_config();
    let Some(bytes) = private_files::read(&path)? else {
        return Ok(false);
    };
    let config: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "read configuration",
            "identity.json",
            error.to_string(),
        )
        .at(&path)
    })?;
    Ok(config.get("model_proxy").is_some())
}

/// Records the proxy's upstream from the login, owner-only, and says where
/// it came from. `true` when the record changed, so a running proxy is
/// started again on it.
pub fn configure(
    layout: &Layout,
    environment: &Environment,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<bool> {
    let upstream = match login_base(environment)? {
        Some(base) => Upstream {
            anthropic: base,
            from: "login".to_owned(),
        },
        None => Upstream {
            anthropic: ANTHROPIC.to_owned(),
            from: "default".to_owned(),
        },
    };
    // Every prompt and reply of every run is kept here: owner-only, and an
    // existing folder others can read is refused by name, never used.
    let proxy = layout.data_dir().join("proxy");
    for folder in [proxy.clone(), proxy.join("home"), proxy.join("state")] {
        private_files::ensure_dir(&folder)?;
    }
    let file = upstream_file(layout);
    let bytes = serde_json::to_vec_pretty(&upstream).map_err(|error| {
        IdentityError::new(
            ErrorKind::RenderFailed,
            "render",
            "upstream.json",
            error.to_string(),
        )
    })?;
    let outcome = private_files::write(&file, &bytes)?;
    say(if upstream.from == "login" {
        "model proxy forwards to the login's own ANTHROPIC_BASE_URL"
    } else {
        "model proxy forwards to Anthropic's API; the login names no ANTHROPIC_BASE_URL"
    });
    Ok(outcome != Outcome::Unchanged)
}

/// The proxy as the install runs it, on the recorded listener.
pub fn unit(layout: &Layout, ports: Ports) -> Unit {
    let proxy = layout.data_dir().join("proxy");
    let args = [
        "proxy",
        "serve",
        "--listen",
        &format!("127.0.0.1:{}", ports.proxy),
        "--home",
        &proxy.join("home").display().to_string(),
        "--state",
        &proxy.join("state").display().to_string(),
        "--upstream",
        &upstream_file(layout).display().to_string(),
    ]
    .map(str::to_string);
    Unit {
        binary: "lys",
        args: args.to_vec(),
        log: layout.logs_dir().join("proxy.log"),
        pid: layout.run_dir().join("proxy.pid"),
        ready: Ready {
            says: LISTENING.to_owned(),
            answers: None,
        },
    }
}

/// Records the upstream, then starts the proxy from `program` unless it
/// already runs on the same upstream, and waits for its start line. A
/// running proxy is otherwise left alone, as the runner is.
pub fn start(
    layout: &Layout,
    ports: Ports,
    program: &Path,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    for directory in [layout.run_dir(), layout.logs_dir(), layout.data_dir()] {
        private_files::ensure_dir(&directory)?;
    }
    let changed = configure(layout, services::login()?, say)?;
    let unit = unit(layout, ports);
    let before = std::fs::metadata(&unit.log).map_or(0, |meta| meta.len());
    let started = services::start_detached(program, &unit.args, &unit.log, &unit.pid, changed)?;
    if started {
        private_files::write(
            &unit.pid.with_extension("offset"),
            before.to_string().as_bytes(),
        )?;
        let mut cursor = LogCursor::at(&unit.log, before);
        log_wait::wait_until("model proxy", &unit.log, &unit.pid, &mut || {
            cursor.says(LISTENING)
        })?;
    }
    say(&format!(
        "model proxy {} on {}",
        if started {
            "started"
        } else {
            "already running"
        },
        ports.proxy_url()
    ));
    Ok(())
}

#[cfg(test)]
#[path = "proxy_tests.rs"]
mod tests;
