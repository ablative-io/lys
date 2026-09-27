# Rauthy client theme map

How the two Rauthy client themes take their colours from the estate colour
tokens, under ADR-021: both themes are dark only, a Rauthy field takes an estate
token only where the estate names the same role, and every other field is a gap
that keeps Rauthy's own default. ADR-010 sets the accents: every product shares
one design and keeps its own accent.

The declared mapping is `deploy/identity/rauthy-themes.json`.
`crates/lys/src/identity/themes.rs` reads and validates it, and
`lys identity configure` writes it to each client's theme.

## Source

- File: `docs/design-system-v2/palette/estate-colour-tokens.json` in the
  ablative docs repository, owned by Waffles, who added the identity entry at
  ablative-docs `385916e`. It is another repository's file: read, never changed.
- Values read from it: `foundation.ink` `#0C0E10`, `foundation.raised`
  `#1E2226`, `foundation.text` `#E8EAEC`, `products.identity.accent` `#D4975A`,
  `products.cambium.accent` `#5E8C6A`.
- Not copied: the purple status token the file lists under `open_items`. The
  estate bans purple, and no purple value appears in the mapping.

## Mapped fields (dark mode)

Rauthy stores each colour as three integers, hue in degrees and saturation and
lightness in percent. Each conversion is the standard hex to HSL conversion,
rounded to the nearest integer; the rounding moves no channel by more than one
step of 255.

| Rauthy field | Estate token | Hex | HSL (h, s%, l%) | Clients |
|---|---|---|---|---|
| `dark.text` | `foundation.text` | `#E8EAEC` | 210, 10, 92 | both |
| `dark.bg` | `foundation.ink` | `#0C0E10` | 210, 14, 5 | both |
| `dark.bg_high` | `foundation.raised` | `#1E2226` | 210, 12, 13 | both |
| `dark.accent` | `products.identity.accent` | `#D4975A` | 30, 59, 59 | platform (the identity product, orange) |
| `dark.accent` | `products.cambium.accent` | `#5E8C6A` | 136, 20, 46 | Cambium (green) |

Conversions, worked:

- `#E8EAEC` = (232, 234, 236): max 236, min 232, lightness 234/255 = 91.8%,
  saturation 4 / (510 - 468) = 9.5%, hue 210 (blue channel largest).
- `#0C0E10` = (12, 14, 16): lightness 14/255 = 5.5%, saturation 4 / 28 =
  14.3%, hue 210.
- `#1E2226` = (30, 34, 38): lightness 34/255 = 13.3%, saturation 8 / 68 =
  11.8%, hue 210.
- `#D4975A` = (212, 151, 90): lightness 151/255 = 59.2%, saturation
  122 / (510 - 302) = 58.7%, hue 60 x (151 - 90) / 122 = 30.
- `#5E8C6A` = (94, 140, 106): lightness 117/255 = 45.9%, saturation 46 / 234
  = 19.7%, hue 120 + 60 x (106 - 94) / 46 = 135.7.

`dark.text` takes `text`, never `muted`: mapping text to muted would set the
body of every page in the secondary colour (ADR-021, rejected).

## Gaps

The estate names no token for these eight. Each is a gap: it keeps Rauthy's own
default, the mapping sets no value for it, and nothing reads it. A design-system
card that names a token fills it; until then configure writes back exactly what
Rauthy holds (ADR-021).

| Gap | Rauthy fields | Keeps |
|---|---|---|
| light mode | every `light.*` field | Rauthy's own default |
| the error colour | `dark.error` | Rauthy's own default |
| the radius | `border_radius` | Rauthy's own default |
| text_high | `dark.text_high` | Rauthy's own default |
| action | `dark.action` | Rauthy's own default |
| btn_text | `dark.btn_text` | Rauthy's own default |
| theme_sun | `dark.theme_sun` | Rauthy's own default |
| theme_moon | `dark.theme_moon` | Rauthy's own default |

The count is eight rather than the six first named because the pinned Rauthy's
theme also carries `theme_sun` and `theme_moon` per mode, for which the estate
names no token.

The pinned Rauthy's dark defaults for the dark gaps, from
`src/data/src/entity/theme.rs` at `dd61ac3c84d6b238108dc8438b53043b5177a662`:
`text_high` 34, 7, 90; `action` 34, 100, 59; `error` 15, 100, 37;
`btn_text` `hsl(var(--bg))`; `theme_sun` `hsla(var(--action) / .7)`;
`theme_moon` `hsla(var(--accent) / .85)`; `border_radius` `5px`.

## Readable contrast

WCAG 2.1 AA, computed with the WCAG relative-luminance formula over six pairs
per client, a gap field measured at the pinned Rauthy's own default. A
translucent value is composited over ink before it is measured.

| Pair | Tier | Platform | Cambium |
|---|---|---|---|
| text over bg (ink) | body text, 4.5:1 | 16.25 | 16.25 |
| text over bg_high (raised) | body text, 4.5:1 | 13.48 | 13.48 |
| text_high over ink | body text, 4.5:1 | 15.61 | 15.61 |
| btn_text over action | body text, 4.5:1 | 9.87 | 9.87 |
| theme_sun over ink | large text, icons and components, 3:1 | 5.24 | 5.24 |
| theme_moon over ink | large text, icons and components, 3:1 | 5.83 | 3.97 |

Every pair is at or above its tier, so no gap default fails over ink and no row
stop is raised. `themes.rs` measures the declared mapping before configure
writes it, and `crates/lys/tests/identity_theme.rs` measures the values the
running Rauthy exports.
