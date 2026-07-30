//! Tests for validity-window parsing.
//!
//! The refusals matter more than the acceptances here. A validity window that
//! parses to something *shorter* than the operator asked for is the dangerous
//! direction — it looks like it worked — so every ambiguous spec is pinned as
//! refused rather than left to whatever `str::parse` happens to do with it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn every_unit_converts_to_the_seconds_it_names() {
    assert_eq!(parse_validity("1s").unwrap(), Duration::from_secs(1));
    assert_eq!(parse_validity("90s").unwrap(), Duration::from_secs(90));
    assert_eq!(parse_validity("1m").unwrap(), Duration::from_secs(60));
    assert_eq!(parse_validity("30m").unwrap(), Duration::from_secs(1_800));
    assert_eq!(parse_validity("1h").unwrap(), Duration::from_secs(3_600));
    assert_eq!(parse_validity("12h").unwrap(), Duration::from_secs(43_200));
    assert_eq!(parse_validity("1d").unwrap(), Duration::from_secs(86_400));
    assert_eq!(parse_validity("7d").unwrap(), Duration::from_secs(604_800));
}

#[test]
fn the_short_windows_the_flag_exists_for_are_expressible() {
    // The gap this closes: `--validity-days` has a floor of one whole day, so
    // none of these could be asked for from the CLI before.
    for spec in ["1m", "5m", "15m", "30m", "1h", "2h"] {
        let window = parse_validity(spec).unwrap();
        assert!(
            window < Duration::from_secs(SECONDS_PER_DAY),
            "{spec} should be under a day"
        );
        assert!(window > Duration::ZERO, "{spec} should be positive");
    }
}

#[test]
fn a_compound_spec_is_refused_rather_than_silently_truncated() {
    // The failure mode worth preventing: reading `1h30m` as one hour, or as
    // thirty minutes, and reporting success either way.
    for spec in ["1h30m", "2d12h", "1m1s"] {
        assert!(
            parse_validity(spec).is_err(),
            "{spec} must be refused, not partially read"
        );
    }
}

#[test]
fn a_zero_length_window_is_refused() {
    // notBefore == notAfter is valid at no instant at all.
    for spec in ["0s", "0m", "0h", "0d", "00d"] {
        assert!(parse_validity(spec).is_err(), "{spec} must be refused");
    }
}

#[test]
fn malformed_specs_are_refused_with_their_reason() {
    for spec in [
        "",       // empty
        "30",     // no unit
        "m",      // no count
        "30x",    // unknown unit
        "30M",    // units are lowercase
        "-30m",   // signs are not digits
        "+30m",   //
        "3 0m",   // internal space
        " 30m",   // leading space
        "30m ",   // trailing space (the unit is then ' ')
        "1_000m", // digit separators are not digits
        "1.5h",   // fractions are not supported
        "٣٠m",    // non-ASCII digits
    ] {
        let error = parse_validity(spec).expect_err("must be refused");
        // The spec is echoed so the operator can see what was actually read,
        // and the reason names the fix.
        let rendered = error.to_string();
        assert!(
            rendered.contains("invalid --validity"),
            "unexpected message for {spec:?}: {rendered}"
        );
    }
}

#[test]
fn an_overflowing_window_fails_loudly_instead_of_wrapping() {
    // Wrapping would produce a *shorter* window than asked for while reporting
    // success, which is the one outcome that must not be possible.
    let huge = format!("{}d", u64::MAX);
    assert!(parse_validity(&huge).is_err());
    assert!(parse_validity("99999999999999999999999d").is_err());

    // The largest window that still fits is accepted, so the check rejects only
    // what genuinely cannot be represented.
    let max_days = u64::MAX / SECONDS_PER_DAY;
    assert!(parse_validity(&format!("{max_days}d")).is_ok());
}

#[test]
fn the_days_flag_keeps_its_exact_previous_meaning() {
    // Backward compatibility, stated as a test: `--validity-days N` must still
    // be N whole days, unchanged by the new flag's existence.
    for days in [1u32, 2, 30, 365] {
        assert_eq!(
            validity_window(Some(days), None).unwrap(),
            Duration::from_secs(u64::from(days) * SECONDS_PER_DAY)
        );
    }
    // And the two flags agree where they overlap.
    assert_eq!(
        validity_window(Some(7), None).unwrap(),
        validity_window(None, Some("7d")).unwrap()
    );
}

#[test]
fn the_days_flag_wins_if_both_somehow_arrive() {
    // Unreachable through the CLI (clap's argument group forbids it), but the
    // resolution is deterministic rather than accidental: the older flag's
    // meaning is preserved if the group is ever misdeclared.
    assert_eq!(
        validity_window(Some(1), Some("30m")).unwrap(),
        Duration::from_secs(SECONDS_PER_DAY)
    );
}

#[test]
fn neither_flag_is_an_error_not_a_default() {
    // A silently-defaulted validity window is exactly the kind of invisible
    // policy this crate should never invent.
    assert!(validity_window(None, None).is_err());
}
