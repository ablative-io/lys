# Rauthy client themes: token map

DIRECTORY-002 R3. Both Rauthy client themes are **dark only** (ADR-021), from Aion's pinned
neutral and text vocabulary and each client's own product accent (ADR-010): the Cambium client
wears Cambium green, the platform (identity) client the identity orange. Neither wears Aion blue,
and the estate's banned purple status token is not copied. Fonts and page layout stay Rauthy's.

`deploy/identity/rauthy-themes.json` is the mapping; `crates/lys/src/identity/themes.rs` reads
and validates it, and `lys identity configure` applies it.

## Source

The estate colour tokens, `docs/design-system-v2/palette/estate-colour-tokens.json` in the ablative
docs repository at `385916e` (owner: Waffles, who added the identity entry there; read, never
changed here).

That repository was not reachable from the seat that wrote this row (`ablative-io/ablative-docs`
does not resolve). The values were therefore read from the estate token file's copy in
`ablative-io/design-system`, `palette/estate-colour-tokens.json` at
`3c3bac715fb60348017f0d5cf40b412cf168da77`, whose identity entry is exactly the one ADR-010
records from `385916e` (accent `#D4975A`, deep `#A86B2E`, wash `#3D2A17`). The mapping file
records both in `source.ref` and `source.read_copy`. Re-reading `385916e` itself is a check for
whoever next holds that repository.

## Mapped fields

A Rauthy field takes an estate token only where the estate names the same role. Conversion is
sRGB hex to HSL, rounded to whole degrees and whole percentages, the form Rauthy stores; each
triple is recomputed from its hex when the mapping is loaded.

| Rauthy dark field | Estate token | Hex | HSL (exact) | HSL as stored |
| --- | --- | --- | --- | --- |
| `text` | `foundation.text` | `#E8EAEC` | 210°, 9.52 %, 91.76 % | `[210, 10, 92]` |
| `bg` | `foundation.ink` | `#0C0E10` | 210°, 14.29 %, 5.49 % | `[210, 14, 5]` |
| `bg_high` | `foundation.raised` | `#1E2226` | 210°, 11.76 %, 13.33 % | `[210, 12, 13]` |
| `accent`, platform client | `products.identity.accent` | `#D4975A` | 30.00°, 58.65 %, 59.22 % | `[30, 59, 59]` |
| `accent`, Cambium client | `products.cambium.accent` | `#5E8C6A` | 135.65°, 19.66 %, 45.88 % | `[136, 20, 46]` |

`text` is never mapped to `foundation.muted`: that would set the body of every page in the
secondary colour (ADR-021, rejected).

## The eight gaps

The estate names no token for these, so each **keeps Rauthy's own default** at the pinned
commit (`vendor/rauthy/src/data/src/entity/theme.rs`, `default_dark` and `default_light`, and
`ThemeCssFull::default` for the radius), and nothing in this repository sets a value for one;
the contrast check and ID001_THEME only read those defaults to measure them (ADR-021). A design-system card that names tokens for them fills them.

| # | Gap | Keeps Rauthy's own default |
| --- | --- | --- |
| 1 | light mode (every light field) | text `[200, 5, 37]`, text_high `[200, 15, 25]`, bg `[34, 25, 97]`, bg_high `[34, 20, 90]`, action `[34, 100, 40]`, accent `[265, 100, 53]`, error `[15, 100, 37]`, btn_text `white`, theme_sun `hsla(var(--action) / .7)`, theme_moon `hsla(var(--accent) / .85)` |
| 2 | the error colour | dark error `[15, 100, 37]` |
| 3 | the radius | `5px` |
| 4 | `text_high` | dark `[34, 7, 90]` |
| 5 | `action` | dark `[34, 100, 59]` |
| 6 | `btn_text` | dark `hsl(var(--bg))`, which resolves to the mapped ink |
| 7 | `theme_sun` | dark `hsla(var(--action) / .7)` |
| 8 | `theme_moon` | dark `hsla(var(--accent) / .85)`, which resolves to the client's mapped accent |

The count is eight rather than the six first named because the pinned Rauthy's theme also
carries `theme_sun` and `theme_moon` per mode, for which the estate names no token. Rauthy's
theme API takes a whole theme on every write, so `configure` reads the theme Rauthy holds and
writes it back with only the four mapped dark fields changed: every gap goes back exactly as
Rauthy returned it.

Rauthy's light defaults include its own purple accent `[265, 100, 53]`. That value is Rauthy's,
kept because light mode is a gap; it is not an estate token and appears nowhere in the mapping.

## Readable contrast

WCAG 2.1 AA, computed with the WCAG relative-luminance formula over six pairs per client, each
gap field at Rauthy's own default, alpha values composited over the colour they sit on. Measured
on the stored HSL values:

| Pair | Tier | Platform | Cambium |
| --- | --- | --- | --- |
| text over bg (ink) | 4.5:1 body text | 16.25 | 16.25 |
| text over bg_high (raised) | 4.5:1 body text | 13.48 | 13.48 |
| text_high over ink | 4.5:1 body text | 15.61 | 15.61 |
| btn_text over action | 4.5:1 body text | 9.87 | 9.87 |
| theme_sun over ink | 3:1 large text, icons and components | 5.24 | 5.24 |
| theme_moon over ink | 3:1 large text, icons and components | 5.83 | 3.97 |

All twelve meet their tier, so no gap stops the row. `configure` refuses to write a theme whose
pairs fall below their tier (`contrast_below_tier`, naming the pair), and `identity_theme`
measures the same pairs again from what the running Rauthy exports.
