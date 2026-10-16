#![cfg(test)]
//! The Linux platform decision gates the real first-run and recovery issuance path.

use super::layout::{Layout, render_deployment};
use super::setup_code::{Purpose, issue_pending};
use crate::identity::error::{ErrorKind, IdentityResult};
use crate::identity::presence::{Presence, authenticate_for};

struct LinuxOperator(bool);

impl Presence for LinuxOperator {
    fn authenticate(&self) -> IdentityResult<()> {
        authenticate_for("linux", || {
            crate::identity::presence::linux::Operator {
                terminals: self.0,
                foreground: true,
                user: 1000,
                effective_user: 1000,
                terminal_owner: 1000,
            }
            .authenticate()
        })
    }
}

#[test]
fn linux_presence_refuses_a_background_or_different_user() {
    use crate::identity::presence::linux::Operator;
    for operator in [
        Operator {
            terminals: true,
            foreground: false,
            user: 1000,
            effective_user: 1000,
            terminal_owner: 1000,
        },
        Operator {
            terminals: true,
            foreground: true,
            user: 1000,
            effective_user: 0,
            terminal_owner: 1000,
        },
        Operator {
            terminals: true,
            foreground: true,
            user: 1000,
            effective_user: 1000,
            terminal_owner: 2000,
        },
    ] {
        assert_eq!(
            operator.authenticate().err().map(|error| error.kind()),
            Some(ErrorKind::PresenceRefused)
        );
    }
}

#[test]
fn linux_present_operator_produces_first_run_and_recovery_codes()
-> Result<(), Box<dyn std::error::Error>> {
    for purpose in [Purpose::FirstRun, Purpose::Password] {
        let root = tempfile::tempdir()?;
        let layout = Layout::at(root.path().to_path_buf());
        std::fs::write(layout.deployment_config(), render_deployment(None))?;
        std::fs::create_dir(root.path().join("state"))?;
        let code = issue_pending(&layout, purpose, &LinuxOperator(true))?;
        assert_eq!(code.expose().len(), 32);
        let pending = std::fs::read_to_string(root.path().join("state/setup-code"))?;
        assert!(!pending.contains(code.expose()));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&pending)?["purpose"],
            purpose.word()
        );
    }
    Ok(())
}

#[test]
fn linux_absent_operator_refuses_without_replacing_a_code() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(layout.deployment_config(), render_deployment(None))?;
    std::fs::create_dir(root.path().join("state"))?;
    let pending = root.path().join("state/setup-code");
    std::fs::write(&pending, "unchanged")?;
    let result = issue_pending(&layout, Purpose::Password, &LinuxOperator(false));
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(ErrorKind::PresenceRefused)
    );
    assert_eq!(std::fs::read_to_string(pending)?, "unchanged");
    assert!(!root.path().join("state/recovery-events").exists());
    Ok(())
}
