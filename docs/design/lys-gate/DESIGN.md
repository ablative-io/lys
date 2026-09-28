---
type: design
cluster: lys-gate
title: Lys Gate — the gate's test leg runs every test binary
---

# Lys Gate — the gate's test leg runs every test binary

> **Cluster:** lys-gate

## Intention

A red gate round tells the whole truth about the tests in one run. When any test fails, the round still runs every test binary in every crate, reports every failure together, and still exits non-zero, so a reader fixes everything that is broken instead of discovering one failure per round.

## Problem

The lys gate's test leg runs cargo test over the workspace with all features and without --no-fail-fast, and cargo then stops at the first failing test binary. A run that fails in one crate reports nothing about the binaries and crates after it, so a gate can go red for one reason while hiding others. The same leg is declared in docs/design/project.json (the project gate every new cluster copies), in the gate arrays of docs/design/directory/design.json, docs/design/home/design.json and docs/design/secrets/design.json, and in .land/gates.sh (the landing hook). CLAUDE.md's 'Gates before any commit' block lists the same test command, and .land/gates.sh says it runs its legs exactly as that block lists them. The CI workflow's test step, in .github/workflows/ci.yml, runs cargo test over the workspace without --no-fail-fast too, and also stops at the first failing binary.

## Solution

One commit changes the test leg's command to `cargo test --workspace --all-features --no-fail-fast` everywhere it is declared as a gate leg: the 'tests' leg of project.json's trees; the 'tests' leg of the gate arrays of the directory, home and secrets designs and of this design, whose gate is copied from project.json; the `leg cargo test` line of .land/gates.sh; and the test line of CLAUDE.md's 'Gates before any commit' block, so .land/gates.sh still matches it exactly. CI's test step gains the same one flag and nothing else. With --no-fail-fast cargo runs every test binary, reports every failure, and exits non-zero when any test failed (ADR-038). Nothing else in any gate moves: the other legs, their order, their requirements and their cadence stay byte-identical. The clusters whose design.json changes are re-rendered with scripts/design/render-cluster.py; the gate array is not rendered into markdown, so their markdown does not change, and scripts/design/gate.sh stays green. The ledger that runs a gate round detects a running battery by the text 'cargo test' in a process's command line (the method's workers/ds2_ledger/gate.py:201-223 at method commit 3c3bac7) and logs each leg's command as a '$ command' line under '-- leg NAME at venue V' (the same file, 409-437); the new command still contains 'cargo test', so the leg is still treated as a battery, and the logged '$ command' line is where that is shown. The proof is four measurements: a grep over the six files that declare a gate test leg, named one by one and never by a glob so that a cluster appearing later cannot turn it red, that prints 0; the '$ command' line of the tests leg in four green gate rounds at the change, one measured against each changed cluster's design.json (directory, home, secrets and this one), since the ledger measures one cluster's gate per round, and the directory and secrets rounds, whose legs require place:here, are run by a ledger whose own place is the venue that serves heavy builds and full gates; one gate round over a local scratch branch that plants a single failing test in the first workspace member, crates/lys, and shows exactly one failed test result with as many test results as this cluster's green round; and a proof document in this cluster that names each of those five rounds' logs by round id, path in the ledger's gate log store and SHA-256, and the scratch round's counts, never their content. The chain's own commits also write this design.json, so for it the one-commit rule is measured on its gate array alone. The scratch branch never reaches origin and is deleted after its round. The method's scripts and the ledger stay as they are (ADR-004).

## Principles

- **P1** — A red test leg reports every failing test binary in the one round; it never stops at the first.
- **P2** — Every place that declares the gate's test leg declares the same command, so a gate copied from any of them carries --no-fail-fast.
- **P3** — A gate change moves one command string and nothing else: leg names, order, requirements and cadence stay byte-identical.
- **P4** — Evidence of a round is named by its path, its SHA-256 and its counts; the log's content is not copied into the documents.
- **P5** — A standard written in CLAUDE.md is measured by a gate leg; a rule no leg measures holds only by care.

## Decisions

- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-038 — The gate's test leg runs every test binary; it never stops at the first failure — Every declaration of the gate's test leg runs `cargo test --workspace --all-features --no-fail-fast`: project.json, every cluster design.json gate, .land/gates.sh and CLAUDE.md's 'Gates before any commit' block; CI's test step gains the same one flag and nothing else. Rejected: keeping cargo's default fail-fast run, which is shorter on a red round but reports only the first failing binary.
- ADR-111 — The 500-line limit on code files is a gate leg, not a sentence in CLAUDE.md — A gate leg, `sh scripts/file-length.sh`, runs the checker's own tests and then measures every tracked Rust and TypeScript source file, and fails naming each file whose code lines exceed 500. It is declared in project.json, in every cluster design.json gate, in .land/gates.sh, in CLAUDE.md's 'Gates before any commit' block and in CI. Rejected: an ast-grep rule, which matches syntax and cannot count lines; a clippy lint, since too_many_lines counts one function, not a file, and says nothing of TypeScript; counting raw lines, which would punish documentation.

## Goals

- docs/design/project.json, the directory, home, secrets and lys-gate design.json gate arrays and .land/gates.sh declare their test leg as exactly `cargo test --workspace --all-features --no-fail-fast`, and the grep over them prints 0.
- CLAUDE.md's 'Gates before any commit' test line and .land/gates.sh's test leg are the same command, `cargo test --workspace --all-features --no-fail-fast`.
- CI's test step runs `cargo test --workspace --no-fail-fast`.
- sh scripts/design/gate.sh exits 0 at the card's head.
- One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the green round at the same head, and no scratch/ ref exists on origin after it.
- Every tracked Rust and TypeScript source file holds at most 500 lines of code, measured by a gate leg that every gate declaration carries (LYSGATE-002).

## Non-Goals

- Changing the other clusters' briefs under docs/design/*/briefs or the release documents that quote cargo commands — They quote cargo commands as acceptance text or instructions, not as gate legs.
- Changing docs/design/lys-core, including its DESIGN.md line 142 and its CHECKLIST.md item C63 — They are acceptance prose in a cluster with no design.json gate, owned by the ast-grep leg card.
- Changing the design.json of a cluster that is not among the four this card changes, including one that appears later — Such a cluster carries the test leg its own card wrote; the proof document records one that lacks --no-fail-fast as a finding for that card by file name.
- Raising a timeout or splitting a test — The words exclude both.
- Adding --all-features to CI's test step — It changes which tests CI runs; it is named as a finding and left for its own unit.
- A mechanism that refuses to land a scratch/ branch — The scratch branch is kept from landing by never reaching origin; a refusal belongs to the landing chain and is left for its own unit.
- Rewording CLAUDE.md's 'All five clean' sentence under the six gate commands — This card changes only the block's test line.
- Changing the method project, including the ledger's gate.py, or the vendored method under scripts/design — The card keeps to the method as it stands; the logged '$ command' line shows the battery text.
- Adding a script for the check that no test leg lacks --no-fail-fast — The check is one grep recorded as evidence.
- Splitting any file in LYSGATE-002 — No file on main 6194599 is over the limit; the leg's first run is green, and a later file over it is split by the build that grows it.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/project.json` | the project gate; trees[0] 'tests' leg command gains --no-fail-fast (LYSGATE-001 R1) |  |
| `docs/design/directory/design.json` | directory cluster gate; its 'tests' leg command gains --no-fail-fast (LYSGATE-001 R2) |  |
| `docs/design/home/design.json` | home cluster gate; its 'tests' leg command gains --no-fail-fast (LYSGATE-001 R2) |  |
| `docs/design/secrets/design.json` | secrets cluster gate; its 'tests' leg command gains --no-fail-fast (LYSGATE-001 R2) |  |
| `docs/design/lys-gate/design.json` | this design; its gate array, copied from project.json, has its 'tests' leg command gain --no-fail-fast with the others (LYSGATE-001 R2) |  |
| `docs/design/lys-gate/DESIGN.md` | rendered design |  |
| `docs/design/lys-gate/checklist.json` | the checklist rows LYSGATE-001 delivers |  |
| `docs/design/lys-gate/CHECKLIST.md` | rendered checklist |  |
| `docs/design/lys-gate/stories.json` | the stories LYSGATE-001 serves |  |
| `docs/design/lys-gate/USER-STORIES.md` | rendered stories |  |
| `docs/design/lys-gate/briefs/LYSGATE-001.json` | the brief that changes the gate's test leg |  |
| `docs/design/lys-gate/briefs/LYSGATE-001.md` | its rendered markdown |  |
| `docs/design/lys-gate/PROOF-LYSGATE-001.md` | the proof: the grep's output, the four green rounds' and the scratch round's ids, log paths in the gate log store and SHA-256s, the scratch round's counts, the ls-remote check, and other clusters' design.json files whose test leg lacks --no-fail-fast, by file name (LYSGATE-001 R9) | LYSGATE-001 |
| `.land/gates.sh` | the landing hook; its `leg cargo test` line gains --no-fail-fast (LYSGATE-001 R3) |  |
| `CLAUDE.md` | 'Gates before any commit' block; its test line gains --no-fail-fast (LYSGATE-001 R4) |  |
| `.github/workflows/ci.yml` | CI; the Test step's cargo test command gains --no-fail-fast and nothing else (LYSGATE-001 R5) |  |
| `scripts/design/gate.sh` | the design leg; validates, checks coverage and compares rendered markdown for every cluster with a design.json; read, not changed |  |
| `scripts/design/render-cluster.py` | renders a cluster's markdown from its JSON; does not render the gate array; read, not changed |  |
| `scripts/check_file_length.py` | the file-length checker (LYSGATE-002 R1) | LYSGATE-002 |
| `scripts/check_file_length_test.py` | its tests (LYSGATE-002 R2) | LYSGATE-002 |
| `scripts/file-length.sh` | the leg's command: tests, then the checker (LYSGATE-002 R3) | LYSGATE-002 |
| `docs/design/decisions-words/design.json` | cluster gate; its '.' tree gains the file-length leg after ast-grep (LYSGATE-002 R4) | LYSGATE-002 |
| `docs/design/lys-anchor/design.json` | cluster gate; its '.' tree gains the file-length leg after ast-grep (LYSGATE-002 R4) | LYSGATE-002 |
| `docs/design/lys-core/design.json` | cluster gate; its '.' tree gains the file-length leg after ast-grep (LYSGATE-002 R4) | LYSGATE-002 |
| `docs/design/lys-log-store/design.json` | cluster gate; its '.' tree gains the file-length leg after ast-grep (LYSGATE-002 R4) | LYSGATE-002 |
| `docs/design/rauthy-rebase/design.json` | cluster gate; its '.' tree gains the file-length leg after ast-grep (LYSGATE-002 R4) | LYSGATE-002 |
| `docs/design/roots/design.json` | cluster gate; its '.' tree gains the file-length leg after ast-grep (LYSGATE-002 R4) | LYSGATE-002 |
| `docs/design/lys-gate/PROOF-LYSGATE-002.md` | the green and red runs of the file-length leg (LYSGATE-002 R6) | LYSGATE-002 |

## Inventory

- `docs/design/project.json` — trees[0].legs[3] is the 'tests' leg, cargo test over the workspace with all features and without --no-fail-fast, requires rust-build, cadence round
- `docs/design/directory/design.json` — gate tree '.' 'tests' leg at line 716, the same command without --no-fail-fast, requires place:here
- `docs/design/home/design.json` — gate tree '.' 'tests' leg at line 776, the same command without --no-fail-fast, requires rust-build
- `docs/design/secrets/design.json` — gate tree '.' 'tests' leg at line 185, the same command without --no-fail-fast, requires place:here
- `.land/gates.sh` — 22 lines; line 19 is the `leg cargo test` line without --no-fail-fast; its leg() wrapper already runs every leg after a red one and echoes `--- $* ---`; its header says the legs are exactly as CLAUDE.md 'Gates before any commit' lists them; it never reads the branch name
- `CLAUDE.md` — line 99, in the 'Gates before any commit' block, is the test gate, without --no-fail-fast
- `.github/workflows/ci.yml` — line 46, the Test step, runs cargo test over the workspace with neither --all-features nor --no-fail-fast, env LYS_REQUIRE_GO=1
- `Cargo.toml` — workspace members in order: crates/lys, crates/lys-anchor, crates/lys-anchor-cli, crates/lys-core, crates/lys-home, crates/lys-log-store
- `crates/lys/tests/certified_attestation_tests.rs` — 380 lines, 6 tests; the first integration test binary of crates/lys by name
- `scripts/design/gate.sh` — 28 lines; the design leg run by .land/gates.sh and by the cluster gates
- `scripts/design/render-cluster.py` — 256 lines; never reads the gate array
- `docs/design/lys-core/DESIGN.md` — the lys-core cluster's hand-written design; line 142's acceptance and CHECKLIST.md's C63 quote cargo test over the workspace as acceptance text, not as a gate leg; the cluster has no design.json and is not edited

## Constraints

- **CN1** — The only change to any gate declaration is the test leg's command string; every other leg's name, command, requirements, cadence and position is byte-identical before and after.
- **CN2** — --all-features stays on every gate test leg, and CI's test step does not gain it.
- **CN3** — The other clusters' briefs under docs/design/*/briefs (every cluster's but lys-gate's), the release documents and everything under docs/design/lys-core are not created, edited or deleted.
- **CN4** — The vendored method under scripts/design (validate.py, check-coverage.py, render-cluster.py, render-brief.py, schemas, workers) and the method project are not edited, and no script is added.
- **CN5** — No crate source, test source or wire format changes on the card's branch.
- **CN6** — No timeout is raised and no test is split.
- **CN7** — The scratch branch and its planted test never reach origin: no ref under scratch/ exists on the lys origin after the scratch round.
- **CN8** — The file-length checker reads the tracked file list once and each file once, in one process, with no subprocess per file and no network.
