#![cfg(test)]

use std::ffi::OsString;
use std::path::Path;

use super::*;

#[test]
fn the_helper_is_beside_the_service_and_the_root_above_its_folder() {
    let own = Path::new("/data/lys/identity/bin/lys-identity-server");
    let (helper, root) = placement(own).unwrap_or_default();
    assert_eq!(helper, Path::new("/data/lys/identity/bin/lys-app"));
    assert_eq!(root, Path::new("/data/lys/identity"));
    assert_eq!(placement(Path::new("/")), None);
}

#[test]
fn the_data_folder_is_removed_only_when_asked() {
    let root = Path::new("/data/lys/identity");
    let kept: Vec<OsString> = ["--uninstall", "--root", "/data/lys/identity"]
        .map(OsString::from)
        .to_vec();
    assert_eq!(helper_args(root, false), kept);
    let mut removed = kept;
    removed.push("--remove-data".into());
    assert_eq!(helper_args(root, true), removed);
}

#[test]
fn what_is_lost_is_named_in_plain_words() {
    assert_eq!(LOST.len(), 5);
    for lost in LOST {
        let lower = lost.to_lowercase();
        assert!(!lower.contains("rauthy"), "{lost}");
        assert!(!lower.contains("postgres"), "{lost}");
    }
}

#[test]
fn a_removal_is_refused_without_the_confirmation() -> Result<(), serde_json::Error> {
    let body: UninstallBody = serde_json::from_str(r#"{"remove_data":true}"#)?;
    assert!(body.remove_data);
    assert!(!body.confirmed);
    let unknown = serde_json::from_str::<UninstallBody>(r#"{"remove_data":false,"all":true}"#);
    assert!(unknown.is_err());
    Ok(())
}
