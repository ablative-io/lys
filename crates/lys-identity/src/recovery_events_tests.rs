use super::{Kind, read, record};

#[test]
fn an_old_install_without_recovery_events_stays_unchanged_until_its_first_event()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let legacy = root.path().join("setup-code");
    std::fs::write(&legacy, b"legacy setup verification record")?;
    let events = root.path().join("recovery-events");
    assert!(read(&events)?.is_empty());
    assert!(!events.exists());
    let issued = record(&events, Kind::PasswordCodeIssued, &"a".repeat(64), 10)?;
    let completed = record(&events, Kind::PasswordResetCompleted, &"a".repeat(64), 11)?;
    assert_eq!(read(&events)?, vec![completed, issued]);
    assert_eq!(std::fs::read(legacy)?, b"legacy setup verification record");
    Ok(())
}

#[test]
fn an_unreadable_complete_recovery_event_is_refused_not_skipped()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("broken.json"), b"{")?;
    assert!(read(root.path()).is_err());
    Ok(())
}
