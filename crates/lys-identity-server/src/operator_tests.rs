#![cfg(test)]
//! Permission repair is observable; a failed repair never releases the token.

use std::cell::RefCell;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use super::{read, read_protected, token_at};
use crate::config::Profile;
use crate::error::ServerError;

type TestResult = Result<(), Box<dyn Error>>;
const TOKEN: &str = "operator-permissions-fixture-0123456789abcdef";

#[test]
fn wider_token_permissions_are_tightened_and_recorded_without_the_secret() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("operator.token");
    std::fs::write(&path, TOKEN)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    let said = RefCell::new(Vec::new());
    let record = |line: &str| said.borrow_mut().push(line.to_owned());
    assert_eq!(read(&path, &record)?.as_str(), TOKEN);
    assert_eq!(
        std::fs::metadata(&path)?.permissions().mode() & 0o7777,
        0o600
    );
    assert_eq!(said.borrow().len(), 1);
    assert!(said.borrow()[0].contains("tightened to 0600"));
    assert!(!said.borrow()[0].contains(TOKEN));
    assert_eq!(read(&path, &record)?.as_str(), TOKEN);
    assert_eq!(said.borrow().len(), 1, "already private needs no repair");
    Ok(())
}

#[test]
fn a_permission_repair_error_is_named_and_releases_no_token() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("operator.token");
    std::fs::write(&path, TOKEN)?;
    // Inject the filesystem's chmod refusal, independent of the test user's uid.
    let refused = read_protected(&path, |_| {
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    });
    let Err(ServerError::ConfigInvalid { reason }) = refused else {
        return Err("a failed permission repair must refuse ConfigInvalid".into());
    };
    assert!(reason.contains("cannot be protected and read"));
    assert!(!reason.contains(TOKEN));
    assert_eq!(std::fs::read_to_string(path)?, TOKEN);
    Ok(())
}

#[test]
fn a_service_install_refuses_a_named_operator_token_and_a_development_install_reads_it()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("operator-token");
    std::fs::write(&path, TOKEN)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let said = RefCell::new(Vec::new());
    let say = |line: &str| said.borrow_mut().push(line.to_owned());
    let refused = token_at(Profile::Service, Some(&path), &say);
    match refused {
        Err(ServerError::ConfigInvalid { reason }) => {
            assert!(
                reason.contains("a service install keeps no operator token"),
                "{reason}"
            );
            assert!(reason.contains("set profile to development"), "{reason}");
            assert!(
                !reason.contains(TOKEN),
                "the refusal never carries the token"
            );
        }
        other => return Err(format!("a service install read the token: {other:?}").into()),
    }
    assert_eq!(
        token_at(Profile::Development, Some(&path), &say)?
            .as_deref()
            .map(String::as_str),
        Some(TOKEN)
    );
    assert_eq!(token_at(Profile::Service, None, &say)?, None);
    assert!(
        said.borrow().is_empty(),
        "nothing to say: {:?}",
        said.borrow()
    );
    Ok(())
}

#[test]
fn the_profile_is_service_unless_the_configuration_says_development() -> TestResult {
    assert_eq!(
        serde_json::from_str::<Profile>("\"service\"")?,
        Profile::Service
    );
    assert_eq!(
        serde_json::from_str::<Profile>("\"development\"")?,
        Profile::Development
    );
    assert!(serde_json::from_str::<Profile>("\"dev\"").is_err());
    assert_eq!(Profile::default(), Profile::Service);
    Ok(())
}
