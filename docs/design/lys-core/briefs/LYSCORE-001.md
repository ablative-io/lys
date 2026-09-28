---
type: brief
id: LYSCORE-001
cluster: lys-core
title: Give lys an ast-grep gate leg with the rule set it can carry, and bring the tree to zero hits
---

# LYSCORE-001: Give lys an ast-grep gate leg with the rule set it can carry, and bring the tree to zero hits

> **Cluster:** lys-core
> **Blocked by:** HOME-013 (roadmap RM-028, brief branch brief/home/a3186728), the split of crates/lys-home/src/record/mod.rs, must land on origin/main first: the leg lands only when R9's count command prints 0 for that file on origin/main., ast-grep installed at the gate place, at 0.44.1 as this brief was measured with, so a leg requiring tool:ast-grep can be placed.
> **Design anchor:**
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
> - ADR-055 — Cambium's ast-grep rules are carried into lys only where they can fire — lys carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and records no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers as not carried with the finding that they cannot fire; the _name rule is its own card. Rejected: copying all six rules, which would look like cover the scan cannot give.
> **Checklist:**
> - C1 — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
> - C2 — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.
> - C3 — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
> - C4 — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
> - C5 — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
> - C6 — Every rule in rules/ast-grep ignores vendor/**.
> - C7 — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.
> - C8 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
> - C9 — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].
> - C10 — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].
> - C11 — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
> - C12 — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.
> - C13 — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.
> - C14 — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.
> - C15 — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.
> - C16 — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
> - C17 — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.
> - C18 — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
> - C19 — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
> - C20 — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
> - C21 — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
> - C22 — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
> - C23 — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.
> **Stories:**
> - S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
> - S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
> - S3 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the same scan run by the design gate, the landing gate and CI so that no path to main skips it.
> - S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.
> - S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.
> - S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.
> - S7 (Lys contributor, Writing tests and library code) — As a lys contributor changing key loading, I want the env-backed identity tests to run without unsafe code so that the test build needs no lint bypass.
> - S8 (Lys contributor, Writing tests and library code) — As a lys contributor reading a module tree, I want mod.rs files to hold only declarations and re-exports so that logic is found in a named file.
> - S9 (Lys contributor, Writing tests and library code) — As a lys contributor, I want no `let _ =` discard in the tree so that no error is swallowed without a decision.
> - S10 (Design reader, Reading the lys-core cluster) — As a reader of the lys-core design, I want the hand-written pre-method documents kept beside the rendered cluster so that the earlier design is not lost when the method's documents replace it.

## Purpose

lys holds itself to rules its gate does not enforce: no unwrap, expect or panic outside tests, no lint bypass, no logic in mod.rs, no `let _ =` discard. This brief gives the gate an ast-grep leg, carries the Cambium rules that can fire on lys, adds the one rule Cambium relies on clippy for, and brings the tree to zero hits by fixing causes, so the leg can land green and refuse the next hit on every path to main (see the design's solution, ADR-054 and ADR-055).

## Task

Start from a clean clone of the lys repository at the commit this brief lands at, never a shared folder; two earlier runs collided on shared state. The brief's own commit already carries the rename of the three hand-written lys-core documents to *-PRE-METHOD.md and the rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md, so the design leg is green when the brief lands. In order: check that the kept and rendered cluster documents still hold (R1); add sgconfig.yml and the four rules (R2); add clippy.toml and the #![cfg(test)] first line to the 90 src test files (R3) and the 30 files under tests/ (R4), removing all 106 #![allow] test opt-outs; replace the env mutation in lys-core's identity tests with a seam and correct the lib.rs comment (R5); move the mod.rs logic into named files (R6); fix the `let _ =` discards (R7); correct CLAUDE.md and Cargo.toml (R8); and wire the leg into project.json, .land/gates.sh and CI (R9). Heavy builds and full gates run at the gate place; only a single crate's clippy or tests runs where the work is written.

Corrections to the words' sentences. The words list no unwrap, expect or panic outside tests, no #[allow], no _name renames and no #[ignore] as rules the estate holds and ask for the same rule set Cambium carries. Measured at Cambium 1be80d8ec, Cambium carries six rules and none for unwrap, expect or panic (it relies on clippy) and none for _name bindings. So the rule set carried is three of Cambium's six (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, the last covering #[allow] and #[ignore]) plus a structural unwrap/expect/panic rule of lys's own; the _name rule is not carried and is its own card; no-std-mutex-in-async and the two door-timer rules are not carried because they cannot fire on lys. The 22 fixture calls and the helper calls in the sibling test files are not hits because their files carry #![cfg(test)], and none of them is rewritten.

Counts measured at 7b53625, which this brief follows where a ruling's count differs: 88 sibling *_tests.rs under src/ plus the 4 *_tests.rs roots of crates/lys make the 92 *_tests.rs files; 28 integration roots and 2 harness modules under tests/ (26 of them carrying #![allow], where the ruling counted 22); 106 #![allow] test opt-outs and 7 #[allow(unsafe_code)]; 224 unwrap/expect/panic calls outside the structural test markers before the markers, 0 in library code, 0 after them.

The kept CHECKLIST-PRE-METHOD.md's C4 says the env-backed tests call set_var under an explicit #[allow]; after R5 the code no longer bears that sentence out, and the kept file is not corrected. The rendered CHECKLIST.md is the current truth. The pre-method C-numbers and this cluster's C-numbers are different lists; this brief's C-numbers are the rendered checklist's.

Out of scope, each its own unit: the _name rule, no-std-mutex-in-async, tightening the unsafe attribute so tests forbid unsafe code too, and the record/mod.rs split.

## Requirements

### R1: Check that the kept *-PRE-METHOD.md documents and the rendered cluster hold on the card's tree

Structural. The brief's own commit renamed docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and carries DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md as scripts/design/render-cluster.py renders them from the cluster's JSON. This requirement checks that this state holds on the card's tree; it performs no rename and no render of its own. The kept files are the historical record and are not edited: CHECKLIST-PRE-METHOD.md's C4 says the env-backed tests call std::env::set_var under an explicit #[allow], which the code no longer bears out after R5, and the rendered CHECKLIST.md is the current truth. THE SYSTEM SHALL NOT edit a *-PRE-METHOD.md file, SHALL NOT hand-edit a rendered markdown file, and SHALL NOT write under any docs/design directory other than lys-core and docs/design/project.json.

**Acceptance:**
- `git show 7b53625:docs/design/lys-core/DESIGN.md | cmp - docs/design/lys-core/DESIGN-PRE-METHOD.md` exits 0.
- `git show 7b53625:docs/design/lys-core/CHECKLIST.md | cmp - docs/design/lys-core/CHECKLIST-PRE-METHOD.md` exits 0.
- `git show 7b53625:docs/design/lys-core/USER-STORIES.md | cmp - docs/design/lys-core/USER-STORIES-PRE-METHOD.md` exits 0.
- `test -f docs/design/lys-core/briefs/LYSCORE-001.md` exits 0.
- `sh scripts/design/gate.sh` exits 0 and prints no line containing `rendered markdown differs`.
- `git diff --name-only origin/main...HEAD -- docs/design | grep -v '^docs/design/lys-core/' | grep -vx 'docs/design/project.json'` prints nothing.

**Checklist:**
- C1 — DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md under docs/design/lys-core are byte-identical to DESIGN.md, CHECKLIST.md and USER-STORIES.md at 7b53625.
- C2 — DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-001.md under docs/design/lys-core are what render-cluster.py renders, and sh scripts/design/gate.sh exits 0.

**Stories:**
- S10 (Design reader, Reading the lys-core cluster) — As a reader of the lys-core design, I want the hand-written pre-method documents kept beside the rendered cluster so that the earlier design is not lost when the method's documents replace it.

### R2: Add sgconfig.yml and rules/ast-grep with the three carried Cambium rules and the structural unwrap/expect/panic rule

Structural. sgconfig.yml at the repository root reads `ruleDirs:` with the one entry `rules/ast-grep`, as Cambium's does. In this requirement's acceptance, `$cambium` is the path of a local clone of the Cambium repository in which commit 1be80d8ec is present, set by whoever takes the measurement; `git -C $cambium cat-file -e 1be80d8ec` exits 0 in it. rules/ast-grep holds exactly four rules. mod-rs-declarations-only.yml, no-let-underscore-on-results.yml and no-lint-bypass-attributes.yml are Cambium's files at 1be80d8ec with one addition each: an `ignores` list holding `vendor/**`. no-unwrap-expect-panic-outside-tests.yml (severity error, language rust) matches `$RECV.unwrap()`, `$RECV.expect($$$ARGS)` and `panic!($$$ARGS)`, and does not report a match that sits in test code, which is: inside a source file whose first child is `#![cfg(test)]`; inside a mod item preceded by `#[cfg(test)]`; inside a function item preceded by `#[test]`, other attributes and comments between them allowed; and any file under `crates/*/tests/`, written as an `ignores` entry beside `vendor/**`. WHEN code outside those four structures calls unwrap, expect or panic, THE SYSTEM SHALL report it as an error-severity hit. THE SYSTEM SHALL NOT name a file or a list of file names in any rule, SHALL NOT recognise test code by a file's name, SHALL NOT scan vendor/, SHALL NOT edit vendor/rauthy, and SHALL NOT edit Cambium's rules or config. Recorded as not carried, each with its finding: no-std-mutex-in-async, because lys has no async fn, no tokio entry and no std Mutex at 7b53625, so it could not fire and would only look like cover; the card that lands lys's first async code carries it and its brief names this ruling. no-timer-in-door-handlers and no-timer-import-in-door-handlers, for the same reason: lys has no door handlers. The _name rule, because Cambium carries none, so the same rule set does not include it: the ruling counts 272 underscore-prefixed bindings at 7b53625 (not reproduced here: an ast-grep count of identifiers beginning with an underscore finds 303, and a grep for `let _x` and `_x:` finds 37), and a card of its own writes the rule and handles each binding by its act. #[ignore] is caught by the carried no-lint-bypass-attributes.

**Acceptance:**
- `cat sgconfig.yml` prints exactly the two lines `ruleDirs:` and `  - rules/ast-grep`.
- `ls rules/ast-grep` prints exactly mod-rs-declarations-only.yml, no-let-underscore-on-results.yml, no-lint-bypass-attributes.yml and no-unwrap-expect-panic-outside-tests.yml.
- `git -C $cambium cat-file -e 1be80d8ec` exits 0, before either acceptance line that reads from $cambium is taken.
- For each of the three carried rules, deleting its `ignores:` line and its `  - "vendor/**"` line leaves a file byte-identical to `git -C $cambium show 1be80d8ec:rules/ast-grep/<name>.yml`.
- `grep -c 'vendor/\*\*' rules/ast-grep/*.yml` prints 1 for each of the four files.
- A scratch file crates/lys-core/src/scratch_markers.rs holding the 14 lines `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }`, `pub fn b(x: Option<u8>) -> u8 { x.expect("b") }`, `pub fn c() { panic!("c") }`, `#[cfg(test)]`, `mod t { fn h(x: Option<u8>) -> u8 { x.unwrap() } }`, `#[test]`, `fn tt() { None::<u8>.unwrap(); }`, `#[cfg(unix)]`, `#[test]`, `// comment`, `fn tt2() { None::<u8>.unwrap(); }`, `#[cfg(not(test))]`, `mod n { fn h(x: Option<u8>) -> u8 { x.unwrap() } }`, `fn not_test_fn() { None::<u8>.expect("x"); }` gives exactly 5 no-unwrap-expect-panic-outside-tests hits from `ast-grep scan --config sgconfig.yml --json=stream crates/lys-core/src/scratch_markers.rs`, at lines 1, 2, 3, 13 and 14; the file is deleted after.
- A scratch file crates/lys-core/src/scratch_first.rs whose two lines are `#![cfg(test)]` and `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }` gives 0 hits from the same scan; the file is deleted after.
- A scratch file crates/lys-core/src/scratch_second.rs whose three lines are `//! doc`, `#![cfg(test)]` and `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }` gives exactly 1 hit, at line 3; the file is deleted after.
- A scratch file crates/lys-core/tests/scratch_path.rs whose one line is `fn h(x: Option<u8>) -> u8 { x.unwrap() }` gives 0 hits from `ast-grep scan --config sgconfig.yml --json=stream`; the file is deleted after.
- A scratch file vendor/scratch_vendor.rs whose one line is `pub fn a(x: Option<u8>) -> u8 { x.unwrap() }` gives 0 hits from `ast-grep scan --config sgconfig.yml --json=stream`; the file is deleted after.
- `git -C $cambium status --porcelain rules sgconfig.yml clippy.toml` prints nothing.

**Files:**
- create: sgconfig.yml
- create: rules/ast-grep/mod-rs-declarations-only.yml
- create: rules/ast-grep/no-let-underscore-on-results.yml
- create: rules/ast-grep/no-lint-bypass-attributes.yml
- create: rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml

**Checklist:**
- C3 — sgconfig.yml at the repository root names rules/ast-grep as its only rule directory.
- C4 — rules/ast-grep carries mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes with the rule bodies Cambium carries at 1be80d8ec.
- C5 — rules/ast-grep carries the error-severity rule no-unwrap-expect-panic-outside-tests, which reports unwrap, expect and panic outside test code recognised by structure.
- C6 — Every rule in rules/ast-grep ignores vendor/**.
- C7 — The brief records no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers and the _name rule as not carried, each with its finding.

**Stories:**
- S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
- S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
- S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.

### R3: Add clippy.toml's test allowances and mark the fixtures and sibling test files with #![cfg(test)]

Structural. clippy.toml at the repository root sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true and nothing else. Each of the 2 fixture.rs files and the 88 sibling *_tests.rs files under crates/*/src gains `#![cfg(test)]` as its first line, and the 80 of them that carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` lose that line. The outer #[cfg(test)] on each parent's `mod x;` declaration stays: measured with clippy 0.1.97, the pair trips no lint. THE SYSTEM SHALL NOT rewrite any unwrap, expect or panic call in these files, SHALL NOT put any #[allow], #![allow] or #[expect] in place of a removed line, and SHALL NOT change what any of these files declares.

**Acceptance:**
- `cat clippy.toml` prints exactly the three lines `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true` and `allow-panic-in-tests = true`.
- `for f in $(find crates -name '*_tests.rs' -path '*/src/*') crates/lys-anchor/src/upward/fixture.rs crates/lys-anchor/src/witness/fixture.rs; do head -1 "$f"; done | sort | uniq -c` prints exactly `  90 #![cfg(test)]`.
- `grep -l '#!\[allow' $(find crates -name '*_tests.rs' -path '*/src/*') crates/lys-anchor/src/upward/fixture.rs crates/lys-anchor/src/witness/fixture.rs` prints nothing.
- For each of these 90 files, `grep -oE '\.unwrap\(\)|\.expect\(|panic!\(' <file> | wc -l` at HEAD is not below the same count at origin/main.
- `ast-grep scan --config sgconfig.yml --filter no-unwrap-expect-panic-outside-tests --json=stream | wc -l` prints 0.
- `cargo clippy -p lys-anchor --all-targets --all-features -- -D warnings` exits 0.

**Files:**
- create: clippy.toml
- modify: crates/lys-anchor-cli/src/cli_tests.rs
- modify: crates/lys-anchor-cli/src/commands/anchor/open_tests.rs
- modify: crates/lys-anchor-cli/src/commands/anchor/policy_tests.rs
- modify: crates/lys-anchor-cli/src/commands/error_tests.rs
- modify: crates/lys-anchor-cli/src/commands/hex_tests.rs
- modify: crates/lys-anchor-cli/src/commands/output_tests.rs
- modify: crates/lys-anchor/src/admission/certificate_tests.rs
- modify: crates/lys-anchor/src/admission/context_tests.rs
- modify: crates/lys-anchor/src/admission/trivial_tests.rs
- modify: crates/lys-anchor/src/anchor/append_tests.rs
- modify: crates/lys-anchor/src/anchor/artifact_tests.rs
- modify: crates/lys-anchor/src/anchor/checkpoint_tests.rs
- modify: crates/lys-anchor/src/anchor/genesis_tests.rs
- modify: crates/lys-anchor/src/anchor/open_tests.rs
- modify: crates/lys-anchor/src/anchor/proof_nodes_tests.rs
- modify: crates/lys-anchor/src/anchor/read_only_tests.rs
- modify: crates/lys-anchor/src/anchor/status_tests.rs
- modify: crates/lys-anchor/src/anchor/submit_tests.rs
- modify: crates/lys-anchor/src/keys/file_signer_tests.rs
- modify: crates/lys-anchor/src/upward/bundle_tests.rs
- modify: crates/lys-anchor/src/upward/pin_tests.rs
- modify: crates/lys-anchor/src/witness/observe_tests.rs
- modify: crates/lys-anchor/src/witness/projection_tests.rs
- modify: crates/lys-core/src/attestation/artifact_tests.rs
- modify: crates/lys-core/src/attestation/encoding_tests.rs
- modify: crates/lys-core/src/attestation/sign_tests.rs
- modify: crates/lys-core/src/bundle/verify_tests.rs
- modify: crates/lys-core/src/ca/authority_tests.rs
- modify: crates/lys-core/src/ca/certificate_tests.rs
- modify: crates/lys-core/src/ca/extensions_tests.rs
- modify: crates/lys-core/src/ca/request_tests.rs
- modify: crates/lys-core/src/cbor_tests.rs
- modify: crates/lys-core/src/checkpoint/body_tests.rs
- modify: crates/lys-core/src/checkpoint/note_tests.rs
- modify: crates/lys-core/src/checkpoint/verifier_key_tests.rs
- modify: crates/lys-core/src/delegation/artifact_tests.rs
- modify: crates/lys-core/src/delegation/encoding_tests.rs
- modify: crates/lys-core/src/delegation/sign_tests.rs
- modify: crates/lys-core/src/error_tests.rs
- modify: crates/lys-core/src/keys/compare_tests.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/keys/ssh_tests.rs
- modify: crates/lys-core/src/merkle/consistency_tests.rs
- modify: crates/lys-core/src/merkle/leaf_tests.rs
- modify: crates/lys-core/src/merkle/proof_tests.rs
- modify: crates/lys-core/src/merkle/reconstruct_tests.rs
- modify: crates/lys-core/src/merkle/tree_tests.rs
- modify: crates/lys-core/src/receipt/artifact_tests.rs
- modify: crates/lys-core/src/receipt/consistency_tests.rs
- modify: crates/lys-core/src/receipt/encoding_tests.rs
- modify: crates/lys-core/src/receipt/sign_tests.rs
- modify: crates/lys-core/src/seal/authenticated_tests.rs
- modify: crates/lys-core/src/seal/sealed_envelope_tests.rs
- modify: crates/lys-core/src/tlog/artifact_tests.rs
- modify: crates/lys-core/src/tlog/build_tests.rs
- modify: crates/lys-core/src/tlog/verify_tests.rs
- modify: crates/lys-home/src/harness/claude_code/events_tests.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/src/harness/claude_code/import_tests.rs
- modify: crates/lys-home/src/harness/claude_code/paths_tests.rs
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs
- modify: crates/lys-home/src/harness/claude_code/seed_tests.rs
- modify: crates/lys-home/src/harness/claude_code/template_tests.rs
- modify: crates/lys-home/src/record/beside_tests.rs
- modify: crates/lys-home/src/record/blocks_tests.rs
- modify: crates/lys-home/src/record/call_tests.rs
- modify: crates/lys-home/src/record/canon_tests.rs
- modify: crates/lys-home/src/record/epilogue_tests.rs
- modify: crates/lys-home/src/record/fork_cut_tests.rs
- modify: crates/lys-home/src/record/fork_tests.rs
- modify: crates/lys-home/src/record/given_tests.rs
- modify: crates/lys-home/src/record/lantern_tests.rs
- modify: crates/lys-home/src/record/reader_tests.rs
- modify: crates/lys-home/src/record/recall_tests.rs
- modify: crates/lys-home/src/record/record_tests.rs
- modify: crates/lys-home/src/record/templates_tests.rs
- modify: crates/lys-log-store/src/file_tests.rs
- modify: crates/lys-log-store/src/log_tests.rs
- modify: crates/lys/src/cli_tests.rs
- modify: crates/lys/src/commands/ca_tests.rs
- modify: crates/lys/src/commands/duration_tests.rs
- modify: crates/lys/src/commands/error_tests.rs
- modify: crates/lys/src/commands/files_tests.rs
- modify: crates/lys/src/commands/hex_tests.rs
- modify: crates/lys/src/commands/log/status_tests.rs
- modify: crates/lys/src/commands/log/store_tests.rs
- modify: crates/lys/src/commands/output_tests.rs
- modify: crates/lys/src/commands/pem_tests.rs
- modify: crates/lys-anchor/src/upward/fixture.rs
- modify: crates/lys-anchor/src/witness/fixture.rs

**Checklist:**
- C8 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true.
- C9 — Both fixture.rs files and the 88 sibling *_tests.rs files under src/ begin with #![cfg(test)] and carry no #![allow].

**Stories:**
- S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.

### R4: Mark the integration test roots and the two harness modules with #![cfg(test)]

Structural. Measured at 7b53625: crates/*/tests holds 28 integration test roots, 24 of which carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, and 2 tests/harness/mod.rs files, both carrying it: 30 files and 26 lines, where the ruling counted 22; this brief follows the measured count. Each of the 30 files gains `#![cfg(test)]` as its first line, the harness modules as an inner attribute of their module, and the 26 #![allow] lines are removed. An integration root is only ever compiled as a test, so the marker changes nothing that is built; measured with clippy 0.1.97, a root or harness helper calling unwrap or expect is reported without the marker and passes with it. THE SYSTEM SHALL NOT rewrite any unwrap, expect or panic call in these files and SHALL NOT put any #[allow], #![allow] or #[expect] in place of a removed line.

**Acceptance:**
- `ls crates/*/tests/*.rs | wc -l` prints 28.
- `for f in crates/*/tests/*.rs crates/*/tests/harness/mod.rs; do head -1 "$f"; done | sort | uniq -c` prints exactly `  30 #![cfg(test)]`.
- `grep -l '#!\[allow' crates/*/tests/*.rs crates/*/tests/harness/*.rs` prints nothing.
- For each of these 30 files, `grep -oE '\.unwrap\(\)|\.expect\(|panic!\(' <file> | wc -l` at HEAD is not below the same count at origin/main.
- `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
- `cargo clippy --all-targets -- -D warnings` exits 0.

**Files:**
- modify: crates/lys-anchor-cli/tests/anchor_cli.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-anchor/tests/cascade.rs
- modify: crates/lys-anchor/tests/checkpoint_note_conformance.rs
- modify: crates/lys-anchor/tests/standalone_is_complete.rs
- modify: crates/lys-anchor/tests/stranger_verification.rs
- modify: crates/lys-core/tests/bundle_conformance.rs
- modify: crates/lys-core/tests/consistency_conformance.rs
- modify: crates/lys-core/tests/consistency_receipt_conformance.rs
- modify: crates/lys-core/tests/cose_conformance.rs
- modify: crates/lys-core/tests/delegation_conformance.rs
- modify: crates/lys-core/tests/delegation_vector.rs
- modify: crates/lys-core/tests/go_conformance.rs
- modify: crates/lys-core/tests/openssl_csr_interop.rs
- modify: crates/lys-core/tests/receipt_conformance.rs
- modify: crates/lys-core/tests/seal_derivation.rs
- modify: crates/lys-core/tests/signed_note_crosscheck.rs
- modify: crates/lys-home/tests/cached_index.rs
- modify: crates/lys-home/tests/claude_code_round_trip.rs
- modify: crates/lys-home/tests/fork.rs
- modify: crates/lys-home/tests/given_record.rs
- modify: crates/lys-home/tests/lantern_cli.rs
- modify: crates/lys-home/tests/lantern_home.rs
- modify: crates/lys-home/tests/launch_template.rs
- modify: crates/lys/tests/certified_attestation_tests.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys/tests/log_tests.rs
- modify: crates/lys-anchor/tests/harness/mod.rs
- modify: crates/lys-core/tests/harness/mod.rs

**Checklist:**
- C10 — The 28 integration test roots and both tests/harness/mod.rs files begin with #![cfg(test)] and carry no #![allow].

**Stories:**
- S5 (Lys contributor, Writing tests and library code) — As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.

### R5: Test from_env through a seam instead of the process environment, and correct the lib.rs comment

Ed25519Identity::from_env keeps its signature `pub fn from_env() -> TrustResult<Self>` and becomes one call: it reads LYS_IDENTITY_KEY with std::env::var and hands the read's Result to a private associated function, from_env_value, that holds today's trimming, decoding, length check and error texts unchanged. The five env-backed tests in identity_tests.rs call from_env_value with the value they used to put in the environment, and with Err(std::env::VarError::NotPresent) for the missing-variable case; they lose #[serial_test::serial], and the EnvCleanup guard with its two #[allow(unsafe_code)] is removed. identity_tests.rs is the only user of serial_test in the tree, so the dev-dependency goes with it: the line `serial_test.workspace = true` under [dev-dependencies] in crates/lys-core/Cargo.toml and the line `serial_test = "3"` under [workspace.dependencies] in Cargo.toml are removed, and Cargo.lock is regenerated by cargo so it no longer lists serial_test or serial_test_derive. WHEN from_env_value is given Ok of the standard base64 of 32 bytes, THE SYSTEM SHALL return the identity whose seed is those bytes; WHEN it is given an Err, THE SYSTEM SHALL return TrustError::KeyManagement saying LYS_IDENTITY_KEY is not set. The comment above `#![cfg_attr(not(test), forbid(unsafe_code))]` in crates/lys-core/src/lib.rs (lines 27-30) is rewritten to say what is true after this change: library code has no unsafe, non-test builds forbid it, test builds fall back to the workspace-level deny, and no test in the crate uses unsafe code. It names no person and no date. THE SYSTEM SHALL NOT change the attribute line, SHALL NOT change any code line of lib.rs other than lines 54 and 59, which R7 changes, SHALL NOT make from_env_value public, SHALL NOT change the three KeyManagement reason texts, and SHALL NOT mutate the process environment in any test, and SHALL NOT change any other dependency line in either Cargo.toml. This touches the key-loading path of a published crate, so the change is reviewed adversarially before landing, as the repository requires for cryptographic changes.

**Acceptance:**
- `grep -cE 'set_var|remove_var|unsafe|serial' crates/lys-core/src/keys/identity_tests.rs` prints 0.
- `grep -c 'pub fn from_env() -> TrustResult<Self>' crates/lys-core/src/keys/identity.rs` prints 1, and the body of from_env is the single expression `Self::from_env_value(std::env::var(KEY_ENV_VAR))`.
- from_env_value(Ok(STANDARD.encode([9u8; 32]))) returns an identity whose public_key_bytes() equals ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]).verifying_key().to_bytes().
- from_env_value(Ok(URL_SAFE_NO_PAD.encode([3u8; 32]))) returns an identity whose public_key_bytes() equals ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]).verifying_key().to_bytes().
- from_env_value(Err(std::env::VarError::NotPresent)) returns TrustError::KeyManagement whose message contains `LYS_IDENTITY_KEY` and `not set`.
- from_env_value(Ok("not-base64!!!@@".to_string())) returns TrustError::KeyManagement whose message contains `invalid base64`.
- from_env_value(Ok(STANDARD.encode([1u8; 16]))) returns TrustError::KeyManagement whose message contains `decoded to 16 bytes, expected 32`.
- `grep -cx '#!\[cfg_attr(not(test), forbid(unsafe_code))\]' crates/lys-core/src/lib.rs` prints 1, and `grep -cE 'set_var|#\[allow' crates/lys-core/src/lib.rs` prints 0.
- Every line `git diff -U0 origin/main...HEAD -- crates/lys-core/src/lib.rs` adds or removes begins with `//` after its leading whitespace, except the removal of original line 54 `use std::fmt::Write;`, the removal of original line 59 `let _ = s.write_fmt(format_args!("{b:02x}"));`, and the one line added in place of line 59.
- `grep -rc serial_test crates Cargo.toml | grep -vc ':0$'` prints 0.
- `grep -c 'name = "serial_test' Cargo.lock` prints 0.
- `cargo test -p lys-core --all-features` exits 0.

**Files:**
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/lib.rs
- modify: crates/lys-core/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C11 — The env-backed tests of Ed25519Identity::from_env call no set_var, remove_var or unsafe code, and from_env keeps its signature and its error texts.
- C12 — The comment above #![cfg_attr(not(test), forbid(unsafe_code))] in lys-core's lib.rs says what is true after the env tests change, and the attribute is byte-identical.

**Stories:**
- S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.
- S7 (Lys contributor, Writing tests and library code) — As a lys contributor changing key loading, I want the env-backed identity tests to run without unsafe code so that the test build needs no lint bypass.

### R6: Move the logic out of the two tests/harness/mod.rs files and lys-home's claude_code/mod.rs into named files

Structural. crates/lys-core/tests/harness/mod.rs keeps its marker, a short module doc and `mod go;` with the re-exports its roots use; find_go, go_or_skip, build_go_tool and run_built_tool move to tests/harness/go.rs, whose module doc opens with the phrase `Shared Go-toolchain harness` that lys-anchor's contract test reads. crates/lys-anchor/tests/harness/mod.rs keeps its marker, its module doc and the declarations of go, scaffold and scaffold_tests with the re-exports its roots use; find_go, go_or_skip, build_go_tool and run_built_tool move to go.rs; GO_ENV, GoScaffold, its impl, ALL_SCAFFOLDS and lys_core_harness move to scaffold.rs, and lys_core_harness names ../lys-core/tests/harness/go.rs; the two #[test] functions move to scaffold_tests.rs unchanged. crates/lys-home/src/harness/claude_code/mod.rs keeps its module doc and declarations; HARNESS, PROVIDER, API and AUTHORED move with their docs to names.rs and are re-exported from mod.rs, so crate::harness::claude_code::HARNESS and the other three resolve as before. THE SYSTEM SHALL NOT change what any moved item does, SHALL NOT loosen the contract test's clause checks, and SHALL NOT touch crates/lys-home/src/record/mod.rs.

**Acceptance:**
- `ast-grep scan --config sgconfig.yml --filter mod-rs-declarations-only --json=stream crates/lys-core/tests/harness/mod.rs crates/lys-anchor/tests/harness/mod.rs crates/lys-home/src/harness/claude_code/mod.rs | wc -l` prints 0.
- `git diff --name-only origin/main...HEAD -- crates/lys-home/src/cli.rs crates/lys-home/src/record/given.rs crates/lys-home/src/harness/claude_code/events.rs crates/lys-home/src/harness/claude_code/template.rs crates/lys-home/src/record/mod.rs` prints nothing.
- `cargo test -p lys-anchor --all-features the_go_environment_contract_matches_the_one_lys_core_wrote_down` passes in every test binary that declares `mod harness`.
- Changing `"GOPROXY", "off"` in crates/lys-core/tests/harness/go.rs to `"GOPROXY", "direct"` makes `cargo test -p lys-anchor --all-features` fail the_go_environment_contract_matches_the_one_lys_core_wrote_down in every binary that declares `mod harness`, with no other test failing; the change is reverted after.
- `cargo test --workspace --all-features` exits 0.

**Files:**
- create: crates/lys-core/tests/harness/go.rs
- create: crates/lys-anchor/tests/harness/go.rs
- create: crates/lys-anchor/tests/harness/scaffold.rs
- create: crates/lys-anchor/tests/harness/scaffold_tests.rs
- create: crates/lys-home/src/harness/claude_code/names.rs
- modify: crates/lys-core/tests/harness/mod.rs
- modify: crates/lys-anchor/tests/harness/mod.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C14 — The tests/harness/mod.rs files of lys-core and lys-anchor and lys-home's harness/claude_code/mod.rs hold only module declarations and re-exports.

**Stories:**
- S8 (Lys contributor, Writing tests and library code) — As a lys contributor reading a module tree, I want mod.rs files to hold only declarations and re-exports so that logic is found in a named file.

### R7: Fix the `let _ =` discards at their cause

The three hex writers (crates/lys/src/commands/hex.rs:15, crates/lys-anchor-cli/src/commands/hex.rs:15 and crates/lys-core/src/lib.rs:59) take a form that returns no Result: each byte pushes its two lowercase digits from a 16-character digit table, and the `use std::fmt::Write;` and the deliberate-discard comment go. `s.push_str(&format!(..))` is not the form, because clippy's format_push_string refuses it under the workspace's pedantic lints. The 14 test-code writes to a String handle their Result with `.expect("writing to a String cannot fail")`: anchor_receipt_conformance.rs:163, sealed_envelope_tests.rs:286, bundle_conformance.rs:673 and :692, consistency_conformance.rs:52, consistency_receipt_conformance.rs:80, delegation_conformance.rs:188, delegation_vector.rs:616, receipt_conformance.rs:48, signed_note_crosscheck.rs:55, given_tests.rs:29, given_record.rs:47, cli_tests.rs:52 and log_tests.rs:81. The two discards of values that are not Results are removed with what they held: identity_tests.rs:589 `let _ = id.public_key_bytes();` is deleted; json_output_tests.rs:375 `let _ = recipient_pub;` is deleted with the binding at line 92, and the key-generate call at line 87 stays as a statement, since json_ok asserts its success. In crates/lys-core/src/lib.rs the new form is the one line that replaces line 59, the deliberate-discard comment above it goes, and line 54's `use std::fmt::Write;`, which the new form leaves unused, is removed; no other code line of lib.rs changes. IF a hex writer is given the bytes 0x00, 0x0f, 0xab, 0xff, THEN THE SYSTEM SHALL return `000fabff`. Each writer's measurement is a new test named hex_lower_writes_000fabff asserting that hex_lower(&[0x00, 0x0f, 0xab, 0xff]) equals "000fabff": in crates/lys/src/commands/hex_tests.rs and crates/lys-anchor-cli/src/commands/hex_tests.rs, beside the existing 0xa5 vectors, which stay as they are; and, because lib.rs gains no test, in crates/lys-core/src/ca/authority_tests.rs, the sibling test file of ca/authority.rs, which calls hex_lower, reaching it through crate::hex_lower. THE SYSTEM SHALL NOT discard a Result, SHALL NOT use `_ =` in place of `let _ =`, and SHALL NOT add any #[allow].

**Acceptance:**
- `ast-grep scan --config sgconfig.yml --filter no-let-underscore-on-results --json=stream | wc -l` prints 0.
- `cargo test -p lys --all-features --bin lys commands::hex::tests::hex_lower_writes_000fabff -- --exact` prints `test commands::hex::tests::hex_lower_writes_000fabff ... ok` and exits 0.
- `cargo test -p lys-anchor-cli --all-features --lib commands::hex::tests::hex_lower_writes_000fabff -- --exact` prints `test commands::hex::tests::hex_lower_writes_000fabff ... ok` and exits 0.
- `cargo test -p lys-core --all-features --lib ca::authority::tests::hex_lower_writes_000fabff -- --exact` prints `test ca::authority::tests::hex_lower_writes_000fabff ... ok` and exits 0.
- `grep -c 'write_fmt\|std::fmt::Write' crates/lys/src/commands/hex.rs crates/lys-anchor-cli/src/commands/hex.rs crates/lys-core/src/lib.rs` prints 0 for each file.
- `grep -c 'recipient_pub' crates/lys/tests/json_output_tests.rs` prints 0.

**Files:**
- modify: crates/lys/src/commands/hex.rs
- modify: crates/lys-anchor-cli/src/commands/hex.rs
- modify: crates/lys-core/src/lib.rs
- modify: crates/lys/src/commands/hex_tests.rs
- modify: crates/lys-anchor-cli/src/commands/hex_tests.rs
- modify: crates/lys-core/src/ca/authority_tests.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-core/src/seal/sealed_envelope_tests.rs
- modify: crates/lys-core/tests/bundle_conformance.rs
- modify: crates/lys-core/tests/consistency_conformance.rs
- modify: crates/lys-core/tests/consistency_receipt_conformance.rs
- modify: crates/lys-core/tests/delegation_conformance.rs
- modify: crates/lys-core/tests/delegation_vector.rs
- modify: crates/lys-core/tests/receipt_conformance.rs
- modify: crates/lys-core/tests/signed_note_crosscheck.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/tests/given_record.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/log_tests.rs

**Checklist:**
- C15 — The `let _ =` hits in the three hex writers, the 14 test-code writes, identity_tests.rs and json_output_tests.rs are gone.

**Stories:**
- S9 (Lys contributor, Writing tests and library code) — As a lys contributor, I want no `let _ =` discard in the tree so that no error is swallowed without a decision.

### R8: Correct CLAUDE.md and the Cargo.toml lint comment to name clippy.toml and the ast-grep leg

Structural. The coding-standards sentence at CLAUDE.md:35 that tells tests to opt out per module with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` is replaced by one naming clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests and the #![cfg(test)] first line that marks a file as test code. The comment at Cargo.toml:69-70 is corrected the same way. CLAUDE.md's `Gates before any commit` block gains the line `ast-grep scan --config sgconfig.yml` after `cargo doc --no-deps`, and its count sentence `All five clean.` becomes `All seven clean.`, matching the seven lines the block then lists. THE SYSTEM SHALL NOT change any other rule CLAUDE.md states and SHALL NOT change any line inside Cargo.toml's [workspace.lints] tables other than those comment lines; the serial_test removal from [workspace.dependencies] is R5's.

**Acceptance:**
- `grep -c 'Tests opt out per-module' CLAUDE.md` prints 0, and `grep -c 'clippy.toml' CLAUDE.md` prints at least 1.
- `grep -c '#!\[allow' Cargo.toml` prints 0, and `grep -c 'clippy.toml' Cargo.toml` prints 1.
- `sed -n '/^## Gates before any commit/,/^All /p' CLAUDE.md | grep -c '^ast-grep scan --config sgconfig.yml$'` prints 1, and the block's last line is `All seven clean. No exceptions.`
- `diff <(git show origin/main:Cargo.toml | sed -n '/^\[workspace\.lints/,$p' | grep -v '^#') <(sed -n '/^\[workspace\.lints/,$p' Cargo.toml | grep -v '^#')` prints nothing, over the [workspace.lints.rust] and [workspace.lints.clippy] tables that end the file.

**Files:**
- modify: CLAUDE.md
- modify: Cargo.toml

**Checklist:**
- C16 — CLAUDE.md's coding-standards sentence and the Cargo.toml lint comment name clippy.toml's allow-*-in-tests in place of the per-module #![allow].
- C17 — CLAUDE.md's gates block lists `ast-grep scan --config sgconfig.yml` and its count sentence matches the lines it lists.

**Stories:**
- S6 (Lys contributor, Writing tests and library code) — As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.

### R9: Wire `ast-grep scan --config sgconfig.yml` into project.json, .land/gates.sh and CI, and land it only at zero hits

docs/design/project.json gains, after the design leg, the leg named `ast-grep` with command `ast-grep scan --config sgconfig.yml`, requires ["tool:ast-grep"] and cadence round. .land/gates.sh gains the line `leg ast-grep scan --config sgconfig.yml` after its last cargo doc leg. .github/workflows/ci.yml's test job gains a step running `cargo install ast-grep --version 0.44.1 --locked` and a step running `ast-grep scan --config sgconfig.yml`. WHEN any rule in rules/ast-grep reports a hit, THE SYSTEM SHALL fail the ast-grep leg in each of the three places. WHILE origin/main's crates/lys-home/src/record/mod.rs has any non-module line, THE SYSTEM SHALL NOT land this leg; the count is taken by `git show origin/main:crates/lys-home/src/record/mod.rs | ast-grep scan --rule rules/ast-grep/mod-rs-declarations-only.yml --stdin --json=stream | python3 -c "import sys,json; print(len({n for h in map(json.loads,sys.stdin) for n in range(h['range']['start']['line'],h['range']['end']['line']+1)}))"`, which prints 521 at 7b53625 and must print 0. THE SYSTEM SHALL NOT change the seven existing legs or their commands, SHALL NOT pass a path to the scan, and SHALL NOT land any scratch file.

**Acceptance:**
- `python3 scripts/design/validate.py docs/design/project.json` exits 0, and the file's last leg is {"name": "ast-grep", "command": "ast-grep scan --config sgconfig.yml", "requires": ["tool:ast-grep"], "cadence": "round"}.
- `git diff origin/main...HEAD -- docs/design/project.json .land/gates.sh | grep '^-[^-]'` prints nothing.
- `git diff --name-only origin/main...HEAD -- docs/design | grep -v '^docs/design/lys-core/'` prints exactly `docs/design/project.json`.
- `grep -cx 'leg ast-grep scan --config sgconfig.yml' .land/gates.sh` prints 1.
- `grep -c 'cargo install ast-grep --version 0.44.1 --locked' .github/workflows/ci.yml` prints 1, and `grep -c 'run: ast-grep scan --config sgconfig.yml' .github/workflows/ci.yml` prints 1.
- The record/mod.rs count command above prints 0 against origin/main before the leg lands.
- `ast-grep scan --config sgconfig.yml` from the repository root prints no hit and exits 0.
- With a scratch file crates/lys-core/src/scratch_red.rs whose one line is `pub fn scratch(x: Option<u8>) -> u8 { x.unwrap() }`, `sh .land/gates.sh` exits 1, prints `--- status 1: ast-grep scan --config sgconfig.yml ---`, and prints `--- status 0:` for each of its other seven legs; the file is deleted after and is in no commit.
- `grep -rnE '#!?\[(allow|expect)\(|#\[ignore' crates --include='*.rs'` prints nothing.

**Files:**
- modify: docs/design/project.json
- modify: .land/gates.sh
- modify: .github/workflows/ci.yml

**Checklist:**
- C13 — grep finds zero #[allow], #![allow], #[expect] and #[ignore] under crates/.
- C18 — docs/design/project.json has a leg running `ast-grep scan --config sgconfig.yml` requiring tool:ast-grep, and validate.py accepts the file.
- C19 — .land/gates.sh runs `leg ast-grep scan --config sgconfig.yml`.
- C20 — .github/workflows/ci.yml installs ast-grep 0.44.1 and runs `ast-grep scan --config sgconfig.yml`.
- C21 — `ast-grep scan --config sgconfig.yml` reports zero hits and exits 0 at the landed commit.
- C22 — A never-landed scratch file with an unwrap and no #![cfg(test)] makes .land/gates.sh show the ast-grep leg red and every other leg green.
- C23 — The leg lands only after origin/main's crates/lys-home/src/record/mod.rs has zero non-module lines, checked by command.

**Stories:**
- S1 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.
- S2 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.
- S3 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want the same scan run by the design gate, the landing gate and CI so that no path to main skips it.
- S4 (Lys maintainer, Landing a card through the gate) — As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.

## Boundaries

- No unwrap, expect or panic call in a fixture, a *_tests.rs file or a file under tests/ is rewritten.
- No rule, config or script names a file or a list of file names to exempt; test code is recognised by structure only.
- No #[allow], #![allow], #[expect] or #[ignore] of any kind is added, in tests or in library code; an #[allow] that cannot be fixed at its cause goes back to the lead as a question with its line.
- #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs stays byte-identical, and no code line of lib.rs changes other than lines 54 and 59.
- crates/lys-home/src/record/mod.rs is not changed.
- Nothing is written under any docs/design directory other than lys-core, except the one ast-grep leg in docs/design/project.json.
- No wire format, domain-separation tag, public API signature or public behaviour of any crate changes.
- vendor/rauthy is neither scanned nor edited, and no file under vendor/ is committed.
- Cambium's rules and config are not edited.
- The seven existing gate legs and their commands are unchanged.
- No scratch file is committed.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps; each exits 0.
- From the repository root: `ast-grep scan --config sgconfig.yml` prints no hit and exits 0.
- From the repository root: `sh scripts/design/gate.sh` exits 0.
- From the repository root: `sh .land/gates.sh` exits 0 with eight `--- status 0:` lines, then again with the R9 scratch file, exiting 1 with the ast-grep leg the only `--- status 1:` line; the scratch file is deleted after.
- `grep -rnE '#!?\[(allow|expect)\(|#\[ignore' crates --include='*.rs'` prints nothing.
- An adversarial review of the crates/lys-core/src/keys/identity.rs diff shows the seed still decoded into Zeroizing buffers, the three KeyManagement reason texts unchanged, and no key material in any error or Debug output.
- `git status --porcelain` after the scratch cases prints nothing that is not part of the card's diff.
