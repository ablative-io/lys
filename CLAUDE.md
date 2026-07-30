# CLAUDE.md — lys

## What this is

`lys` is cryptographic trust infrastructure for AI agents: identity, tamper-evident history, verifiable provenance. It is the extraction and elevation of the hardened `meridian-trust` crate into a standalone, open-source project.

Read [docs/VISION.md](docs/VISION.md) for why this exists, [docs/DESIGN.md](docs/DESIGN.md) for the architecture, and [docs/ROADMAP.md](docs/ROADMAP.md) for the plan and current phase.

## Crates

- **`lys-core`** — the library. All trust logic lives here: `keys`, `ca`, `merkle`, `checkpoint`, `tlog`, `attestation`, `seal`, the shared `error` type, and — behind the off-by-default `unstable-anchor` feature — `receipt` and `bundle`. Domain-agnostic — no concept of agents, sessions, or workspaces. This is what consumers depend on and what gets published to crates.io.
- **`lys-log-store`** — durable persistence for a log, so `lys-core` stays free of I/O. Owns the `LeafStore` trait (what a log needs from storage), a file-backed implementation of it, and the `Log` that maintains the RFC 6962 tree over either. **The trait exposes no `fork`, no `merge`, no `delete` and no way to rewrite a leaf** — for an append-only log a branch is equivocation with a nicer name, and that absence is the crate's reason to exist rather than an omission to fill in later.
- **`lys`** — the CLI binary. Thin surface over `lys-core`. The "everything is a library + CLI + MCP surface" principle: logic lives in the library, the binary only parses arguments and formats output.

Future crates (later phases): `lys-anchor` (transparency-ledger service) and `lys-mcp` (MCP server surface).

**`unstable-anchor` gates the draft wire formats and is exempt from semver.** A format is frozen by publishing a crate that exposes it or by signing a durable artifact under its tag — so shipping `receipt` and `bundle` ungated would freeze two drafts that have already had three specification bugs found in them by implementation. The gate keeps them changeable until they are ratified; it comes off when a real anchor exists.

## The one rule that governs everything

**This is trust infrastructure. Its entire value is that strangers can verify it.** Every design decision serves verifiability-by-third-parties, not cleverness, not performance for its own sake. When choosing between a "better" primitive and the boring interoperable one, choose boring — the verification world speaks SHA-256, Ed25519, and RFC 6962, and a receipt nobody can verify with standard tooling is worthless. Verification must outlive the vendor.

## Coding standards

Non-negotiable, enforced by CI (`clippy --all-targets -- -D warnings`):

- **No `unwrap` / `expect` / `panic` / `todo` / `unimplemented` / `unreachable` in library code.** Tests opt out per-module with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- **No silent failures.** Every error handled or propagated with operation-specific context. `thiserror` for the library error type; the CLI may use `anyhow` at the top level only.
- **Private key material never appears in `Debug`, logs, or error messages.** Redaction is tested, not assumed. Seed buffers are `Zeroizing`.
- **No file over 500 lines** of code (excluding tests/comments/whitespace). `mod.rs` carries only `pub mod` / `pub use` / module docs. Logic goes in named files; tests in sibling `*_tests.rs` files.
- **`unsafe_code = "deny"`.** All dependencies pure Rust.
- **Every public item documented** (`missing_docs = "warn"` under `-D warnings`). Module-level `//!` docs state invariants, not just descriptions.
- **Cryptographic changes require an adversarial review** before landing. Not a light-model pass — construct actual attacks (forgeries, malleability, cross-protocol confusion, timing oracles) and prove they fail. See the meridian-trust hardening in `docs/ROADMAP.md` for the standard.

Silencing a lint with `#[allow]`, an `#[ignore]`d test, a `_`-prefixed unused variable, or `#[cfg(any())]` is a bypass, not a fix. Fix the code.

## A test needs a second party, or it only agrees with itself

A suite written from the implementation encodes what the code *does*. There is then
nothing for it to disagree with, so it passes for exactly as long as the behaviour is
wrong. This is not carelessness and more care does not fix it — with one party,
agreement is structural.

`lys-log-store`'s file store fsynced nothing while promising durable-on-return. Every
test passed for as long as that code existed. It was found the moment the code moved
somewhere its contract had to be written down: **writing the contract created the
second party.**

So arrange one *before* you need it:

- **State the intent somewhere the implementation cannot quietly agree with it** — a
  trait contract, a module-doc invariant, a wire draft. Prose that can be wrong is
  worth more than a test that cannot.
- **Key a check on what the other side supplied**, never on a value your own code
  substituted, and never on a property that merely happens to be unique today.
- **Count what fired, not what passed.** A loop that runs zero times satisfies every
  assertion inside it, and a control that never fires is indistinguishable from one
  that passed. Assert the case count; assert rejections, not just successes.
- **A drift injection proves nothing unless exactly one test fails, and it is the test
  built for that check.** Two checks guarding one rule means neither is proven by the
  obvious case — isolate each with a case only it can catch.
- **Say which axis the independence is on.** Two implementations agreeing is
  independence of *algorithm*; run on one machine, one toolchain, one dependency
  resolution, it is not independence of *platform*. Claim the axis you have.

And when a check is skipped for a defensible reason — "docs-only, it cannot break
behaviour" — that reasoning is more dangerous than negligence, because it survives
review. Run the gates anyway.

## Wire formats are forever

Once a signature is produced or a leaf is logged under a format, that format is frozen — changing it breaks every historical verification. Domain-separation tags (`lys/attestation/v2`, `lys/sealed-envelope/v1`) and leaf encodings are versioned wire contracts. Evolving one means a new version alongside, never a mutation of the shipped one. This is why the extraction renames the tags *before* anything durable is signed under them.

## Gates before any commit

```
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo test --workspace --all-features
cargo doc --no-deps --all-features
```

All five clean. No exceptions.

**`--all-features` is not optional decoration.** The `unstable-anchor` feature is
off by default, and **81 tests compile out without it** — a bare
`cargo test --workspace` reports a green suite while never running the receipt or
bundle gates. That is the exact shape of silent coverage loss this repo refuses
elsewhere, so the feature-full run is the gate and the default run exists only to
prove the shape consumers actually get still builds.

**`cargo doc` is in the set because clippy does not catch rustdoc lints.**
`private_intra_doc_links`, `redundant_explicit_link_target` and broken intra-doc
links pass clippy and fail `cargo doc`. The baseline is zero warnings, so any
regression is visible.
