//! Installed copies are discovered and cached on an explicit search path.

use std::error::Error;

use lys_identity_server::error::ServerError;
use lys_identity_server::harness_catalogue::Catalogue;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn discovery_catalogue() -> Result<Catalogue, ServerError> {
    let mut source: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))
    .map_err(|error| ServerError::HarnessCatalogueUnreadable {
        file: "fixture.json".to_owned(),
        reason: error.to_string(),
    })?;
    source["command"] = json!("claude");
    let text = serde_json::to_string(&source).map_err(|error| {
        ServerError::HarnessCatalogueUnreadable {
            file: "fixture.json".to_owned(),
            reason: error.to_string(),
        }
    })?;
    Catalogue::read(&[("fixture.json", &text)])
}

fn fake_command(folder: &std::path::Path, body: &str) -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let file = folder.join("claude");
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n"))?;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[test]
fn a_command_on_an_injected_path_is_offered_without_any_profile() -> TestResult {
    let directory = tempfile::tempdir()?;
    fake_command(
        directory.path(),
        "test \"$1\" = --version || exit 9\nprintf 'fake-claude 1.2.3\\n'",
    )?;
    let answer = discovery_catalogue()?.installed_in(directory.path().as_os_str())?;
    let value = serde_json::to_value(answer)?;
    assert_eq!(value["programs"][0]["builds"][0]["from"], "installed");
    assert_eq!(
        value["programs"][0]["builds"][0]["program"],
        std::fs::canonicalize(directory.path().join("claude"))?
            .to_str()
            .ok_or("path is not UTF-8")?
    );
    assert_eq!(
        value["programs"][0]["builds"][0]["package"],
        "fake-claude 1.2.3"
    );
    assert!(value["programs"][0]["not_found"].is_null());
    Ok(())
}

#[test]
fn the_installed_version_is_queried_once_for_the_same_path() -> TestResult {
    let directory = tempfile::tempdir()?;
    fake_command(
        directory.path(),
        &format!(
            "printf 'called\\n' >> '{}'\nprintf 'fake-claude 1.2.3\\n'",
            directory.path().join("calls").display()
        ),
    )?;
    let catalogue = discovery_catalogue()?;
    catalogue.installed_in(directory.path().as_os_str())?;
    catalogue.installed_in(directory.path().as_os_str())?;
    assert_eq!(
        std::fs::read_to_string(directory.path().join("calls"))?,
        "called\n"
    );
    Ok(())
}

#[test]
fn a_missing_command_has_no_copy_and_a_served_reason() -> TestResult {
    let directory = tempfile::tempdir()?;
    let answer =
        serde_json::to_value(discovery_catalogue()?.installed_in(directory.path().as_os_str())?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("CommandNotFound")
    );
    Ok(())
}

#[test]
fn a_failed_version_never_falls_through_to_a_second_copy() -> TestResult {
    let broken = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    fake_command(broken.path(), "exit 7")?;
    fake_command(other.path(), "printf 'other-version\\n'")?;
    let path = std::env::join_paths([broken.path(), other.path()])?;
    let answer = serde_json::to_value(discovery_catalogue()?.installed_in(&path)?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("VersionCommandFailed")
    );
    Ok(())
}

#[test]
fn unreadable_version_output_names_its_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    fake_command(directory.path(), "printf '\\377'")?;
    let answer =
        serde_json::to_value(discovery_catalogue()?.installed_in(directory.path().as_os_str())?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("VersionOutputUnreadable")
    );
    Ok(())
}

#[test]
fn a_relative_path_entry_is_refused_without_running_a_copy() -> TestResult {
    let answer = serde_json::to_value(
        discovery_catalogue()?.installed_in(std::ffi::OsStr::new("relative"))?,
    )?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("UnsafeSearchPath")
    );
    Ok(())
}

#[test]
fn a_copy_installed_after_a_missing_read_is_discovered() -> TestResult {
    let directory = tempfile::tempdir()?;
    let catalogue = discovery_catalogue()?;
    let missing = serde_json::to_value(catalogue.installed_in(directory.path().as_os_str())?)?;
    assert_eq!(missing["programs"][0]["builds"], json!([]));
    fake_command(directory.path(), "printf 'new-version\\n'")?;
    let found = serde_json::to_value(catalogue.installed_in(directory.path().as_os_str())?)?;
    assert_eq!(found["programs"][0]["builds"][0]["package"], "new-version");
    Ok(())
}

#[test]
fn a_path_with_control_characters_is_not_used_for_discovery() -> TestResult {
    let directory = tempfile::tempdir()?;
    let unsafe_path = directory.path().join("unsafe\nentry");
    std::fs::create_dir(&unsafe_path)?;
    fake_command(&unsafe_path, "printf 'must-not-run\\n'")?;
    let answer =
        serde_json::to_value(discovery_catalogue()?.installed_in(unsafe_path.as_os_str())?)?;
    assert_eq!(answer["programs"][0]["builds"], json!([]));
    assert!(
        answer["programs"][0]["not_found"]
            .as_str()
            .ok_or("missing reason")?
            .contains("UnsafeSearchPath")
    );
    Ok(())
}
