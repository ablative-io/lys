//! The theme mapping `deploy/identity/rauthy-themes.json` declares, read,
//! validated (every HSL triple in range, readable contrast) and turned into
//! the theme record Rauthy takes for one client.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use super::error::{IdentityError, IdentityResult};
use super::rauthy::{Theme, ThemeCss};

/// The mapping file.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeMap {
    /// The border radius every theme uses, a CSS value.
    pub border_radius: String,
    /// The themes by name: `identity` and `cambium`.
    pub themes: BTreeMap<String, ThemePair>,
}

/// A theme's two palettes.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemePair {
    /// The light palette.
    pub light: Palette,
    /// The dark palette.
    pub dark: Palette,
}

/// One palette as the file declares it; the recorded contrast ratios are
/// documentation and are recomputed here rather than trusted.
#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    /// Body text.
    pub text: [u16; 3],
    /// Emphasised text.
    pub text_high: [u16; 3],
    /// Page background.
    pub bg: [u16; 3],
    /// Raised background.
    pub bg_high: [u16; 3],
    /// The action colour.
    pub action: [u16; 3],
    /// The accent colour.
    pub accent: [u16; 3],
    /// The error colour.
    pub error: [u16; 3],
    /// The button text colour, a CSS value.
    pub btn_text: String,
    /// The sun icon colour, a CSS value.
    pub theme_sun: String,
    /// The moon icon colour, a CSS value.
    pub theme_moon: String,
}

/// The least contrast body text may have on its background.
pub const TEXT_CONTRAST: f64 = 4.5;
/// The least contrast the action colour may have on the page background.
pub const ACTION_CONTRAST: f64 = 3.0;

impl ThemeMap {
    /// Read and validate the mapping at `path`.
    pub fn load(path: &Path) -> IdentityResult<Self> {
        let bytes = std::fs::read(path).map_err(|source| IdentityError::Io {
            operation: "read the theme mapping",
            path: path.to_path_buf(),
            source,
        })?;
        let map: Self = serde_json::from_slice(&bytes).map_err(|error| IdentityError::Config {
            operation: "read the theme mapping",
            path: path.to_path_buf(),
            detail: error.to_string(),
        })?;
        for (name, pair) in &map.themes {
            pair.light.validate(name, "light")?;
            pair.dark.validate(name, "dark")?;
        }
        Ok(map)
    }

    /// The theme record for client `client_id` from the named theme.
    pub fn theme_for(&self, name: &str, client_id: &str) -> IdentityResult<Theme> {
        let pair = self
            .themes
            .get(name)
            .ok_or_else(|| IdentityError::Invalid {
                operation: "read the theme mapping",
                resource: name.to_string(),
                detail: "the mapping declares no theme of that name".to_string(),
            })?;
        Ok(Theme {
            client_id: client_id.to_string(),
            light: pair.light.css(),
            dark: pair.dark.css(),
            border_radius: self.border_radius.clone(),
        })
    }
}

impl Palette {
    fn css(&self) -> ThemeCss {
        ThemeCss {
            text: self.text,
            text_high: self.text_high,
            bg: self.bg,
            bg_high: self.bg_high,
            action: self.action,
            accent: self.accent,
            error: self.error,
            btn_text: self.btn_text.clone(),
            theme_sun: self.theme_sun.clone(),
            theme_moon: self.theme_moon.clone(),
        }
    }

    fn validate(&self, name: &str, mode: &str) -> IdentityResult<()> {
        let resource = format!("{name}.{mode}");
        for (field, hsl) in [
            ("text", self.text),
            ("text_high", self.text_high),
            ("bg", self.bg),
            ("bg_high", self.bg_high),
            ("action", self.action),
            ("accent", self.accent),
            ("error", self.error),
        ] {
            if hsl[0] > 360 || hsl[1] > 100 || hsl[2] > 100 {
                return Err(IdentityError::Invalid {
                    operation: "validate the theme mapping",
                    resource,
                    detail: format!(
                        "{field} is out of range: hue to 360, saturation and lightness to 100"
                    ),
                });
            }
        }
        for (pair, fore, back, least) in [
            ("text on bg", self.text, self.bg, TEXT_CONTRAST),
            (
                "text_high on bg_high",
                self.text_high,
                self.bg_high,
                TEXT_CONTRAST,
            ),
            ("action on bg", self.action, self.bg, ACTION_CONTRAST),
        ] {
            let ratio = contrast(fore, back);
            if ratio < least {
                return Err(IdentityError::Invalid {
                    operation: "validate the theme mapping",
                    resource,
                    detail: format!("{pair} has contrast {ratio:.2}, under {least}"),
                });
            }
        }
        Ok(())
    }
}

/// The WCAG contrast ratio of two HSL colours.
#[must_use]
pub fn contrast(fore: [u16; 3], back: [u16; 3]) -> f64 {
    let (a, b) = (luminance(fore), luminance(back));
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

fn luminance(hsl: [u16; 3]) -> f64 {
    let (r, g, b) = rgb(hsl);
    let channel = |value: f64| {
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn rgb(hsl: [u16; 3]) -> (f64, f64, f64) {
    let hue = f64::from(hsl[0]) / 360.0;
    let sat = f64::from(hsl[1]) / 100.0;
    let light = f64::from(hsl[2]) / 100.0;
    if sat == 0.0 {
        return (light, light, light);
    }
    let upper = if light < 0.5 {
        light * (1.0 + sat)
    } else {
        light + sat - light * sat
    };
    let lower = 2.0f64.mul_add(light, -upper);
    let channel = |offset: f64| {
        let turn = if offset < 0.0 {
            offset + 1.0
        } else if offset > 1.0 {
            offset - 1.0
        } else {
            offset
        };
        if turn < 1.0 / 6.0 {
            (upper - lower).mul_add(6.0 * turn, lower)
        } else if turn < 0.5 {
            upper
        } else if turn < 2.0 / 3.0 {
            (upper - lower).mul_add((2.0 / 3.0 - turn) * 6.0, lower)
        } else {
            lower
        }
    };
    (
        channel(hue + 1.0 / 3.0),
        channel(hue),
        channel(hue - 1.0 / 3.0),
    )
}
