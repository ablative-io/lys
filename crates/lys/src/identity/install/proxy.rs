//! Lys's model proxy beside the service: `lys proxy serve` on loopback, the
//! address every model call of a Claude Code run Lys starts is sent to. It
//! forwards each call unchanged and records it in its own home under the
//! session the call names. It runs like the runner: its own Unit with a pid
//! file, an exit lock and a log, from the `lys` the install runs. Install
//! starts it when it is not running; upgrade stops it with the runner before
//! anything is placed and starts it again after the services, and a call it
//! held at the stop is recorded `lost` when it next starts.

use std::path::Path;

use super::super::error::IdentityResult;
use super::super::private_files;
use super::super::upgrade::{Ready, Unit};
use super::layout::Layout;
use super::log_wait::{self, LogCursor};
use super::ports::Ports;
use super::services;

/// The words the proxy's start line carries once it listens.
const LISTENING: &str = r#""proxy":"listening""#;

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

/// Starts the proxy from `program` unless it already runs, and waits for
/// its start line. A running proxy is left alone, as the runner is.
pub fn start(
    layout: &Layout,
    ports: Ports,
    program: &Path,
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    for directory in [layout.run_dir(), layout.logs_dir(), layout.data_dir()] {
        private_files::ensure_dir(&directory)?;
    }
    let unit = unit(layout, ports);
    let before = std::fs::metadata(&unit.log).map_or(0, |meta| meta.len());
    let started = services::start_detached(program, &unit.args, &unit.log, &unit.pid, false)?;
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
