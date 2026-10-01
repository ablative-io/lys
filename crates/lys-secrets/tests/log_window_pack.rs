#![cfg(test)]
//! A recorded fixture must be complete and must refuse altered bytes or metadata.

use std::error::Error;

#[path = "support/log_window_pack.rs"]
mod pack;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn recorded_pack_has_ten_thousand_hashed_records() -> TestResult {
    let (archive, manifest) = pack::read()?;
    let (manifest, frames) = pack::validate(&archive, &manifest)?;
    assert_eq!(frames.len(), pack::RECORDS + 4);
    assert!(frames.iter().all(|frame| !frame.is_empty()));
    assert_eq!(manifest.audit_key_fingerprint.len(), 64);
    assert_eq!(manifest.root.len(), 64);
    Ok(())
}

#[test]
fn altered_archive_is_refused() -> TestResult {
    let (mut archive, manifest) = pack::read()?;
    let last = archive.last_mut().ok_or("fixture archive is empty")?;
    *last ^= 1;
    let error = pack::validate(&archive, &manifest).unwrap_err();
    assert!(
        error.to_string().starts_with("FixtureDigestMismatch:"),
        "{error}"
    );
    Ok(())
}

#[test]
fn altered_manifest_is_refused() -> TestResult {
    let (archive, manifest) = pack::read()?;
    let original: serde_json::Value = serde_json::from_slice(&manifest)?;
    for (field, replacement, refusal) in [
        (
            "records",
            serde_json::json!(9_999),
            "FixtureManifestInvalid:",
        ),
        (
            "sha256",
            serde_json::json!("0".repeat(64)),
            "FixtureDigestMismatch:",
        ),
        (
            "frame_sha256",
            serde_json::json!([]),
            "FixtureManifestInvalid:",
        ),
    ] {
        let mut altered = original.clone();
        altered[field] = replacement;
        let error = pack::validate(&archive, &serde_json::to_vec(&altered)?).unwrap_err();
        assert!(error.to_string().starts_with(refusal), "{field}: {error}");
    }
    let mut altered = original;
    altered["frame_sha256"][0] = serde_json::json!("0".repeat(64));
    let error = pack::validate(&archive, &serde_json::to_vec(&altered)?).unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("FixtureFrameMismatch: frame 0"),
        "{error}"
    );
    Ok(())
}
