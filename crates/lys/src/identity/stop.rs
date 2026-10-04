//! `lys identity stop`: the master off switch at this computer's console,
//! and `lys identity start`, its inverse.
//!
//! Stop pulls the cord and then turns everything Lys started off, in order:
//!
//! 1. the service is asked to stop everything, so every runner it drives,
//!    on this computer and on others, ends every session and each is
//!    recorded with who pulled the cord and why. A service that does not
//!    answer is named, and so is what that leaves unreached: runners on
//!    other computers;
//! 2. this computer's runner is told to stop everything itself, which ends
//!    any session the service did not reach and records the same words;
//! 3. the runner, the model proxy, the service and the broker are stopped
//!    through their exit locks, each waited on until it has exited;
//! 4. the database, the sign-in service and the permission service the
//!    install's compose file runs are stopped, their data kept.
//!
//! Every part is done whatever an earlier part answered, and every part
//! that failed is named in the one refusal the command ends with. Nothing
//! is judged on an exit status alone: each answer is read for what it says.
//!
//! The request to the service is signed with the service's own key, which
//! only the install's owner can read, over a fresh nonce: holding the
//! install's files is the authority to turn it off.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::{Act, Answer, Client};

use super::config::DeploymentConfig;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::install::layout::Layout;
use super::install::ports::Ports;
use super::install::{login, proxy, services};
use super::loopback_http::{Authority, Request, exchange};
use super::upgrade::{self, swap};
use crate::commands::output::Emitter;

/// The domain the service's console request is signed under.
pub const CONSOLE_DOMAIN: &str = "lys-identity/console-stop/v1";

/// The header carrying the console request's signature.
pub const CONSOLE_SIGNATURE: &str = "x-lys-console-signature";

/// What `lys identity stop` is asked.
#[derive(Debug, Clone)]
pub struct StopOptions {
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
    /// Why, in the words of the person pulling the cord.
    pub reason: String,
    /// End each session's process group at once instead of hanging it up.
    pub kill: bool,
}

fn layout_at(root: Option<&PathBuf>) -> IdentityResult<Layout> {
    match root {
        Some(root) => Ok(Layout::at(root.clone())),
        None => Layout::discover(),
    }
}

fn failed(part: &str, detail: impl std::fmt::Display) -> String {
    format!("{part}: {detail}")
}

/// Who pulls the cord at this console: the login's own user.
fn console_person() -> IdentityResult<String> {
    let user = login::login()?
        .variables()
        .find(|(name, _)| *name == "USER")
        .map(|(_, value)| value.to_owned())
        .filter(|user| !user.trim().is_empty())
        .ok_or_else(|| {
            IdentityError::new(
                ErrorKind::StopIncomplete,
                "stop",
                "console",
                "the login names no USER, so the stop could not say who pulled the cord",
            )
        })?;
    Ok(format!("{user} at the console of this computer"))
}

/// Runs `lys identity stop`.
pub fn run(options: &StopOptions, json: bool) -> IdentityResult<()> {
    let reason = options.reason.trim().to_owned();
    if reason.is_empty() {
        return Err(IdentityError::new(
            ErrorKind::StopIncomplete,
            "stop",
            "reason",
            "say why everything is stopped: --reason <words>",
        ));
    }
    let layout = layout_at(options.root.as_ref())?;
    let by = console_person()?;
    let mut emitter = Emitter::new(json);
    emitter.field("root", "root", layout.root.display().to_string());
    let mut failures = Vec::new();
    let key = Ed25519Identity::load(&layout.service_key()).map(Arc::new);
    match &key {
        Ok(key) => {
            match ask_service(&layout, key, &by, &reason, options.kill) {
                Ok(words) => emitter.note(&words),
                Err(Unanswered::Unreached(detail)) => {
                    emitter.note(&format!(
                        "the service did not answer ({detail}): runners on other computers were not reached, and their sessions were not stopped"
                    ));
                    failures.push(failed(
                        "service",
                        "did not answer; runners on other computers were not reached",
                    ));
                }
                Err(Unanswered::Refused(detail)) => {
                    emitter.note(&format!("the service refused to stop everything: {detail}"));
                    failures.push(failed("service", detail));
                }
            }
            match stop_runner_sessions(&layout, key, &by, &reason, options.kill) {
                Ok(words) => emitter.note(&words),
                Err(detail) => failures.push(failed("this computer's runner", detail)),
            }
        }
        Err(error) => failures.push(failed(
            "service key",
            format!("{error}: neither the service nor this computer's runner could be asked"),
        )),
    }
    stop_processes(&layout, &mut emitter, &mut failures);
    match DeploymentConfig::load_install(&layout.deployment_config())
        .and_then(|config| services::compose_stop(&layout, &config))
    {
        Ok(()) => emitter.note("database, sign-in and permission services stopped"),
        Err(error) => failures.push(failed("compose services", error)),
    }
    if failures.is_empty() {
        emitter.note("everything Lys started is stopped");
        emitter.finish();
        return Ok(());
    }
    emitter.finish();
    Err(IdentityError::new(
        ErrorKind::StopIncomplete,
        "stop",
        "install",
        failures.join("; "),
    ))
}

/// Stop this computer's runner, the model proxy, the service and the
/// broker, each through its exit lock.
fn stop_processes(layout: &Layout, emitter: &mut Emitter, failures: &mut Vec<String>) {
    let runner = layout.run_dir().join("runner.pid");
    match services::stop(&runner) {
        Ok(true) => emitter.note("runner stopped through its exit lock"),
        Ok(false) => emitter.note("runner was not running"),
        Err(error) => failures.push(failed("runner", error)),
    }
    match Ports::load(layout) {
        Ok(ports) => match services::stop(&proxy::unit(layout, ports).pid) {
            Ok(true) => emitter.note("model proxy stopped through its exit lock"),
            Ok(false) => emitter.note("model proxy was not running"),
            Err(error) => failures.push(failed("model proxy", error)),
        },
        Err(error) => failures.push(failed("model proxy", error)),
    }
    match upgrade::units(layout) {
        Ok(units) => {
            if let Err(error) = swap::stop_all(&units, &mut |line| emitter.note(line)) {
                failures.push(failed("service and broker", error));
            }
        }
        Err(error) => failures.push(failed("service and broker", error)),
    }
}

/// Tell this computer's runner to stop everything, when one answers.
fn stop_runner_sessions(
    layout: &Layout,
    key: &Arc<Ed25519Identity>,
    by: &str,
    reason: &str,
    kill: bool,
) -> Result<String, String> {
    let socket = layout.runner_socket();
    if !services::alive(&layout.run_dir().join("runner.pid")) {
        return Ok("this computer's runner was not running".to_owned());
    }
    let client = Client::new(socket, Arc::clone(key));
    let act = Act::StopEverything {
        by: by.to_owned(),
        reason: reason.to_owned(),
        kill,
        // A person at this computer asked: never the server settling a start.
        settling: false,
    };
    match client.ask(&act) {
        Ok(Answer::StoppedEverything { sessions, running }) if running.is_empty() => Ok(format!(
            "this computer's runner stopped {} sessions{}",
            sessions.len(),
            listed(&sessions)
        )),
        Ok(Answer::StoppedEverything { running, .. }) => Err(format!(
            "these sessions had not ended when the runner answered: {}",
            running.join(", ")
        )),
        Ok(other) => Err(format!("a stop of everything was answered {other:?}")),
        Err(error) => Err(error.to_string()),
    }
}

fn listed(sessions: &[String]) -> String {
    if sessions.is_empty() {
        String::new()
    } else {
        format!(": {}", sessions.join(", "))
    }
}

/// Why the service did not stop everything.
enum Unanswered {
    /// No answer came: the service is down or did not finish answering.
    Unreached(String),
    /// It answered, and refused or reported a part it could not do.
    Refused(String),
}

/// The service's address and the console route under its prefix.
fn console_endpoint(layout: &Layout) -> Result<(Authority, &'static str), Unanswered> {
    let path = layout.service_config();
    let unreadable = |detail: String| Unanswered::Refused(format!("{}: {detail}", path.display()));
    let bytes = std::fs::read(&path).map_err(|error| unreadable(error.to_string()))?;
    let config: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| unreadable(error.to_string()))?;
    let listen = config
        .get("listen")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| unreadable("the installed service has no listen address".to_owned()))?;
    let route = match config.get("surface_dir") {
        None | Some(serde_json::Value::Null) => "/runtime/stop-everything/console",
        Some(_) => "/api/runtime/stop-everything/console",
    };
    let authority = Authority::parse(listen, None)
        .map_err(|error| unreadable(format!("the listen address {listen}: {error}")))?;
    Ok((authority, route))
}

/// Ask the service to stop everything, signed with its own key.
fn ask_service(
    layout: &Layout,
    key: &Ed25519Identity,
    by: &str,
    reason: &str,
    kill: bool,
) -> Result<String, Unanswered> {
    let (authority, route) = console_endpoint(layout)?;
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let body = serde_json::to_vec(&serde_json::json!({
        "operation": format!("op-{}", lys_runner::protocol::hex(&rand::random::<[u8; 16]>())),
        "by": by,
        "reason": reason,
        "kill": kill,
        "nonce": lys_runner::protocol::hex(&rand::random::<[u8; 32]>()),
        "at": at,
    }))
    .map_err(|error| Unanswered::Refused(error.to_string()))?;
    let mut signed = format!("{CONSOLE_DOMAIN}\n").into_bytes();
    signed.extend_from_slice(&body);
    let signature = lys_runner::protocol::hex(&key.sign(&signed));
    let headers = [
        ("Content-Type", b"application/json".as_slice()),
        (CONSOLE_SIGNATURE, signature.as_bytes()),
    ];
    let request = Request {
        method: "POST",
        path: route,
        headers: &headers,
        body: &body,
    };
    let answer = exchange(&authority, &request)
        .map_err(|failure| Unanswered::Unreached(format!("{failure:?}")))?;
    let read: serde_json::Value = serde_json::from_slice(&answer.body).map_err(|error| {
        Unanswered::Refused(format!(
            "the service answered {} with no JSON: {error}",
            answer.status
        ))
    })?;
    if !(200..300).contains(&answer.status) {
        return Err(Unanswered::Refused(format!("{} {read}", answer.status)));
    }
    service_words(&read)
}

/// The service's answer read for what it did, whether it answers the
/// pull's result or how the cord stands with the result as its `last`:
/// every session still running, every runner not reached and every agent
/// whose credentials were not confirmed ended is a failure, named.
fn service_words(read: &serde_json::Value) -> Result<String, Unanswered> {
    let result = if read.get("stopped").is_some() {
        read
    } else {
        read.get("last").unwrap_or(&serde_json::Value::Null)
    };
    let names = |key: &str| -> Vec<String> {
        result
            .get(key)
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        item.get("session")
                            .or_else(|| item.get("machine"))
                            .or_else(|| item.get("agent"))
                            .and_then(serde_json::Value::as_str)
                            .map_or_else(|| item.to_string(), str::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    if read.get("pulled").is_none_or(serde_json::Value::is_null) {
        return Err(Unanswered::Refused(format!(
            "the service did not say the cord is pulled: {read}"
        )));
    }
    let unfinished: BTreeMap<&str, Vec<String>> = [
        ("still running", names("still_running")),
        ("runners not reached", names("unreached")),
        ("credentials not confirmed ended", names("handles_refused")),
    ]
    .into_iter()
    .filter(|(_, items)| !items.is_empty())
    .collect();
    if unfinished.is_empty() {
        let stopped = names("stopped");
        return Ok(format!(
            "the service stopped everything: {} sessions{}",
            stopped.len(),
            listed(&stopped)
        ));
    }
    Err(Unanswered::Refused(
        unfinished
            .iter()
            .map(|(what, items)| format!("{what}: {}", items.join(", ")))
            .collect::<Vec<_>>()
            .join("; "),
    ))
}

/// What `lys identity start` is asked.
#[derive(Debug, Clone)]
pub struct StartOptions {
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
}

/// Runs `lys identity start`: starts whatever of the install is down, with
/// the same steps the install starts it with, and leaves running whatever
/// runs. A cord pulled in the service stays pulled until the administrator
/// lets agents start again.
pub fn start(options: &StartOptions, json: bool) -> IdentityResult<()> {
    let layout = layout_at(options.root.as_ref())?;
    let mut emitter = Emitter::new(json);
    emitter.field("root", "root", layout.root.display().to_string());
    let ports = Ports::load(&layout)?;
    let units = upgrade::units_at(&layout, ports);
    super::install::start_installed(&layout, ports, &units, &mut |line| {
        emitter.note(line);
    })?;
    emitter.field("open", "url", ports.service_url());
    emitter.finish();
    Ok(())
}
