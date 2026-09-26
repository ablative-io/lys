# Contributing to Lys

Thank you for your interest in contributing to Lys. This document covers
what you need to build, test, and submit changes.

## Prerequisites

- **Rust** stable (>= 1.85) with `rustfmt` and `clippy` components.
  Install via [rustup](https://rustup.rs):

  ```console
  $ rustup toolchain install stable
  $ rustup component add rustfmt clippy
  ```

- **Go** stable (for wire-format conformance tests only). The test suite
  round-trips signed notes against Go's `sumdb/note` and COSE attestations
  against `veraison/go-cose`. Without Go installed, those tests skip
  gracefully on your machine but are enforced in CI via `LYS_REQUIRE_GO=1`.

- **cargo-deny** (optional, runs in CI). Audits dependency licenses and
  advisories:

  ```console
  $ cargo install cargo-deny
  ```

## Repository layout

```
lys/
  crates/
    lys-core/     # Library -- all trust logic lives here
    lys/          # CLI binary -- thin surface over lys-core
  docs/
    DESIGN.md     # Architecture and primitive decisions
    ROADMAP.md    # Phase plan and current status
    VISION.md     # Why this exists
    design/       # Wire format specs, future crate designs
  deny.toml       # cargo-deny configuration
  CLAUDE.md       # AI contributor context
```

**`lys-core`** is the domain-agnostic library. It has no concept of agents,
sessions, or workspaces -- just cryptographic primitives: `keys`, `ca`,
`merkle`, `attestation`, `seal`, `checkpoint`, `tlog`. This is what
consumers depend on and what gets published to crates.io.

**`lys`** is the CLI binary. It parses arguments and formats output;
all logic lives in `lys-core`. The principle: everything is a
library + CLI + (future) MCP surface.

## Build, test, lint

All three must pass before any commit:

```console
$ cargo fmt --check
$ cargo clippy --workspace --all-targets -- -D warnings
$ cargo test --workspace
```

To run with the Go conformance gates enforced (as CI does):

```console
$ LYS_REQUIRE_GO=1 cargo test --workspace
```

## Coding standards

These are non-negotiable and enforced by CI:

- **No `unwrap` / `expect` / `panic` / `todo` / `unimplemented`** in
  library code. Tests opt out per-module with
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- **No silent failures.** Every error handled or propagated with
  operation-specific context.
- **Private key material never appears in `Debug`, logs, or error
  messages.** Redaction is tested.
- **No file over 500 lines** of logic (excluding tests, comments,
  whitespace).
- **`unsafe_code = "deny"`.** All dependencies pure Rust.
- **Every public item documented** (`missing_docs = "warn"` under
  `-D warnings`).

Silencing a lint with `#[allow]` is a bypass, not a fix. Fix the code.

## Wire format stability

Once a signature is produced or a leaf is logged under a format, that
format is frozen. Changing it breaks every historical verification.
Evolving a format means a new version alongside, never a mutation of
the shipped one. See [docs/design/WIRE-FORMATS.md](docs/design/WIRE-FORMATS.md).

## Submitting changes

1. Fork the repository and create a feature branch.
2. Make your changes, ensuring all three gates pass locally.
3. Write clear commit messages explaining *why*, not just *what*.
4. Open a pull request against `main`.

**Cryptographic changes** (new primitives, format modifications, key
handling) require an adversarial review before landing -- construct
actual attacks and prove they fail. See the hardening standard
described in [docs/ROADMAP.md](docs/ROADMAP.md).

## License

By contributing, you agree that your contributions will be licensed
under the [Apache-2.0 license](LICENSE).
