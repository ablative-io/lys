//! The declared estate palette mapping for both Rauthy client themes, its
//! validation, and the WCAG 2.1 AA contrast measure.
//!
//! Invariants (ADR-021, ADR-010):
//!
//! - Both themes are dark only. The mapping sets exactly four dark fields:
//!   `text` from the estate `text` token (never `muted`), `bg` from `ink`,
//!   `bg_high` from `raised`, and `accent` from the client's own product
//!   accent: the identity orange for the platform client, Cambium green for
//!   Cambium's. [`ThemeMapping::apply`] copies every other field, the eight
//!   gaps, from what Rauthy already holds; nothing here chooses a value the
//!   estate does not name.
//! - Every declared HSL triple is the rounding of its declared hex value, so
//!   the recorded conversion cannot drift from the token it cites.
//! - No accent is Aion blue, and no purple token or value enters the mapping.
//! - Readable contrast is measured, never asserted: six pairs per client with
//!   the WCAG relative-luminance formula, a translucent CSS value composited
//!   over what lies beneath it. A pair below its tier refuses the theme by the
//!   pair's name; the fix is a token, not a value chosen here.

use std::path::Path;

use serde::Deserialize;

use crate::identity::config::ClientRole;
use crate::identity::error::{IdentityError, IdentityResult};
use crate::identity::rauthy::{ThemeColours, ThemeDocument};

/// The mapping file's schema tag.
pub const SCHEMA: &str = "lys/identity-rauthy-themes/v1";

/// The estate colour token file, in the ablative docs repository.
pub const TOKEN_FILE: &str = "docs/design-system-v2/palette/estate-colour-tokens.json";

/// Aion's accent, which neither client may wear.
pub const AION_BLUE: &str = "#6B96D1";

/// The estate's banned purple status value.
pub const BANNED_PURPLE: &str = "#A78BFA";

/// The body-text tier: 4.5 to 1.
pub const BODY_TEXT: Tier = Tier {
    name: "body text",
    minimum: 4.5,
};

/// The tier for large text, icons and interface components: 3 to 1.
pub const LARGE_TEXT: Tier = Tier {
    name: "large text, icons and components",
    minimum: 3.0,
};

/// A WCAG 2.1 AA contrast tier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tier {
    /// The tier's name.
    pub name: &'static str,
    /// The minimum ratio.
    pub minimum: f64,
}

/// One measured pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    /// The pair's name, e.g. `text over bg (ink)`.
    pub pair: &'static str,
    /// The measured ratio.
    pub ratio: f64,
    /// The tier it must meet.
    pub tier: Tier,
}

impl Measurement {
    /// Whether the ratio meets the tier.
    pub fn passes(&self) -> bool {
        self.ratio >= self.tier.minimum
    }
}

/// `deploy/identity/rauthy-themes.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeMapping {
    /// Schema tag.
    pub schema: String,
    /// The decision the mapping follows.
    pub decision: String,
    /// Where the token values were read.
    pub source: TokenSource,
    /// The only mode the mapping sets.
    pub mode: String,
    /// The shared neutral tokens.
    pub foundation: Foundation,
    /// Each client's accent.
    pub clients: MappedClients,
}

/// The estate token file the values come from.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenSource {
    /// Repository name.
    pub repository: String,
    /// Commit the values were read at.
    pub commit: String,
    /// Path in that repository.
    pub path: String,
    /// The file's owner.
    pub owner: String,
}

/// The foundation tokens the mapping uses.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Foundation {
    /// Estate `text`, for Rauthy `text`.
    pub text: TokenColour,
    /// Estate `ink`, for Rauthy `bg`.
    pub bg: TokenColour,
    /// Estate `raised`, for Rauthy `bg_high`.
    pub bg_high: TokenColour,
}

/// Each client's accent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappedClients {
    /// The platform (identity product) client.
    pub platform: ClientAccent,
    /// Cambium's client.
    pub cambium: ClientAccent,
}

/// One client's accent.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientAccent {
    /// The product accent token.
    pub accent: TokenColour,
}

/// One estate token with its hex value and its declared HSL conversion.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenColour {
    /// The token's path in the estate file.
    pub token: String,
    /// The token's value.
    pub hex: String,
    /// The value as Rauthy's hue, saturation and lightness integers.
    pub hsl: [u16; 3],
}

impl ThemeMapping {
    /// Read and validate the mapping at `path`.
    pub fn load(path: &Path) -> IdentityResult<Self> {
        let refuse = |reason: String| IdentityError::ThemesInvalid {
            path: path.to_path_buf(),
            reason,
        };
        let text = std::fs::read_to_string(path).map_err(|error| refuse(error.to_string()))?;
        let mapping: Self = serde_json::from_str(&text).map_err(|error| refuse(error.to_string()))?;
        mapping.validate().map_err(refuse)?;
        Ok(mapping)
    }

    /// The accent token of the client playing `role`.
    pub fn accent(&self, role: ClientRole) -> &TokenColour {
        match role {
            ClientRole::Platform => &self.clients.platform.accent,
            ClientRole::Cambium => &self.clients.cambium.accent,
        }
    }

    /// The theme to write for `client_id`: `current` with exactly the four
    /// mapped dark fields replaced. Every gap keeps what Rauthy holds.
    pub fn apply(&self, role: ClientRole, client_id: &str, current: &ThemeDocument) -> ThemeDocument {
        let mut theme = current.clone();
        theme.client_id = client_id.to_string();
        theme.dark.text = self.foundation.text.hsl;
        theme.dark.bg = self.foundation.bg.hsl;
        theme.dark.bg_high = self.foundation.bg_high.hsl;
        theme.dark.accent = self.accent(role).hsl;
        theme
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("schema is {:?}, expected {SCHEMA:?}", self.schema));
        }
        if self.decision != "ADR-021" {
            return Err(format!("decision is {:?}, expected \"ADR-021\"", self.decision));
        }
        if self.source.path != TOKEN_FILE
            || [&self.source.repository, &self.source.commit, &self.source.owner]
                .iter()
                .any(|value| value.is_empty())
        {
            return Err(format!(
                "source must name the repository, commit and owner of {TOKEN_FILE}"
            ));
        }
        if self.mode != "dark" {
            return Err(format!("mode is {:?}; the themes are dark only (ADR-021)", self.mode));
        }
        let expected = [
            (&self.foundation.text, "foundation.text"),
            (&self.foundation.bg, "foundation.ink"),
            (&self.foundation.bg_high, "foundation.raised"),
            (&self.clients.platform.accent, "products.identity.accent"),
            (&self.clients.cambium.accent, "products.cambium.accent"),
        ];
        for (colour, token) in expected {
            if colour.token != token {
                return Err(format!("{:?} is mapped where {token:?} belongs", colour.token));
            }
            check_colour(colour)?;
        }
        for accent in [&self.clients.platform.accent, &self.clients.cambium.accent] {
            if accent.hex.eq_ignore_ascii_case(AION_BLUE) {
                return Err(format!("{} is Aion blue; each product keeps its own accent", accent.token));
            }
        }
        Ok(())
    }
}

fn check_colour(colour: &TokenColour) -> Result<(), String> {
    if colour.token.to_ascii_lowercase().contains("purple") || colour.hex.eq_ignore_ascii_case(BANNED_PURPLE) {
        return Err(format!("{} is purple, which the estate bans", colour.token));
    }
    let rgb = parse_hex(&colour.hex).ok_or_else(|| format!("{}: {:?} is not #RRGGBB", colour.token, colour.hex))?;
    let [h, s, l] = colour.hsl;
    if h > 360 || s > 100 || l > 100 {
        return Err(format!("{}: {:?} is outside Rauthy's HSL ranges", colour.token, colour.hsl));
    }
    let exact = rgb_to_hsl(rgb);
    let declared = [f64::from(h), f64::from(s), f64::from(l)];
    let within = declared
        .iter()
        .zip(exact.iter())
        .all(|(declared, exact)| (declared - exact).abs() <= 0.5 + 1e-9);
    if !within {
        return Err(format!(
            "{}: {:?} is not the rounded conversion of {} ({:.2}, {:.2}, {:.2})",
            colour.token, colour.hsl, colour.hex, exact[0], exact[1], exact[2]
        ));
    }
    Ok(())
}

/// The six pairs of one client's dark theme, measured.
pub fn measure(client: &str, colours: &ThemeColours) -> IdentityResult<Vec<Measurement>> {
    let ink = hsl_to_rgb(colours.bg);
    let raised = hsl_to_rgb(colours.bg_high);
    let text = hsl_to_rgb(colours.text);
    let text_high = hsl_to_rgb(colours.text_high);
    let action = hsl_to_rgb(colours.action);
    let resolve = |field: &'static str, value: &str, beneath: [f64; 3]| {
        resolve_css(value, colours, beneath).ok_or_else(|| IdentityError::ThemeValueUnmeasurable {
            client: client.to_string(),
            field,
            value: value.to_string(),
        })
    };
    let btn_text = resolve("btn_text", &colours.btn_text, action)?;
    let theme_sun = resolve("theme_sun", &colours.theme_sun, ink)?;
    let theme_moon = resolve("theme_moon", &colours.theme_moon, ink)?;
    let pair = |pair, first, second, tier| Measurement {
        pair,
        ratio: contrast_ratio(first, second),
        tier,
    };
    Ok(vec![
        pair("text over bg (ink)", text, ink, BODY_TEXT),
        pair("text over bg_high (raised)", text, raised, BODY_TEXT),
        pair("text_high over ink", text_high, ink, BODY_TEXT),
        pair("btn_text over action", btn_text, action, BODY_TEXT),
        pair("theme_sun over ink", theme_sun, ink, LARGE_TEXT),
        pair("theme_moon over ink", theme_moon, ink, LARGE_TEXT),
    ])
}

/// Measure one client's dark theme and refuse it if any pair is below its
/// tier, naming the first such pair and its ratio.
pub fn check_contrast(client: &str, colours: &ThemeColours) -> IdentityResult<Vec<Measurement>> {
    let measured = measure(client, colours)?;
    if let Some(failing) = measured.iter().find(|measurement| !measurement.passes()) {
        return Err(IdentityError::ContrastBelowTier {
            client: client.to_string(),
            pair: failing.pair,
            ratio: failing.ratio,
            tier: failing.tier.name,
            required: failing.tier.minimum,
        });
    }
    Ok(measured)
}

/// Resolve the CSS values Rauthy's theme uses to an opaque sRGB colour,
/// composited over `beneath`. `None` for any other form.
fn resolve_css(value: &str, colours: &ThemeColours, beneath: [f64; 3]) -> Option<[f64; 3]> {
    let value = value.trim();
    match value {
        "white" => return Some([1.0, 1.0, 1.0]),
        "black" => return Some([0.0, 0.0, 0.0]),
        _ => {}
    }
    let (inner, alpha) = if let Some(inner) = value.strip_prefix("hsla(").and_then(|v| v.strip_suffix(')')) {
        let (variable, alpha) = inner.split_once('/')?;
        (variable.trim(), alpha.trim().parse::<f64>().ok()?)
    } else {
        let inner = value.strip_prefix("hsl(")?.strip_suffix(')')?;
        (inner.trim(), 1.0)
    };
    if !(0.0..=1.0).contains(&alpha) {
        return None;
    }
    let name = inner.strip_prefix("var(--")?.strip_suffix(')')?;
    let hsl = match name {
        "text" => colours.text,
        "text-high" | "text_high" => colours.text_high,
        "bg" => colours.bg,
        "bg-high" | "bg_high" => colours.bg_high,
        "action" => colours.action,
        "accent" => colours.accent,
        "error" => colours.error,
        _ => return None,
    };
    let colour = hsl_to_rgb(hsl);
    Some([0, 1, 2].map(|i| colour[i] * alpha + beneath[i] * (1.0 - alpha)))
}

/// CSS `hsl()` to sRGB channels in `0..=1`.
pub fn hsl_to_rgb(hsl: [u16; 3]) -> [f64; 3] {
    let hue = f64::from(hsl[0] % 360);
    let saturation = f64::from(hsl[1].min(100)) / 100.0;
    let lightness = f64::from(hsl[2].min(100)) / 100.0;
    let a = saturation * lightness.min(1.0 - lightness);
    let channel = |n: f64| {
        let k = (n + hue / 30.0) % 12.0;
        lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    [channel(0.0), channel(8.0), channel(4.0)]
}

/// sRGB channels in `0..=1` to exact (unrounded) hue, saturation and
/// lightness in Rauthy's units.
pub fn rgb_to_hsl(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = f64::midpoint(max, min);
    let delta = max - min;
    if delta <= f64::EPSILON {
        return [0.0, 0.0, lightness * 100.0];
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let sector = if (max - r).abs() <= f64::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f64::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    [sector * 60.0, saturation * 100.0, lightness * 100.0]
}

/// `#RRGGBB` to sRGB channels in `0..=1`.
pub fn parse_hex(hex: &str) -> Option<[f64; 3]> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok().map(|v| f64::from(v) / 255.0);
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// WCAG 2.1 relative luminance of an sRGB colour.
pub fn relative_luminance(rgb: [f64; 3]) -> f64 {
    let linear = |c: f64| {
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

/// WCAG 2.1 contrast ratio of two colours, lighter over darker.
pub fn contrast_ratio(first: [f64; 3], second: [f64; 3]) -> f64 {
    let a = relative_luminance(first);
    let b = relative_luminance(second);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[cfg(test)]
#[path = "themes_tests.rs"]
mod tests;
