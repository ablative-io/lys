#![cfg(test)]
//! Held email replacement and provenance share the foundings durable write.
use super::*;

#[test]
fn replacing_a_held_email_is_durable_and_preserves_the_original() -> Result<(), ServerError> {
    let dir = tempfile::tempdir().map_err(|error| unusable(Path::new("tmp"), &error))?;
    let file = dir.path().join("foundings.json");
    let mut held = Foundings::open(file.clone())?;
    held.hold_email("person-1", "dean@example.test", false)?;
    held.hold_email("person-1", "annabel@example.test", false)?;
    let reopened = Foundings::open(file)?;
    assert_eq!(reopened.email_for("person-1"), Some("annabel@example.test"));
    assert_eq!(reopened.original_email_for("person-1"), Some("dean@example.test"));
    assert!(!reopened.recorded_at_sign_in("person-1"));
    Ok(())
}

#[test]
fn a_refused_email_save_restores_all_memory_and_disk_fields() -> Result<(), ServerError> {
    let dir = tempfile::tempdir().map_err(|error| unusable(Path::new("tmp"), &error))?;
    let file = dir.path().join("foundings.json");
    let mut held = Foundings::open(file.clone())?;
    held.hold_email("person-1", "dean@example.test", true)?;
    let before = std::fs::read(&file).map_err(|error| unusable(&file, &error))?;
    held.file = file.join("refused");
    assert!(held.hold_email("person-1", "annabel@example.test", false).is_err());
    assert_eq!(held.email_for("person-1"), Some("dean@example.test"));
    assert_eq!(held.original_email_for("person-1"), Some("dean@example.test"));
    assert!(held.recorded_at_sign_in("person-1"));
    assert_eq!(std::fs::read(&file).map_err(|error| unusable(&file, &error))?, before);
    let reopened = Foundings::open(file)?;
    assert_eq!(reopened.email_for("person-1"), Some("dean@example.test"));
    assert!(reopened.recorded_at_sign_in("person-1"));
    Ok(())
}
