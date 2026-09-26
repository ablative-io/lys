//! Read the declared estate palette mapping (deploy/identity/rauthy-themes.json)
//! and validate both client themes.
//!
//! # Invariants
//!
//! - Dark only (ADR-021). The mapping sets exactly the dark `text`, `bg`,
//!   `bg_high` and `accent` of each client, each from the estate token of the
//!   same role. Every other field is a gap that keeps Rauthy's own default,
//!   and nothing here writes one: [`DarkMapping::apply`] copies them from
//!   the theme Rauthy holds.
//! - Every HSL triple is recomputed from its token's hex and must equal the
//!   declared triple, so the file cannot drift from its own conversion.
//! - The platform client wears the identity orange and the Cambium client
//!   Cambium green. No client wears Aion's accent, and the estate's banned
//!   purple appears nowhere in the mapping.
//! - Readable contrast is measured, not asserted: WCAG 2.1 AA over six named
//!   pairs per client, each gap field at the value Rauthy actually holds. A
//!   pair below its tier is `contrast_below_tier` naming the pair — a stop
//!   for the lead under CN9, never a reason to choose a colour the estate
//!   does not name.

use std::path::Path;

use serde::Deserialize;

use super::error::IdentityError;
use super::rauthy::ThemeCss;

/// The estate colour tokens the mapping must cite (owner: Waffles, in the
/// ablative docs repository; read, never changed here).
pub const TOKEN_PATH: &str = "docs/design-system-v2/palette/estate-colour-tokens.json";

/// The status purple the estate guide bans; it must not be copied.
const BANNED_PURPLE: &str = "#A78BFA";

/// The mapping file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFile {
    /// Where the token values were read from.
    pub source: TokenSource,
    /// Always `dark` (ADR-021).
    pub mode: String,
    /// One entry per managed client.
    pub clients: Vec<ClientTheme>,
}

/// The token file and the commit it was read at.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenSource {
    /// The repository holding the tokens.
    pub repository: String,
    /// The token file within it.
    pub path: String,
    /// The commit the values were read at.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Who owns the token file.
    pub owner: String,
    /// The copy the values were actually read from, when it is not the
    /// cited file itself; theme-map.md records why.
    pub read_copy: String,
}

/// One client's mapping.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientTheme {
    /// The config key of the client: `platform` or `cambium`.
    pub client: String,
    /// The estate product whose accent the client wears.
    pub product: String,
    /// The four dark fields the estate names tokens for.
    pub dark: DarkMapping,
}

/// The dark fields the mapping sets; every other field is a gap.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarkMapping {
    /// From `foundation.text`.
    pub text: TokenColour,
    /// From `foundation.ink`.
    pub bg: TokenColour,
    /// From `foundation.raised`.
    pub bg_high: TokenColour,
    /// From `products.<product>.accent`.
    pub accent: TokenColour,
}

impl DarkMapping {
    /// `dark` with the four mapped fields replaced and every gap unchanged.
    pub fn apply(&self, dark: &ThemeCss) -> ThemeCss {
        ThemeCss {
            text: self.text.hsl,
            bg: self.bg.hsl,
            bg_high: self.bg_high.hsl,
            accent: self.accent.hsl,
            ..dark.clone()
        }
    }
}

/// One estate token and its conversion.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenColour {
    /// Dotted token path in the estate file, e.g. `foundation.ink`.
    pub token: String,
    /// The token's value, `#RRGGBB`.
    pub hex: String,
    /// Its HSL as Rauthy stores it: whole degrees and percentages.
    pub hsl: [u16; 3],
}

impl ThemeFile {
    /// The mapping for the client declared under `key`.
    pub fn for_client(&self, key: &str) -> Option<&ClientTheme> {
        self.clients.iter().find(|entry| entry.client == key)
    }
}

/// Reads and validates the mapping at `path`.
pub fn load(path: &Path) -> Result<ThemeFile, IdentityError> {
    let text = std::fs::read_to_string(path).map_err(|source| IdentityError::Io {
        operation: "read theme mapping",
        path: path.to_path_buf(),
        source,
    })?;
    let invalid = |reason: String| IdentityError::ThemeInvalid {
        path: path.to_path_buf(),
        reason,
    };
    let file: ThemeFile =
        serde_json::from_str(&text).map_err(|error| invalid(error.to_string()))?;
    validate(&file).map_err(invalid)?;
    Ok(file)
}

fn validate(file: &ThemeFile) -> Result<(), String> {
    if file.source.path != TOKEN_PATH || file.source.repository.is_empty() {
        return Err(format!("source must cite {TOKEN_PATH} and its repository"));
    }
    let source = &file.source;
    if source.reference.is_empty() || source.owner.is_empty() || source.read_copy.is_empty() {
        return Err("source must name the commit, the owner and the copy read".to_string());
    }
    if file.mode != "dark" {
        return Err("mode must be dark: light mode is a gap (ADR-021)".to_string());
    }
    let expected = [("platform", "identity"), ("cambium", "cambium")];
    if file.clients.len() != expected.len() {
        return Err("exactly the platform and cambium clients are mapped".to_string());
    }
    for (client, product) in expected {
        let entry = file
            .for_client(client)
            .ok_or_else(|| format!("client {client} is not mapped"))?;
        if entry.product != product {
            return Err(format!("client {client} must wear the {product} accent"));
        }
        let accent_token = format!("products.{product}.accent");
        let dark = &entry.dark;
        for (field, colour, token) in [
            ("text", &dark.text, "foundation.text"),
            ("bg", &dark.bg, "foundation.ink"),
            ("bg_high", &dark.bg_high, "foundation.raised"),
            ("accent", &dark.accent, accent_token.as_str()),
        ] {
            check_colour(client, field, colour, token)?;
        }
    }
    Ok(())
}

fn check_colour(
    client: &str,
    field: &str,
    colour: &TokenColour,
    token: &str,
) -> Result<(), String> {
    let place = format!("client {client} dark.{field}");
    if colour.token != token {
        return Err(format!(
            "{place} must come from {token}, not {}",
            colour.token
        ));
    }
    if colour.hex.eq_ignore_ascii_case(BANNED_PURPLE) {
        return Err(format!("{place} is the estate's banned purple"));
    }
    let rgb =
        parse_hex(&colour.hex).ok_or_else(|| format!("{place}: {} is not #RRGGBB", colour.hex))?;
    let converted = hsl_of(rgb);
    if converted != colour.hsl {
        return Err(format!(
            "{place}: {} converts to {converted:?}, not the declared {:?}",
            colour.hex, colour.hsl
        ));
    }
    Ok(())
}

fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// sRGB to HSL in whole degrees and percentages, as Rauthy stores themes.
pub fn hsl_of(rgb: [u8; 3]) -> [u16; 3] {
    let [red, green, blue] = rgb;
    let high = red.max(green).max(blue);
    let low = red.min(green).min(blue);
    let unit = |channel: u8| f64::from(channel) / 255.0;
    let lightness = f64::midpoint(unit(high), unit(low));
    if high == low {
        return [0, 0, nearest(lightness * 100.0, 100)];
    }
    let delta = unit(high) - unit(low);
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let sixths = if high == red {
        ((unit(green) - unit(blue)) / delta).rem_euclid(6.0)
    } else if high == green {
        (unit(blue) - unit(red)) / delta + 2.0
    } else {
        (unit(red) - unit(green)) / delta + 4.0
    };
    [
        nearest(sixths * 60.0, 360) % 360,
        nearest(saturation * 100.0, 100),
        nearest(lightness * 100.0, 100),
    ]
}

/// The whole number in `0..=max` nearest `value`, without a float cast.
fn nearest(value: f64, max: u16) -> u16 {
    (0..=max)
        .min_by(|a, b| {
            (f64::from(*a) - value)
                .abs()
                .total_cmp(&(f64::from(*b) - value).abs())
        })
        .unwrap_or(0)
}

/// The WCAG 2.1 AA tier a pair is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// 4.5 to 1: body text.
    Body,
    /// 3 to 1: large text, icons and interface components.
    Large,
}

impl Tier {
    /// The minimum ratio.
    pub fn minimum(self) -> f64 {
        match self {
            Self::Body => 4.5,
            Self::Large => 3.0,
        }
    }

    /// The tier as output names it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Body => "4.5:1 body text",
            Self::Large => "3:1 large text, icons and components",
        }
    }
}

/// One measured pair.
#[derive(Debug, Clone, Copy)]
pub struct Measure {
    /// The pair's name.
    pub pair: &'static str,
    /// Its WCAG contrast ratio.
    pub ratio: f64,
    /// The tier it is held to.
    pub tier: Tier,
}

/// Measures the six named pairs of one client's dark theme, then refuses
/// the first pair below its tier by name.
pub fn check_contrast(client: &str, dark: &ThemeCss) -> Result<Vec<Measure>, IdentityError> {
    let colour = |field: &'static str, value: &str, under: [f64; 3]| {
        resolve(value, dark, under).ok_or_else(|| IdentityError::ThemeValueUnresolved {
            client: client.to_string(),
            field,
            value: value.to_string(),
        })
    };
    let ink = rgb_of_hsl(dark.bg);
    let raised = rgb_of_hsl(dark.bg_high);
    let action = rgb_of_hsl(dark.action);
    let text = rgb_of_hsl(dark.text);
    let text_high = rgb_of_hsl(dark.text_high);
    let pairs = [
        ("text over bg (ink)", text, ink, Tier::Body),
        ("text over bg_high (raised)", text, raised, Tier::Body),
        ("text_high over ink", text_high, ink, Tier::Body),
        (
            "btn_text over action",
            colour("btn_text", &dark.btn_text, action)?,
            action,
            Tier::Body,
        ),
        (
            "theme_sun over ink",
            colour("theme_sun", &dark.theme_sun, ink)?,
            ink,
            Tier::Large,
        ),
        (
            "theme_moon over ink",
            colour("theme_moon", &dark.theme_moon, ink)?,
            ink,
            Tier::Large,
        ),
    ];
    let measures: Vec<Measure> = pairs
        .into_iter()
        .map(|(pair, fore, back, tier)| Measure {
            pair,
            ratio: contrast(fore, back),
            tier,
        })
        .collect();
    if let Some(low) = measures
        .iter()
        .find(|measure| measure.ratio < measure.tier.minimum())
    {
        return Err(IdentityError::ContrastBelowTier {
            client: client.to_string(),
            pair: low.pair,
            ratio: format!("{:.2}", low.ratio),
            tier: low.tier.label(),
        });
    }
    Ok(measures)
}

/// Resolves the CSS forms Rauthy's theme values take — `hsl(var(--x))`,
/// `hsla(var(--x) / a)`, `white`, `black` — composited over `under`.
fn resolve(value: &str, dark: &ThemeCss, under: [f64; 3]) -> Option<[f64; 3]> {
    let value = value.trim();
    match value {
        "white" => return Some([1.0, 1.0, 1.0]),
        "black" => return Some([0.0, 0.0, 0.0]),
        _ => {}
    }
    let inner = value
        .strip_prefix("hsla(")
        .or_else(|| value.strip_prefix("hsl("))?
        .strip_suffix(')')?;
    let (variable, alpha) = match inner.split_once('/') {
        Some((variable, alpha)) => (variable.trim(), alpha.trim().parse::<f64>().ok()?),
        None => (inner.trim(), 1.0),
    };
    let name = variable.strip_prefix("var(--")?.strip_suffix(')')?;
    let hsl = match name {
        "text" => dark.text,
        "text-high" => dark.text_high,
        "bg" => dark.bg,
        "bg-high" => dark.bg_high,
        "action" => dark.action,
        "accent" => dark.accent,
        "error" => dark.error,
        _ => return None,
    };
    let fore = rgb_of_hsl(hsl);
    let alpha = alpha.clamp(0.0, 1.0);
    Some([0, 1, 2].map(|index| alpha * fore[index] + (1.0 - alpha) * under[index]))
}

/// CSS `hsl()` to sRGB channels in `0..=1`.
pub fn rgb_of_hsl(hsl: [u16; 3]) -> [f64; 3] {
    let hue = f64::from(hsl[0]);
    let saturation = f64::from(hsl[1]) / 100.0;
    let lightness = f64::from(hsl[2]) / 100.0;
    let amount = saturation * lightness.min(1.0 - lightness);
    let channel = |offset: f64| {
        let k = (offset + hue / 30.0).rem_euclid(12.0);
        lightness - amount * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    [channel(0.0), channel(8.0), channel(4.0)]
}

/// WCAG 2.1 relative luminance of sRGB channels in `0..=1`.
pub fn luminance(rgb: [f64; 3]) -> f64 {
    let linear = |value: f64| {
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

/// WCAG 2.1 contrast ratio of two colours.
pub fn contrast(first: [f64; 3], second: [f64; 3]) -> f64 {
    let (one, two) = (luminance(first), luminance(second));
    (one.max(two) + 0.05) / (one.min(two) + 0.05)
}

#[cfg(test)]
#[path = "themes_tests.rs"]
mod tests;
