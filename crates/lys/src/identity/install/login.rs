//! The environment every service and the runner start with: the login's,
//! never the shell that ran the installer. ADR-111 keeps it out of
//! `services.rs`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde::Serialize;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};

fn refuse(action: &'static str, resource: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(ErrorKind::Unready, action, resource, detail)
}

/// The variables a service keeps from the process that starts it: the
/// login's identity, its temporary folder and its locale. Nothing else the
/// invoking shell carries reaches a service or, through the runner, a run:
/// not a harness's config folder, not a credential, not a proxy setting.
/// `PATH` is not kept; it comes from the login shell, see [`login`].
pub const KEPT: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "LC_COLLATE",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_MONETARY",
    "LC_NUMERIC",
    "LC_TIME",
];

/// The environment every service and the runner start with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    kept: BTreeMap<String, String>,
    path: String,
}

/// What the install record says about the services' environment: the names
/// kept from the login and the `PATH` its login shell answered. Values of
/// the kept variables are not recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvironmentRecord {
    /// The names from [`KEPT`] the login had, in order.
    pub kept: Vec<String>,
    /// The `PATH` the login shell answered.
    pub path: String,
}

impl Environment {
    /// Every variable a service starts with, by name: the kept ones and `PATH`.
    pub fn variables(&self) -> impl Iterator<Item = (&str, &str)> {
        self.kept
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .chain(std::iter::once(("PATH", self.path.as_str())))
    }

    /// What the install record says about this environment.
    pub fn record(&self) -> EnvironmentRecord {
        EnvironmentRecord {
            kept: self.kept.keys().cloned().collect(),
            path: self.path.clone(),
        }
    }
}

/// What the login shell prints before and after `PATH`, so that anything a
/// login profile prints on its own, or a logout file prints after, is left
/// outside it and never taken as PATH.
const PATH_MARKER: &str = "LYS_LOGIN_PATH:";
const PATH_END: &str = ":LYS_LOGIN_PATH_END";

static LOGIN: OnceLock<Environment> = OnceLock::new();

/// The login's environment, read once for this process: the [`KEPT`]
/// variables this process has, and `PATH` as the login shell (`SHELL`,
/// started as a login shell with only those variables) answers it. Refused
/// when there is no `SHELL`, when the shell does not run, when it does not
/// answer, or when it answers an empty `PATH`.
pub fn login() -> IdentityResult<&'static Environment> {
    if let Some(environment) = LOGIN.get() {
        return Ok(environment);
    }
    let environment = login_from(std::env::vars_os())?;
    Ok(LOGIN.get_or_init(|| environment))
}

/// [`login`] from `process` as the starting process's variables, read now,
/// so a test can hand it variables it must drop.
pub fn login_from(
    process: impl IntoIterator<Item = (OsString, OsString)>,
) -> IdentityResult<Environment> {
    let mut kept = BTreeMap::new();
    for (name, value) in process {
        let Some(name) = name.to_str().filter(|name| KEPT.contains(name)) else {
            continue;
        };
        let value = value.into_string().map_err(|value| {
            refuse(
                "read login",
                "environment",
                format!("{name} is not text: {}", value.to_string_lossy()),
            )
        })?;
        kept.insert(name.to_string(), value);
    }
    let shell = kept
        .get("SHELL")
        .filter(|shell| !shell.is_empty())
        .ok_or_else(|| {
            refuse(
                "read login",
                "environment",
                "SHELL is not set; start the install from a login",
            )
        })?;
    let output = Command::new(shell)
        .env_clear()
        .envs(&kept)
        .args([
            "-l",
            "-c",
            &format!(r#"printf '\n%s%s%s' '{PATH_MARKER}' "$PATH" '{PATH_END}'"#),
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            refuse(
                "read login",
                "environment",
                format!("login shell {shell} could not run: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(refuse(
            "read login",
            "environment",
            format!(
                "login shell {shell} exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    let answer = String::from_utf8(output.stdout).map_err(|error| {
        refuse(
            "read login",
            "environment",
            format!("login shell {shell} answered a PATH that is not text: {error}"),
        )
    })?;
    let Some((_, after_mark)) = answer.rsplit_once(PATH_MARKER) else {
        return Err(refuse(
            "read login",
            "environment",
            format!(
                "login shell {shell} did not answer its PATH: {}",
                answer.trim()
            ),
        ));
    };
    let Some((path, _)) = after_mark.split_once(PATH_END) else {
        return Err(refuse(
            "read login",
            "environment",
            format!(
                "login shell {shell} did not close its PATH answer: {}",
                answer.trim()
            ),
        ));
    };
    if path.is_empty() {
        return Err(refuse(
            "read login",
            "environment",
            format!("login shell {shell} answered an empty PATH"),
        ));
    }
    Ok(Environment {
        kept,
        path: path.to_string(),
    })
}

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;
