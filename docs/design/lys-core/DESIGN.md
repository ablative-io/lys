---
type: design
cluster: lys-core
title: lys-core — the ast-grep leg and test code by structure
---

# lys-core — the ast-grep leg and test code by structure

> **Cluster:** lys-core

## Intention

The rules lys holds itself to are enforced by the gate, not by whoever happens to be reading the diff. A reader of CLAUDE.md, the rule directory, clippy.toml and the gate should find one policy stated four ways that agree, and a stranger should be able to run the same scan the landing runs and get the same answer.

Test code is recognised by what the file itself says it is, never by a list of names someone keeps. A file that is only ever compiled as a test says so in its first line, and both clippy and ast-grep read that line. Nothing is exempted, nothing is rewritten to dodge a rule, and nothing is silenced: where the tree broke a rule, the cause is fixed.

A rule is carried only where it can fire. A rule that cannot fire on this tree would only look like cover, and every claim of zero hits is paired with a scratch case the rule reports, so the rule is shown to fire before its silence is believed.

## Problem

The lys gate (docs/design/project.json, run by scripts/design/gate.sh and by .land/gates.sh at landing) has no ast-grep leg, so unwrap, expect and panic outside tests, #[allow] and #[ignore] are caught only where clippy happens to overlap them, and mod.rs logic and `let _ =` discards are not caught at all. Cambium carries sgconfig.yml, a rule directory and an ast-grep leg; lys has none of them. Measured at 7b53625 with Cambium's rules and ast-grep 0.44.1 over crates/: 199 hits (113 no-lint-bypass-attributes, 67 mod-rs-declarations-only, 19 no-let-underscore-on-results). The 113 bypasses are 106 per-module #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] test opt-outs and 7 #[allow(unsafe_code)] around set_var and remove_var in lys-core's env-backed identity tests. A structural unwrap/expect/panic rule finds 224 calls outside a #[test] fn, a #[cfg(test)] mod and tests/, all in the 2 fixtures and 47 sibling *_tests.rs files, and none in library code. The tree's own documents say the opposite of the policy the gate should enforce: CLAUDE.md line 35 and Cargo.toml lines 69-70 tell tests to opt out per module with #![allow], and the lib.rs comment above the unsafe attribute explains it by a set_var #[allow] that this work removes. The lys-core cluster itself is three hand-written pre-method documents with no design.json, so scripts/design/gate.sh has never measured it.

## Solution

Cluster documents. In the brief's own commit, the three hand-written documents are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md, byte for byte, before the cluster is rendered, and the rendering is committed with the JSON, so the earlier design is kept beside the rendered DESIGN.md, CHECKLIST.md and USER-STORIES.md. gate.sh compares only the markdown render-cluster.py writes, and its temporary copy carries the *-PRE-METHOD.md files unchanged, so they pass the byte comparison. The kept files are not edited (P6 applies to comments in code, not to a kept record). The card itself checks that state rather than performing it.

Rule set. sgconfig.yml at the repository root names rules/ast-grep, as Cambium's does. Of Cambium's six rules at 1be80d8ec, three can fire on lys and are carried with their bodies unchanged: mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes (which also catches #[ignore]). no-std-mutex-in-async and the two door-timer rules are not carried (ADR-055). Cambium carries no _name rule and no unwrap rule; lys adds one rule of its own, no-unwrap-expect-panic-outside-tests, which reports `.unwrap()`, `.expect(..)` and `panic!(..)` except inside test code as ADR-054 defines it. Every rule carries an `ignores` entry for vendor/**, so an initialised vendor/rauthy is never scanned (ADR-009); target/ is already skipped because ast-grep honours .gitignore. The command stays exactly `ast-grep scan --config sgconfig.yml`, run from the root with no path.

Test code by structure (ADR-054). clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and every file that is only ever compiled as a test begins with #![cfg(test)]: the 2 fixtures, the 88 sibling *_tests.rs files under src/, the 28 integration roots under tests/ (4 of which are the *_tests.rs roots of crates/lys) and the 2 tests/harness/mod.rs files. Measured with clippy 0.1.97 on a probe crate: a #[cfg(test)] mod ancestor already covers a sibling file's helpers, the integration roots and harness modules are covered only once they carry the marker, and the outer #[cfg(test)] on the parent's `mod x;` beside the inner marker trips no lint, so the outer attribute stays. With that, all 106 #![allow] test opt-outs go. The 7 #[allow(unsafe_code)] go by fixing their cause: from_env reads the variable and passes the read's result to a private seam that holds today's decoding, and the tests feed the seam directly instead of mutating the process environment, so no test needs unsafe. The lib.rs comment is then rewritten to say what is true, and the attribute under it is left exactly as it is.

Hits fixed at their cause. The 19 `let _ =` discards: the three hex writers take a form that returns no Result, the 14 test-code writes handle their Result, and the two discards of values that are not Results are removed with what they held. The lys-core hex writer at lib.rs line 59 takes the same no-Result form, and line 54's `use std::fmt::Write;`, which that form leaves unused, goes with it; no other code line of lib.rs changes. The logic in the two tests/harness/mod.rs files and in lys-home's harness/claude_code/mod.rs moves into named sibling files, leaving declarations and re-exports. The 47 hits in lys-home's record/mod.rs belong to HOME-013 and are not touched here.

The leg. `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep is added to docs/design/project.json, as a `leg` line to .land/gates.sh, and as a pinned install and scan step to .github/workflows/ci.yml, so a hit is refused on every path to main. CLAUDE.md's gates block gains the line. The leg lands only when the whole tree is at zero hits, which needs HOME-013 landed first; a scratch file without the marker, never landed, shows the leg red through .land/gates.sh while every other leg stays green.

## Principles

- **P1** — Test code is recognised by structure the file carries: a first-line #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory. No rule and no config holds a list of file names.
- **P2** — A rule is carried only where it can fire on this tree; a rule that cannot fire is recorded as not carried with the finding and the act that brings it back.
- **P3** — Count what fired: every zero-hit claim is paired with a never-landed scratch case the rule reports, and each structural marker has a case of its own.
- **P4** — A bypass is fixed at its cause and never replaced by another bypass; one that cannot be fixed goes back to the lead as a question with its line.
- **P5** — One leg on every landing path: project.json, .land/gates.sh and CI run the same command.
- **P6** — A comment the code no longer bears out is corrected in the change that made it false.

## Decisions

- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
- ADR-055 — Cambium's ast-grep rules are carried into lys only where they can fire — lys carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and records no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers as not carried with the finding that they cannot fire; the _name rule is its own card. Rejected: copying all six rules, which would look like cover the scan cannot give.

## Goals

- `ast-grep scan --config sgconfig.yml` from the repository root reports zero hits and exits 0 at the landed commit.
- A never-landed scratch file with an unwrap and no #![cfg(test)] turns the ast-grep leg of .land/gates.sh red while every other leg stays green.
- grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/, and both clippy legs pass with -D warnings.
- docs/design/project.json, .land/gates.sh and .github/workflows/ci.yml each run `ast-grep scan --config sgconfig.yml`.
- sh scripts/design/gate.sh measures lys-core for the first time and exits 0, with the three pre-method documents kept byte for byte.

## Non-Goals

- A _name binding rule and the handling of each underscore-prefixed binding — Cambium carries no such rule, so the same rule set does not include it; a card of its own writes the rule and handles each binding by its act.
- Carrying no-std-mutex-in-async — lys has no async fn, no tokio and no std Mutex, so it cannot fire (ADR-055); the card that lands lys's first async code carries it.
- Carrying no-timer-in-door-handlers and no-timer-import-in-door-handlers — lys has no door handlers, so they cannot fire (ADR-055).
- Tightening #![cfg_attr(not(test), forbid(unsafe_code))] so tests forbid unsafe code too — The attribute stays exactly as it is in this work; tightening it is a further unit.
- Splitting crates/lys-home/src/record/mod.rs — Its 47 hits are HOME-013's, which this work waits on.
- Correcting CHECKLIST-PRE-METHOD.md's C4 — The kept documents are the historical record and stay exactly as they were; the rendered CHECKLIST.md is the current truth.
- A rule for todo!, unimplemented! and unreachable! — The words and the survey name unwrap, expect and panic; the other three stay with clippy's workspace lints.
- ast-grep rule tests (`ast-grep test`) in the repository — No gate leg would run them; the never-landed scratch cases in the brief's acceptance are the measurement the words ask for.
- Editing Cambium's rules or config to match lys — Neither project is edited to match the other; lys takes the opposite clippy.toml setting on tests from Cambium's.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/lys-core/design.json` | This design | LYSCORE-001 |
| `docs/design/lys-core/checklist.json` | The rows LYSCORE-001 delivers | LYSCORE-001 |
| `docs/design/lys-core/stories.json` | The stories LYSCORE-001 serves | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.json` | The ast-grep leg brief | LYSCORE-001 |
| `docs/design/lys-core/briefs/LYSCORE-001.md` | Its rendered markdown | LYSCORE-001 |
| `docs/design/lys-core/DESIGN.md` | The rendering of design.json, committed with the brief; R1 checks it | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST.md` | The rendering of checklist.json, committed with the brief; R1 checks it | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES.md` | The rendering of stories.json, committed with the brief; R1 checks it | LYSCORE-001 |
| `docs/design/lys-core/DESIGN-PRE-METHOD.md` | The hand-written lys-core design, renamed in the brief's own commit and kept byte for byte as it stood at 7b53625 | LYSCORE-001 |
| `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` | The hand-written lys-core checklist (C1 to C65 of the extraction), renamed in the brief's own commit and kept byte for byte; its C4 sentence is not borne out by the code after R5 | LYSCORE-001 |
| `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` | The hand-written lys-core user stories, renamed in the brief's own commit and kept byte for byte | LYSCORE-001 |
| `sgconfig.yml` | ast-grep project config naming rules/ast-grep as its one rule directory | LYSCORE-001 |
| `rules/ast-grep/mod-rs-declarations-only.yml` | Carried from Cambium: mod.rs holds no function, struct, enum, trait, impl, const or static | LYSCORE-001 |
| `rules/ast-grep/no-let-underscore-on-results.yml` | Carried from Cambium: no `let _ =` discard | LYSCORE-001 |
| `rules/ast-grep/no-lint-bypass-attributes.yml` | Carried from Cambium: no #[allow], #![allow], #[expect] or #[ignore] | LYSCORE-001 |
| `rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml` | New: unwrap, expect and panic reported everywhere except test code recognised by structure | LYSCORE-001 |
| `clippy.toml` | allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests set to true | LYSCORE-001 |
| `crates/lys-core/tests/harness/go.rs` | The Go-toolchain logic moved out of lys-core's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-anchor/tests/harness/go.rs` | The Go-toolchain logic moved out of lys-anchor's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-anchor/tests/harness/scaffold.rs` | GO_ENV, GoScaffold, ALL_SCAFFOLDS and the path to lys-core's harness, moved out of lys-anchor's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-anchor/tests/harness/scaffold_tests.rs` | The two contract tests moved out of lys-anchor's tests/harness/mod.rs | LYSCORE-001 |
| `crates/lys-home/src/harness/claude_code/names.rs` | HARNESS, PROVIDER, API and AUTHORED, moved out of claude_code/mod.rs | LYSCORE-001 |
| `docs/design/project.json` | The tree's gate legs; gains the ast-grep leg |  |
| `.land/gates.sh` | The whole gate the landing runs; gains `leg ast-grep scan --config sgconfig.yml` |  |
| `.github/workflows/ci.yml` | CI; gains a pinned ast-grep install and the same scan |  |
| `CLAUDE.md` | The test opt-out sentence and the gates block, corrected |  |
| `Cargo.toml` | The workspace lint comment, corrected to name clippy.toml; the serial_test workspace dev-dependency, removed |  |
| `crates/lys-core/Cargo.toml` | The serial_test dev-dependency, removed with the last #[serial_test::serial] |  |
| `Cargo.lock` | Regenerated by cargo without serial_test and serial_test_derive |  |
| `crates/lys-core/src/lib.rs` | The comment above #![cfg_attr(not(test), forbid(unsafe_code))], rewritten; the attribute stays byte-identical; hex_lower's line 59 takes a form that returns no Result and line 54's import goes |  |
| `crates/lys-core/src/keys/identity.rs` | Ed25519Identity::from_env reads the variable and hands the result to a private seam the tests feed |  |
| `crates/lys-core/src/keys/identity_tests.rs` | The env-backed tests call the seam; no set_var, remove_var, unsafe or #[serial] |  |
| `crates/lys/src/commands/hex.rs` | hex_lower writes without a Result |  |
| `crates/lys-anchor-cli/src/commands/hex.rs` | hex_lower writes without a Result |  |
| `crates/lys-home/src/harness/claude_code/mod.rs` | Declarations and re-exports only after R6 |  |
| `crates/lys-core/tests/harness/mod.rs` | Declarations and re-exports only after R6, first line #![cfg(test)] |  |
| `crates/lys-anchor/tests/harness/mod.rs` | Declarations and re-exports only after R6, first line #![cfg(test)] |  |
| `crates/lys-anchor/src/upward/fixture.rs` | Test fixture; first line becomes #![cfg(test)] |  |
| `crates/lys-anchor/src/witness/fixture.rs` | Test fixture; first line becomes #![cfg(test)] |  |
| `crates/lys/src` | lys sources; its 10 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys/tests` | lys integration tests; its 4 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor/src` | lys-anchor sources; its 17 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor/tests` | lys-anchor integration tests; its 5 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor-cli/src` | lys-anchor-cli sources; its 6 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-anchor-cli/tests` | lys-anchor-cli integration tests; its 1 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-core/src` | lys-core sources; its 33 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-core/tests` | lys-core integration tests; its 11 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-home/src` | lys-home sources; its 20 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `crates/lys-home/tests` | lys-home integration tests; its 7 roots gain #![cfg(test)] as their first line |  |
| `crates/lys-log-store/src` | lys-log-store sources; its 2 sibling *_tests.rs files gain #![cfg(test)] as their first line |  |
| `docs/design/roadmap.json` | RM-035 carries this work |  |
| `docs/design/decisions.json` | ADR-054 and ADR-055 |  |

## Inventory

- `docs/design/lys-core/DESIGN-PRE-METHOD.md` — Renamed byte for byte from DESIGN.md in the brief's own commit. Hand-written pre-method design of the Phase 1/2 extraction (255 lines); no design.json beside it
- `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` — Renamed byte for byte from CHECKLIST.md in the brief's own commit. Hand-written checklist C1 to C65 (101 lines); C4 records the forbid-to-deny relaxation for set_var tests under an explicit #[allow]
- `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` — Renamed byte for byte from USER-STORIES.md in the brief's own commit. Hand-written stories S1 to S23 (55 lines)
- `docs/design/project.json` — Seven gate legs (fmt, clippy-all-features, clippy, tests, doc-all-features, doc, design); no ast-grep leg
- `.land/gates.sh` — Seven `leg` lines, run by the landing as the whole gate; no ast-grep
- `.github/workflows/ci.yml` — fmt, clippy and test steps on ubuntu-latest; ast-grep not installed
- `scripts/design/gate.sh` — Validates, checks coverage of, and byte-compares the rendering of every cluster with a design.json; lys-core has none yet
- `sgconfig.yml` — Absent
- `rules/ast-grep` — Absent
- `clippy.toml` — Absent
- `Cargo.toml` — Lines 69-70 say tests opt out per-module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]; unsafe_code = "deny"; unwrap_used, expect_used and panic warn
- `CLAUDE.md` — Line 35 says tests opt out per-module with #![allow]; the gates block lists six commands under 'All five clean' and no ast-grep
- `crates/lys-core/src/lib.rs` — Lines 27-30 explain the relaxed forbid through the set_var tests' #[allow(unsafe_code)]; line 31 is the attribute; line 59 is `let _ = s.write_fmt(...)` in hex_lower, and line 54 is the `use std::fmt::Write;` that write needs
- `crates/lys-core/src/keys/identity.rs` — from_env (line 241) reads LYS_IDENTITY_KEY with std::env::var and decodes it in place
- `crates/lys-core/src/keys/identity_tests.rs` — Five #[serial] env tests and the EnvCleanup guard carry the 7 #[allow(unsafe_code)] around set_var and remove_var (lines 847-1068)
- `crates/lys-anchor/src/upward/fixture.rs` — First line #![allow(clippy::unwrap_used, ...)]; declared only as #[cfg(test)] mod from pin.rs; 7 unwrap/expect/panic calls
- `crates/lys-anchor/src/witness/fixture.rs` — First line #![allow(clippy::unwrap_used, ...)]; declared only as #[cfg(test)] mod from report.rs; 15 unwrap/expect/panic calls
- `crates/*/src/**/*_tests.rs` — 88 sibling test files, 78 carrying #![allow]; none carries #![cfg(test)]
- `crates/*/tests/*.rs` — 28 integration test roots, 24 carrying #![allow], among them the 4 *_tests.rs roots of crates/lys
- `crates/lys-core/tests/harness/mod.rs` — 127 lines, #![allow] at line 18, 4 functions (mod-rs-declarations-only hits); lys-anchor's contract test reads this file's text
- `crates/lys-anchor/tests/harness/mod.rs` — 252 lines, #![allow] at line 45, 12 mod-rs-declarations-only hits including two #[test] fns
- `crates/lys-home/src/harness/claude_code/mod.rs` — 4 const items (HARNESS, PROVIDER, API, AUTHORED), mod-rs-declarations-only hits
- `crates/lys-home/src/record/mod.rs` — 630 lines, 47 mod-rs-declarations-only hits covering 521 lines on origin/main at 7b53625; split by HOME-013, not here
- `crates/lys/src/commands/hex.rs` — Line 15 `let _ = s.write_fmt(...)`
- `crates/lys-anchor-cli/src/commands/hex.rs` — Line 15 `let _ = s.write_fmt(...)`
- `vendor/rauthy` — Pinned Rauthy submodule (ADR-009), not initialised in a fresh clone
- `$cambium/rules/ast-grep/mod-rs-declarations-only.yml` — Source of the carried rule at cambium 1be80d8ec
- `$cambium/rules/ast-grep/no-let-underscore-on-results.yml` — Source of the carried rule at cambium 1be80d8ec
- `$cambium/rules/ast-grep/no-lint-bypass-attributes.yml` — Source of the carried rule at cambium 1be80d8ec
- `$cambium/rules/ast-grep/no-std-mutex-in-async.yml` — Not carried: lys has no async fn, tokio or std Mutex
- `$cambium/rules/ast-grep/no-timer-in-door-handlers.yml` — Not carried: lys has no door handlers
- `$cambium/rules/ast-grep/no-timer-import-in-door-handlers.yml` — Not carried: lys has no door handlers

## Constraints

- **CN1** — No unwrap, expect or panic call in a fixture or *_tests.rs file is rewritten: each such file's count of `.unwrap()`, `.expect(` and `panic!(` is not below its count at the base.
- **CN2** — No rule and no config names a file or a list of file names to exempt; test code is recognised by structure only (P1).
- **CN3** — No #[allow], #![allow], #[expect] or #[ignore] of any kind replaces a removed one, in tests or in library code.
- **CN4** — #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs stays byte-identical, and the card's diff to lib.rs changes comment lines plus lines 54 and 59 only.
- **CN5** — crates/lys-home/src/record/mod.rs is not changed by this work.
- **CN6** — No wire format, domain-separation tag, public API signature or public behaviour of lys-core changes; Ed25519Identity::from_env keeps its signature and its three error texts.
- **CN7** — vendor/rauthy is neither scanned nor edited, and no file under vendor/ is committed (ADR-009).
- **CN8** — Cambium's rules and config are not edited.
- **CN9** — The seven existing gate legs and their commands are unchanged in docs/design/project.json and .land/gates.sh.
- **CN10** — Only this card writes under docs/design/lys-core.
