//! Lys started at login for this person: a launch agent that runs the
//! install's own `lys-app --at-login` when they log in.
//!
//! The agent's definition names the app binary the install keeps in its
//! `bin/`, never the app where it was opened, so Lys starts at login
//! wherever the app is moved and whether or not it is still there. The
//! agent runs once at login (`RunAtLoad`), leaves the processes it starts
//! running when it exits (`AbandonProcessGroup`), and writes its output to
//! the install's own log. Registering writes the definition, which the
//! system loads at the person's next login; unregistering unloads it when
//! it is loaded and removes it.
//!
//! Invariants: the definition is the one file written outside the data
//! root, it is removed by uninstall, and a definition that cannot be
//! written, unloaded or removed is refused by name.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lys_install::install::layout::Layout;

use crate::bundle::APP;
use crate::refusal::Refusal;

/// The launch agent's label: the app's bundle identifier.
pub const LABEL: &str = "au.com.ablative.lys";

/// The argument the login item runs the app with.
pub const AT_LOGIN: &str = "--at-login";

/// Where the launch agent's definition is for the home `home`.
pub fn agent_path(home: &Path) -> PathBuf {
    home.join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

/// Escapes `text` for an XML property list string.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The launch agent's definition, running `program --at-login` with its
/// output appended to `log`.
pub fn definition(program: &Path, log: &Path) -> String {
    let program = escape(&program.display().to_string());
    let log = escape(&log.display().to_string());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{program}</string>
		<string>{AT_LOGIN}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>AbandonProcessGroup</key>
	<true/>
	<key>LimitLoadToSessionType</key>
	<string>Aqua</string>
	<key>StandardOutPath</key>
	<string>{log}</string>
	<key>StandardErrorPath</key>
	<string>{log}</string>
</dict>
</plist>
"#
    )
}

fn home() -> Result<PathBuf, Refusal> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            Refusal::app(
                "home_unknown",
                "Lys could not find your home folder.",
                "HOME is not set",
            )
        })
}

fn failed(action: &str, path: &Path, error: impl std::fmt::Display) -> Refusal {
    Refusal::new(
        "login_item_failed",
        "Lys could not set itself to start when you log in.",
        "Press Try again.",
        format!("{action} {}: {error}", path.display()),
    )
}

/// Registers the install under `layout` to start at login, writing the
/// definition only when it differs. `true` when it was written.
pub fn register(layout: &Layout) -> Result<bool, Refusal> {
    register_at(&home()?, layout)
}

/// [`register`] for the home `home`.
pub fn register_at(home: &Path, layout: &Layout) -> Result<bool, Refusal> {
    let path = agent_path(home);
    let text = definition(&layout.binary(APP), &layout.logs_dir().join("lys-app.log"));
    if std::fs::read_to_string(&path).is_ok_and(|held| held == text) {
        return Ok(false);
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|error| failed("make", folder, error))?;
    }
    let placing = path.with_extension("plist.placing");
    std::fs::write(&placing, text).map_err(|error| failed("write", &placing, error))?;
    std::fs::rename(&placing, &path).map_err(|error| failed("place", &path, error))?;
    Ok(true)
}

/// The launch domain's name for this person's login item.
#[cfg(unix)]
fn service_target() -> String {
    format!("gui/{}/{LABEL}", rustix::process::getuid().as_raw())
}

fn launchctl(args: &[&str]) -> Result<std::process::Output, Refusal> {
    Command::new("/bin/launchctl")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| failed("run", Path::new("/bin/launchctl"), error))
}

/// Removes the login item: unloaded when the system has it loaded, its
/// definition removed. `true` when there was one.
#[cfg(unix)]
pub fn unregister() -> Result<bool, Refusal> {
    let path = agent_path(&home()?);
    let target = service_target();
    if launchctl(&["print", &target])?.status.success() {
        let out = launchctl(&["bootout", &target])?;
        if !out.status.success() {
            let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(failed("unload", &path, said));
        }
    }
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(failed("remove", &path, error)),
    }
}

/// A login item is a Mac's launch agent, which a host without Unix user
/// ids has no domain for.
#[cfg(not(unix))]
pub fn unregister() -> Result<bool, Refusal> {
    Err(Refusal::app(
        "login_item_needs_macos",
        "Lys can start at login only on a Mac.",
        format!("no launch domain for {}", agent_path(&home()?).display()),
    ))
}

#[cfg(test)]
#[path = "login_item_tests.rs"]
mod tests;
