# Lys-Core — Checklist

## Cluster documents

- [ ] **C1** — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
- [ ] **C2** — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.

## Rule set

- [ ] **C3** — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
- [ ] **C4** — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
- [ ] **C5** — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
- [ ] **C6** — Every rule in rules/ast-grep ignores vendor/**.
- [ ] **C7** — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.

## Test code by structure

- [ ] **C8** — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
- [ ] **C9** — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].
- [ ] **C10** — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].
- [ ] **C11** — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
- [ ] **C12** — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.
- [ ] **C13** — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.

## Hits fixed at their cause

- [ ] **C14** — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.
- [ ] **C15** — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.

## Documents naming the policy

- [ ] **C16** — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
- [ ] **C17** — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.

## The gate leg

- [ ] **C18** — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
- [ ] **C19** — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
- [ ] **C20** — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
- [ ] **C21** — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
- [ ] **C22** — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
- [ ] **C23** — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.
