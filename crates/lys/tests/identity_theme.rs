//! `ID001_THEME` and its counted contrast leg: both client themes, as the
//! running Rauthy exports them, carry the declared estate mapping in dark
//! mode and Rauthy's defaults everywhere else, persist after a restart, and
//! meet WCAG 2.1 AA over six pairs per client. The contrast is computed here
//! independently of lys, from the exported values. Container-backed: runs
//! only on the identity leg of .land/gates.sh.

pub mod identity_support;

use identity_support::compose::{self, require_runtime};
use identity_support::fixtures::{Deployment, TestResult, repository_root, succeeded};
use identity_support::server::{rauthy_json, request};
use serde_json::{Value, json};

fn pinned_light() -> Value {
    json!({
        "text": [200, 5, 37], "text_high": [200, 15, 25], "bg": [34, 25, 97],
        "bg_high": [34, 20, 90], "action": [34, 100, 40], "accent": [265, 100, 53],
        "error": [15, 100, 37], "btn_text": "white",
        "theme_sun": "hsla(var(--action) / .7)", "theme_moon": "hsla(var(--accent) / .85)"
    })
}

fn pinned_dark_gaps() -> Value {
    json!({
        "text_high": [34, 7, 90], "action": [34, 100, 59], "error": [15, 100, 37],
        "btn_text": "hsl(var(--bg))", "theme_sun": "hsla(var(--action) / .7)",
        "theme_moon": "hsla(var(--accent) / .85)"
    })
}

fn hsl(value: &Value) -> TestResult<[f64; 3]> {
    let parts = value.as_array().ok_or("not an HSL triple")?;
    let mut out = [0.0; 3];
    for (slot, part) in out.iter_mut().zip(parts) {
        *slot = part.as_f64().ok_or("not a number")?;
    }
    Ok(out)
}

fn rgb([hue, saturation, lightness]: [f64; 3]) -> [f64; 3] {
    let (sat, light) = (saturation / 100.0, lightness / 100.0);
    let chroma = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let sector = (hue % 360.0) / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let [red, green, blue] = match sector.floor() {
        s if s < 1.0 => [chroma, second, 0.0],
        s if s < 2.0 => [second, chroma, 0.0],
        s if s < 3.0 => [0.0, chroma, second],
        s if s < 4.0 => [0.0, second, chroma],
        s if s < 5.0 => [second, 0.0, chroma],
        _ => [chroma, 0.0, second],
    };
    let offset = light - chroma / 2.0;
    [red + offset, green + offset, blue + offset]
}

fn luminance(colour: [f64; 3]) -> f64 {
    let linear = colour.map(|c| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

fn ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Resolves a Rauthy CSS colour value (`hsl(var(--x))` or
/// `hsla(var(--x) / alpha)`) in `mode`, composited over its background.
fn css(value: &Value, mode: &Value) -> TestResult<[f64; 3]> {
    let text = value.as_str().ok_or("not a CSS value")?;
    let inner = text
        .trim_start_matches("hsla(")
        .trim_start_matches("hsl(")
        .trim_end_matches(')');
    let (reference, alpha) = match inner.split_once('/') {
        Some((reference, alpha)) => (reference.trim(), alpha.trim().parse::<f64>()?),
        None => (inner.trim(), 1.0),
    };
    let field = reference
        .strip_prefix("var(--")
        .ok_or("not a variable")?
        .trim_end_matches(')')
        .replace('-', "_");
    let colour = rgb(hsl(&mode[field.as_str()])?);
    let page = rgb(hsl(&mode["bg"])?);
    Ok([0, 1, 2].map(|i| alpha * colour[i] + (1.0 - alpha) * page[i]))
}

fn pairs(dark: &Value) -> TestResult<Vec<(&'static str, f64, f64)>> {
    let ink = rgb(hsl(&dark["bg"])?);
    Ok(vec![
        ("text over bg", ratio(rgb(hsl(&dark["text"])?), ink), 4.5),
        (
            "text over bg_high",
            ratio(rgb(hsl(&dark["text"])?), rgb(hsl(&dark["bg_high"])?)),
            4.5,
        ),
        (
            "text_high over ink",
            ratio(rgb(hsl(&dark["text_high"])?), ink),
            4.5,
        ),
        (
            "btn_text over action",
            ratio(css(&dark["btn_text"], dark)?, rgb(hsl(&dark["action"])?)),
            4.5,
        ),
        (
            "theme_sun over ink",
            ratio(css(&dark["theme_sun"], dark)?, ink),
            3.0,
        ),
        (
            "theme_moon over ink",
            ratio(css(&dark["theme_moon"], dark)?, ink),
            3.0,
        ),
    ])
}

fn declared() -> TestResult<Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        repository_root().join("deploy/identity/rauthy-themes.json"),
    )?)?)
}

fn check_exports(deployment: &Deployment, mapping: &Value) -> TestResult<Vec<(String, Value)>> {
    let mut exports = Vec::new();
    for (role, client_id) in [("platform", "lys-platform"), ("cambium", "cambium")] {
        let theme = rauthy_json(deployment, "POST", &format!("/auth/v1/theme/{client_id}"))?;
        let dark = &theme["dark"];
        for field in ["text", "bg", "bg_high", "accent"] {
            assert_eq!(
                dark[field], mapping["clients"][role]["dark"][field]["hsl"],
                "{role} {field}"
            );
        }
        let gaps = pinned_dark_gaps();
        for (field, value) in gaps.as_object().ok_or("gaps")? {
            assert_eq!(
                &dark[field.as_str()],
                value,
                "{role} dark {field} left its default"
            );
        }
        assert_eq!(
            theme["light"],
            pinned_light(),
            "{role} light mode left its defaults"
        );
        assert_eq!(theme["border_radius"], "5px");
        let (status, page_css) = request(
            &deployment.rauthy_address(),
            "GET",
            &format!("/auth/v1/theme/{client_id}/0"),
            &[],
            None,
        )?;
        assert_eq!(status, 200);
        let accent = &dark["accent"];
        let expected = format!("--accent:{} {} {};", accent[0], accent[1], accent[2]);
        assert!(
            page_css.contains(&expected),
            "{client_id} login page CSS lacks {expected}"
        );
        exports.push((client_id.to_string(), theme));
    }
    Ok(exports)
}

#[test]
fn contrast_and_id001_theme_persist_after_restart() -> TestResult {
    require_runtime()?;
    let mapping = declared()?;
    assert!(
        !serde_json::to_string(&mapping)?
            .to_ascii_lowercase()
            .contains("purple")
    );
    assert_eq!(
        mapping["clients"]["cambium"]["dark"]["accent"]["hex"],
        "#5E8C6A"
    );
    assert_eq!(
        mapping["clients"]["platform"]["dark"]["accent"]["hex"],
        "#D4975A"
    );
    for role in ["platform", "cambium"] {
        assert_ne!(
            mapping["clients"][role]["dark"]["accent"]["hex"], "#6B96D1",
            "Aion blue"
        );
    }
    let deployment = Deployment::new("theme", |text| text)?;
    compose::render(&deployment)?;
    compose::up(&deployment, &[])?;
    compose::wait_ready(&deployment)?;
    succeeded(&deployment.lys("configure")?, "configure")?;
    let exports = check_exports(&deployment, &mapping)?;

    succeeded(
        &compose::compose(&deployment, &["restart", "rauthy"])?,
        "restart rauthy",
    )?;
    compose::wait_ready(&deployment)?;
    assert_eq!(
        check_exports(&deployment, &mapping)?,
        exports,
        "a theme changed on restart"
    );

    let mut measured = 0;
    let mut below = Vec::new();
    for (client_id, theme) in &exports {
        println!("theme export {client_id}: {theme}");
        for (pair, value, tier) in pairs(&theme["dark"])? {
            let tier_name = if tier > 4.0 {
                "body text 4.5"
            } else {
                "components 3"
            };
            println!("{client_id} {pair}: {value:.2} (tier {tier_name})");
            measured += 1;
            if value < tier {
                below.push(format!("{client_id} {pair} {value:.2}"));
            }
        }
    }
    let passing = measured - below.len();
    println!("contrast: {measured} pairs measured, {passing} at or above their tier");
    assert!(below.is_empty(), "below their tier: {}", below.join(", "));
    assert_eq!(measured, 12);
    Ok(())
}
