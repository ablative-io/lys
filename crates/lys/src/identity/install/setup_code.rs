//! The one-time setup code that opens Lys's setup page, and
//! `lys identity setup-code`.
//!
//! A code is 32 letters and digits from the secure random source. Its
//! SHA-256 digest, never the code, is written with its purpose to the state
//! directory's `setup-code` file, owner-only, as
//! `{"purpose": "first-run" | "password", "sha256": "<lowercase hex>"}`; the
//! directory service reads that file and removes it when the code is used
//! (crates/lys-identity-server/src/setup.rs). The code itself is never
//! printed and never written where a person reads it: it rides only in the
//! fragment of the setup page's address, handed to the browser opener, and
//! a fragment never reaches a server. When no browser can be opened the
//! code is written to an owner-only file in the install root and one line
//! names that file.
//!
//! A first-run code is written while no administrator exists; once one
//! does, `setup-code` writes a password code, with which the setup page
//! sets the administrator a new password. That is the one way back in for a
//! sole administrator who forgot theirs, and it needs the machine.

use std::path::Path;
use std::process::{Command, Stdio};

use sha2::{Digest, Sha256};

use super::super::config::DeploymentConfig;
use super::super::credentials::{Credential, Shape};
use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::presence::{Presence, SystemPresence};
use super::super::private_files;
use super::layout::Layout;
use crate::commands::output::Emitter;

/// The file under the state directory the pending code's digest is kept in.
pub const CODE_FILE: &str = "setup-code";

/// How many letters and digits a setup code has.
const CODE_LENGTH: usize = 32;

/// What a setup code lets the setup page do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Make the first administrator.
    FirstRun,
    /// Set the administrator a new password.
    Password,
}

impl Purpose {
    /// The purpose as the code file names it.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::FirstRun => "first-run",
            Self::Password => "password",
        }
    }
}

/// A fresh code, wiped from memory when dropped.
#[must_use]
pub fn generate() -> Credential {
    Credential::generate("setup code", Shape::Alphanumeric(CODE_LENGTH))
}

/// Lowercase hex SHA-256 of `code`.
#[must_use]
pub fn digest(code: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let hash = Sha256::digest(code.as_bytes());
    let mut text = String::with_capacity(hash.len() * 2);
    for byte in hash {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The code file's text for `code` with `purpose`.
#[must_use]
pub fn pending_text(purpose: Purpose, code: &str) -> String {
    serde_json::json!({ "purpose": purpose.word(), "sha256": digest(code) }).to_string()
}

/// Write `code`'s digest with `purpose` to the state directory, owner-only,
/// replacing any code pending before it.
fn write_pending(state_dir: &Path, purpose: Purpose, code: &str) -> IdentityResult<()> {
    private_files::write(
        &state_dir.join(CODE_FILE),
        pending_text(purpose, code).as_bytes(),
    )?;
    Ok(())
}

/// The address a person opens for this install: its public origin, or the
/// local listener with the reason.
fn person_address(layout: &Layout) -> IdentityResult<super::ports::PersonAddress> {
    super::ports::PersonAddress::load(layout, super::ports::Ports::load(layout)?)
}

/// The setup page's address carrying `code` in its fragment.
#[must_use]
pub fn address_with(code: &str, setup: &str) -> String {
    format!("{setup}#code={code}")
}

/// Open `address` in the person's browser, answering whether a browser
/// opener took it. Nothing the opener writes is shown, so the address is
/// never printed.
#[must_use]
pub fn open_in_browser(address: &str) -> bool {
    if std::env::var_os("SSH_CONNECTION").is_some() {
        return false;
    }
    let opener = if std::env::consts::OS == "macos" {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(opener)
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Hand `code` to the browser through `open`, or, when no browser takes it,
/// write it to the install root's owner-only headless file and say so in
/// one line naming that file. The code is never printed.
///
/// # Errors
/// Returns named setup-code, administrator-read or delivery failures.
pub fn hand_over(
    layout: &Layout,
    code: &str,
    open: &dyn Fn(&str) -> bool,
    emitter: &mut Emitter,
) -> IdentityResult<()> {
    let address = person_address(layout)?;
    address.say_fallback(emitter);
    let setup = address.setup_url();
    if open(&address_with(code, &setup)) {
        emitter.note("the setup page is open in your browser");
        return Ok(());
    }
    let requested = layout.headless_setup_code();
    let path = std::path::absolute(&requested).map_err(|error| {
        IdentityError::new(
            ErrorKind::PrivateFileIo,
            "locate readable setup code",
            "setup code file",
            error.to_string(),
        )
        .at(&requested)
    })?;
    private_files::write(&path, code.as_bytes())?;
    emitter.note(&headless_line(&path, &setup));
    if emitter.is_json() {
        emitter.field(
            "setup code file",
            "setup_code_file",
            path.display().to_string(),
        );
    }
    Ok(())
}

/// The one line said when no browser takes the code: the file holding it
/// and the setup page to enter it on.
#[must_use]
pub fn headless_line(path: &Path, setup: &str) -> String {
    format!(
        "no browser could be opened: the setup code is in {} (only you can read it); open {setup} and enter it in the Setup code field",
        path.display()
    )
}

/// Hand over the first-run code only when issuance returned one.
pub(super) fn hand_over_first_run(
    layout: &Layout,
    code: Option<Credential>,
    emitter: &mut Emitter,
) -> IdentityResult<()> {
    if let Some(code) = code {
        emitter.field("setup", "setup", person_address(layout)?.setup_url());
        hand_over(layout, code.expose(), &open_in_browser, emitter)?;
    }
    Ok(())
}

/// Whether the install has an administrator: one first-run setup recorded,
/// or one its configuration names.
///
/// # Errors
/// Returns named setup-code, administrator-read or delivery failures.
pub fn has_administrator(layout: &Layout) -> IdentityResult<bool> {
    if layout.administrator_file().exists() {
        return Ok(true);
    }
    let path = layout.service_config();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(IdentityError::new(
                ErrorKind::ConfigUnreadable,
                "read",
                "identity.json",
                error.to_string(),
            )
            .at(&path));
        }
    };
    let rendered: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "read",
            "identity.json",
            error.to_string(),
        )
        .at(&path)
    })?;
    Ok(rendered
        .get("administrator")
        .is_some_and(|administrator| !administrator.is_null()))
}

/// Runs `lys identity setup-code`. A failure is said in Lys's words,
/// because the person setting Lys up reads it.
///
/// # Errors
/// Returns named setup-code, administrator-read or delivery failures.
pub fn run(root: Option<std::path::PathBuf>, json: bool) -> IdentityResult<()> {
    issue(root, json).map_err(IdentityError::said_in_lys_words)
}

fn issue(root: Option<std::path::PathBuf>, json: bool) -> IdentityResult<()> {
    let layout = match root {
        Some(root) => Layout::at(root),
        None => Layout::discover()?,
    };
    let mut emitter = Emitter::new(json);
    issue_with(&layout, &SystemPresence, &open_in_browser, &mut emitter)?;
    emitter.finish();
    Ok(())
}

fn issue_with(
    layout: &Layout,
    presence: &dyn Presence,
    open: &dyn Fn(&str) -> bool,
    emitter: &mut Emitter,
) -> IdentityResult<()> {
    let purpose = if has_administrator(layout)? {
        Purpose::Password
    } else {
        Purpose::FirstRun
    };
    let code = issue_pending(layout, purpose, presence)?;
    emitter.field("setup", "setup", person_address(layout)?.setup_url());
    emitter.field("purpose", "purpose", purpose.word());
    hand_over(layout, code.expose(), open, emitter)?;
    Ok(())
}

/// The install and a later recovery use the same issuance boundary.
pub(super) fn issue_pending(
    layout: &Layout,
    purpose: Purpose,
    presence: &dyn Presence,
) -> IdentityResult<Credential> {
    presence.authenticate()?;
    let config = DeploymentConfig::load_install(&layout.deployment_config())?;
    let code = generate();
    let kind = match purpose {
        Purpose::FirstRun => lys_identity::recovery_events::Kind::FirstRunCodeIssued,
        Purpose::Password => lys_identity::recovery_events::Kind::PasswordCodeIssued,
    };
    let at = u64::try_from(chrono::Utc::now().timestamp()).map_err(|error| {
        IdentityError::new(
            ErrorKind::RecoveryEventsUnavailable,
            "record",
            "recovery event",
            error.to_string(),
        )
    })?;
    lys_identity::recovery_events::record(
        &config.state_dir().join("recovery-events"),
        kind,
        &digest(code.expose()),
        at,
    )
    .map_err(|error| {
        IdentityError::new(
            ErrorKind::RecoveryEventsUnavailable,
            "record",
            "recovery event",
            error.to_string(),
        )
    })?;
    write_pending(&config.state_dir(), purpose, code.expose())?;
    Ok(code)
}

#[cfg(test)]
#[path = "setup_code_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "setup_presence_tests.rs"]
mod presence_tests;
