//! The declared estate palette mapping for both Rauthy client themes, its
//! validation, and the WCAG 2.1 AA contrast check (ADR-010, ADR-021).
//!
//! Dark mode only: `text`, `bg`, `bg_high` and `accent` take estate tokens; every
//! other field is a gap that keeps the pinned Rauthy's own default, and the
//! mapping cannot name one, because its field set is closed. Contrast is
//! measured over six pairs per client with gap fields at the value Rauthy
//! holds, resolving the CSS variable forms Rauthy's defaults use.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::config::ClientRole;
use super::error::{ErrorKind, IdentityError, IdentityResult};
use super::rauthy::{Theme, ThemeCss};

/// The mapping as reviewed, carried in the binary so configure applies
/// exactly the file in the repository.
pub const DECLARED: &str = include_str!("../../../deploy/identity/rauthy-themes.json");

/// The body-text tier of WCAG 2.1 AA.
pub const BODY_TIER: f64 = 4.5;

/// The tier for large text, icons and interface components.
pub const COMPONENT_TIER: f64 = 3.0;

const AION_BLUE: &str = "#6B96D1";

/// The whole declared mapping.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeMapping {
    /// The decision the mapping carries out.
    pub decision: String,
    /// The one mode the mapping sets.
    pub mode: String,
    /// Where the colour values were read.
    pub source: Source,
    /// Per client role, the dark fields taken from the estate.
    pub clients: BTreeMap<String, ClientTheme>,
}

/// The estate colour tokens the values were read from.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The repository that owns the tokens.
    pub repository: String,
    /// The tokens file in that repository.
    pub path: String,
    /// The commit the values were read at.
    pub commit: String,
    /// The tokens' owner.
    pub owner: String,
}

/// One client's mapping.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientTheme {
    /// The estate product whose accent the client wears.
    pub product: String,
    /// The four dark fields the estate names a token for.
    pub dark: DarkFields,
}

/// The only fields the mapping may set.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarkFields {
    /// Body text from `foundation.text`.
    pub text: Field,
    /// Page background from `foundation.ink`.
    pub bg: Field,
    /// Raised background from `foundation.raised`.
    pub bg_high: Field,
    /// Accent from the product's accent.
    pub accent: Field,
}

/// One mapped field: its source token, the token's hex value and the HSL
/// Rauthy is given.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// The token's path in the estate tokens file.
    pub token: String,
    /// The token's value.
    pub hex: String,
    /// The value as whole-number hue, saturation and lightness.
    pub hsl: [u16; 3],
}

/// One measured contrast pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    /// Foreground over background, by Rauthy field names.
    pub name: &'static str,
    /// The WCAG contrast ratio.
    pub ratio: f64,
    /// The tier the pair must meet.
    pub tier: f64,
}

impl Pair {
    /// Whether the ratio meets the tier.
    pub fn passes(&self) -> bool {
        self.ratio >= self.tier
    }
}

fn invalid(resource: &str, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(
        ErrorKind::ThemeInvalid,
        "validate theme mapping",
        resource,
        detail,
    )
}

/// The pinned Rauthy's dark defaults for every field the mapping leaves a gap.
pub fn rauthy_dark_defaults() -> ThemeCss {
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

impl ThemeMapping {
    /// Parses and validates the declared mapping.
    pub fn declared() -> IdentityResult<Self> {
        Self::parse(DECLARED)
    }

    /// Parses and validates a mapping.
    pub fn parse(text: &str) -> IdentityResult<Self> {
        let mapping: Self =
            serde_json::from_str(text).map_err(|error| invalid("mapping", error.to_string()))?;
        mapping.validate()?;
        Ok(mapping)
    }

    fn validate(&self) -> IdentityResult<()> {
        if self.decision != "ADR-021" || self.mode != "dark" {
            return Err(invalid(
                "mapping",
                "the mapping carries ADR-021 and sets dark mode only",
            ));
        }
        let roles: Vec<&str> = self.clients.keys().map(String::as_str).collect();
        if roles != ["cambium", "platform"] {
            return Err(invalid(
                "clients",
                "exactly the cambium and platform clients are themed",
            ));
        }
        for (role, client) in &self.clients {
            let product = if role == "platform" {
                "identity"
            } else {
                "cambium"
            };
            if client.product != product {
                return Err(invalid(
                    role,
                    format!("the {role} client wears the {product} accent"),
                ));
            }
            let accent_token = format!("products.{product}.accent");
            let expected = [
                ("text", &client.dark.text, "foundation.text"),
                ("bg", &client.dark.bg, "foundation.ink"),
                ("bg_high", &client.dark.bg_high, "foundation.raised"),
                ("accent", &client.dark.accent, accent_token.as_str()),
            ];
            for (name, field, token) in expected {
                if field.token != token {
                    return Err(invalid(
                        role,
                        format!("{name} takes {token}, not {}", field.token),
                    ));
                }
                let converted = hex_to_hsl(&field.hex)
                    .map_err(|refusal| invalid(role, format!("{name}: {refusal}")))?;
                if converted != field.hsl {
                    return Err(invalid(
                        role,
                        format!(
                            "{name}: {} converts to {converted:?}, not {:?}",
                            field.hex, field.hsl
                        ),
                    ));
                }
            }
            let accent = &client.dark.accent;
            if accent.hex.eq_ignore_ascii_case(AION_BLUE) || is_purple(accent.hsl) {
                return Err(invalid(
                    role,
                    "the accent is the product's own, never Aion blue or purple",
                ));
            }
            let pairs = contrast_pairs(&self.apply_to(role, &rauthy_dark_defaults()))?;
            if let Some(failing) = pairs.iter().find(|pair| !pair.passes()) {
                return Err(invalid(
                    role,
                    format!(
                        "{} measures {:.2} against its tier {:.1}",
                        failing.name, failing.ratio, failing.tier
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The client mapping for `role`.
    pub fn client(&self, role: ClientRole) -> IdentityResult<&ClientTheme> {
        self.clients
            .get(role.key())
            .ok_or_else(|| invalid(role.key(), "no mapping for this client"))
    }

    fn apply_to(&self, role: &str, current: &ThemeCss) -> ThemeCss {
        let mut dark = current.clone();
        if let Some(client) = self.clients.get(role) {
            dark.text = client.dark.text.hsl;
            dark.bg = client.dark.bg.hsl;
            dark.bg_high = client.dark.bg_high.hsl;
            dark.accent = client.dark.accent.hsl;
        }
        dark
    }

    /// The theme `current` becomes: the four mapped dark fields replaced,
    /// every other field left exactly as Rauthy holds it.
    pub fn apply(&self, role: ClientRole, current: &Theme) -> Theme {
        Theme {
            client_id: current.client_id.clone(),
            light: current.light.clone(),
            dark: self.apply_to(role.key(), &current.dark),
            border_radius: current.border_radius.clone(),
        }
    }
}

fn is_purple(hsl: [u16; 3]) -> bool {
    (255..=320).contains(&hsl[0]) && hsl[1] >= 15
}

/// Converts `#RRGGBB` to whole-number HSL, or names why it cannot.
pub fn hex_to_hsl(hex: &str) -> Result<[u16; 3], String> {
    let malformed = || format!("{hex} is not #RRGGBB");
    let digits = hex
        .strip_prefix('#')
        .filter(|d| d.len() == 6)
        .ok_or_else(malformed)?;
    let channel = |at: usize| {
        digits
            .get(at..at + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .ok_or_else(malformed)
    };
    let rgb = [channel(0)?, channel(2)?, channel(4)?].map(|c| f64::from(c) / 255.0);
    let max = rgb.iter().copied().fold(f64::MIN, f64::max);
    let min = rgb.iter().copied().fold(f64::MAX, f64::min);
    let lightness = f64::midpoint(max, min);
    let delta = max - min;
    if delta == 0.0 {
        return Ok([0, 0, to_whole("lightness", lightness * 100.0, 100)?]);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let [red, green, blue] = rgb;
    let hue = if (max - red).abs() < f64::EPSILON {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if (max - green).abs() < f64::EPSILON {
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    Ok([
        to_whole("hue", hue, 360)? % 360,
        to_whole("saturation", saturation * 100.0, 100)?,
        to_whole("lightness", lightness * 100.0, 100)?,
    ])
}

/// Rounds `value` to the nearest whole number in `0..=max`, refusing a value
/// outside that range (or not a number) by the component it measures. The
/// whole number is found by bisecting the range, so no float is cast.
fn to_whole(component: &str, value: f64, max: u16) -> Result<u16, String> {
    let rounded = value.round();
    if !(0.0..=f64::from(max)).contains(&rounded) {
        return Err(format!("{component} {value} is outside 0 to {max}"));
    }
    let (mut low, mut high) = (0_u16, max);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if f64::from(middle) <= rounded {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(low)
}

fn hsl_to_rgb([hue, saturation, lightness]: [u16; 3]) -> [f64; 3] {
    let sat = f64::from(saturation) / 100.0;
    let light = f64::from(lightness) / 100.0;
    let chroma = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let sector = f64::from(hue % 360) / 60.0;
    let second = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let [red, green, blue] = match hue % 360 / 60 {
        0 => [chroma, second, 0.0],
        1 => [second, chroma, 0.0],
        2 => [0.0, chroma, second],
        3 => [0.0, second, chroma],
        4 => [second, 0.0, chroma],
        _ => [chroma, 0.0, second],
    };
    let offset = light - chroma / 2.0;
    [red + offset, green + offset, blue + offset]
}

fn relative_luminance(rgb: [f64; 3]) -> f64 {
    let linear = rgb.map(|c| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

/// The WCAG contrast ratio of two opaque colours.
pub fn contrast_ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Resolves one of Rauthy's CSS colour values against the mode it sits in:
/// `hsl(var(--x))`, `hsla(var(--x) / alpha)`, `white` or `black`, composited
/// over the page background when it carries an alpha.
pub fn resolve_css(value: &str, mode: &ThemeCss) -> Option<[f64; 3]> {
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
    let (reference, alpha) = match inner.split_once('/') {
        Some((reference, alpha)) => (reference.trim(), alpha.trim().parse::<f64>().ok()?),
        None => (inner.trim(), 1.0),
    };
    let variable = reference.strip_prefix("var(--")?.strip_suffix(')')?;
    let base = match variable {
        "text" => mode.text,
        "text-high" => mode.text_high,
        "bg" => mode.bg,
        "bg-high" => mode.bg_high,
        "action" => mode.action,
        "accent" => mode.accent,
        "error" => mode.error,
        _ => return None,
    };
    let colour = hsl_to_rgb(base);
    let page = hsl_to_rgb(mode.bg);
    Some([0, 1, 2].map(|i| alpha * colour[i] + (1.0 - alpha) * page[i]))
}

/// The six pairs of one client's dark mode, with every value as `dark`
/// holds it.
pub fn contrast_pairs(dark: &ThemeCss) -> IdentityResult<Vec<Pair>> {
    let resolve = |name: &str, value: &str| {
        resolve_css(value, dark).ok_or_else(|| invalid(name, format!("cannot resolve {value}")))
    };
    let ink = hsl_to_rgb(dark.bg);
    let pairs = [
        ("text over bg", hsl_to_rgb(dark.text), ink, BODY_TIER),
        (
            "text over bg_high",
            hsl_to_rgb(dark.text),
            hsl_to_rgb(dark.bg_high),
            BODY_TIER,
        ),
        (
            "text_high over ink",
            hsl_to_rgb(dark.text_high),
            ink,
            BODY_TIER,
        ),
        (
            "btn_text over action",
            resolve("btn_text", &dark.btn_text)?,
            hsl_to_rgb(dark.action),
            BODY_TIER,
        ),
        (
            "theme_sun over ink",
            resolve("theme_sun", &dark.theme_sun)?,
            ink,
            COMPONENT_TIER,
        ),
        (
            "theme_moon over ink",
            resolve("theme_moon", &dark.theme_moon)?,
            ink,
            COMPONENT_TIER,
        ),
    ];
    Ok(pairs
        .into_iter()
        .map(|(name, foreground, background, tier)| Pair {
            name,
            ratio: contrast_ratio(foreground, background),
            tier,
        })
        .collect())
}

#[cfg(test)]
#[path = "themes_tests.rs"]
mod tests;
