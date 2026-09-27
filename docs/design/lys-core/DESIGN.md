---
type: design
cluster: lys-core
title: Lys Core — the ast-grep rule set and its gate leg
---

# Lys Core — the ast-grep rule set and its gate leg

> **Cluster:** lys-core

## Intention

The rules lys holds about its own code are enforced by a machine at every place a land is gated, not by whichever clippy lint happens to overlap them and not by a reviewer remembering them. A card author learns of an unwrap in library code, a lint bypass, a dropped Result or logic in a mod.rs from the gate, which names the rule and the line.

Test code is recognised by what the file itself carries, never by a list of names kept somewhere else, so the rule and the file cannot drift apart. Each rule is shown to fire before it is trusted, and a rule that cannot fire on this tree is not carried, because a control that never fires is indistinguishable from one that passed.

## Problem

The lys gate has no ast-grep leg. lys has no sgconfig.yml and no rules directory, so its rules against unwrap, expect and panic outside tests, against #[allow] and #[ignore], against dropped Results and against logic in mod.rs are caught only where a clippy lint overlaps them. Measured at 4dd1c33, the rules that would be carried report 113 lint-bypass hits, 19 let-underscore hits, 20 mod-rs hits and 224 unwrap/expect/panic hits, so a leg added alone would turn every lys round red. The per-module test opt-out that CLAUDE.md sanctions is itself an #[allow]. The gate is held in three places that do not follow each other: docs/design/project.json, .land/gates.sh, which repo_land runs as the whole gate, and CI; and scripts/design/gate.sh validates project.json without running any leg.

## Solution

sgconfig.yml at the repository root names one rule directory, rules/ast-grep, as Cambium's does. It holds four rules at severity error (ADR-063). Three are Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, carried with the same id, severity, files and rule, a message that cites lys's CLAUDE.md where Cambium's cites its own AGENTS.md, and vendor/** ignored so an initialised Rauthy submodule is never scanned. The fourth is lys's own no-unwrap-expect-panic-outside-tests. It treats four structures as test code: a file whose first item is the inner attribute #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/. It reports unwrap, expect and panic everywhere else. The leg is one command, `ast-grep scan --config sgconfig.yml`, run from the repository root. It exits non-zero on any hit and zero on none, and it runs in docs/design/project.json, in .land/gates.sh and in CI.

The tree is brought to zero hits in the same card, so the leg is green when it lands. Every sibling *_tests.rs file and both fixture.rs files gain #![cfg(test)] as their first line, which states what their parent's #[cfg(test)] mod already makes true. Every integration test root and the two tests/harness/mod.rs files gain it too: an integration test root is only ever compiled as a test, so the attribute changes nothing about what is built and lets clippy treat its helper fns as test code. The per-module #![allow] opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests, which clippy applies to code it knows is test code. The seven #[allow(unsafe_code)] lines in lys-core's identity tests go with the environment mutation that needed them: the from_env tests exercise the rules applied to the variable's value directly, and the process environment is never written, and the comment above lib.rs's unsafe_code attribute is corrected to say so. Dropped Results are handled, and the three infallible String writes in library and CLI code use a form that returns no Result. Logic in three mod.rs files moves into named sibling files. The 47 mod-rs hits in lys-home's record/mod.rs on main belong to HOME-013's split, which this card is built on. The proof that the unwrap rule fires is one scratch file without the marker, reported by the scan and never committed.

no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are not carried (ADR-064). No _name-rename rule is written, because Cambium carries none. Before the cluster is rendered, the hand-written lys-core documents are renamed to *-PRE-METHOD.md (ADR-065), so the method's render takes the names DESIGN.md, CHECKLIST.md and USER-STORIES.md without overwriting the earlier design.

## Principles

- **P10** — Test code is recognised by structure the file carries: its first item, an enclosing #[cfg(test)] mod or #[test] fn, or a path under tests/. Never by a list of file names.
- **P11** — A rule is trusted only once shown to fire, and a rule that cannot fire on this tree is not carried.
- **P12** — The leg lands green: every hit a carried rule reports is fixed at its cause in the same card, never exempted, narrowed or silenced.
- **P13** — Every place a land is gated runs the same command, so no route to main skips the rules.

## Decisions

- ADR-063 — lys enforces its code rules with ast-grep, recognising test code by structure the file carries — Carry Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and add lys's own no-unwrap-expect-panic-outside-tests. That rule treats as test code a file whose first item is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/. Every *_tests.rs file, both fixture.rs files, every integration test root and both tests/harness/mod.rs files gain #![cfg(test)] as their first line. The per-module #![allow] opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests. Every existing hit is fixed at its cause in the same card, so the leg is green when it lands. The leg `ast-grep scan --config sgconfig.yml` runs in docs/design/project.json, .land/gates.sh and CI. Rejected: an exemption list of file names; rewriting the calls in test files; narrowing no-lint-bypass-attributes to spare the test opt-outs; carrying only the rules that were clean at the time.
- ADR-064 — A rule that cannot fire on the lys tree is not carried — no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are recorded as not carried, with that finding. The card that lands the first async code in lys carries no-std-mutex-in-async with it, and its brief names this decision. Rejected: carrying all six of Cambium's rules because the words said the same rule set.
- ADR-065 — The hand-written lys-core documents are kept as *-PRE-METHOD.md, not replaced by the method's render — Before this cluster is rendered, the three documents are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the method's render takes the old names. The live citations of the old paths are re-pointed to the new ones. LYSCORE-003's card is the one owner of docs/design/lys-core. Rejected: overwriting the documents with the render, and deleting them.

## Goals

- `ast-grep scan --config sgconfig.yml` exits 0 at the repository root of the landed tree and reports 0 hits.
- One scratch file holding an unwrap outside test code, added under crates/ and never committed, makes the scan exit 1 with exactly one hit, from no-unwrap-expect-panic-outside-tests.
- The command `ast-grep scan --config sgconfig.yml` appears once in each of docs/design/project.json, .land/gates.sh and .github/workflows/ci.yml.
- Both clippy legs pass with -D warnings, and no #[allow] line remains under crates/.
- docs/design/lys-core holds DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md byte-equal to the hand-written files, and sh scripts/design/gate.sh exits 0.

## Non-Goals

- Carrying no-std-mutex-in-async — lys has no async fn, no tokio and no std Mutex, so it could not fire (ADR-064). The card that lands the first async code carries it and names ADR-064.
- Carrying no-timer-in-door-handlers and no-timer-import-in-door-handlers — Their files are Cambium door-handler paths that lys does not have, so they could not fire (ADR-064).
- A _name-rename rule and the existing _-prefixed bindings — Cambium carries no such rule. A card of its own writes the rule and handles each binding by its act.
- Splitting crates/lys-home/src/record/mod.rs — HOME-013 owns that split; this card is built on it and lands after it.
- Tightening lys-core's unsafe_code attribute so test builds forbid unsafe code too — The attribute stays exactly as it is; only the comment above it is corrected. Tightening it is a further unit.
- Changing Cambium's rules or policy — Cambium's tree is read only; its policy of no unwrap even in tests is not imported.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `sgconfig.yml` | ast-grep root config; ruleDirs names rules/ast-grep | LYSCORE-003 |
| `rules/ast-grep` | lys's ast-grep rule directory, one <rule id>.yml per carried rule | LYSCORE-003 |
| `rules/ast-grep/mod-rs-declarations-only.yml` | Cambium's rule, its message citing lys's CLAUDE.md and vendor/** ignored | LYSCORE-003 |
| `rules/ast-grep/no-let-underscore-on-results.yml` | Cambium's rule, vendor/** ignored | LYSCORE-003 |
| `rules/ast-grep/no-lint-bypass-attributes.yml` | Cambium's rule, its message citing lys's CLAUDE.md and vendor/** ignored | LYSCORE-003 |
| `rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml` | lys's rule: unwrap, expect and panic outside the four recognised test structures | LYSCORE-003 |
| `clippy.toml` | allow-unwrap-in-tests, allow-expect-in-tests, allow-panic-in-tests | LYSCORE-003 |
| `crates/lys-core/tests/harness/go.rs` | lys-core's Go-toolchain harness, moved whole out of harness/mod.rs | LYSCORE-003 |
| `crates/lys-anchor/tests/harness/go.rs` | lys-anchor's Go-toolchain harness and its two contract tests, moved whole out of harness/mod.rs | LYSCORE-003 |
| `crates/lys-home/src/harness/claude_code/names.rs` | the HARNESS, PROVIDER, API and AUTHORED constants, moved out of claude_code/mod.rs | LYSCORE-003 |
| `docs/design/lys-core/DESIGN-PRE-METHOD.md` | the hand-written pre-method design, renamed with its bytes unchanged | LYSCORE-003 |
| `docs/design/lys-core/CHECKLIST-PRE-METHOD.md` | the hand-written pre-method checklist C85 to C65, renamed with its bytes unchanged | LYSCORE-003 |
| `docs/design/lys-core/USER-STORIES-PRE-METHOD.md` | the hand-written pre-method stories S30 to S23, renamed with its bytes unchanged | LYSCORE-003 |
| `docs/design/lys-core/design.json` | this design | LYSCORE-003 |
| `docs/design/lys-core/checklist.json` | the rows LYSCORE-003 delivers | LYSCORE-003 |
| `docs/design/lys-core/stories.json` | the stories LYSCORE-003 serves | LYSCORE-003 |
| `docs/design/lys-core/briefs/LYSCORE-003.json` | the brief | LYSCORE-003 |
| `docs/design/lys-core/briefs/LYSCORE-003.md` | the brief, rendered | LYSCORE-003 |
| `docs/design/lys-core/DESIGN.md` | rendered from design.json once the hand-written file is renamed away |  |
| `docs/design/lys-core/CHECKLIST.md` | rendered from checklist.json once the hand-written file is renamed away |  |
| `docs/design/lys-core/USER-STORIES.md` | rendered from stories.json once the hand-written file is renamed away |  |
| `docs/design/project.json` | the project's gate trees; gains the ast-grep leg |  |
| `.land/gates.sh` | the landing gate repo_land runs as the whole gate; gains the ast-grep leg |  |
| `.github/workflows/ci.yml` | CI; its test job gains an ast-grep install and the scan |  |
| `CLAUDE.md` | coding standards; the test opt-out sentence names clippy.toml |  |
| `Cargo.toml` | workspace lint table, whose unwrap/expect/panic comment names clippy.toml, and workspace dependencies, which lose serial_test |  |
| `crates/lys-core/Cargo.toml` | lys-core's manifest; its dev-dependencies lose serial_test |  |
| `Cargo.lock` | the resolved dependency graph, without serial_test and the packages only it pulled in |  |
| `docs/PEN-REGISTRATION.md` | cites the pre-method lys-core design and checklist by path |  |
| `crates/lys-core/src/lib.rs` | crate root; hex_lower, and the comment above the unsafe_code attribute |  |
| `crates/lys-core/src/keys/identity.rs` | Ed25519Identity, including from_env |  |
| `crates/lys-core/tests/harness/mod.rs` | lys-core's Go-toolchain harness, logic in a mod.rs today |  |
| `crates/lys-anchor/tests/harness/mod.rs` | lys-anchor's Go-toolchain harness, logic in a mod.rs today |  |
| `crates/lys-home/src/harness/claude_code/mod.rs` | the Claude Code profile module; four constants in a mod.rs today |  |
| `crates/lys/src/commands/hex.rs` | the CLI's hex helper |  |
| `crates/lys-anchor-cli/src/commands/hex.rs` | the anchor CLI's hex helper |  |
| `crates/lys/src` | the CLI crate; sibling *_tests.rs files |  |
| `crates/lys/tests` | the CLI's integration tests |  |
| `crates/lys-anchor/src` | lys-anchor; sibling *_tests.rs files and the two fixture.rs files |  |
| `crates/lys-anchor/tests` | lys-anchor's integration tests |  |
| `crates/lys-anchor-cli/src` | the anchor CLI crate; sibling *_tests.rs files |  |
| `crates/lys-anchor-cli/tests` | the anchor CLI's integration tests |  |
| `crates/lys-core/src` | lys-core; sibling *_tests.rs files |  |
| `crates/lys-core/tests` | lys-core's integration tests |  |
| `crates/lys-home/src` | lys-home; sibling *_tests.rs files |  |
| `crates/lys-home/tests` | lys-home's integration tests |  |
| `crates/lys-log-store/src` | lys-log-store; sibling *_tests.rs files |  |
| `crates/lys-home/src/harness/claude_code` | the Claude Code profile; sibling *_tests.rs files |  |
| `tests/identity_contract/tests` | the directory contract's integration test roots |  |

## Inventory

- `docs/design/project.json` — One tree '.' with seven legs (fmt, clippy-all-features, clippy, tests, doc-all-features, doc, design); no ast-grep leg.
- `scripts/design/gate.sh` — Validates decisions.json and project.json and, for every cluster with a design.json, validates it, checks its coverage and compares its rendered markdown byte for byte. It runs no leg's command.
- `.land/gates.sh` — The gate repo_land runs as the whole gate: gate.sh, cargo fmt --check, both clippies, the tests and both cargo doc runs. No ast-grep line.
- `.github/workflows/ci.yml` — Job test runs fmt, clippy and tests; job audit runs cargo-deny. No ast-grep install and no scan.
- `sgconfig.yml` — Absent.
- `rules` — Absent.
- `clippy.toml` — Absent.
- `Cargo.toml` — Workspace lints: unwrap_used, expect_used and panic at warn, under a comment saying tests opt out per module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)].
- `CLAUDE.md` — Coding standards: tests opt out per module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]; the same file calls #[allow], #[ignore], _-prefixed variables and #[cfg(any())] a bypass.
- `crates` — 333 Rust files under crates/ and tests/ in eight workspace members; no async fn, no tokio and no std Mutex or RwLock. At 4dd1c33 the lys rules report 113 lint-bypass hits (106 test opt-outs and seven #[allow(unsafe_code)] in lys-core keys/identity_tests.rs, none outside test code), 19 let-underscore hits in 18 files, 20 mod-rs hits in three files and 224 unwrap/expect/panic hits in 49 files (22 in the two fixture.rs files, 202 in helper fns of 47 *_tests.rs files, none in library code).
- `crates/**/*_tests.rs` — 104 files: sibling test files under src/ included by a #[cfg(test)] mod in their parent, and four integration test roots under crates/lys/tests. None begins with #![cfg(test)]; 82 carry the unwrap/expect/panic opt-out.
- `crates/lys-anchor/src/witness/fixture.rs` — Line 1 is the opt-out; 15 unwrap/expect/panic calls; declared from report.rs as a #[cfg(test)] #[path] mod.
- `crates/lys-anchor/src/upward/fixture.rs` — Line 1 is the opt-out; 7 unwrap/expect/panic calls; declared from pin.rs as a #[cfg(test)] #[path] mod.
- `crates/*/tests and tests/*/tests (31 roots and two harness modules not named *_tests.rs)` — Integration test roots and the two tests/harness/mod.rs files. 22 of them carry the opt-out, and their helper fns are not test code to clippy unless the file is marked.
- `crates/lys-home/src/record/mod.rs` — 47 mod-rs hits on main at 7b53625; 0 at 4dd1c33, where HOME-013 moved its logic into home.rs, session.rs and helpers.rs.
- `crates/lys-anchor/tests/harness/mod.rs` — Reads ../lys-core/tests/harness/mod.rs as text in a contract test that checks lys-core's Go environment clauses.
- `crates/lys-core/src/keys/identity_tests.rs` — Five from_env tests mutate LYS_IDENTITY_KEY with std::env::set_var/remove_var under seven #[allow(unsafe_code)] lines, #[serial_test::serial] and an EnvCleanup guard.
- `docs/design/lys-core` — Pre-method cluster: DESIGN.md (255 lines), CHECKLIST.md (C85 to C65), USER-STORIES.md (S30 to S23); no design.json before this design.
- `crates/lys-core/tests/seal_derivation.rs` — Lines 7-8 cite docs/design/lys-core/DESIGN.md (D6) and CHECKLIST.md (C45).
- `docs/PEN-REGISTRATION.md` — Line 50 cites docs/design/lys-core/DESIGN.md and docs/design/lys-core/CHECKLIST.md.
- `crates/lys-core/src/lib.rs` — Lines 27-30 say test builds relax forbid because the env-backed tests call set_var under #[allow(unsafe_code)]; line 59 discards the Result of a String write.
- `vendor/rauthy` — A git submodule, empty until initialised; once initialised its Rust sources sit under the scan root, and ast-grep walks into it.
- `$cambium/sgconfig.yml` — ruleDirs: rules/ast-grep. Read only.
- `$cambium/rules/ast-grep` — Six error-severity rules at 1be80d8ec, unchanged since df4dca5: mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers. No unwrap/expect/panic rule and no _name rule. Read only.

## Constraints

- **CN19** — No wire format, domain-separation tag, test vector or signed fixture changes.
- **CN20** — The public API of lys-core, lys and lys-log-store does not change.
- **CN21** — No hit is cleared by an #[allow], an #[expect], an #[ignore], a _-prefixed rename, #[cfg(any())], or an ignores or files entry beyond tests/** in the unwrap rule and vendor/** in every rule.
- **CN22** — No unwrap, expect or panic call in a fixture.rs or *_tests.rs file is rewritten, moved out of its file or removed.
- **CN23** — Cambium's tree is not changed.
- **CN24** — The seven existing legs of docs/design/project.json keep their names, commands, requirements, cadence and order.
- **CN25** — Only this card writes under docs/design/lys-core. The pre-method documents are renamed, never edited or deleted.
- **CN26** — Every carried rule is at severity error.
- **CN27** — The attribute #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs does not change; the lines above it are comment lines and say what is true.
