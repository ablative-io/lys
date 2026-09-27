#![cfg(test)]
use std::error::Error;

use super::*;

fn default_theme(client_id: &str) -> Theme {
    Theme {
        client_id: client_id.to_string(),
        light: ThemeCss {
            text: [200, 5, 37],
            text_high: [200, 15, 25],
            bg: [34, 25, 97],
            bg_high: [34, 20, 90],
            action: [34, 100, 40],
            accent: [265, 100, 53],
            error: [15, 100, 37],
            btn_text: "white".to_string(),
            theme_sun: "hsla(var(--action) / .7)".to_string(),
            theme_moon: "hsla(var(--accent) / .85)".to_string(),
        },
        dark: rauthy_dark_defaults(),
        border_radius: "5px".to_string(),
    }
}

#[test]
fn the_declared_mapping_is_valid() -> Result<(), Box<dyn Error>> {
    let mapping = ThemeMapping::declared()?;
    assert_eq!(
        mapping.source.commit,
        "385916eb437cae62c4269e6db1db2e542af5749e"
    );
    assert_eq!(
        mapping.client(ClientRole::Cambium)?.dark.accent.hex,
        "#5E8C6A"
    );
    assert_eq!(
        mapping.client(ClientRole::Platform)?.dark.accent.hex,
        "#D4975A"
    );
    Ok(())
}

#[test]
fn estate_hex_values_convert_to_the_declared_hsl() {
    assert_eq!(hex_to_hsl("#E8EAEC"), Some([210, 10, 92]));
    assert_eq!(hex_to_hsl("#0C0E10"), Some([210, 14, 5]));
    assert_eq!(hex_to_hsl("#1E2226"), Some([210, 12, 13]));
    assert_eq!(hex_to_hsl("#5E8C6A"), Some([136, 20, 46]));
    assert_eq!(hex_to_hsl("#D4975A"), Some([30, 59, 59]));
    assert_eq!(hex_to_hsl("#FFFFFF"), Some([0, 0, 100]));
    assert_eq!(hex_to_hsl("E8EAEC"), None);
}

#[test]
fn twelve_pairs_meet_their_tier_with_gap_fields_at_rauthys_defaults() -> Result<(), Box<dyn Error>>
{
    let mapping = ThemeMapping::declared()?;
    let mut measured = 0;
    for role in [ClientRole::Platform, ClientRole::Cambium] {
        let themed = mapping.apply(role, &default_theme(role.key()));
        for pair in contrast_pairs(&themed.dark)? {
            assert!(
                pair.passes(),
                "{} {} {:.2}",
                role.key(),
                pair.name,
                pair.ratio
            );
            measured += 1;
        }
    }
    assert_eq!(measured, 12);
    Ok(())
}

#[test]
fn applying_the_mapping_sets_four_dark_fields_and_nothing_else() -> Result<(), Box<dyn Error>> {
    let mapping = ThemeMapping::declared()?;
    let current = default_theme("cambium");
    let themed = mapping.apply(ClientRole::Cambium, &current);
    assert_eq!(themed.light, current.light);
    assert_eq!(themed.border_radius, current.border_radius);
    let (dark, default) = (&themed.dark, &current.dark);
    assert_eq!(dark.text, [210, 10, 92]);
    assert_eq!(dark.bg, [210, 14, 5]);
    assert_eq!(dark.bg_high, [210, 12, 13]);
    assert_eq!(dark.accent, [136, 20, 46]);
    assert_eq!(
        (&dark.text_high, &dark.action, &dark.error),
        (&default.text_high, &default.action, &default.error)
    );
    assert_eq!(
        (&dark.btn_text, &dark.theme_sun, &dark.theme_moon),
        (&default.btn_text, &default.theme_sun, &default.theme_moon)
    );
    Ok(())
}

fn refused(text: &str) -> bool {
    ThemeMapping::parse(text).is_err_and(|error| error.kind() == ErrorKind::ThemeInvalid)
}

#[test]
fn a_gap_field_cannot_be_named_in_the_mapping() {
    let text = DECLARED.replacen(
        "\"accent\": { \"token\": \"products.cambium.accent\"",
        "\"error\": { \"token\": \"foundation.text\", \"hex\": \"#E8EAEC\", \"hsl\": [210, 10, 92] },\n        \"accent\": { \"token\": \"products.cambium.accent\"",
        1,
    );
    assert_ne!(text, DECLARED);
    assert!(refused(&text));
}

#[test]
fn text_mapped_to_muted_is_refused() {
    let text = DECLARED.replacen(
        "\"text\": { \"token\": \"foundation.text\", \"hex\": \"#E8EAEC\", \"hsl\": [210, 10, 92] }",
        "\"text\": { \"token\": \"foundation.muted\", \"hex\": \"#8A9199\", \"hsl\": [212, 7, 57] }",
        1,
    );
    assert_ne!(text, DECLARED);
    assert!(refused(&text));
}

#[test]
fn aion_blue_or_purple_accents_are_refused() {
    let blue = DECLARED.replace(
        "\"hex\": \"#5E8C6A\", \"hsl\": [136, 20, 46]",
        "\"hex\": \"#6B96D1\", \"hsl\": [215, 53, 62]",
    );
    assert_ne!(blue, DECLARED);
    assert!(refused(&blue));
    let purple = DECLARED.replace(
        "\"hex\": \"#D4975A\", \"hsl\": [30, 59, 59]",
        "\"hex\": \"#A78BFA\", \"hsl\": [255, 92, 76]",
    );
    assert_ne!(purple, DECLARED);
    assert!(refused(&purple));
}

#[test]
fn a_hex_that_does_not_convert_to_its_hsl_is_refused() {
    let text = DECLARED.replace("[30, 59, 59]", "[30, 60, 59]");
    assert!(refused(&text));
}

#[test]
fn a_failing_pair_is_named_with_its_ratio() -> Result<(), Box<dyn Error>> {
    let mut dark = rauthy_dark_defaults();
    dark.text = dark.bg;
    let pairs = contrast_pairs(&dark)?;
    let failing = pairs
        .iter()
        .find(|pair| !pair.passes())
        .ok_or("no pair failed")?;
    assert_eq!(failing.name, "text over bg");
    assert!(failing.ratio < 1.01);
    Ok(())
}

#[test]
fn css_forms_resolve_against_their_mode() -> Result<(), Box<dyn Error>> {
    let dark = rauthy_dark_defaults();
    let bg = resolve_css("hsl(var(--bg))", &dark).ok_or("bg")?;
    assert!((contrast_ratio(bg, hsl_to_rgb(dark.bg)) - 1.0).abs() < 1e-9);
    assert!(resolve_css("hsla(var(--accent) / .85)", &dark).is_some());
    assert!(resolve_css("var(--nothing)", &dark).is_none());
    Ok(())
}
