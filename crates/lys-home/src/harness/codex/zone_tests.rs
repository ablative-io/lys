#![cfg(test)]
//! Gates on the translation's checks (HOME-009 R1): the measured Codex
//! version only, an IANA zone name resolved through the bundled database,
//! and the refusals that name the act answering each.

use std::error::Error;
use std::path::PathBuf;

use crate::error::HomeError;
use crate::harness::codex::zone::{check_version, local_time, zone_of};

type Gate = Result<(), Box<dyn Error>>;

#[test]
fn version_other_than_measured_is_refused() -> Gate {
    let refused = check_version("0.157.0");
    assert!(matches!(
        refused,
        Err(HomeError::UnmeasuredCodexVersion { .. })
    ));
    assert_eq!(
        refused.err().map(|e| e.to_string()).as_deref(),
        Some(
            "Codex 0.157.0 has no measured rollout shape: render for 0.156.0 or card a measurement of the new version"
        )
    );
    assert!(matches!(
        check_version("0.156"),
        Err(HomeError::UnmeasuredCodexVersion { .. })
    ));
    check_version("0.156.0")?;
    Ok(())
}

#[test]
fn unnamed_zone_is_refused() {
    let absent = zone_of(None);
    assert!(matches!(absent, Err(HomeError::UnnamedTimeZone { .. })));
    assert_eq!(
        absent.err().map(|e| e.to_string()).as_deref(),
        Some("TZ unset is not an IANA time zone name: set TZ to an IANA name")
    );
    for value in [
        "",
        ":/etc/localtime",
        "/Australia/Sydney",
        "Australia//Sydney",
    ] {
        assert!(
            matches!(zone_of(Some(value)), Err(HomeError::UnnamedTimeZone { .. })),
            "{value}"
        );
    }
}

#[test]
fn unknown_zone_is_refused() {
    let refused = zone_of(Some("Mars/Olympus"));
    assert!(matches!(refused, Err(HomeError::UnknownTimeZone { .. })));
    assert_eq!(
        refused.err().map(|e| e.to_string()).as_deref(),
        Some("time zone Mars/Olympus is not in the time zone database: set TZ to an IANA name")
    );
}

#[test]
fn named_zone_resolves() -> Gate {
    let sydney = zone_of(Some("Australia/Sydney"))?;
    let local = local_time("e1", "2000-01-02T03:04:05.678Z", &sydney)?;
    assert_eq!(local.to_string(), "2000-01-02T14:04:05");
    let utc = zone_of(Some("UTC"))?;
    let local = local_time("e1", "2000-01-02T03:04:05.678Z", &utc)?;
    assert_eq!(local.to_string(), "2000-01-02T03:04:05");
    Ok(())
}

#[test]
fn translation_refusals_name_the_act() {
    let exists = HomeError::TranslationTargetExists {
        path: PathBuf::from("a/b.jsonl"),
    };
    assert_eq!(
        exists.to_string(),
        "translate-codex target already exists: a/b.jsonl; choose another --out"
    );
    let stamp = HomeError::StampNotRfc3339 {
        entry: "e1".to_owned(),
    };
    assert_eq!(
        stamp.to_string(),
        "entry e1 has a stamp that is not RFC 3339: re-import the source file"
    );
    let old = HomeError::Exists {
        path: PathBuf::from("a/b.jsonl"),
    };
    assert_eq!(old.to_string(), "session file already exists: a/b.jsonl");
}
