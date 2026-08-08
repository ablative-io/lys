//! `lys-anchor` — command-line surface for a lys transparency anchor.
//!
//! A thin surface over [`lys_anchor`]: it parses arguments, dispatches to the
//! subcommand implementations in [`commands`], and maps their results to process
//! exit codes. All logic lives in the libraries — this crate parses and formats.
//!
//! # Why this is not `lys anchor …`
//!
//! `docs/design/lys-anchor/BUILD-PLAN.md` §2.5 rules that the anchor CLI must be
//! its own binary rather than subcommands on `lys`: `lys` is published, and a
//! published crate that exposes a wire format **freezes** it, so `lys anchor …`
//! would freeze `lys/anchor-receipt/v1` as a side effect of a release nobody
//! meant as a ratification. The rule is mechanical as well as principled —
//! `cargo publish` refuses a dependency with no version, and `lys-anchor` is
//! `publish = false` and has none.
//!
//! # Why the code is in a library and `src/main.rs` is three lines
//!
//! Not architecture for its own sake — it is what keeps a gate working. The
//! binary target is named `lys-anchor`, which rustdoc normalises to the crate
//! name `lys_anchor`: **exactly the `lys-anchor` library's**. Two documented
//! units writing one `target/doc/lys_anchor/index.html` is a cargo warning, and
//! `cargo doc --no-deps` at zero warnings is a merge gate here. The reachable
//! fix is `doc = false` on the binary — and that would silence rustdoc for
//! every module in this crate, which is precisely the check CLAUDE.md added the
//! doc gate for, since clippy does not catch `broken_intra_doc_links`.
//!
//! So the modules live in a library named `lys_anchor_cli`, which documents to a
//! directory of its own and is checked, and the undocumented binary is a shim
//! that calls [`dispatch::run`]. The collision is inherent to a binary named
//! after a library and would arise identically had this file lived in
//! `crates/lys-anchor/src/bin/` as §2.5 lays out.
//!
//! Exit codes: `0` on success, `1` on any operational failure (with a diagnostic
//! on stderr), `2` for argument-parsing errors (clap's convention).

pub mod cli;
pub mod commands;
pub mod dispatch;
