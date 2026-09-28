# Identity issuer branding

`theme.json` configures Rauthy's supported per-client theme for `lys-directory`.
Its colours come from `src/styles/tokens.css`, converted to Rauthy's integer HSL
format. Both modes use the identity surface's current dark palette. The radius
matches `--radius-md`. `logo.svg` carries the same ID seal as the identity rail.

Apply through the authenticated Rauthy API: PUT `/auth/v1/theme/lys-directory`
with the JSON body, then PUT `/auth/v1/clients/lys-directory/logo` with the SVG
as a multipart file named `logo`. The API key needs `Clients.Read` and
`Clients.Update`; it never belongs in this directory. Save the previous theme
and logo first, and read the theme back with POST on the same theme route.

The upstream branding contract controls colours, radius and logo, but not fonts
or input layout. Matching those requires the issuer's frontend stylesheet to be
built and deployed using the pinned patch below. Authentication,
PKCE, redirect URIs, permissions and the sign-in forms remain Rauthy's.

## Build the matching issuer

`vendor/rauthy` pins upstream commit
`dd61ac3c84d6b238108dc8438b53043b5177a662` (0.36.2).
`rauthy-0.36.2-lys.patch` applies to that exact clean source and contains the
client-scoped stylesheet, local DM Sans font and OFL license, and the build step
that emits the font's Brotli and gzip encodings. No upstream authentication code
is changed. The patch SHA256 is
`31ef2a81caa76386d597f521b9077ff0ec269661c177af64b0d7cae0fec40ccd`.

Use an isolated build copy of that commit. Run `git apply --check` on the patch
before applying it. Follow upstream's build order: compile its `spow` and `md`
WASM helpers, install frontend dependencies from the lockfile, run `npm run
check` and `npm run build` in `frontend`, then build the release binary.

`builder-arm64.Dockerfile` provides native Linux ARM64 tools using upstream's
Rust 1.95.0 and Debian Bookworm. Build the binary with the lockfile, the upstream
release profile, and `JEMALLOC_SYS_WITH_LG_PAGE=16`:

```sh
cargo build --locked --release --target aarch64-unknown-linux-gnu -p rauthy
```

Copy the executable to upstream's `out/rauthy_arm64`. From the issuer source root,
run `node /absolute/path/to/this/directory/verify-assets.mjs`. It refuses to
package unless the executable embeds the exact plain, Brotli and gzip font
assets and both compressed encodings decode to the source bytes. This matters
because the server selects a precompressed asset from Accept-Encoding; the
upstream frontend's precompression does not include TTF files.

Package with upstream's Dockerfile for `linux/arm64`, then run the image with
`--network none` and `--version` before transferring it. Record source, patch,
binary, image and archive hashes with the install receipt. The build and version
check do not start or modify a live service. Coordinate the issuer swap with its
operator, then verify the real sign-in flow and rendered font in the browser.

The September 28 build passed Svelte checks and production compilation. Its
ESLint run was blocked before checking files: upstream installs ESLint 10 but
ships only legacy `.eslintrc.cjs` configuration. That remains an explicit unrun
check, not a successful lint result.
