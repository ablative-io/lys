# Rauthy client themes: the estate token map

Both Rauthy client themes are dark only, from the estate colour tokens (ADR-021),
with each client wearing its own product accent (ADR-010). The mapping itself is
`deploy/identity/rauthy-themes.json`; `crates/lys-install/src/themes.rs` reads and
validates it, and `lys identity configure` applies it.

## Source

- Repository: ablative-docs, owner Waffles
- File: `docs/design-system-v2/palette/estate-colour-tokens.json`
- Commit: `385916eb437cae62c4269e6db1db2e542af5749e` (385916e, the commit that added the
  identity entry). The file is read, never changed.
- The tokens' purple status entry (`open_items.aion_status_special`) is not copied.

## Mapped fields (dark mode)

A Rauthy field takes an estate token only where the estate names the same role.

| Rauthy field | Estate token | Hex | HSL given to Rauthy | Clients |
|---|---|---|---|---|
| `text` | `foundation.text` | `#E8EAEC` | 210 10 92 | both |
| `bg` | `foundation.ink` | `#0C0E10` | 210 14 5 | both |
| `bg_high` | `foundation.raised` | `#1E2226` | 210 12 13 | both |
| `accent` | `products.cambium.accent` (Cambium green) | `#5E8C6A` | 136 20 46 | cambium |
| `accent` | `products.identity.accent` (identity orange) | `#D4975A` | 30 59 59 | platform |

`text` is not mapped to `foundation.muted`: that would set the body of every page in
the secondary colour.

## Conversion

Each hex value is read as 8-bit sRGB channels scaled to 0..1. Lightness is the mean
of the largest and smallest channel; saturation is their difference over
`1 - |2L - 1|`; hue is the standard sextant formula in degrees. All three are
rounded to whole numbers, because Rauthy stores HSL as three integers. `themes.rs`
repeats the conversion and refuses a mapping whose HSL differs from its hex.

## Gaps

Every Rauthy field the estate names no token for is a gap. A gap keeps Rauthy's own
default, is not set by `lys identity configure`, and cannot be named in the mapping
file, whose field set is closed (ADR-021). There are eight:

1. light mode — every light field keeps Rauthy's default
2. the error colour (`error`) — keeps Rauthy's default
3. the radius (`border_radius`) — keeps Rauthy's default
4. `text_high` — keeps Rauthy's default
5. `action` — keeps Rauthy's default
6. `btn_text` — keeps Rauthy's default
7. `theme_sun` — keeps Rauthy's default
8. `theme_moon` — keeps Rauthy's default

The count is eight rather than six because the pinned Rauthy's theme also carries
`theme_sun` and `theme_moon` per mode. The fix for any gap is a token the
design-system card names, not a value chosen here.

The pinned Rauthy's dark defaults for the gaps (v0.36.2,
`src/data/src/entity/theme.rs`): `text_high` 34 7 90, `action` 34 100 59, `error`
15 100 37, `btn_text` `hsl(var(--bg))`, `theme_sun` `hsla(var(--action) / .7)`,
`theme_moon` `hsla(var(--accent) / .85)`, radius `5px`.

## Contrast

WCAG 2.1 AA, relative-luminance formula, six pairs per client, gap fields at
Rauthy's default, CSS variable forms resolved in the dark mode and alpha composited
over ink:

| Pair | Tier | Cambium | Platform |
|---|---|---|---|
| text over bg (ink) | 4.5 | 16.25 | 16.25 |
| text over bg_high (raised) | 4.5 | 13.48 | 13.48 |
| text_high over ink | 4.5 | 15.61 | 15.61 |
| btn_text over action | 4.5 | 9.87 | 9.87 |
| theme_sun over ink | 3 | 5.24 | 5.24 |
| theme_moon over ink | 3 | 3.97 | 5.83 |

These are computed from the declared mapping and the pinned defaults; the counted
leg of `identity_theme` measures the same pairs from the values the running Rauthy
exports. No gap default fails over ink, so no pair is stopped under CN9.
