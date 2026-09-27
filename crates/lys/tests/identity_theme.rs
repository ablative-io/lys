//! ID001_THEME: both client themes, as the running Rauthy exports them, carry
//! the declared dark mapping (estate text, ink, raised, and each product's
//! accent), keep every gap and every light field at the pinned Rauthy's
//! default, persist across a restart, and meet WCAG 2.1 AA over six pairs per
//! client. The contrast is computed here, apart from `themes.rs`, from the
//! values Rauthy exports.
//!
//! Container-backed: declared `test = false`, run only on the identity leg of
//! `.land/gates.sh`.

pub mod identity_support;

use identity_support::fixtures::repo_root;
use identity_support::{Stack, TestResult};
use serde_json::Value;

const CLIENTS: [&str; 2] = ["platform", "cambium"];

/// The pinned Rauthy's dark defaults for the dark gaps, and its light mode,
/// from `src/data/src/entity/theme.rs` at
/// dd61ac3c84d6b238108dc8438b53043b5177a662.
fn pinned_gaps() -> Value {
    serde_json::json!({
        "dark": {
            "text_high": [34, 7, 90],
            "action": [34, 100, 59],
            "error": [15, 100, 37],
            "btn_text": "hsl(var(--bg))",
            "theme_sun": "hsla(var(--action) / .7)",
            "theme_moon": "hsla(var(--accent) / .85)"
        },
        "light": {
            "text": [200, 5, 37], "text_high": [200, 15, 25], "bg": [34, 25, 97],
            "bg_high": [34, 20, 90], "action": [34, 100, 40], "accent": [265, 100, 53],
            "error": [15, 100, 37], "btn_text": "white",
            "theme_sun": "hsla(var(--action) / .7)", "theme_moon": "hsla(var(--accent) / .85)"
        },
        "border_radius": "5px"
    })
}

fn mapping() -> TestResult<Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("deploy/identity/rauthy-themes.json"),
    )?)?)
}

fn export(stack: &Stack, client: &str) -> TestResult<Value> {
    let reply = stack.rauthy_admin("POST", &format!("/auth/v1/theme/{client}"))?;
    if reply.status != 200 {
        return Err(format!("theme export of {client} returned {}", reply.status).into());
    }
    Ok(serde_json::from_str(&reply.body)?)
}

fn hsl(value: &Value) -> TestResult<[f64; 3]> {
    let triple = value.as_array().ok_or("not an HSL triple")?;
    let part = |i: usize| triple.get(i).and_then(Value::as_f64).ok_or("not an HSL number");
    Ok([part(0)?, part(1)?, part(2)?])
}

/// CSS Color 4 `hsl()` to sRGB, by the chroma construction.
fn rgb(hsl: [f64; 3]) -> [f64; 3] {
    let hue = hsl[0].rem_euclid(360.0);
    let saturation = hsl[1] / 100.0;
    let lightness = hsl[2] / 100.0;
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let second = chroma * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let sector = [
        [chroma, second, 0.0],
        [second, chroma, 0.0],
        [0.0, chroma, second],
        [0.0, second, chroma],
        [second, 0.0, chroma],
        [chroma, 0.0, second],
    ];
    let index = [60.0, 120.0, 180.0, 240.0, 300.0]
        .iter()
        .take_while(|bound| hue >= **bound)
        .count();
    let offset = lightness - chroma / 2.0;
    sector[index].map(|channel| channel + offset)
}

fn luminance(colour: [f64; 3]) -> f64 {
    let channel = |c: f64| if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    0.2126 * channel(colour[0]) + 0.7152 * channel(colour[1]) + 0.0722 * channel(colour[2])
}

fn ratio(first: [f64; 3], second: [f64; 3]) -> f64 {
    let (a, b) = (luminance(first), luminance(second));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// A theme CSS value Rauthy exports, resolved against the theme's own
/// variables and composited over `beneath`.
fn css(dark: &Value, value: &str, beneath: [f64; 3]) -> TestResult<[f64; 3]> {
    if value == "white" {
        return Ok([1.0; 3]);
    }
    let (variable, alpha) = if let Some(inner) = value.strip_prefix("hsla(var(--").and_then(|v| v.strip_suffix(')')) {
        let (name, alpha) = inner.split_once(") /").ok_or("an hsla() without alpha")?;
        (name.to_string(), alpha.trim().parse::<f64>()?)
    } else {
        let name = value
            .strip_prefix("hsl(var(--")
            .and_then(|v| v.strip_suffix("))"))
            .ok_or_else(|| format!("unmeasurable CSS value {value:?}"))?;
        (name.to_string(), 1.0)
    };
    let colour = rgb(hsl(&dark[variable.replace('-', "_")])?);
    Ok([0, 1, 2].map(|i| colour[i] * alpha + beneath[i] * (1.0 - alpha)))
}

fn pairs(dark: &Value) -> TestResult<Vec<(&'static str, f64, &'static str, f64)>> {
    let ink = rgb(hsl(&dark["bg"])?);
    let raised = rgb(hsl(&dark["bg_high"])?);
    let text = rgb(hsl(&dark["text"])?);
    let action = rgb(hsl(&dark["action"])?);
    let text_str = |field: &str| dark[field].as_str().map(str::to_string).ok_or_else(|| format!("{field} is not a string"));
    let btn = css(dark, &text_str("btn_text")?, action)?;
    let sun = css(dark, &text_str("theme_sun")?, ink)?;
    let moon = css(dark, &text_str("theme_moon")?, ink)?;
    let body = ("body text", 4.5);
    let large = ("large text, icons and components", 3.0);
    Ok(vec![
        ("text over bg (ink)", ratio(text, ink), body.0, body.1),
        ("text over bg_high (raised)", ratio(text, raised), body.0, body.1),
        ("text_high over ink", ratio(rgb(hsl(&dark["text_high"])?), ink), body.0, body.1),
        ("btn_text over action", ratio(btn, action), body.0, body.1),
        ("theme_sun over ink", ratio(sun, ink), large.0, large.1),
        ("theme_moon over ink", ratio(moon, ink), large.0, large.1),
    ])
}

#[test]
fn contrast() -> TestResult {
    let stack = Stack::up("contrast")?;
    stack.configure()?;
    let mut measured = 0;
    let mut passing = 0;
    let mut failing = Vec::new();
    for client in CLIENTS {
        let theme = export(&stack, client)?;
        for (pair, value, tier, minimum) in pairs(&theme["dark"])? {
            println!("contrast {client}: {pair} {value:.2}:1 ({tier}, {minimum:.1}:1)");
            measured += 1;
            if value >= minimum {
                passing += 1;
            } else {
                failing.push(format!("{client}: {pair} {value:.2}:1 below {minimum:.1}:1"));
            }
        }
    }
    println!("contrast: {measured} pairs measured, {passing} at or above their tier");
    assert!(failing.is_empty(), "below tier: {}", failing.join("; "));
    assert_eq!((measured, passing), (12, 12));
    Ok(())
}

#[test]
fn id001_theme_mapping_and_gaps_persist_after_restart() -> TestResult {
    let stack = Stack::up("theme")?;
    stack.configure()?;
    let declared = mapping()?;
    let gaps = pinned_gaps();
    let accents = [("platform", "#D4975A"), ("cambium", "#5E8C6A")];
    let mut exports = Vec::new();
    for (client, hex) in accents {
        let theme = export(&stack, client)?;
        let dark = &theme["dark"];
        assert_eq!(dark["text"], declared["foundation"]["text"]["hsl"], "{client}");
        assert_eq!(dark["bg"], declared["foundation"]["bg"]["hsl"], "{client}");
        assert_eq!(dark["bg_high"], declared["foundation"]["bg_high"]["hsl"], "{client}");
        assert_eq!(dark["accent"], declared["clients"][client]["accent"]["hsl"], "{client}");
        assert_eq!(declared["clients"][client]["accent"]["hex"], hex, "{client}");
        for (field, value) in gaps["dark"].as_object().ok_or("gaps")? {
            assert_eq!(&dark[field], value, "{client}: dark.{field} is a gap");
        }
        assert_eq!(theme["light"], gaps["light"], "{client}: light mode is a gap");
        assert_eq!(theme["border_radius"], gaps["border_radius"], "{client}: the radius is a gap");
        println!("ID001_THEME export {client} (source {}@{}): {theme}", declared["source"]["repository"], declared["source"]["commit"]);
        exports.push(theme);
    }
    stack.restart()?;
    for ((client, _), before) in accents.iter().zip(&exports) {
        assert_eq!(&export(&stack, client)?, before, "{client}'s theme did not persist");
    }
    Ok(())
}

#[test]
fn accents_are_the_estate_product_accents_and_never_aion_blue_or_purple() -> TestResult {
    let text = std::fs::read_to_string(repo_root().join("deploy/identity/rauthy-themes.json"))?;
    let declared: Value = serde_json::from_str(&text)?;
    assert_eq!(declared["clients"]["cambium"]["accent"]["token"], "products.cambium.accent");
    assert_eq!(declared["clients"]["platform"]["accent"]["token"], "products.identity.accent");
    let lowered = text.to_ascii_lowercase();
    assert!(!lowered.contains("#6b96d1"), "Aion blue");
    assert!(!lowered.contains("purple") && !lowered.contains("#a78bfa"));
    Ok(())
}
