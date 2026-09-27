---
type: brief
id: LYSCORE-003
cluster: lys-core
title: Give lys the ast-grep rule set and a gate leg that fails on a hit
---

# LYSCORE-003: Give lys the ast-grep rule set and a gate leg that fails on a hit

> **Cluster:** lys-core
> **Blocked by:** HOME-013's record/mod.rs split landed on main. This card is built on origin/hand/lys-stack at 4dd1c33, which carries it: `git fetch origin main && git show origin/main:crates/lys-home/src/record/mod.rs | ast-grep scan --inline-rules '{id: mod-rs-logic, language: rust, severity: error, rule: {any: [{kind: function_item}, {kind: struct_item}, {kind: enum_item}, {kind: trait_item}, {kind: impl_item}, {kind: const_item}, {kind: static_item}]}}' --stdin --json=stream | wc -l` prints 0 (it prints 47 at 7b53625 and 0 on 4dd1c33)., A configured gate venue declares tool:ast-grep; a leg no venue satisfies is unmeasured by name and stops the round., Sign-off of this brief before it is dispatched.
> **Design anchor:**
> - ADR-063 — lys enforces its code rules with ast-grep, recognising test code by structure the file carries — Carry Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and add lys's own no-unwrap-expect-panic-outside-tests. That rule treats as test code a file whose first item is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/. Every *_tests.rs file, both fixture.rs files, every integration test root and both tests/harness/mod.rs files gain #![cfg(test)] as their first line. The per-module #![allow] opt-outs are replaced by clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests. Every existing hit is fixed at its cause in the same card, so the leg is green when it lands. The leg `ast-grep scan --config sgconfig.yml` runs in docs/design/project.json, .land/gates.sh and CI. Rejected: an exemption list of file names; rewriting the calls in test files; narrowing no-lint-bypass-attributes to spare the test opt-outs; carrying only the rules that were clean at the time.
> - ADR-064 — A rule that cannot fire on the lys tree is not carried — no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are recorded as not carried, with that finding. The card that lands the first async code in lys carries no-std-mutex-in-async with it, and its brief names this decision. Rejected: carrying all six of Cambium's rules because the words said the same rule set.
> - ADR-065 — The hand-written lys-core documents are kept as *-PRE-METHOD.md, not replaced by the method's render — Before this cluster is rendered, the three documents are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the method's render takes the old names. The live citations of the old paths are re-pointed to the new ones. LYSCORE-003's card is the one owner of docs/design/lys-core. Rejected: overwriting the documents with the render, and deleting them.
> **Checklist:**
> - C85 — docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the two live citations name the new paths.
> - C86 — The lys-core cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-003.md are rendered from its JSON, and sh scripts/design/gate.sh exits 0.
> - C87 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and CLAUDE.md and Cargo.toml name it in place of the per-module #![allow].
> - C88 — Every *_tests.rs file, both fixture.rs files, every integration test root and the two tests/harness/mod.rs files begin with #![cfg(test)], and no #![allow] line remains in any of them.
> - C89 — crates/lys-core/src/keys/identity_tests.rs holds no #[allow(unsafe_code)] and no unsafe block, and the comment above lib.rs's unsafe_code attribute says what is true after that.
> - C90 — No `let _ =` statement remains under crates/.
> - C91 — crates/lys-core/tests/harness/mod.rs, crates/lys-anchor/tests/harness/mod.rs and crates/lys-home/src/harness/claude_code/mod.rs hold no fn, struct, enum, trait, impl, const or static item.
> - C92 — sgconfig.yml names rules/ast-grep, which holds mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes and no-unwrap-expect-panic-outside-tests, each at severity error.
> - C93 — The leg `ast-grep scan --config sgconfig.yml` is in docs/design/project.json requiring tool:ast-grep, in .land/gates.sh and in CI.
> - C94 — The scan reports zero hits over the landed tree, and exactly one hit on an uncommitted scratch file holding an unwrap outside test code.
> **Stories:**
> - S30 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an unwrap, a lint bypass, a dropped Result or logic in a mod.rs so that a rule break is caught before review rather than by a reviewer's memory.
> - S31 (Test writer, Writing tests in sibling files) — As a test writer, I want a test file recognised as test code by a marker it carries so that I can unwrap in test helpers without an #[allow].
> - S32 (Lead, Trusting the gate) — As the lead for lys, I want proof that the unwrap rule fires so that a green leg means the rule held and not that nothing was measured.
> - S33 (Reader of the design, Tracing the earlier lys-core design) — As a reader of the lys-core design, I want the hand-written pre-method documents kept under their own names so that the earlier design and the citations of it survive the method's render.

## Purpose

lys's rules against unwrap, expect and panic outside tests, against lint bypasses, against dropped Results and against logic in mod.rs are enforced today only where clippy overlaps them. This brief adds sgconfig.yml, four ast-grep rules and one leg, `ast-grep scan --config sgconfig.yml`, which runs in docs/design/project.json, .land/gates.sh and CI and fails the gate on any hit. The rules are Cambium's mod-rs-declarations-only, no-let-underscore-on-results and no-lint-bypass-attributes, and lys's own no-unwrap-expect-panic-outside-tests. The brief clears every existing hit at its cause in the same card, so the leg is green when it lands (ADR-063). It records the rules that are not carried (ADR-064). It keeps the hand-written lys-core documents as *-PRE-METHOD.md before rendering this cluster (ADR-065).

## Task

Bring the lys tree to zero hits under four ast-grep rules and add the scan to every gate, in the order R1 to R9. R1 and R2 rename the hand-written lys-core documents and then render this cluster. R3 to R7 clear the existing hits at their cause: clippy.toml, the #![cfg(test)] markers, the env-mutation tests and the serial_test dependency they alone used, the let-underscore statements and the three mod.rs files. R8 adds the rules, and R9 adds the leg and proves it fails on a hit. Every path is relative to the repository root, and every count is measured at 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa (4dd1c33), the head of origin/hand/lys-stack this card is built on.

Corrections to the words, recorded here as the lead ruled. (1) scripts/design/gate.sh does not run the gate's legs. It validates docs/design/project.json and the decision ledger and re-renders every cluster. The legs are run by .land/gates.sh, which repo_land runs as the whole gate, and by CI, so the leg goes into all three (R9). (2) Cambium carries no rule against _name renames, so the same rule set does not include one, and this card does not catch them. A _name-rename rule is not carried. `git grep -hoE '(let (mut )?|\(|, |\|)_[a-z][a-z0-9_]*( *[:=|,)])' 4dd1c33 -- 'crates/*.rs' 'tests/*.rs' | wc -l` prints 306, many of them TempDir keep-alive guards, unused trait-impl parameters and ignored closure arguments. A card of its own writes the rule and handles each binding by its act: a keep-alive guard is held by a real name and dropped by name, and an unused trait parameter is used or the trait is changed. That card is filed on this board from this finding. (3) Cambium carries no unwrap/expect/panic rule either. no-unwrap-expect-panic-outside-tests is lys's own, written to the lead's ruling. #[ignore] is caught by no-lint-bypass-attributes, with 0 hits. (4) The unwrap rule reports 22 calls in the two fixture.rs files and 202 in helper fns of 47 *_tests.rs files. None is in library code. Every one stops being a hit once its file carries #![cfg(test)] (R4), and no call is rewritten. (5) Every integration test root (31 files under crates/*/tests and tests/*/tests) and the two tests/harness/mod.rs files gain #![cfg(test)] as their first line; the 22 of them that carry the opt-out lose it. An integration test root is only ever compiled as a test, so the attribute changes nothing about what is built; the two harness files take it as an inner attribute of their module. No #[allow] of any kind replaces a removed line.

Not carried, with the finding. no-std-mutex-in-async, no-timer-in-door-handlers and no-timer-import-in-door-handlers are not carried (ADR-064). The 333 Rust files under crates/ and tests/ hold 0 async fn, 0 mentions of tokio and 0 std::sync::Mutex or RwLock, so no-std-mutex-in-async could not fire. The two door-timer rules are limited by their files key to Cambium door-handler paths that lys does not have. The act that brings no-std-mutex-in-async back is the card that lands the first async code in lys: that card carries the rule with it, and its brief names ADR-064.

Build notes. Heavy legs (both clippies, the test run, both cargo docs and .land/gates.sh) run on the gate venue. The single-file ast-grep scans can run anywhere. The scratch file in R9 is created, measured and deleted, and never committed. R5 touches key-loading code without changing what from_env accepts or rejects: the reviewer confirms that from_env_value's error for Err(VarError::NotPresent) equals from_env's error for an unset variable. The coverage split: C93 and C94 are both delivered by R9.

## Requirements

### R1: Rename the hand-written lys-core documents to *-PRE-METHOD.md

Before the cluster is rendered, THE SYSTEM SHALL rename, with git mv and their bytes unchanged, docs/design/lys-core/DESIGN.md to docs/design/lys-core/DESIGN-PRE-METHOD.md, docs/design/lys-core/CHECKLIST.md to docs/design/lys-core/CHECKLIST-PRE-METHOD.md and docs/design/lys-core/USER-STORIES.md to docs/design/lys-core/USER-STORIES-PRE-METHOD.md. THE SYSTEM SHALL change the two live citations of the old paths to name the new ones: crates/lys-core/tests/seal_derivation.rs lines 7-8 (DESIGN.md for D6, CHECKLIST.md for C45) and docs/PEN-REGISTRATION.md line 50 (DESIGN.md and CHECKLIST.md). THE SYSTEM SHALL NOT edit, reflow or delete any content of the three documents. It SHALL NOT change any other line of seal_derivation.rs or PEN-REGISTRATION.md. It SHALL NOT change the citations in docs/REVIEW-23-07.md and docs/design/WIRE-FORMATS.md, which record what those paths held when they were written.

**Acceptance:**
- `shasum -a 256 docs/design/lys-core/DESIGN-PRE-METHOD.md docs/design/lys-core/CHECKLIST-PRE-METHOD.md docs/design/lys-core/USER-STORIES-PRE-METHOD.md` prints e3d14cce3009b705c5af33e95cf7100e011b9c919b18c630035e1fa697c317ac, dafef8f1ac4657bcc03953c7a214fb1d9f8317e26271c84802742d29d91d4148 and 246bace1db3a8752f8141b7b25ba65462be40a1d08c13f33a3fd4abf1d2ad7e2, the hashes of `git show 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa:docs/design/lys-core/DESIGN.md`, `...:CHECKLIST.md` and `...:USER-STORIES.md`.
- `git grep -nE 'lys-core/(DESIGN|CHECKLIST)\.md' -- crates/lys-core/tests/seal_derivation.rs docs/PEN-REGISTRATION.md` prints nothing, and `git grep -oE 'lys-core/[A-Z-]+-PRE-METHOD\.md' -- crates/lys-core/tests/seal_derivation.rs docs/PEN-REGISTRATION.md` prints four matches: DESIGN-PRE-METHOD.md and CHECKLIST-PRE-METHOD.md once in each file.
- `git diff 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa -U0 -- crates/lys-core/tests/seal_derivation.rs` changes only lines 7-8 for the two citations, adds line 1 `#![cfg(test)]` (R4), removes the opt-out line (R4) and the blank line cargo fmt collapses after it. `git diff --numstat 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa -- docs/PEN-REGISTRATION.md` prints `1\t1\tdocs/PEN-REGISTRATION.md`.

**Files:**
- create: docs/design/lys-core/DESIGN-PRE-METHOD.md
- create: docs/design/lys-core/CHECKLIST-PRE-METHOD.md
- create: docs/design/lys-core/USER-STORIES-PRE-METHOD.md
- modify: crates/lys-core/tests/seal_derivation.rs
- modify: docs/PEN-REGISTRATION.md
- delete: docs/design/lys-core/DESIGN.md
- delete: docs/design/lys-core/CHECKLIST.md
- delete: docs/design/lys-core/USER-STORIES.md

**Checklist:**
- C85 — docs/design/lys-core/DESIGN.md, CHECKLIST.md and USER-STORIES.md are renamed to DESIGN-PRE-METHOD.md, CHECKLIST-PRE-METHOD.md and USER-STORIES-PRE-METHOD.md with their bytes unchanged, and the two live citations name the new paths.

**Stories:**
- S33 (Reader of the design, Tracing the earlier lys-core design) — As a reader of the lys-core design, I want the hand-written pre-method documents kept under their own names so that the earlier design and the citations of it survive the method's render.

### R2: Render the lys-core cluster from its JSON

WHEN the renames of R1 are made, THE SYSTEM SHALL run `python3 scripts/design/render-cluster.py docs/design/lys-core` and commit the rendered docs/design/lys-core/DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-003.md beside their JSON. The first three paths are free after R1 renames the hand-written files away. THE SYSTEM SHALL NOT render before the renames, because the render would overwrite the hand-written documents. It SHALL NOT hand-edit rendered markdown, and SHALL NOT write under docs/design/lys-core anything other than these, R1's renames and the cluster's own JSON.

**Acceptance:**
- `sh scripts/design/gate.sh` exits 0, which includes the byte comparison of a fresh render of docs/design/lys-core against the committed markdown.
- `python3 scripts/design/check-coverage.py docs/design/lys-core` exits 0.
- `grep -c '^# Lys Core — the ast-grep rule set and its gate leg$' docs/design/lys-core/DESIGN.md` prints 1 and the same command over docs/design/lys-core/DESIGN-PRE-METHOD.md prints 0.

**Files:**
- create: docs/design/lys-core/DESIGN.md
- create: docs/design/lys-core/CHECKLIST.md
- create: docs/design/lys-core/USER-STORIES.md
- create: docs/design/lys-core/briefs/LYSCORE-003.md

**Checklist:**
- C86 — The lys-core cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSCORE-003.md are rendered from its JSON, and sh scripts/design/gate.sh exits 0.

**Stories:**
- S33 (Reader of the design, Tracing the earlier lys-core design) — As a reader of the lys-core design, I want the hand-written pre-method documents kept under their own names so that the earlier design and the citations of it survive the method's render.

### R3: Replace the per-module test opt-out with clippy.toml

A file clippy.toml at the repository root holds exactly three settings: `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true` and `allow-panic-in-tests = true`. In CLAUDE.md's coding standards, the sentence `Tests opt out per-module with #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)].` is replaced by one saying that tests are exempt through clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests, which clippy applies to code it knows is test code, and that a *_tests.rs file, a fixture file and an integration test root carry #![cfg(test)] as their first line. In Cargo.toml, the comment above `unwrap_used = "warn"` in [workspace.lints.clippy] is changed in the same way. No other line of CLAUDE.md changes, and no other line of Cargo.toml changes in this requirement; the one other Cargo.toml line that changes is `serial_test = "3"`, which R5 removes. clippy.toml SHALL NOT set any other key, and no lint level in Cargo.toml SHALL change.

**Acceptance:**
- `grep -vc '^[[:space:]]*$' clippy.toml` prints 3, and `grep -c '= true$' clippy.toml` prints 3, one line for each of allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests.
- `grep -c 'allow(clippy::unwrap_used' CLAUDE.md Cargo.toml` prints `CLAUDE.md:0` and `Cargo.toml:0`, and `grep -c 'clippy.toml' CLAUDE.md Cargo.toml` prints `CLAUDE.md:1` and `Cargo.toml:1`.
- Every line that `git diff -U0 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa -- Cargo.toml` adds or removes begins with `+#` or `-#`, except the one removed line `-serial_test = "3"` (R5).

**Files:**
- create: clippy.toml
- modify: CLAUDE.md
- modify: Cargo.toml

**Checklist:**
- C87 — clippy.toml sets allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests to true, and CLAUDE.md and Cargo.toml name it in place of the per-module #![allow].

**Stories:**
- S31 (Test writer, Writing tests in sibling files) — As a test writer, I want a test file recognised as test code by a marker it carries so that I can unwrap in test helpers without an #[allow].

### R4: Mark every test file with #![cfg(test)] and remove its #![allow] opt-out

THE SYSTEM SHALL make `#![cfg(test)]` line 1 of each of the 139 files in files.modify: the 104 *_tests.rs files under crates/ (the sibling test files under src/ and the four integration test roots under crates/lys/tests named *_tests.rs), the two fixture.rs files under crates/lys-anchor/src, the 31 other integration test roots under crates/*/tests and tests/*/tests, and the two tests/harness/mod.rs files, where the line is an inner attribute of the harness module. It SHALL remove from each of them every line equal to `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. That line is present in 106 of the 139 files: 82 of the *_tests.rs files, both fixtures and 22 of the files under crates/*/tests not named *_tests.rs. Where it is line 1 it is replaced by the marker. Where it is a later line, it is deleted and the marker goes above the module docs. Any other inner attribute, such as `#![cfg(feature = "unstable-anchor")]`, stays where it is. THE SYSTEM SHALL NOT change any other line of these files in this requirement, other than the blank lines cargo fmt collapses, and SHALL NOT rewrite, move or remove any unwrap, expect or panic call in them. It SHALL NOT put any #[allow] or #[expect] of any kind in place of a removed line, and SHALL NOT add the marker to a module file that is neither a *_tests.rs file, a fixture, a test root nor a harness mod.rs.

**Acceptance:**
- `git grep -lF '#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]' 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa -- crates | wc -l` prints 106, and on the landed tree `git grep -nF '#![allow(' -- crates tests` prints nothing and exits 1.
- `for f in $(git ls-files 'crates/**/*_tests.rs' 'crates/**/fixture.rs' 'crates/*/tests/harness/mod.rs') $(git ls-files 'crates/*/tests/*.rs' 'tests/*/tests/*.rs' | grep -vE '/tests/[^/]+/'); do head -1 "$f"; done | sort | uniq -c` prints the single line `    139 #![cfg(test)]`.
- No unwrap, expect or panic call is removed from the 139 files except by R5's five from_env tests: `git grep -hoE '\.unwrap\(\)|\.expect\(|panic!\(' <rev> -- 'crates/**/*_tests.rs' 'crates/**/fixture.rs' ':!crates/lys-core/src/keys/identity_tests.rs' | wc -l` prints the same number with <rev> as 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa and as the landed HEAD, apart from the `.expect(` calls R6 adds to test String writes, which the reviewer counts from the R6 diff.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo clippy --workspace --all-targets -- -D warnings` each exit 0.

**Files:**
- modify: crates/lys-anchor-cli/src/cli_tests.rs
- modify: crates/lys-anchor-cli/src/commands/anchor/open_tests.rs
- modify: crates/lys-anchor-cli/src/commands/anchor/policy_tests.rs
- modify: crates/lys-anchor-cli/src/commands/error_tests.rs
- modify: crates/lys-anchor-cli/src/commands/hex_tests.rs
- modify: crates/lys-anchor-cli/src/commands/output_tests.rs
- modify: crates/lys-anchor-cli/tests/anchor_cli.rs
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
- modify: crates/lys-anchor/src/upward/fixture.rs
- modify: crates/lys-anchor/src/upward/pin_tests.rs
- modify: crates/lys-anchor/src/witness/fixture.rs
- modify: crates/lys-anchor/src/witness/observe_tests.rs
- modify: crates/lys-anchor/src/witness/projection_tests.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-anchor/tests/cascade.rs
- modify: crates/lys-anchor/tests/checkpoint_note_conformance.rs
- modify: crates/lys-anchor/tests/harness/mod.rs
- modify: crates/lys-anchor/tests/standalone_is_complete.rs
- modify: crates/lys-anchor/tests/stranger_verification.rs
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
- modify: crates/lys-core/tests/bundle_conformance.rs
- modify: crates/lys-core/tests/consistency_conformance.rs
- modify: crates/lys-core/tests/consistency_receipt_conformance.rs
- modify: crates/lys-core/tests/cose_conformance.rs
- modify: crates/lys-core/tests/delegation_conformance.rs
- modify: crates/lys-core/tests/delegation_vector.rs
- modify: crates/lys-core/tests/go_conformance.rs
- modify: crates/lys-core/tests/harness/mod.rs
- modify: crates/lys-core/tests/openssl_csr_interop.rs
- modify: crates/lys-core/tests/receipt_conformance.rs
- modify: crates/lys-core/tests/seal_derivation.rs
- modify: crates/lys-core/tests/signed_note_crosscheck.rs
- modify: crates/lys-home/src/cli_translate_tests.rs
- modify: crates/lys-home/src/harness/claude_code/events_tests.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/src/harness/claude_code/import_tests.rs
- modify: crates/lys-home/src/harness/claude_code/paths_tests.rs
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs
- modify: crates/lys-home/src/harness/claude_code/seed_tests.rs
- modify: crates/lys-home/src/harness/claude_code/template_tests.rs
- modify: crates/lys-home/src/harness/codex/account_tests.rs
- modify: crates/lys-home/src/harness/codex/beside_tests.rs
- modify: crates/lys-home/src/harness/codex/leaf_tests.rs
- modify: crates/lys-home/src/harness/codex/parts_tests.rs
- modify: crates/lys-home/src/harness/codex/rollout_tests.rs
- modify: crates/lys-home/src/harness/codex/zone_tests.rs
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
- modify: crates/lys-home/tests/cached_index.rs
- modify: crates/lys-home/tests/claude_code_round_trip.rs
- modify: crates/lys-home/tests/codex_translation.rs
- modify: crates/lys-home/tests/fork.rs
- modify: crates/lys-home/tests/given_record.rs
- modify: crates/lys-home/tests/lantern_cli.rs
- modify: crates/lys-home/tests/lantern_home.rs
- modify: crates/lys-home/tests/launch_template.rs
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
- modify: crates/lys/src/identity/config_tests.rs
- modify: crates/lys/src/identity/credentials_tests.rs
- modify: crates/lys/src/identity/error_tests.rs
- modify: crates/lys/src/identity/prepare_tests.rs
- modify: crates/lys/src/identity/themes_tests.rs
- modify: crates/lys/tests/certified_attestation_tests.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/identity_deploy.rs
- modify: crates/lys/tests/identity_refusals.rs
- modify: crates/lys/tests/identity_restart.rs
- modify: crates/lys/tests/identity_shared_db.rs
- modify: crates/lys/tests/identity_theme.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys/tests/log_tests.rs
- modify: tests/identity_contract/tests/events.rs

**Checklist:**
- C88 — Every *_tests.rs file, both fixture.rs files, every integration test root and the two tests/harness/mod.rs files begin with #![cfg(test)], and no #![allow] line remains in any of them.

**Stories:**
- S31 (Test writer, Writing tests in sibling files) — As a test writer, I want a test file recognised as test code by a marker it carries so that I can unwrap in test helpers without an #[allow].

### R5: Remove the seven #[allow(unsafe_code)] lines by ending the env mutation that needed them, and correct the lib.rs comment

THE SYSTEM SHALL move the body of Ed25519Identity::from_env in crates/lys-core/src/keys/identity.rs into a private associated fn `from_env_value(read: Result<String, std::env::VarError>) -> TrustResult<Self>`, and from_env SHALL become the one call `Self::from_env_value(std::env::var(KEY_ENV_VAR))`. WHEN read is an Err, from_env_value SHALL return the TrustError::KeyManagement that from_env returns today for an unset variable. The trimming, both base64 decodings, the 32-byte length check, the Zeroizing of the decoded bytes and every error message SHALL be unchanged. The five from_env tests in crates/lys-core/src/keys/identity_tests.rs SHALL call from_env_value with Ok of the value each sets today, or Err(std::env::VarError::NotPresent) for the unset case, and SHALL keep every assertion. THE SYSTEM SHALL remove EnvCleanup and its Drop impl, every unsafe block, the seven #[allow(unsafe_code)] lines and the #[serial_test::serial] attribute of those five tests. The doc line of TEST_ENV_VAR SHALL say it is the name the error messages are checked against. With its last use gone, THE SYSTEM SHALL remove the serial_test dev-dependency: the line `serial_test.workspace = true` from crates/lys-core/Cargo.toml, the line `serial_test = "3"` from the root Cargo.toml's workspace dependencies, and the packages Cargo.lock then no longer resolves. THE SYSTEM SHALL rewrite the comment lines above `#![cfg_attr(not(test), forbid(unsafe_code))]` in crates/lys-core/src/lib.rs so they say what is true after this requirement: library code has no unsafe, no test in the crate uses unsafe code, non-test builds forbid it, and test builds are held by the workspace-level deny. The comment names no person and no date. THE SYSTEM SHALL NOT change the attribute line or any other line of lib.rs in this requirement. It SHALL NOT change from_env's signature, documentation or accepted inputs. It SHALL NOT write the process environment anywhere in lys-core, and SHALL NOT make from_env_value visible outside identity.rs and its test module.

**Acceptance:**
- `git grep -nE 'unsafe \{|allow\(unsafe_code\)|set_var|remove_var|EnvCleanup' -- crates/lys-core` prints nothing.
- `git grep -n serial_test -- Cargo.toml crates` prints nothing, and `grep -c 'name = "serial_test"' Cargo.lock` prints 0 (it prints 1 at 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa).
- `cargo test -p lys-core --lib keys::identity::tests::from_env` reports `5 passed; 0 failed`.
- `grep -c 'pub fn from_env() -> TrustResult<Self>' crates/lys-core/src/keys/identity.rs` prints 1, and `grep -c 'fn from_env_value(read: Result<String, std::env::VarError>) -> TrustResult<Self>' crates/lys-core/src/keys/identity.rs` prints 1, with no `pub` before it.
- The R5 commit's diff to crates/lys-core/src/lib.rs touches comment lines only: every line it adds or removes begins with `+//` or `-//`, and `grep -c '^#!\[cfg_attr(not(test), forbid(unsafe_code))\]$' crates/lys-core/src/lib.rs` prints 1.

**Files:**
- modify: crates/lys-core/src/keys/identity.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/lib.rs
- modify: crates/lys-core/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C89 — crates/lys-core/src/keys/identity_tests.rs holds no #[allow(unsafe_code)] and no unsafe block, and the comment above lib.rs's unsafe_code attribute says what is true after that.

### R6: Fix every let _ = at its cause

THE SYSTEM SHALL remove every `let _ = <expr>;` statement under crates/. There are 19 at 4dd1c33, in the 18 files of files.modify, and each is fixed at its cause. (a) hex_lower in crates/lys-core/src/lib.rs, crates/lys/src/commands/hex.rs and crates/lys-anchor-cli/src/commands/hex.rs builds its string from a 16-character digit table, pushing two chars per byte, a form that returns no Result; the `use std::fmt::Write;` and the deliberate-discard comment go, and the output is unchanged. `s.push_str(&format!(..))` is not the form, because clippy's format_push_string refuses it under the workspace's pedantic lints. (b) The 14 writes into a String in test code (write!, writeln! and write_fmt, in the other files listed except the two below) handle the fmt::Result they dropped with `.expect("writing to a String cannot fail")`, and a comment beside one that said the Result is discarded says instead that the expect never fires. (c) crates/lys-core/src/keys/identity_tests.rs, which discards `id.public_key_bytes()` after load_or_generate creates a key file, instead asserts that Ed25519Identity::load of that file returns the same public key. (d) crates/lys/tests/json_output_tests.rs, which discards recipient_pub, asserts `recipient_pub.len()` is 64, with a message naming public_key_ed25519. THE SYSTEM SHALL NOT replace any of them with a named or _-prefixed binding, SHALL NOT add an #[allow], and SHALL NOT change any hex output. The R6 change to crates/lys-core/src/lib.rs is to hex_lower's body only.

**Acceptance:**
- `ast-grep scan --config sgconfig.yml --filter no-let-underscore-on-results --json=stream | wc -l` prints 0 (it prints 19 at 4dd1c33 with only sgconfig.yml and rules/ast-grep added).
- `cargo test -p lys --lib commands::hex` and `cargo test -p lys-anchor-cli --lib commands::hex` each pass, including the assertion that hex_lower(&[0x00, 0x0f, 0xa5, 0xff]) is "000fa5ff".
- `grep -c 'recipient_pub.len()' crates/lys/tests/json_output_tests.rs` prints 1, and `cargo test -p lys-core --lib keys::identity::tests::load_or_generate_creates_file_when_missing` passes.
- `git diff 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa -- crates | grep -cE '^\+.*let _[a-z]'` prints 0: no _-prefixed binding is added.

**Files:**
- modify: crates/lys-anchor-cli/src/commands/hex.rs
- modify: crates/lys-anchor/tests/anchor_receipt_conformance.rs
- modify: crates/lys-core/src/keys/identity_tests.rs
- modify: crates/lys-core/src/lib.rs
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
- modify: crates/lys/src/commands/hex.rs
- modify: crates/lys/tests/cli_tests.rs
- modify: crates/lys/tests/json_output_tests.rs
- modify: crates/lys/tests/log_tests.rs

**Checklist:**
- C90 — No `let _ =` statement remains under crates/.

**Stories:**
- S30 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an unwrap, a lint bypass, a dropped Result or logic in a mod.rs so that a rule break is caught before review rather than by a reviewer's memory.

### R7: Move the logic out of three mod.rs files into named sibling files

THE SYSTEM SHALL move every item of crates/lys-core/tests/harness/mod.rs, with its module docs, `use` lines, doc comments and attributes, into crates/lys-core/tests/harness/go.rs, and the same from crates/lys-anchor/tests/harness/mod.rs into crates/lys-anchor/tests/harness/go.rs, the two #[test] fns included. Each harness mod.rs keeps R4's `#![cfg(test)]` as its first line, then a one-line module doc naming go.rs, `mod go;` and a `pub use go::{...};` of exactly the names its including test crates reach as `harness::<name>` today: build_go_tool, go_or_skip and run_built_tool for lys-core, and GoScaffold with the same three for lys-anchor. From crates/lys-home/src/harness/claude_code/mod.rs, HARNESS, PROVIDER, API and AUTHORED move into crates/lys-home/src/harness/claude_code/names.rs, with a module doc; claude_code/mod.rs keeps its module docs and declarations and adds `mod names;` and `pub use names::{API, AUTHORED, HARNESS, PROVIDER};`, so every caller's path is unchanged. lys_core_harness() in the lys-anchor harness SHALL read ../lys-core/tests/harness/go.rs, the file that then holds the phrase `Shared Go-toolchain harness` and the `.env("GOFLAGS", "-mod=vendor")`, `.env("GOPROXY", "off")`, `.env("GOTOOLCHAIN", "local")` and `.env("GOCACHE", gocache)` clauses. In the module docs of crates/lys-anchor/tests/stranger_verification.rs, the citation `lys-core/tests/harness/mod.rs` SHALL become `lys-core/tests/harness/go.rs`. THE SYSTEM SHALL NOT change the body of any moved item other than that one path, and SHALL NOT change crates/lys-home/src/record/mod.rs.

**Acceptance:**
- `for f in crates/lys-core/tests/harness/mod.rs crates/lys-anchor/tests/harness/mod.rs crates/lys-home/src/harness/claude_code/mod.rs; do ast-grep scan --inline-rules '{id: mod-rs-logic, language: rust, severity: error, rule: {any: [{kind: function_item}, {kind: struct_item}, {kind: enum_item}, {kind: trait_item}, {kind: impl_item}, {kind: const_item}, {kind: static_item}]}}' --json=stream "$f" | wc -l; done` prints 0 three times (it prints 4, 12 and 4 at 4dd1c33).
- `cargo test -p lys-anchor --test checkpoint_note_conformance the_go_environment_contract_matches_the_one_lys_core_wrote_down` reports `1 passed; 0 failed`, and with the line `.env("GOPROXY", "off")` deleted from crates/lys-core/tests/harness/go.rs the same command reports `0 passed; 1 failed`, its message naming GOPROXY. The line is restored afterwards.
- `git grep -n 'lys-core/tests/harness/mod.rs' -- crates` prints nothing, and `git grep -c 'lys-core/tests/harness/go.rs' -- crates/lys-anchor/tests/harness/go.rs crates/lys-anchor/tests/stranger_verification.rs` prints 1 for each file.
- `cargo test -p lys-home` exits 0.

**Files:**
- create: crates/lys-core/tests/harness/go.rs
- create: crates/lys-anchor/tests/harness/go.rs
- create: crates/lys-home/src/harness/claude_code/names.rs
- modify: crates/lys-core/tests/harness/mod.rs
- modify: crates/lys-anchor/tests/harness/mod.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs
- modify: crates/lys-anchor/tests/stranger_verification.rs

**Checklist:**
- C91 — crates/lys-core/tests/harness/mod.rs, crates/lys-anchor/tests/harness/mod.rs and crates/lys-home/src/harness/claude_code/mod.rs hold no fn, struct, enum, trait, impl, const or static item.

**Stories:**
- S30 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an unwrap, a lint bypass, a dropped Result or logic in a mod.rs so that a rule break is caught before review rather than by a reviewer's memory.

### R8: Add sgconfig.yml and the four rules

sgconfig.yml at the repository root holds `ruleDirs:` with the one entry `rules/ast-grep`, as Cambium's does. rules/ast-grep holds exactly four files, each at severity error. Three of them are mod-rs-declarations-only.yml, no-let-underscore-on-results.yml and no-lint-bypass-attributes.yml. Each is the file of the same name in the cambium repository (github.com/ablative-io/cambium) at commit 1be80d8ec, where each is unchanged since df4dca5de6899df89b06b94cdeeec3619877fc1d, with two changes. A key `ignores:` with the one entry `"vendor/**"` is added directly after `message:`. In the messages, `(CLAUDE.md / AGENTS.md)` becomes `(CLAUDE.md)` and `(AGENTS.md)` becomes `(CLAUDE.md)`. The fourth is no-unwrap-expect-panic-outside-tests.yml, with exactly this content:

id: no-unwrap-expect-panic-outside-tests
language: rust
severity: error
message: "unwrap, expect or panic outside test code. Test code is a file whose first item is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a file under tests/ (CLAUDE.md)."
ignores:
  - "**/tests/**"
  - "vendor/**"
rule:
  any:
    - pattern: $X.unwrap()
    - pattern: $X.expect($$$)
    - pattern: panic!($$$)
  not:
    any:
      - inside:
          kind: source_file
          stopBy: end
          has:
            pattern: "#![cfg(test)]"
            nthChild: 1
      - inside:
          kind: mod_item
          stopBy: end
          follows:
            pattern: "#[cfg(test)]"
            stopBy:
              not:
                any: [{kind: attribute_item}, {kind: line_comment}, {kind: block_comment}]
      - inside:
          kind: function_item
          stopBy: end
          follows:
            pattern: "#[test]"
            stopBy:
              not:
                any: [{kind: attribute_item}, {kind: line_comment}, {kind: block_comment}]

It treats as test code a file whose first item is the inner attribute #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, and a path under tests/, and it reports unwrap, expect and panic everywhere else. THE SYSTEM SHALL NOT carry no-std-mutex-in-async, no-timer-in-door-handlers or no-timer-import-in-door-handlers, and SHALL NOT write a _name-rename rule. It SHALL NOT add any ignores or files entry beyond those stated, SHALL NOT recognise test code by a list of file names, and SHALL NOT set any rule below severity error.

**Acceptance:**
- `ls rules/ast-grep` prints exactly mod-rs-declarations-only.yml, no-let-underscore-on-results.yml, no-lint-bypass-attributes.yml and no-unwrap-expect-panic-outside-tests.yml, and `grep -h '^severity:' rules/ast-grep/*.yml | sort | uniq -c` prints the single line `   4 severity: error`.
- With $cambium the path of a local clone of github.com/ablative-io/cambium that contains commit df4dca5de6899df89b06b94cdeeec3619877fc1d, and for each of the three copied rules R, `diff <(git -C $cambium show df4dca5de6899df89b06b94cdeeec3619877fc1d:rules/ast-grep/R.yml | grep -v '^message:') <(grep -v -e '^message:' -e '^ignores:' -e '^  - "vendor/\*\*"$' rules/ast-grep/R.yml)` prints nothing, and `grep -c 'AGENTS.md' rules/ast-grep/*.yml` prints 0 for every file.
- In a checkout of 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa with only sgconfig.yml and rules/ast-grep added, `ast-grep scan --config sgconfig.yml --json=stream | python3 -c "import json,sys,collections; print(sorted(collections.Counter(json.loads(l)['ruleId'] for l in sys.stdin).items()))"` prints `[('mod-rs-declarations-only', 20), ('no-let-underscore-on-results', 19), ('no-lint-bypass-attributes', 113), ('no-unwrap-expect-panic-outside-tests', 224)]`.
- With a directory vendor/probe holding a file lib.rs whose one fn calls `.unwrap()`, `ast-grep scan --config sgconfig.yml --json=stream | grep -c vendor/probe` prints 0. The directory is removed afterwards and never committed.

**Files:**
- create: sgconfig.yml
- create: rules/ast-grep/mod-rs-declarations-only.yml
- create: rules/ast-grep/no-let-underscore-on-results.yml
- create: rules/ast-grep/no-lint-bypass-attributes.yml
- create: rules/ast-grep/no-unwrap-expect-panic-outside-tests.yml

**Checklist:**
- C92 — sgconfig.yml names rules/ast-grep, which holds mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes and no-unwrap-expect-panic-outside-tests, each at severity error.

**Stories:**
- S30 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an unwrap, a lint bypass, a dropped Result or logic in a mod.rs so that a rule break is caught before review rather than by a reviewer's memory.

### R9: Run the scan as a gate leg in project.json, .land/gates.sh and CI, and prove it fails on a hit

THE SYSTEM SHALL append to tree `.` of docs/design/project.json an eighth leg, `{"name": "ast-grep", "command": "ast-grep scan --config sgconfig.yml", "requires": ["tool:ast-grep"], "cadence": "round"}`. It SHALL append the line `leg ast-grep scan --config sgconfig.yml` to .land/gates.sh after `leg cargo doc --no-deps`, and the file's header comment SHALL say it runs CLAUDE.md's list and the ast-grep scan. It SHALL add two steps to job test of .github/workflows/ci.yml, after the Test step: one installing ast-grep with `cargo install ast-grep --locked --version 0.44.1`, and one running `ast-grep scan --config sgconfig.yml`. WHEN any carried rule reports a hit, THE SYSTEM SHALL fail the leg, the .land/gates.sh run and the CI job. THE SYSTEM SHALL NOT change the seven existing legs, the other lines of .land/gates.sh or the other CI steps. It SHALL NOT pass any flag to the scan beyond --config sgconfig.yml, and SHALL NOT land while the scan reports any hit.

**Acceptance:**
- `python3 scripts/design/validate.py docs/design/project.json` exits 0; `python3 -c "import json; l=json.load(open('docs/design/project.json'))['trees'][0]['legs']; print(len(l), l[7])"` prints `8 {'name': 'ast-grep', 'command': 'ast-grep scan --config sgconfig.yml', 'requires': ['tool:ast-grep'], 'cadence': 'round'}`; and legs 0 to 6 equal those of `git show 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa:docs/design/project.json`.
- `grep -c '^leg ast-grep scan --config sgconfig.yml$' .land/gates.sh` prints 1, `grep -c 'ast-grep scan --config sgconfig.yml' .github/workflows/ci.yml` prints 1, and `grep -c 'cargo install ast-grep --locked --version 0.44.1' .github/workflows/ci.yml` prints 1.
- On the landed tree, `ast-grep scan --config sgconfig.yml` exits 0 and `ast-grep scan --config sgconfig.yml --json=stream | wc -l` prints 0.
- On the landed tree, `ast-grep scan --config sgconfig.yml --filter no-lint-bypass-attributes --json=stream | wc -l` prints 0; at 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa with only sgconfig.yml and rules/ast-grep added, the same command prints 113.
- With crates/lys-core/src/scratch_unwrap.rs added, declared from no module and holding exactly:

//! Scratch file for the rule's proof; never committed.
pub fn scratch(value: Option<u8>) -> u8 {
    value.unwrap()
}

#[cfg(test)]
mod tests {
    fn helper(value: Option<u8>) -> u8 {
        value.expect("scratch")
    }
}

#[test]
#[should_panic(expected = "scratch")]
fn scratch_test() {
    panic!("scratch");
}

`ast-grep scan --config sgconfig.yml --json=stream` prints exactly one line, whose ruleId is no-unwrap-expect-panic-outside-tests, file is crates/lys-core/src/scratch_unwrap.rs and range.start.line is 2 (line 3, the unwrap). The expect on line 9 and the panic on line 16 are not reported, and `ast-grep scan --config sgconfig.yml` exits 1.
- With the same scratch file present, `sh .land/gates.sh` prints `--- status 1: ast-grep scan --config sgconfig.yml ---`, prints `--- status 0: ...` for each of its seven other legs, and exits 1.
- After the scratch file is deleted, `git status --porcelain` prints nothing and `git log --all --oneline -- crates/lys-core/src/scratch_unwrap.rs` prints nothing.

**Files:**
- modify: docs/design/project.json
- modify: .land/gates.sh
- modify: .github/workflows/ci.yml

**Checklist:**
- C93 — The leg `ast-grep scan --config sgconfig.yml` is in docs/design/project.json requiring tool:ast-grep, in .land/gates.sh and in CI.
- C94 — The scan reports zero hits over the landed tree, and exactly one hit on an uncommitted scratch file holding an unwrap outside test code.

**Stories:**
- S30 (Card author, Landing work in lys) — As a card author landing work in lys, I want the gate to refuse an unwrap, a lint bypass, a dropped Result or logic in a mod.rs so that a rule break is caught before review rather than by a reviewer's memory.
- S32 (Lead, Trusting the gate) — As the lead for lys, I want proof that the unwrap rule fires so that a green leg means the rule held and not that nothing was measured.

## Boundaries

- SHALL NOT rewrite, move out of its file or remove any unwrap, expect or panic call in a fixture.rs or *_tests.rs file.
- SHALL NOT recognise test code by a list of file names, and SHALL NOT add any ignores or files entry beyond tests/** in the unwrap rule and vendor/** in every rule.
- SHALL NOT add an #[allow], #[expect], #[ignore], _-prefixed rename or #[cfg(any())] anywhere.
- SHALL NOT change the cambium repository.
- SHALL NOT change a wire format, domain-separation tag, test vector, signed fixture, or the public API of lys-core, lys or lys-log-store.
- SHALL NOT change the seven existing legs of docs/design/project.json.
- SHALL NOT carry no-std-mutex-in-async, no-timer-in-door-handlers, no-timer-import-in-door-handlers or a _name-rename rule.
- SHALL NOT change crates/lys-home/src/record/mod.rs, which belongs to HOME-013.
- SHALL NOT edit or delete the pre-method lys-core documents; they are only renamed.
- SHALL NOT commit any scratch or probe file.
- SHALL NOT change the attribute #![cfg_attr(not(test), forbid(unsafe_code))] in crates/lys-core/src/lib.rs.

## Verification

- python3 scripts/design/validate.py docs/design/lys-core exits 0, and python3 scripts/design/check-coverage.py docs/design/lys-core exits 0.
- sh scripts/design/gate.sh exits 0.
- cargo fmt --all leaves the tree unchanged.
- cargo clippy --workspace --all-targets --all-features -- -D warnings exits 0, and cargo clippy --workspace --all-targets -- -D warnings exits 0.
- cargo test --workspace --all-features exits 0, and the sum of its `passed` counts equals the sum at 4dd1c337bc94f0f3dea0fd05c853a9d367b28cfa.
- cargo doc --no-deps --all-features and cargo doc --no-deps each exit 0 with no warning.
- ast-grep scan --config sgconfig.yml exits 0 and reports 0 hits.
- git grep -n '#\[allow\|#!\[allow' -- crates tests prints nothing.
- sh .land/gates.sh exits 0 on the landed tree and prints `--- status 0: ast-grep scan --config sgconfig.yml ---`.
