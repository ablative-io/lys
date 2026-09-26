use std::error::Error;
use std::path::PathBuf;

use super::*;

type TestResult = Result<(), Box<dyn Error>>;

fn shipped() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deploy/identity/rauthy-themes.json")
}

/// Rauthy's own dark default at the pinned commit, transcribed from
/// vendor/rauthy/src/data/src/entity/theme.rs (`default_dark`, lines
/// 459-472): the values every gap field keeps.
fn rauthy_default_dark() -> ThemeCss {
    ThemeCss {
        text: [34, 5, 75],
        text_high: [34, 7, 90],
        bg: [200, 40, 6],
        bg_high: [200, 20, 17],
        action: [34, 100, 59],
        accent: [265, 100, 53],
        error: [15, 100, 37],
        btn_text: "hsl(var(--bg))".to_string(),
        theme_sun: "hsla(var(--action) / .7)".to_string(),
        theme_moon: "hsla(var(--accent) / .85)".to_string(),
    }
}

/// Writes an edited copy of the shipped mapping and loads it.
fn load_edited(
    edit: impl Fn(&str) -> String,
) -> Result<Result<ThemeFile, IdentityError>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("themes.json");
    std::fs::write(&path, edit(&std::fs::read_to_string(shipped())?))?;
    Ok(load(&path))
}

#[test]
fn the_shipped_mapping_validates_and_sets_only_the_four_named_fields() -> TestResult {
    let file = load(&shipped())?;
    let mut applied = 0;
    for (client, accent) in [("platform", [30, 59, 59]), ("cambium", [136, 20, 46])] {
        let entry = file.for_client(client).ok_or("client not mapped")?;
        let default = rauthy_default_dark();
        let dark = entry.dark.apply(&default);
        assert_eq!(dark.text, [210, 10, 92]);
        assert_eq!(dark.bg, [210, 14, 5]);
        assert_eq!(dark.bg_high, [210, 12, 13]);
        assert_eq!(dark.accent, accent);
        // Every gap keeps Rauthy's own default.
        assert_eq!(dark.text_high, default.text_high);
        assert_eq!(dark.action, default.action);
        assert_eq!(dark.error, default.error);
        assert_eq!(dark.btn_text, default.btn_text);
        assert_eq!(dark.theme_sun, default.theme_sun);
        assert_eq!(dark.theme_moon, default.theme_moon);
        applied += 1;
    }
    assert_eq!(applied, 2);
    Ok(())
}

#[test]
fn hex_to_hsl_matches_the_estate_tokens() {
    let cases = [
        ([0x0C, 0x0E, 0x10], [210, 14, 5]),
        ([0x1E, 0x22, 0x26], [210, 12, 13]),
        ([0xE8, 0xEA, 0xEC], [210, 10, 92]),
        ([0xD4, 0x97, 0x5A], [30, 59, 59]),
        ([0x5E, 0x8C, 0x6A], [136, 20, 46]),
        ([0xFF, 0xFF, 0xFF], [0, 0, 100]),
        ([0xFF, 0x00, 0x00], [0, 100, 50]),
    ];
    for (rgb, hsl) in cases {
        assert_eq!(hsl_of(rgb), hsl, "{rgb:?}");
    }
}

#[test]
fn contrast_follows_the_wcag_formula() {
    let white = [1.0, 1.0, 1.0];
    let black = [0.0, 0.0, 0.0];
    assert!((contrast(white, black) - 21.0).abs() < 1e-9);
    assert!((contrast(black, black) - 1.0).abs() < 1e-9);
    assert!((contrast(white, black) - contrast(black, white)).abs() < 1e-12);
}

#[test]
fn both_clients_meet_every_tier_with_the_gaps_at_rauthys_defaults() -> TestResult {
    let file = load(&shipped())?;
    let mut measured = 0;
    for client in ["platform", "cambium"] {
        let entry = file.for_client(client).ok_or("client not mapped")?;
        let dark = entry.dark.apply(&rauthy_default_dark());
        for measure in check_contrast(client, &dark)? {
            assert!(
                measure.ratio >= measure.tier.minimum(),
                "{client} {}",
                measure.pair
            );
            measured += 1;
        }
    }
    assert_eq!(measured, 12);
    Ok(())
}

#[test]
fn a_pair_below_its_tier_is_refused_by_name() -> TestResult {
    let mut dark = rauthy_default_dark();
    dark.accent = [210, 14, 6];
    dark.bg = [210, 14, 5];
    let message = check_contrast("cambium", &dark)
        .err()
        .ok_or("an unreadable pair was accepted")?
        .to_string();
    assert!(
        message.starts_with("contrast_below_tier: client cambium pair theme_moon over ink"),
        "got {message}"
    );
    Ok(())
}

#[test]
fn every_mapping_refusal_fires() -> TestResult {
    let cases: [(&str, &str, &str); 6] = [
        (
            "\"mode\": \"dark\"",
            "\"mode\": \"light\"",
            "mode must be dark",
        ),
        ("#D4975A", "#A78BFA", "banned purple"),
        (
            "\"token\": \"foundation.text\", \"hex\": \"#E8EAEC\"",
            "\"token\": \"foundation.muted\", \"hex\": \"#8A9199\"",
            "must come from foundation.text",
        ),
        (
            "\"hsl\": [30, 59, 59]",
            "\"hsl\": [30, 60, 59]",
            "converts to",
        ),
        (
            "\"product\": \"cambium\"",
            "\"product\": \"aion\"",
            "must wear the cambium accent",
        ),
        (
            "\"owner\": \"Waffles\"",
            "\"owner\": \"\"",
            "source must name",
        ),
    ];
    let mut refused = 0;
    for (from, to, expected) in cases {
        let loaded = load_edited(|text| text.replacen(from, to, 1))?;
        let message = loaded
            .err()
            .ok_or("an invalid mapping was accepted")?
            .to_string();
        assert!(message.starts_with("theme_invalid"), "got {message}");
        assert!(message.contains(expected), "{to}: got {message}");
        refused += 1;
    }
    assert_eq!(refused, cases.len());
    Ok(())
}
