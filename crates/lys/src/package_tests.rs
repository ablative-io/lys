#![cfg(test)]

use std::path::Path;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// Bytes shaped as a binary holding `name`'s version line for `build`
/// among other text.
fn binary(name: &str, build: &str) -> Vec<u8> {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "\x00\x7fELF junk {name} usage text{version} (not a stamp) more {version} ({build})tail\x00"
    )
    .into_bytes()
}

#[test]
fn the_build_stamp_is_read_from_the_binary_itself() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        stamp_in(&binary("lys", COMMIT), version),
        Some(COMMIT.to_string())
    );
    let dirty = format!("{COMMIT}; dirty");
    assert_eq!(stamp_in(&binary("lys", &dirty), version), Some(dirty));
    let words = "not built from a git commit";
    assert_eq!(
        stamp_in(&binary("lys", words), version),
        Some(words.to_string())
    );
    assert_eq!(stamp_in(b"no stamp here", version), None);
    let two = [binary("lys", COMMIT), binary("lys", OTHER)].concat();
    assert_eq!(
        stamp_in(&two, version),
        None,
        "two stamps name no one build"
    );
}

/// Binaries built from two commits are refused, naming the binary that
/// differs, and the check fires only on the difference.
#[test]
fn binaries_from_two_commits_are_refused_naming_the_binary() {
    let reading = |who: &str, build: &str| (who.to_string(), build.to_string());
    let same: Vec<(String, String)> = BINARIES
        .iter()
        .map(|name| reading(&format!("arm64/{name}"), COMMIT))
        .collect();
    assert_eq!(same_build(&same).ok(), Some(COMMIT.to_string()));
    let mut mixed = same;
    mixed.push(reading("x86_64/lys-secrets", OTHER));
    let refused = same_build(&mixed).err();
    assert_eq!(
        refused.as_ref().map(|error| error.name),
        Some("build_mismatch")
    );
    let detail = refused.map(|error| error.detail).unwrap_or_default();
    assert!(
        detail.contains("x86_64/lys-secrets is built from 8"),
        "{detail}"
    );
    assert_eq!(
        same_build(&[]).err().map(|error| error.name),
        Some("binary_missing")
    );
}

#[test]
fn with_no_signing_identity_a_release_is_refused_by_name() {
    for missing in [None, Some(""), Some("  ")] {
        let refused = Signing::from_flags(missing, Some("lys-notary"), false).err();
        assert_eq!(
            refused.map(|error| error.name),
            Some("signing_identity_missing")
        );
    }
    let refused = Signing::from_flags(Some("Developer ID Application: X (T)"), None, false).err();
    assert_eq!(
        refused.map(|error| error.name),
        Some("notary_profile_missing")
    );
}

/// Without a Developer ID the app is named for development, never with the
/// release name.
#[test]
fn a_development_build_never_carries_the_release_name() -> TestResult {
    let development = Signing::from_flags(None, None, true)?;
    assert_eq!(development.name(), "Lys (development)");
    assert_ne!(development.bundle_id(), "au.com.ablative.lys");
    assert_eq!(development.codesign_args(), ["--force", "--sign", "-"]);
    let release = Signing::from_flags(Some("Developer ID Application: X (T)"), Some("p"), false)?;
    assert_eq!(release.name(), "Lys");
    let args = release.codesign_args();
    assert!(
        args.windows(2).any(|pair| pair == ["--options", "runtime"]),
        "{args:?}"
    );
    assert!(args.contains(&"--timestamp".to_string()));
    Ok(())
}

#[test]
fn the_property_list_names_the_app_its_executable_and_its_build() -> TestResult {
    let signing = Signing::from_flags(None, None, true)?;
    let info = render_info(&signing, "0.2.0", COMMIT);
    assert!(info.contains("<key>CFBundleExecutable</key>\n\t<string>lys-app</string>"));
    assert!(info.contains("<string>Lys (development)</string>"));
    assert!(info.contains("<string>au.com.ablative.lys.development</string>"));
    assert!(info.contains(&format!("<key>LysBuild</key>\n\t<string>{COMMIT}</string>")));
    assert!(info.contains("<key>CFBundleIconFile</key>\n\t<string>Lys</string>"));
    assert!(!info.contains("{{"), "{info}");
    Ok(())
}

/// Two folders of binaries whose builds are `arm64` and `x86_64`.
fn folders(root: &Path, arm64: &str, x86_64: &str) -> std::io::Result<()> {
    for (arch, build) in [("arm64", arm64), ("x86_64", x86_64)] {
        std::fs::create_dir_all(root.join(arch))?;
        for name in BINARIES {
            std::fs::write(root.join(arch).join(name), binary(name, build))?;
        }
    }
    Ok(())
}

/// Folders built from two commits are refused before anything is written,
/// naming the binary.
#[test]
fn packaging_binaries_from_two_commits_writes_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    folders(root.path(), COMMIT, OTHER)?;
    let out = root.path().join("out");
    let options = Options {
        out: &out,
        arm64: &root.path().join("arm64"),
        x86_64: &root.path().join("x86_64"),
        surface: &root.path().join("surface"),
        signing: Signing::Development,
    };
    let refused = run(&options, true).err().ok_or("it was not refused")?;
    assert_eq!(refused.name, "build_mismatch");
    assert!(
        refused.detail.starts_with("x86_64/lys-app is built from"),
        "{}",
        refused.detail
    );
    assert!(!out.exists(), "the output folder was made");
    Ok(())
}

/// A release whose Developer ID the keychain does not hold is refused
/// `signing_identity_missing`, and no `Lys.dmg` is written.
#[test]
fn a_release_without_its_signing_identity_writes_no_disk_image() -> TestResult {
    let root = tempfile::tempdir()?;
    folders(root.path(), COMMIT, COMMIT)?;
    let out = root.path().join("out");
    let identity = format!("Developer ID Application: no such identity {OTHER} (NONE)");
    let options = Options {
        out: &out,
        arm64: &root.path().join("arm64"),
        x86_64: &root.path().join("x86_64"),
        surface: &root.path().join("surface"),
        signing: Signing::from_flags(Some(&identity), Some("lys-notary"), false)?,
    };
    let refused = run(&options, true).err().map(|error| error.name);
    assert_eq!(refused, Some("signing_identity_missing"));
    assert!(!out.join("Lys.dmg").exists(), "a Lys.dmg was written");
    assert!(!out.exists(), "the output folder was made");
    Ok(())
}

/// A folder short of a binary is refused by name before anything is
/// written.
#[test]
fn a_missing_binary_is_refused_by_name() -> TestResult {
    let root = tempfile::tempdir()?;
    folders(root.path(), COMMIT, COMMIT)?;
    std::fs::remove_file(root.path().join("x86_64/lys-home"))?;
    let out = root.path().join("out");
    let options = Options {
        out: &out,
        arm64: &root.path().join("arm64"),
        x86_64: &root.path().join("x86_64"),
        surface: &root.path().join("surface"),
        signing: Signing::Development,
    };
    let refused = run(&options, true).err().map(|error| error.name);
    assert_eq!(refused, Some("binary_missing"));
    assert!(!out.exists());
    Ok(())
}
