use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use lys_identity::LoginBinding;

use super::{read_administrator, record_administrator};

#[test]
fn recording_an_administrator_preserves_the_previous_file_until_publication()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("administrator.json");
    let previous = LoginBinding::new("https://issuer.example.test", "previous")?;
    let next = LoginBinding::new("https://issuer.example.test", "next")?;
    record_administrator(&path, &previous)?;
    let held = directory.path().join("previous.json");
    std::fs::hard_link(&path, &held)?;
    record_administrator(&path, &next)?;
    assert_eq!(read_administrator(&path)?, Some(next));
    assert_eq!(read_administrator(&held)?, Some(previous));
    Ok(())
}

#[test]
fn a_recorded_administrator_is_private() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("administrator.json");
    std::fs::write(&path, b"old file")?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    record_administrator(
        &path,
        &LoginBinding::new("https://issuer.example.test", "administrator")?,
    )?;
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o777,
        0o600
    );
    Ok(())
}
