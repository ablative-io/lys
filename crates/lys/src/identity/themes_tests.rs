use std::error::Error;
use std::path::PathBuf;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MAPPING: &str = include_str!("../../../../deploy/identity/rauthy-themes.json");
const THEME_MAP: &str = include_str!("../../../../deploy/identity/theme-map.md");

fn mapping_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deploy/identity/rauthy-themes.json")
}

/// The pinned Rauthy's default theme, written out here from
/// `src/data/src/entity/theme.rs` at dd61ac3c84d6b238108dc8438b53043b5177a662
/// (`default_light`, `default_dark`, `border_radius`), as a second party to
/// the mapping: a gap must come out of [`ThemeMapping::apply`] equal to this.
fn pinned_default(client_id: &str) -> ThemeDocument {
    let mode = |text, text_high, bg, bg_high, action, btn_text: &str| ThemeColours {
        text,
        text_high,
        bg,
        bg_high,
        action,
        accent: [265, 100, 53],
        error: [15, 100, 37],
        btn_text: btn_text.to_string(),
        theme_sun: "hsla(var(--action) / .7)".to_string(),
        theme_moon: "hsla(var(--accent) / .85)".to_string(),
    };
    ThemeDocument {
        client_id: client_id.to_string(),
        light: mode([200, 5, 37], [200, 15, 25], [34, 25, 97], [34, 20, 90], [34, 100, 40], "white"),
        dark: mode([34, 5, 75], [34, 7, 90], [200, 40, 6], [200, 20, 17], [34, 100, 59], "hsl(var(--bg))"),
        border_radius: "5px".to_string(),
    }
}

#[test]
fn the_declared_mapping_loads_and_names_its_source() -> TestResult {
    let mapping = ThemeMapping::load(&mapping_path())?;
    assert_eq!(mapping.source.path, TOKEN_FILE);
    assert_eq!(mapping.source.commit, "385916e");
    assert_eq!(mapping.decision, "ADR-021");
    assert_eq!(mapping.mode, "dark");
    Ok(())
}

#[test]
fn each_client_wears_its_own_product_accent_and_neither_is_aion_blue_or_purple() -> TestResult {
    let mapping = ThemeMapping::load(&mapping_path())?;
    let platform = mapping.accent(ClientRole::Platform);
    let cambium = mapping.accent(ClientRole::Cambium);
    assert_eq!((platform.token.as_str(), platform.hex.as_str()), ("products.identity.accent", "#D4975A"));
    assert_eq!((cambium.token.as_str(), cambium.hex.as_str()), ("products.cambium.accent", "#5E8C6A"));
    for accent in [platform, cambium] {
        assert!(!accent.hex.eq_ignore_ascii_case(AION_BLUE));
    }
    let lowered = MAPPING.to_ascii_lowercase();
    assert!(!lowered.contains("purple"));
    assert!(!lowered.contains(&BANNED_PURPLE.to_ascii_lowercase()));
    assert!(!lowered.contains("muted"), "text maps to text, never muted");
    Ok(())
}

#[test]
fn apply_sets_exactly_the_four_mapped_dark_fields() -> TestResult {
    let mapping = ThemeMapping::load(&mapping_path())?;
    let mut checked = 0;
    for (role, client) in [(ClientRole::Platform, "platform"), (ClientRole::Cambium, "cambium")] {
        let default = pinned_default("rauthy");
        let applied = mapping.apply(role, client, &default);
        assert_eq!(applied.client_id, client);
        assert_eq!(applied.light, default.light, "light mode is a gap");
        assert_eq!(applied.border_radius, default.border_radius, "the radius is a gap");
        assert_eq!(applied.dark.text, [210, 10, 92]);
        assert_eq!(applied.dark.bg, [210, 14, 5]);
        assert_eq!(applied.dark.bg_high, [210, 12, 13]);
        assert_eq!(applied.dark.accent, mapping.accent(role).hsl);
        let mut gaps_only = applied.dark.clone();
        gaps_only.text = default.dark.text;
        gaps_only.bg = default.dark.bg;
        gaps_only.bg_high = default.dark.bg_high;
        gaps_only.accent = default.dark.accent;
        assert_eq!(gaps_only, default.dark, "every other dark field is a gap");
        checked += 1;
    }
    assert_eq!(checked, 2);
    Ok(())
}

#[test]
fn both_mapped_themes_meet_wcag_aa_over_the_pinned_gap_defaults() -> TestResult {
    let mapping = ThemeMapping::load(&mapping_path())?;
    let expected = [
        (ClientRole::Platform, [16.25, 13.48, 15.61, 9.87, 5.24, 5.83]),
        (ClientRole::Cambium, [16.25, 13.48, 15.61, 9.87, 5.24, 3.97]),
    ];
    let mut measured_pairs = 0;
    for (role, ratios) in expected {
        let theme = mapping.apply(role, role.key(), &pinned_default(role.key()));
        let measured = check_contrast(role.key(), &theme.dark)?;
        assert_eq!(measured.len(), 6);
        for (measurement, expected) in measured.iter().zip(ratios) {
            assert!(measurement.passes(), "{measurement:?}");
            assert!(
                (measurement.ratio - expected).abs() < 0.01,
                "{}: {:.2} against {expected:.2} in theme-map.md",
                measurement.pair,
                measurement.ratio
            );
            measured_pairs += 1;
        }
    }
    assert_eq!(measured_pairs, 12);
    Ok(())
}

#[test]
fn a_failing_pair_is_refused_by_its_name_and_ratio() {
    let mut theme = pinned_default("platform").dark;
    theme.text_high = theme.bg;
    let refused = check_contrast("platform", &theme);
    assert!(
        matches!(&refused, Err(IdentityError::ContrastBelowTier { pair, ratio, .. })
            if *pair == "text_high over ink" && (*ratio - 1.0).abs() < 1e-9),
        "{refused:?}"
    );
}

#[test]
fn the_wcag_formula_gives_its_known_extremes() -> TestResult {
    let white = parse_hex("#FFFFFF").ok_or("white")?;
    let black = parse_hex("#000000").ok_or("black")?;
    assert!((contrast_ratio(white, black) - 21.0).abs() < 1e-9);
    assert!((contrast_ratio(black, black) - 1.0).abs() < 1e-9);
    Ok(())
}

#[test]
fn a_declared_conversion_that_is_not_the_token_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("rauthy-themes.json");
    let drifted = MAPPING.replacen("[30, 59, 59]", "[30, 59, 60]", 1);
    assert_ne!(drifted, MAPPING);
    std::fs::write(&path, drifted)?;
    let refused = ThemeMapping::load(&path);
    assert!(matches!(refused, Err(IdentityError::ThemesInvalid { .. })), "{refused:?}");
    Ok(())
}

#[test]
fn theme_map_names_exactly_the_eight_gaps_each_keeping_rauthys_default() -> TestResult {
    assert!(THEME_MAP.contains("ADR-021"));
    let section = THEME_MAP
        .split("## Gaps")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .ok_or("theme-map.md has no Gaps section")?;
    let rows: Vec<&str> = section
        .lines()
        .filter(|line| line.starts_with("| ") && !line.starts_with("| Gap") && !line.starts_with("|---"))
        .collect();
    let names: Vec<&str> = rows
        .iter()
        .filter_map(|row| row.trim_start_matches("| ").split(" |").next())
        .collect();
    assert_eq!(
        names,
        [
            "light mode",
            "the error colour",
            "the radius",
            "text_high",
            "action",
            "btn_text",
            "theme_sun",
            "theme_moon"
        ]
    );
    assert!(rows.iter().all(|row| row.contains("Rauthy's own default")));
    Ok(())
}
