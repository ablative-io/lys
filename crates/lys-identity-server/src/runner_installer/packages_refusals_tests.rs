#![cfg(test)]
//! Every refusal the installer names is produced here, each by the failure
//! it names: an oversize or long-named archive member, a writer that
//! refuses, a package tool that is missing or fails, a package tool that
//! leaves no package or the wrong kind, and a staging folder that stays.

use std::error::Error;
use std::path::Path;
use std::process::Command;

use zeroize::Zeroizing;

use super::{ar_file, build_macos, gzip_into, octal, run_tool, settled, tar_file};
use crate::error::ServerError;

type TestResult = Result<(), Box<dyn Error>>;

fn refusal(error: &ServerError) -> &str {
    match error {
        ServerError::Runner { refusal, .. } => refusal,
        _ => "",
    }
}

#[test]
fn a_member_past_the_format_limit_is_named() {
    let mut field = [0_u8; 4];
    let error = octal(&mut field, 0o7777).err();
    assert_eq!(error.as_ref().map(refusal), Some("installer_archive_limit"));
    let mut ar = Vec::new();
    let error = ar_file(&mut ar, "a-member-name-longer-than-sixteen", b"x").err();
    assert_eq!(error.as_ref().map(refusal), Some("installer_archive_limit"));
    assert!(ar.is_empty(), "nothing is written for a refused member");
}

#[test]
fn a_long_or_non_ascii_member_path_is_named() {
    let mut tar = Vec::new();
    for path in ["a/".repeat(51), "lys/é".to_owned()] {
        let error = tar_file(&mut tar, &path, b"x", 0o644).err();
        assert_eq!(error.as_ref().map(refusal), Some("installer_archive_path"));
    }
    assert!(tar.is_empty(), "nothing is written for a refused path");
}

struct Refusing;

impl std::io::Write for Refusing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("the archive's writer is full"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_writer_that_refuses_the_archive_is_named() {
    let tar = Zeroizing::new(vec![0_u8; 4096]);
    let error = gzip_into(tar, Refusing).err();
    assert_eq!(
        error.as_ref().map(refusal),
        Some("installer_archive_failed")
    );
    assert!(error.is_some_and(|error| error.to_string().contains("the archive's writer is full")));
}

#[test]
fn a_package_tool_that_is_missing_or_fails_is_named_by_its_tool() {
    for (name, unavailable, failed) in [
        (
            "pkgbuild",
            "installer_pkgbuild_unavailable",
            "installer_pkgbuild_failed",
        ),
        (
            "productbuild",
            "installer_productbuild_unavailable",
            "installer_productbuild_failed",
        ),
    ] {
        let missing = Command::new("/nonexistent/lys-package-tool");
        let error = run_tool(missing, name).err();
        assert_eq!(error.as_ref().map(refusal), Some(unavailable));

        let mut failing = Command::new("/bin/sh");
        failing.args(["-c", "echo the signing identity is missing >&2; exit 3"]);
        let error = run_tool(failing, name).err();
        assert_eq!(error.as_ref().map(refusal), Some(failed));
        assert!(
            error.is_some_and(|error| error.to_string().contains("signing identity is missing"))
        );
    }
}

fn binaries(folder: &Path) -> Result<[std::path::PathBuf; 2], Box<dyn Error>> {
    let lys = folder.join("lys");
    let entry = folder.join("lys-containment-entry");
    std::fs::write(&lys, b"runner")?;
    std::fs::write(&entry, b"containment")?;
    Ok([lys, entry])
}

#[test]
fn a_package_tool_that_leaves_no_package_or_the_wrong_kind_is_named() -> TestResult {
    let sources = tempfile::tempdir()?;
    let [lys, entry] = binaries(sources.path())?;
    let root = tempfile::tempdir()?;
    let error = build_macos(root.path(), b"{}", [&lys, &entry], "0.0.0", |_, _| Ok(())).err();
    assert_eq!(
        error.as_ref().map(refusal),
        Some("installer_package_unreadable")
    );
    let root = tempfile::tempdir()?;
    let error = build_macos(root.path(), b"{}", [&lys, &entry], "0.0.0", |root, _| {
        std::fs::write(root.join("lys.pkg"), b"PK\x03\x04 a zip, not xar").map_err(|error| {
            ServerError::Runner {
                refusal: "fixture".to_owned(),
                words: error.to_string(),
            }
        })
    })
    .err();
    assert_eq!(
        error.as_ref().map(refusal),
        Some("installer_package_invalid")
    );
    Ok(())
}

#[test]
fn a_staging_folder_that_stays_is_named_beside_the_build_failure() {
    let stayed = || Err(std::io::Error::other("the staging folder is busy"));
    let error = settled(Ok(vec![1_u8]), stayed()).err();
    assert_eq!(
        error.as_ref().map(refusal),
        Some("installer_cleanup_failed")
    );
    let build = ServerError::Runner {
        refusal: "installer_pkgbuild_failed".to_owned(),
        words: "the build failed".to_owned(),
    };
    let error = settled::<Vec<u8>>(Err(build), stayed()).err();
    assert_eq!(
        error.as_ref().map(refusal),
        Some("installer_cleanup_failed")
    );
    assert!(error.is_some_and(|error| {
        let words = error.to_string();
        words.contains("the build failed") && words.contains("the staging folder is busy")
    }));
    assert_eq!(settled(Ok(7), Ok(())).ok(), Some(7));
}
