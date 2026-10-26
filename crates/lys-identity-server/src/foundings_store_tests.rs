#![cfg(test)]
use super::*;

fn founding(email: &str, operation: &str) -> Founding {
    Founding {
        organisation: "organisation-1".to_owned(),
        founder: "person-1".to_owned(),
        email: email.to_owned(),
        operation: operation.to_owned(),
        spent: false,
        withdrawn: false,
        withdrawal_event: None,
        handed: false,
        kind: Kind::Founding,
        continue_to: None,
    }
}

#[test]
fn a_token_is_thirty_two_bytes_and_only_its_digest_is_stored() -> Result<(), ServerError> {
    let token = new_token()?;
    assert_eq!(URL_SAFE_NO_PAD.decode(&token).map(|b| b.len()), Ok(32));
    assert_ne!(new_token()?, token);
    let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
    let file = dir.path().join("foundings.json");
    let mut foundings = Foundings::open(file.clone())?;
    foundings.record(digest(&token), founding("ada@example.test", "op-1"), None)?;
    let written = std::fs::read_to_string(&file).map_err(|e| unusable(&file, &e))?;
    assert!(!written.contains(&token), "the token is in the file");
    assert!(written.contains(&digest(&token)));
    let reopened = Foundings::open(file)?;
    assert_eq!(
        reopened.by_digest(&digest(&token)),
        Some(&founding("ada@example.test", "op-1"))
    );
    assert_eq!(
        reopened
            .unspent_for_email("ada@example.test")
            .map(|(d, _)| d),
        Some(digest(&token).as_str())
    );
    Ok(())
}

#[test]
fn a_second_founding_for_the_address_replaces_the_first_in_one_write() -> Result<(), ServerError>
{
    let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
    let file = dir.path().join("foundings.json");
    let mut foundings = Foundings::open(file.clone())?;
    foundings.record("d1".to_owned(), founding("ada@example.test", "op-1"), None)?;
    let (old, _) = foundings
        .unspent_for_email("ada@example.test")
        .map(|(d, f)| (d.to_owned(), f.clone()))
        .ok_or(ServerError::ProofRefused)?;
    foundings.record(
        "d2".to_owned(),
        founding("ada@example.test", "op-1"),
        Some(&old),
    )?;
    assert!(foundings.by_digest("d1").is_none());
    assert_eq!(
        foundings
            .unspent_for_email("ada@example.test")
            .map(|(d, _)| d),
        Some("d2")
    );
    foundings.spend("d2")?;
    assert!(foundings.unspent_for_email("ada@example.test").is_none());
    assert!(foundings.by_digest("d2").is_some_and(|f| f.spent));
    assert!(matches!(
        foundings.spend("d9"),
        Err(ServerError::ProofRefused)
    ));
    let reopened = Foundings::open(file)?;
    assert!(reopened.by_digest("d2").is_some_and(|f| f.spent));
    Ok(())
}

#[test]
fn a_file_of_another_format_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let file = dir.path().join("foundings.json");
    std::fs::write(&file, br#"{"format":"other/1","foundings":{}}"#).expect("written");
    assert!(matches!(
        Foundings::open(file),
        Err(ServerError::DirectoryUnavailable { .. })
    ));
}

#[test]
fn withdrawing_an_admission_keeps_its_record_and_a_later_link_works() -> Result<(), ServerError>
{
    let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
    let file = dir.path().join("foundings.json");
    let mut held = Foundings::open(file.clone())?;
    let mut admission = founding("ada@example.test", "op-1");
    admission.kind = Kind::Admission;
    held.record("d1".to_owned(), admission.clone(), None)?;
    held.withdraw("d1", vec![1, 2, 3])?;
    assert!(held.unspent_admission_for("person-1").is_none());
    assert!(held.by_digest("d1").is_some_and(|f| f.withdrawn));
    assert_eq!(
        held.by_digest("d1")
            .and_then(|f| f.withdrawal_event.as_deref()),
        Some([1, 2, 3].as_slice())
    );
    let mut held = Foundings::open(file)?;
    assert!(held.by_digest("d1").is_some_and(|f| f.withdrawn));
    held.record("d2".to_owned(), admission, None)?;
    assert_eq!(held.unspent_admission_for("person-1"), Some("d2"));
    assert!(held.by_digest("d1").is_some_and(|f| f.withdrawn));
    assert!(matches!(
        held.withdraw("d1", vec![1, 2, 3]),
        Err(ServerError::NoUnspentAdmission)
    ));
    Ok(())
}

#[test]
fn a_refused_withdrawal_save_leaves_memory_and_disk_whole() -> Result<(), ServerError> {
    let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
    let file = dir.path().join("foundings.json");
    let mut held = Foundings::open(file.clone())?;
    let mut admission = founding("ada@example.test", "op-1");
    admission.kind = Kind::Admission;
    held.record("d1".to_owned(), admission, None)?;
    let before = std::fs::read(&file).map_err(|e| unusable(&file, &e))?;
    let writable = held.file.clone();
    held.file = file.join("refused");
    assert!(matches!(
        held.withdraw("d1", vec![1, 2, 3]),
        Err(ServerError::DirectoryUnavailable { .. })
    ));
    assert_eq!(held.unspent_admission_for("person-1"), Some("d1"));
    assert!(
        held.by_digest("d1")
            .is_some_and(|f| !f.withdrawn && f.withdrawal_event.is_none())
    );
    assert_eq!(
        std::fs::read(&writable).map_err(|e| unusable(&writable, &e))?,
        before
    );
    assert_eq!(
        Foundings::open(writable)?.unspent_admission_for("person-1"),
        Some("d1")
    );
    Ok(())
}

#[test]
fn a_retained_founding_is_read_without_rewriting_and_link_replacement_keeps_its_email() -> Result<(), ServerError> {
    let dir = tempfile::tempdir().map_err(|error| unusable(Path::new("tmp"), &error))?;
    let file = dir.path().join("foundings.json");
    let original = founding("dean@example.test", "op-1");
    std::fs::write(&file, serde_json::json!({"format":FORMAT, "foundings":{"old":original}}).to_string())
        .map_err(|error| unusable(&file, &error))?;
    let before = std::fs::read(&file).map_err(|error| unusable(&file, &error))?;
    let mut held = Foundings::open(file.clone())?;
    assert_eq!(held.email_for("person-1"), Some("dean@example.test"));
    assert!(!held.recorded_at_sign_in("person-1"));
    assert_eq!(std::fs::read(&file).map_err(|error| unusable(&file, &error))?, before);
    held.record("new".to_owned(), founding("annabel@example.test", "op-2"), Some("old"))?;
    assert_eq!(held.email_for("person-1"), Some("dean@example.test"));
    let reopened = Foundings::open(file)?;
    assert_eq!(reopened.email_for("person-1"), Some("dean@example.test"));
    Ok(())
}
