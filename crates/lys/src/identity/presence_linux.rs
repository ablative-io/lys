//! A foreground controlling terminal owned by the installing user proves Linux presence.

use super::{ErrorKind, IdentityResult, refused};

#[cfg(any(target_os = "linux", test))]
pub(crate) struct Operator {
    pub(crate) terminals: bool,
    pub(crate) foreground: bool,
    pub(crate) user: u32,
    pub(crate) effective_user: u32,
    pub(crate) terminal_owner: u32,
}

#[cfg(any(target_os = "linux", test))]
impl super::Presence for Operator {
    fn authenticate(&self) -> IdentityResult<()> {
        if self.terminals
            && self.foreground
            && self.user == self.effective_user
            && self.user == self.terminal_owner
        {
            return Ok(());
        }
        Err(refused(
            ErrorKind::PresenceRefused,
            "run code issuance as the installing user in the foreground of their own terminal",
        ))
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn authenticate() -> IdentityResult<()> {
    use super::Presence;
    use std::io::IsTerminal;

    let unavailable = |error: std::io::Error| {
        refused(
            ErrorKind::PresenceUnavailable,
            format!("the installing user's controlling terminal could not be verified: {error}"),
        )
    };
    let terminal = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(unavailable)?;
    // /dev/tty is the shared controlling-terminal device; stdin names the operator's terminal.
    let owner = rustix::fs::fstat(std::io::stdin())
        .map_err(|error| unavailable(error.into()))?
        .st_uid;
    let foreground = rustix::termios::tcgetpgrp(&terminal)
        .map_err(|error| unavailable(error.into()))?
        == rustix::process::getpgrp();
    Operator {
        terminals: terminal.is_terminal() && std::io::stdin().is_terminal(),
        foreground,
        user: rustix::process::getuid().as_raw(),
        effective_user: rustix::process::geteuid().as_raw(),
        terminal_owner: owner,
    }
    .authenticate()
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn authenticate() -> IdentityResult<()> {
    Err(refused(
        ErrorKind::PresenceUnavailable,
        "Linux terminal presence is unavailable on this platform",
    ))
}
