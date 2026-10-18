---
type: brief
id: LYSGATE-003
cluster: lys-gate
title: The whole gate's ast-grep findings outside app names: test-only files say so by structure, and the two real ones are fixed
---

# LYSGATE-003: The whole gate's ast-grep findings outside app names: test-only files say so by structure, and the two real ones are fixed

> **Cluster:** lys-gate
> **Depends on:** LYSGATE-002
> **Design anchor:**
> - ADR-063 — lys enforces its code rules with ast-grep, recognising test code by structure the file carries — Carry Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and add lys's own no-unwrap-expect-panic-outside-tests. That rule treats as test code a file whose first item is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/. Every *_tests.rs file, both fixture.rs files, every integration test root and both tests/harness/mod.rs files gain #![cfg(test)] as their first line. The per-module #![allow] opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests. Every existing hit is fixed at its cause in the same card, so the leg is green when it lands. The leg `ast-grep scan --config sgconfig.yml` runs in docs/design/project.json, .land/gates.sh and CI. Rejected: an exemption list of file names; rewriting the calls in test files; narrowing no-lint-bypass-attributes to spare the test opt-outs; carrying only the rules that were clean at the time.
> - ADR-111 — The 500-line limit on code files is a gate leg, not a sentence in CLAUDE.md — A gate leg, `sh scripts/file-length.sh`, runs the checker's own tests and then measures every tracked Rust and TypeScript source file, and fails naming each file whose code lines exceed 500. It is declared in project.json, in every cluster design.json gate, in .land/gates.sh, in CLAUDE.md's 'Gates before any commit' block and in CI. Rejected: an ast-grep rule, which matches syntax and cannot count lines; a clippy lint, since too_many_lines counts one function, not a file, and says nothing of TypeScript; counting raw lines, which would punish documentation.
> **Checklist:**
> - C17 — Every test-only module file the whole gate's ast-grep named carries #![cfg(test)] as its first item, after its mod site was read and found under #[cfg(test)], and ast-grep names none of them.
> - C18 — No unwrap, expect, panic or discarded Result remains in Lys code that is not test code: the fixture-binding panic in refusals_follow.rs is under test or returns the refusal by who calls it, and warn_lines.rs writes its field with push_str; ast-grep's only findings are the app-name rules', counts unchanged.
> **Stories:**
> - S6 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the ast-grep leg to name only what is mine to fix, so that a test-only file says it is test code by its own first line and a real panic in production code is a red I fix, never a red I learn to ignore.

## Purpose

The first whole run of the declared gate on Lys main (fc669616, 19 October 2026) is red on ast-grep with 166 findings. 146 are app names inside Lys's screens and help (ADR-116; a brief of its own in the directory cluster). The other 20 are this brief's: 19 unwraps, expects and panics the rule reads as outside test code, 17 of them in files that are test-only modules whose parents carry the #[cfg(test)], so the file carries no structure of its own that says so; one panic in a pub mod's fixture-binding helper; and one let _ that discards a write into a String. The tokei stage's 94 files are the gate's own defect (it counts raw lines of test files and vendor trees where ADR-111 counts code lines of source; scripts/file-length.sh answers 0 over at the same head) and are reported to the gate's lead, not paid here.

## Task

Give the nine test-only module files the #![cfg(test)] first item ADR-063 recognises, after reading each mod site; fix the panic in refusals_follow.rs by moving it under test or returning the refusal, by who calls it; replace the discarded write! in warn_lines.rs with push_str; prove ast-grep names nothing but the app-name rules, with their counts unchanged.

## Requirements

### R1: A test-only module file says so by structure: #![cfg(test)] as its first item

Behavioural. ADR-063 and the rule no-unwrap-expect-panic-outside-tests recognise test code by structure the file itself carries: a file whose first item is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a file under tests/. Nine files the whole gate names at fc669616 are test-only modules included from a parent under #[cfg(test)] (some through #[path]), so their parents know they are test code and the file does not: crates/lys-home/src/proxy/forward_codex_tests.rs, crates/lys-identity-server/src/mcp_test_answer.rs, crates/lys-identity-server/src/rights_batch_tests.rs, crates/lys-identity-server/src/runner_installer_fixture_tests.rs, crates/lys-identity-server/src/spicedb/batch_tests.rs, crates/lys-runner/src/harness_control/events/tests.rs, crates/lys-runner/src/operations/delivery_composer_tests.rs, crates/lys-secrets/src/bin/lys-secrets/account_binding_tests.rs, crates/lys/src/commands/launch_catalog_tests.rs. Each of these gains #![cfg(test)] as its first item, after its doc comment if it has one, and nothing else in it changes. Before adding it the lane reads each file's mod site and confirms it is under #[cfg(test)]; a file whose mod site is not under #[cfg(test)] is not test code and is listed in the report as R2's, never given the attribute.

**Acceptance:**
- gate -stages ast-grep at the head names none of the nine files under any rule; the record's findings copied into the report
- gate -stages clippy green: the files compile under test and only under test, nothing outside test depended on them
- For each crate touched, one detached gate -stages nextest -crates <crate> -test 'package(<crate>)' green, one crate at a time, counts in the report

**Files:**
- modify: crates/lys-home/src/proxy/forward_codex_tests.rs
- modify: crates/lys-identity-server/src/mcp_test_answer.rs
- modify: crates/lys-identity-server/src/rights_batch_tests.rs
- modify: crates/lys-identity-server/src/runner_installer_fixture_tests.rs
- modify: crates/lys-identity-server/src/spicedb/batch_tests.rs
- modify: crates/lys-runner/src/harness_control/events/tests.rs
- modify: crates/lys-runner/src/operations/delivery_composer_tests.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/account_binding_tests.rs
- modify: crates/lys/src/commands/launch_catalog_tests.rs

**Checklist:**
- C17 — Every test-only module file the whole gate's ast-grep named carries #![cfg(test)] as its first item, after its mod site was read and found under #[cfg(test)], and ast-grep names none of them.

**Stories:**
- S6 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the ast-grep leg to name only what is mine to fix, so that a test-only file says it is test code by its own first line and a real panic in production code is a red I fix, never a red I learn to ignore.

### R2: The findings in code that is not test code are fixed, not exempted

Behavioural. crates/lys-identity-server/src/refusals_follow.rs line 257 panics when the fixture runner cannot be bound, in a pub mod the crate compiles for everyone. The lane reads who calls that function: if only test code does, the function and what serves only it move under #[cfg(test)] (a #[cfg(test)] mod body of the same file, or the file's own test module), so the structure says it is test code; if production code calls it, the panic becomes the ServerError the caller already returns, the refusal named as the crate names refusals (read refusals_api.rs for the shape), and the caller propagates it. crates/lys-identity-server/src/warn_lines.rs line 76 discards the Result of write! into a String with let _; fmt::Write on a String cannot fail, so the line becomes fields.rest.push_str(&format!(...)), which has no Result to discard, and the rule finds nothing there. No #[allow], no sgconfig change, no rule change: the rules stand as written (the no-lint-bypass-attributes rule forbids the first; ADR-063 the rest).

**Acceptance:**
- Red first for refusals_follow.rs where production code calls it: a test asserts the refusal the caller answers when the runner cannot be bound, by name, and no panic; where only tests call it, the report says so with the call sites and the test is the existing one that exercises it
- gate -stages ast-grep at the head names only the no-app-names-tsx, no-app-names-typescript and no-app-names-rust rules, their counts unchanged from fc669616 (95, 47, 4), and no finding under no-unwrap-expect-panic-outside-tests or no-let-underscore-on-results; the record copied into the report
- gate -stages nextest -crates lys-identity-server -test 'package(lys-identity-server)' green, counts in the report

**Files:**
- modify: crates/lys-identity-server/src/refusals_follow.rs
- modify: crates/lys-identity-server/src/warn_lines.rs

**Checklist:**
- C18 — No unwrap, expect, panic or discarded Result remains in Lys code that is not test code: the fixture-binding panic in refusals_follow.rs is under test or returns the refusal by who calls it, and warn_lines.rs writes its field with push_str; ast-grep's only findings are the app-name rules', counts unchanged.

**Stories:**
- S6 (AI Agent, Runs the gates by hand before a commit) — As an agent building a card, I want the ast-grep leg to name only what is mine to fix, so that a test-only file says it is test code by its own first line and a real panic in production code is a red I fix, never a red I learn to ignore.

## Boundaries

- No rule under rules/ast-grep changes, sgconfig.yml does not change, and no #[allow] or #[expect] is added anywhere (no-lint-bypass-attributes)
- The app-name findings (no-app-names-tsx, -typescript, -rust) are not touched: they are DIRECTORY-106's, under ADR-116
- No file is split or reformatted beyond the lines named; cargo fmt --all before each commit
- No timer, no fixed wait, no poll and no deadline anywhere, in code or test
- Nothing installed, the installed Lys untouched

## Verification

- gate -stages ast-grep, its record read: findings only under the three no-app-names rules, counts 95, 47 and 4
- gate -stages clippy
- gate -stages nextest -crates <crate> -test 'package(<crate>)' for each crate touched, one at a time
- python3 scripts/design/validate.py docs/design/lys-gate and python3 scripts/design/check-coverage.py docs/design/lys-gate read for FAIL lines
