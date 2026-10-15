//! Issuance decisions are injected; tests never display a system prompt.

use super::{Purpose, issue_pending, issue_with};
use crate::commands::output::Emitter;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use crate::identity::install::layout::{Layout, render_deployment};
use crate::identity::presence::Presence;
use std::cell::Cell;

struct Answer {
    refusal: Option<ErrorKind>,
    asked: Cell<usize>,
}

impl Presence for Answer {
    fn authenticate(&self) -> IdentityResult<()> {
        self.asked.set(self.asked.get() + 1);
        self.refusal.map_or(Ok(()), |kind| {
            Err(IdentityError::new(
                kind,
                "authenticate",
                "recovery",
                "injected presence decision",
            ))
        })
    }
}

fn fixture() -> Result<(tempfile::TempDir, Layout), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::write(layout.deployment_config(), render_deployment(None))?;
    std::fs::create_dir(root.path().join("state"))?;
    Ok((root, layout))
}

#[test]
fn failed_cancelled_or_unavailable_presence_leaves_all_recovery_files_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    for purpose in [Purpose::FirstRun, Purpose::Password] {
        for kind in [
            ErrorKind::PresenceRefused,
            ErrorKind::PresenceCancelled,
            ErrorKind::PresenceUnavailable,
        ] {
            let (root, layout) = fixture()?;
            let code = root.path().join("state/setup-code");
            std::fs::write(&code, b"previous pending code")?;
            let presence = Answer {
                refusal: Some(kind),
                asked: Cell::new(0),
            };
            let result = issue_pending(&layout, purpose, &presence);
            assert_eq!(result.err().map(|error| error.kind()), Some(kind));
            assert_eq!(presence.asked.get(), 1);
            assert_eq!(std::fs::read(&code)?, b"previous pending code");
            assert!(!root.path().join("state/recovery-events").exists());
            assert!(!layout.headless_setup_code().exists());
        }
    }
    Ok(())
}

#[test]
fn successful_presence_records_a_security_event_before_handing_over_each_code()
-> Result<(), Box<dyn std::error::Error>> {
    for purpose in [Purpose::FirstRun, Purpose::Password] {
        let (root, layout) = fixture()?;
        let presence = Answer {
            refusal: None,
            asked: Cell::new(0),
        };
        let code = issue_pending(&layout, purpose, &presence)?;
        let events =
            lys_identity::recovery_events::read(&root.path().join("state/recovery-events"))?;
        assert_eq!(presence.asked.get(), 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].correlation, super::digest(code.expose()));
        assert_eq!(
            events[0].kind,
            match purpose {
                Purpose::FirstRun => lys_identity::recovery_events::Kind::FirstRunCodeIssued,
                Purpose::Password => lys_identity::recovery_events::Kind::PasswordCodeIssued,
            }
        );
        let serialized = serde_json::to_string(&events)?;
        assert!(!serialized.contains(code.expose()));
    }
    Ok(())
}

#[test]
fn a_refused_command_never_opens_a_browser_or_creates_a_headless_code()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, layout) = fixture()?;
    let presence = Answer {
        refusal: Some(ErrorKind::PresenceCancelled),
        asked: Cell::new(0),
    };
    let opened = Cell::new(false);
    let result = issue_with(
        &layout,
        &presence,
        &|_| {
            opened.set(true);
            true
        },
        &mut Emitter::new(true),
    );
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(ErrorKind::PresenceCancelled)
    );
    assert!(!opened.get());
    assert!(!layout.headless_setup_code().exists());
    assert!(!root.path().join("state/setup-code").exists());
    Ok(())
}

#[test]
fn an_unwritable_audit_store_refuses_issuance_without_replacing_the_pending_code()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, layout) = fixture()?;
    let pending = root.path().join("state/setup-code");
    std::fs::write(&pending, b"previous pending code")?;
    std::fs::write(
        root.path().join("state/recovery-events"),
        b"not a directory",
    )?;
    let presence = Answer {
        refusal: None,
        asked: Cell::new(0),
    };
    let result = issue_pending(&layout, Purpose::Password, &presence);
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(ErrorKind::RecoveryEventsUnavailable)
    );
    assert_eq!(presence.asked.get(), 1);
    assert_eq!(std::fs::read(pending)?, b"previous pending code");
    assert!(!layout.headless_setup_code().exists());
    Ok(())
}

#[test]
fn an_authenticated_headless_code_is_never_printed_and_is_kept_only_as_its_digest()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let (root, layout) = fixture()?;
    let presence = Answer {
        refusal: None,
        asked: Cell::new(0),
    };
    let mut emitter = Emitter::new(true);
    issue_with(&layout, &presence, &|_| false, &mut emitter)?;
    let value = emitter.into_value().ok_or("JSON output missing")?;
    let path = root.path().join("setup-code");
    let code = std::fs::read_to_string(&path)?;
    assert_eq!(code.len(), 32);
    assert!(code.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(value["setup_code_file"], path.display().to_string());
    assert!(!value.to_string().contains(&code));
    let pending = std::fs::read_to_string(root.path().join("state/setup-code"))?;
    assert!(!pending.contains(&code));
    let pending: serde_json::Value = serde_json::from_str(&pending)?;
    assert_eq!(pending["purpose"], "first-run");
    assert_eq!(pending["sha256"], super::digest(&code));
    assert_eq!(presence.asked.get(), 1);
    Ok(())
}

#[test]
fn a_relative_headless_layout_reports_a_full_path_after_authentication()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, _) = fixture()?;
    let cwd = std::env::current_dir()?;
    let absolute = root.path().canonicalize()?;
    let common = cwd
        .components()
        .zip(absolute.components())
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = std::path::PathBuf::new();
    for _ in cwd.components().skip(common) {
        relative.push("..");
    }
    for component in absolute.components().skip(common) {
        relative.push(component);
    }
    let layout = Layout::at(relative);
    let presence = Answer {
        refusal: None,
        asked: Cell::new(0),
    };
    let mut emitter = Emitter::new(true);
    issue_with(&layout, &presence, &|_| false, &mut emitter)?;
    let value = emitter.into_value().ok_or("JSON output missing")?;
    let path = std::path::Path::new(
        value["setup_code_file"]
            .as_str()
            .ok_or("headless path missing")?,
    );
    assert!(path.is_absolute());
    assert_eq!(
        path.canonicalize()?,
        root.path().join("setup-code").canonicalize()?
    );
    assert_eq!(presence.asked.get(), 1);
    Ok(())
}
