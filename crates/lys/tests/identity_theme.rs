//! `ID001_THEME`: both client themes, dark only (ADR-021), as the running
//! Rauthy exports them. The four mapped fields hold the declared estate
//! tokens; the eight gaps (light mode, error, radius, `text_high`, action,
//! `btn_text`, `theme_sun`, `theme_moon`) equal the pinned Rauthy's own
//! defaults; everything persists across a restart; and readable contrast is
//! measured, WCAG 2.1 AA, over six pairs per client.
//!
//! The contrast here is a second implementation, independent of
//! crates/lys/src/identity/themes.rs: another HSL conversion, and the
//! defaults read from the pinned Rauthy source rather than from this
//! repository. Run the counted leg with
//! `cargo test -p lys --all-features --test identity_theme contrast -- --nocapture`.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! .land/gates.sh, and refused as `container_runtime_missing` without a
//! runtime. Test identities only (CN2).

mod identity_support;

use identity_support::fixtures::{Failure, Venue, repo_root};
use identity_support::server::Stack;
use serde_json::{Map, Value};

/// The estate's Aion accent and its banned status purple: neither may
/// appear in the mapping.
const FORBIDDEN: [&str; 2] = ["#6B96D1", "#A78BFA"];

/// The dark fields the mapping sets; every other field is a gap.
const MAPPED: [&str; 4] = ["text", "bg", "bg_high", "accent"];

/// One mode of a theme, field by field, as JSON.
type Mode = Map<String, Value>;

#[test]
fn id001_theme_contrast() -> Result<(), Failure> {
    let path = repo_root().join("deploy/identity/rauthy-themes.json");
    let source = std::fs::read_to_string(path)?;
    for colour in FORBIDDEN {
        assert!(
            !source.to_uppercase().contains(colour),
            "{colour} appears in the mapping"
        );
    }
    let mapping: Value = serde_json::from_str(&source)?;
    let (dark_default, light_default, radius_default) = pinned_defaults()?;

    let venue = Venue::fresh("theme", None)?;
    let stack = Stack::up(&venue)?;
    stack.wait_ready()?;
    let first = stack.configure()?;
    let themes = first["themes"]
        .as_array()
        .ok_or("configure exported no themes")?;
    assert_eq!(themes.len(), 2);

    let mut checked = 0;
    for (client, product) in [("platform", "identity"), ("cambium", "cambium")] {
        let entry = mapping["clients"]
            .as_array()
            .and_then(|all| all.iter().find(|entry| entry["client"] == client))
            .ok_or(format!("{client} is not mapped"))?;
        assert_eq!(entry["product"], product);
        assert_eq!(
            entry["dark"]["accent"]["token"],
            format!("products.{product}.accent")
        );
        let theme = themes
            .iter()
            .find(|theme| theme["client_id"] == client)
            .ok_or(format!("{client} has no exported theme"))?;
        for field in MAPPED {
            assert_eq!(
                theme["dark"][field], entry["dark"][field]["hsl"],
                "{client} dark.{field}"
            );
        }
        for (field, value) in &dark_default {
            if !MAPPED.contains(&field.as_str()) {
                assert_eq!(&theme["dark"][field], value, "{client} gap dark.{field}");
            }
        }
        assert_eq!(
            theme["light"],
            Value::Object(light_default.clone()),
            "{client} light mode"
        );
        assert_eq!(
            theme["border_radius"],
            radius_default.as_str(),
            "{client} radius"
        );
        checked += 1;
    }
    assert_eq!(checked, 2);

    let (measured, passed, low) = contrast(themes)?;
    println!("contrast: {measured} pairs measured, {passed} at or above their tier");
    if let Some(low) = low {
        return Err(low.into());
    }
    assert_eq!(measured, 12);

    // Everything persists across a restart of Rauthy.
    stack.compose().ok(&["restart", "rauthy"])?;
    stack.wait_ready()?;
    let second = stack.configure()?;
    assert_eq!(
        second["themes"], first["themes"],
        "the themes changed across restart"
    );
    let operations = second["operations"].as_array().ok_or("no operations")?;
    assert!(
        operations.iter().all(|op| op["outcome"] == "unchanged"),
        "{second}"
    );
    Ok(())
}

/// Prints one line per pair per client and counts them; returns the count,
/// how many met their tier, and the first pair that did not.
fn contrast(themes: &[Value]) -> Result<(usize, usize, Option<String>), Failure> {
    let mut measured = 0;
    let mut passed = 0;
    let mut low = None;
    for theme in themes {
        let client = theme["client_id"]
            .as_str()
            .ok_or("a theme names no client")?;
        let dark = &theme["dark"];
        let ink = rgb(hsl(&dark["bg"])?);
        let action = rgb(hsl(&dark["action"])?);
        let pairs = [
            ("text over bg (ink)", rgb(hsl(&dark["text"])?), ink, 4.5),
            (
                "text over bg_high (raised)",
                rgb(hsl(&dark["text"])?),
                rgb(hsl(&dark["bg_high"])?),
                4.5,
            ),
            (
                "text_high over ink",
                rgb(hsl(&dark["text_high"])?),
                ink,
                4.5,
            ),
            (
                "btn_text over action",
                css(dark, "btn_text", action)?,
                action,
                4.5,
            ),
            ("theme_sun over ink", css(dark, "theme_sun", ink)?, ink, 3.0),
            (
                "theme_moon over ink",
                css(dark, "theme_moon", ink)?,
                ink,
                3.0,
            ),
        ];
        for (pair, fore, back, tier) in pairs {
            let ratio = ratio(fore, back);
            let label = if tier > 4.0 {
                "4.5:1 body text"
            } else {
                "3:1 large text, icons and components"
            };
            println!("contrast {client} {pair}: {ratio:.2} (tier {label})");
            measured += 1;
            if ratio >= tier {
                passed += 1;
            } else if low.is_none() {
                low = Some(format!(
                    "contrast_below_tier: {client} {pair} measures {ratio:.2}"
                ));
            }
        }
    }
    Ok((measured, passed, low))
}

fn hsl(value: &Value) -> Result<[f64; 3], Failure> {
    let parts: Vec<f64> = value
        .as_array()
        .ok_or("not an HSL triple")?
        .iter()
        .filter_map(Value::as_f64)
        .collect();
    match parts.as_slice() {
        [hue, saturation, lightness] => Ok([*hue, *saturation, *lightness]),
        _ => Err(format!("not an HSL triple: {value}").into()),
    }
}

/// HSL to sRGB by the classic two-point interpolation.
fn rgb([hue, saturation, lightness]: [f64; 3]) -> [f64; 3] {
    let (hue, sat, light) = (hue / 360.0, saturation / 100.0, lightness / 100.0);
    let high = if light < 0.5 {
        light * (1.0 + sat)
    } else {
        light + sat - light * sat
    };
    let low = 2.0 * light - high;
    [
        interpolate(low, high, hue + 1.0 / 3.0),
        interpolate(low, high, hue),
        interpolate(low, high, hue - 1.0 / 3.0),
    ]
}

fn interpolate(low: f64, high: f64, position: f64) -> f64 {
    let position = position.rem_euclid(1.0);
    if position < 1.0 / 6.0 {
        low + (high - low) * 6.0 * position
    } else if position < 0.5 {
        high
    } else if position < 2.0 / 3.0 {
        low + (high - low) * (2.0 / 3.0 - position) * 6.0
    } else {
        low
    }
}

/// A CSS theme value as Rauthy exports it, composited over `under`.
fn css(dark: &Value, field: &str, under: [f64; 3]) -> Result<[f64; 3], Failure> {
    let value = dark[field]
        .as_str()
        .ok_or(format!("{field} is not a CSS value"))?;
    match value.trim() {
        "white" => return Ok([1.0; 3]),
        "black" => return Ok([0.0; 3]),
        _ => {}
    }
    let variable = value
        .split("var(--")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .ok_or(format!("{field} {value:?} is not a form this check reads"))?;
    let alpha = match value.split_once('/') {
        Some((_, rest)) => rest.trim().trim_end_matches(')').trim().parse::<f64>()?,
        None => 1.0,
    };
    let colour = rgb(hsl(&dark[variable.replace('-', "_")])?);
    Ok([0, 1, 2].map(|index| alpha * colour[index] + (1.0 - alpha) * under[index]))
}

fn luminance(colour: [f64; 3]) -> f64 {
    let linear = colour.map(|channel| {
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

fn ratio(fore: [f64; 3], back: [f64; 3]) -> f64 {
    let (one, two) = (luminance(fore), luminance(back));
    (one.max(two) + 0.05) / (one.min(two) + 0.05)
}

/// The pinned Rauthy's own default dark and light themes and radius, read
/// from its source at vendor/rauthy, which the recursive clone checks out.
fn pinned_defaults() -> Result<(Mode, Mode, String), Failure> {
    let path = repo_root().join("vendor/rauthy/src/data/src/entity/theme.rs");
    let source = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "vendor_rauthy_missing: {} ({error}); clone with --recurse-submodules",
            path.display()
        )
    })?;
    let dark = theme_block(&source, "fn default_dark() -> Self {")?;
    let light = theme_block(&source, "fn default_light() -> Self {")?;
    let radius = source
        .split("border_radius: \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .ok_or("the pinned source has no default radius")?
        .to_string();
    Ok((dark, light, radius))
}

/// The ten fields of one `ThemeCss` literal in the pinned source.
fn theme_block(source: &str, marker: &str) -> Result<Mode, Failure> {
    let body = source
        .split(marker)
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .ok_or(format!("the pinned source has no {marker}"))?;
    let mut fields = Map::new();
    for line in body.lines() {
        let Some((name, value)) = line.trim().split_once(": ") else {
            continue;
        };
        let value = value.trim_end_matches(',');
        let parsed = if let Some(list) = value
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let numbers: Result<Vec<u64>, _> =
                list.split(',').map(|n| n.trim().parse::<u64>()).collect();
            Value::from(numbers?)
        } else {
            let text = value
                .strip_prefix('"')
                .and_then(|rest| rest.split('"').next())
                .ok_or(format!("{name}: {value} is neither a triple nor a string"))?;
            Value::from(text)
        };
        fields.insert(name.to_string(), parsed);
    }
    if fields.len() != 10 {
        return Err(format!("{marker} holds {} fields, not ten", fields.len()).into());
    }
    Ok(fields)
}
