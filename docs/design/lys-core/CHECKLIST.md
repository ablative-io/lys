# Lys-Core — Checklist

## Pre-method documents

- [ ] **C85** — docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the two live citations name the new paths.
- [ ] **C86** — The lys-core cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-003.md are rendered from its JSON, and sh scripts/design/gate.sh exits 0.

## Test code recognition

- [ ] **C87** — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and CLAUDE.md and Cargo.toml name it in place of the per-module #![allow].
- [ ] **C88** — Every *_tests.rs file, both fixture.rs files, every integration test root and the two tests/harness/mod.rs files begin with #![cfg(test)], and no #![allow] line remains in any of them.
- [ ] **C89** — crates/lys-core/src/keys/identity_tests.rs holds no #[allow(unsafe_code)] and no unsafe block, and the comment above lib.rs's unsafe_code attribute says what is true after that.

## Existing hits cleared

- [ ] **C90** — No `let _ =` statement remains under crates/.
- [ ] **C91** — crates/lys-core/tests/harness/mod.rs, crates/lys-anchor/tests/harness/mod.rs and crates/lys-home/src/harness/claude_code/mod.rs hold no fn, struct, enum, trait, impl, const or static item.

## Rules and leg

- [ ] **C92** — sgconfig.yml names rules/ast-grep, which holds mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes and no-unwrap-expect-panic-outside-tests, each at severity error.
- [ ] **C93** — The leg `ast-grep scan --config sgconfig.yml` is in docs/design/project.json requiring tool:ast-grep, in .land/gates.sh and in CI.
- [ ] **C94** — The scan reports zero hits over the landed tree, and exactly one hit on an uncommitted scratch file holding an unwrap outside test code.
