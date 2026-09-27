//! Gates on the version and zone checks (HOME-009 R1): only 0.156.0 is
//! rendered, compared exactly; an unnamed zone and an unknown zone are
//! refused by name; a named zone resolves in the bundled database; and the
//! translation's own refusals name their act while the shared
//! session-exists refusal is unchanged.

use std::error::Error;
use std::path::PathBuf;

use crate::error::HomeError;
use crate::harness::codex::zone::{Local, check_version, parse_stamp, resolve_zone};

type Gate = Result<(), Box<dyn Error>>;

/// The refusal a check ended in; a check that passed is a failure here.
fn refusal<T>(result: Result<T, HomeError>) -> Result<HomeError, Box<dyn Error>> {
    match result {
        Ok(_) => Err("the check passed".into()),
        Err(refused) => Ok(refused),
    }
}

#[test]
fn version_other_than_measured_is_refused() -> Gate {
    let refused = refusal(check_version("0.157.0"))?;
    assert_eq!(
        refused.to_string(),
        "Codex 0.157.0 has no measured rollout shape: render for 0.156.0 or card a measurement of the new version"
    );
    let prefix = refusal(check_version("0.156"))?;
    assert!(matches!(prefix, HomeError::UnmeasuredCodexVersion { .. }));
    check_version("0.156.0")?;
    Ok(())
}

#[test]
fn unnamed_zone_is_refused() -> Gate {
    let absent = refusal(resolve_zone(None))?;
    assert_eq!(
        absent.to_string(),
        "TZ unset is not an IANA time zone name: set TZ to an IANA name"
    );
    let mut refused = 0;
    for value in ["", ":/etc/localtime"] {
        let error = refusal(resolve_zone(Some(value)))?;
        assert!(matches!(error, HomeError::UnnamedTimeZone { .. }));
        refused += 1;
    }
    assert_eq!(refused, 2);
    Ok(())
}

#[test]
fn unknown_zone_is_refused() -> Gate {
    let error = refusal(resolve_zone(Some("Mars/Olympus")))?;
    assert_eq!(
        error.to_string(),
        "time zone Mars/Olympus is not in the time zone database: set TZ to an IANA name"
    );
    assert!(matches!(error, HomeError::UnknownTimeZone { .. }));
    Ok(())
}

#[test]
fn named_zone_resolves() -> Gate {
    let sydney = resolve_zone(Some("Australia/Sydney"))?;
    let instant = parse_stamp("e1", "2000-01-02T03:04:05.678Z")?;
    assert_eq!(Local::of(instant, &sydney).iso(), "2000-01-02T14:04:05");
    let utc = resolve_zone(Some("UTC"))?;
    assert_eq!(Local::of(instant, &utc).iso(), "2000-01-02T03:04:05");
    Ok(())
}

#[test]
fn translation_refusals_name_the_act() {
    let target = HomeError::TranslationTargetExists {
        path: PathBuf::from("a/b.jsonl"),
    };
    assert_eq!(
        target.to_string(),
        "translate-codex target already exists: a/b.jsonl; choose another --out"
    );
    let id = "e1".to_owned();
    let stamp = HomeError::StampNotRfc3339 { id };
    assert_eq!(
        stamp.to_string(),
        "entry e1 has a stamp that is not RFC 3339: re-import the source file"
    );
    let other = HomeError::Exists {
        path: PathBuf::from("a/b.jsonl"),
    };
    assert_eq!(other.to_string(), "session file already exists: a/b.jsonl");
}
