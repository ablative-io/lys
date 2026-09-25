# Theme map: sources and conversions

Every colour in `rauthy-themes.json`, its source token, and its conversion to the HSL triple Rauthy takes. Neutral, text and radius come from Aion's console tokens (`apps/aion-ops-console/src/index.css` in the aion repository); each accent comes from the estate colour tokens (`docs/design-system-v2/palette/estate-colour-tokens.json` in the ablative docs repository, owner Waffles), read and never changed. The estate file's purple status token is not copied.

Border radius: `--radius` = `0.625rem`.

| theme | mode | field | source | hex | HSL |
|---|---|---|---|---|---|
| identity | light | text | aion --text-secondary | `#52525b` | [240, 5, 34] |
| identity | light | text_high | aion --text-primary | `#18181b` | [240, 6, 10] |
| identity | light | bg | aion --surface-base | `#f4f4f5` | [240, 5, 96] |
| identity | light | bg_high | aion --surface-card | `#ffffff` | [0, 0, 100] |
| identity | light | action | estate identity.deep | `#A86B2E` | [30, 57, 42] |
| identity | light | accent | estate identity.accent | `#D4975A` | [30, 59, 59] |
| identity | light | error | a red of the estate family, not in the tokens (recorded here) | `#c94f4f` | [0, 53, 55] |
| identity | dark | text | aion --text-secondary | `#a1a1aa` | [240, 5, 65] |
| identity | dark | text_high | aion --text-primary | `#f4f4f5` | [240, 5, 96] |
| identity | dark | bg | aion --surface-base | `#0f0f14` | [240, 14, 7] |
| identity | dark | bg_high | aion --surface-card | `#1a1a22` | [240, 13, 12] |
| identity | dark | action | estate identity.accent | `#D4975A` | [30, 59, 59] |
| identity | dark | accent | estate identity.deep | `#A86B2E` | [30, 57, 42] |
| identity | dark | error | a red of the estate family, not in the tokens (recorded here) | `#c94f4f` | [0, 53, 55] |
| cambium | light | text | aion --text-secondary | `#52525b` | [240, 5, 34] |
| cambium | light | text_high | aion --text-primary | `#18181b` | [240, 6, 10] |
| cambium | light | bg | aion --surface-base | `#f4f4f5` | [240, 5, 96] |
| cambium | light | bg_high | aion --surface-card | `#ffffff` | [0, 0, 100] |
| cambium | light | action | estate cambium.deep | `#42664D` | [138, 21, 33] |
| cambium | light | accent | estate cambium.accent | `#5E8C6A` | [136, 20, 46] |
| cambium | light | error | a red of the estate family, not in the tokens (recorded here) | `#c94f4f` | [0, 53, 55] |
| cambium | dark | text | aion --text-secondary | `#a1a1aa` | [240, 5, 65] |
| cambium | dark | text_high | aion --text-primary | `#f4f4f5` | [240, 5, 96] |
| cambium | dark | bg | aion --surface-base | `#0f0f14` | [240, 14, 7] |
| cambium | dark | bg_high | aion --surface-card | `#1a1a22` | [240, 13, 12] |
| cambium | dark | action | estate cambium.accent | `#5E8C6A` | [136, 20, 46] |
| cambium | dark | accent | estate cambium.deep | `#42664D` | [138, 21, 33] |
| cambium | dark | error | a red of the estate family, not in the tokens (recorded here) | `#c94f4f` | [0, 53, 55] |

Conversion: sRGB hex to HSL by the standard formula, hue rounded to the degree, saturation and lightness to the percent. Contrast ratios (WCAG relative luminance) are recorded under `_contrast` in the JSON and checked by `crates/lys/src/identity/themes.rs`: text on bg and text_high on bg_high at least 4.5, action on bg at least 3.
