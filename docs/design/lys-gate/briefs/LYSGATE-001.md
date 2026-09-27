---
type: brief
id: LYSGATE-001
cluster: lys-gate
title: Run every test binary in the gate's test leg with --no-fail-fast
---

# LYSGATE-001: Run every test binary in the gate's test leg with --no-fail-fast

> **Cluster:** lys-gate
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-038 — The gate's test leg runs every test binary; it never stops at the first failure — Every declaration of the gate's test leg runs `cargo test --workspace --all-features --no-fail-fast`: project.json, every cluster design.json gate, .land/gates.sh and CLAUDE.md's 'Gates before any commit' block; CI's test step gains the same one flag and nothing else. Rejected: keeping cargo's default fail-fast run, which is shorter on a red round but reports only the first failing binary.
> **Checklist:**
> - C1 — docs/design/project.json's 'tests' leg command is exactly `cargo test --workspace --all-features --no-fail-fast`.
> - C2 — The 'tests' leg command of the directory, home, secrets and lys-gate design.json gate arrays is exactly `cargo test --workspace --all-features --no-fail-fast`, and every other leg is byte-identical to before.
> - C3 — .land/gates.sh's test leg line is exactly `leg cargo test --workspace --all-features --no-fail-fast`.
> - C4 — CLAUDE.md's 'Gates before any commit' test line is exactly `cargo test --workspace --all-features --no-fail-fast`, the same command as .land/gates.sh's test leg.
> - C5 — .github/workflows/ci.yml's Test step runs exactly `cargo test --workspace --no-fail-fast`.
> - C6 — The changes of C1 to C5 are one commit, and the grep over docs/design/project.json, every docs/design/*/design.json and .land/gates.sh for test legs without --no-fail-fast prints 0.
> - C7 — sh scripts/design/gate.sh exits 0 at the card's head, and the green gate round's log shows the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.
> - C8 — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the green round at the same head, and no ref under scratch/ exists on origin after it.
> - C9 — docs/design/lys-gate/PROOF-LYSGATE-001.md names the scratch round's log by its path in the gate log store, its SHA-256 and its counts, and holds no line of the log.
> **Stories:**
> - S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.
> - S2 (AI Agent, Runs the gates by hand before a commit) — As an agent running the gates by hand from CLAUDE.md, I want its test line to be the landing hook's test command so that my run measures what landing measures.
> - S3 (Contributor, Reads CI results on a pull request) — As a contributor reading CI on a pull request, I want the test step to report every failing test binary so that I can fix them all from one run.

## Purpose

The gate's test leg stops at the first failing test binary, so a red round can hide every failure after the first (the design's problem). This brief changes the leg to `cargo test --workspace --all-features --no-fail-fast` everywhere it is declared as a gate leg, in one commit, so every test binary runs, every failure is reported in the one round, and the leg still exits non-zero when any test fails (ADR-038), and proves it with a grep, the gate log and one scratch round whose log is named, never copied.

## Task

In one commit, change the test leg's command to exactly `cargo test --workspace --all-features --no-fail-fast` in docs/design/project.json, in the gate arrays of docs/design/directory/design.json, docs/design/home/design.json, docs/design/secrets/design.json and docs/design/lys-gate/design.json, in .land/gates.sh, and in the test line of CLAUDE.md's 'Gates before any commit' block, so .land/gates.sh still matches that block exactly as its header says; and add --no-fail-fast to the Test step of .github/workflows/ci.yml, which then runs `cargo test --workspace --no-fail-fast`. That one commit touches these eight files and no other. The lys-gate design.json is among them because its gate array is copied from project.json and is one of the docs/design/*/design.json the check covers. Re-render every cluster whose design.json changed with scripts/design/render-cluster.py (the gate array is not rendered, so no markdown changes) and keep scripts/design/gate.sh green. Then prove it: the grep over the declared legs prints 0; the green gate round's '$ command' line for the tests leg; one gate round over a local scratch branch with one planted failing test in crates/lys; and docs/design/lys-gate/PROOF-LYSGATE-001.md, which names that round's log and never copies it. In scope: those files and that evidence. Out of scope: every other leg, whose command, order, requirements and cadence stay exactly as they are; raising any timeout; splitting any test; the briefs under docs/design/*/briefs and the release documents, which quote cargo commands as acceptance text or instructions, not as gate legs, and are not changed; docs/design/lys-core, whose DESIGN.md line 142 and CHECKLIST.md item C63 quote cargo test over the workspace as acceptance prose in a cluster with no design.json gate, owned by the ast-grep leg card, and are not changed; the vendored method under scripts/design, the method project and the ledger, which are not changed; adding any script. Finding, not changed here: CI's Test step runs without --all-features, so it compiles out the tests behind the unstable-anchor feature that the gate's test leg runs; that difference stays as it is and is left for its own unit. The words' 'each changed leg must still be classed as a battery, shown in the gate log' is read as the logged command line: the ledger matches its battery markers, among them 'cargo test', against the command lines of running processes (the method's workers/ds2_ledger/gate.py:201-223 at method commit 3c3bac7) and logs each leg as '-- leg NAME at venue V' followed by '$ command' (the same file, 409-437), so the tests leg's '$ command' line containing 'cargo test' followed by '--no-fail-fast' is what shows the leg is still a battery. The scratch branch is kept from landing by never reaching origin: it lives in a local worktree at the venue that serves heavy builds and full gates, no pull request is opened for it, and the worktree and branch are deleted after its round. Nothing refuses a scratch/ branch at landing today and this brief adds no such mechanism.

## Requirements

### R1: Change the project gate's tests leg in docs/design/project.json

Structural. In docs/design/project.json, the command of the leg named 'tests' in trees[0].legs becomes exactly `cargo test --workspace --all-features --no-fail-fast`. The leg's name, its requires (rust-build), its cadence (round) and its position (the fourth leg) do not change. The file SHALL NOT gain or lose any key, SHALL NOT change any other string, and SHALL NOT lose --all-features from the command.

**Acceptance:**
- python3 -c "import json;print(json.load(open('docs/design/project.json'))['trees'][0]['legs'][3]['command'])" prints exactly `cargo test --workspace --all-features --no-fail-fast`.
- git diff <change>~1 <change> --numstat -- docs/design/project.json, where <change> is the commit of R6, prints `1	1	docs/design/project.json`, and the one removed line and the one added line differ only by ` --no-fail-fast` inserted before the closing quote.
- python3 scripts/design/validate.py docs/design/project.json exits 0.

**Files:**
- modify: docs/design/project.json

**Checklist:**
- C1 — docs/design/project.json's 'tests' leg command is exactly `cargo test --workspace --all-features --no-fail-fast`.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R2: Change the tests leg of the four cluster gates and re-render them

Structural. In docs/design/directory/design.json, docs/design/home/design.json, docs/design/secrets/design.json and docs/design/lys-gate/design.json, the command of the gate leg named 'tests' becomes exactly `cargo test --workspace --all-features --no-fail-fast`. Each leg's name, requires, cadence and position, every other leg, every other tree and every shorthand stay byte-identical. Each of the four clusters is re-rendered with python3 scripts/design/render-cluster.py <cluster directory>. THE SYSTEM SHALL NOT change any field of these files other than the one command string, SHALL NOT edit any file under docs/design/*/briefs, and SHALL NOT create, edit or delete anything under docs/design/lys-core.

**Acceptance:**
- For each of docs/design/directory/design.json, docs/design/home/design.json, docs/design/secrets/design.json and docs/design/lys-gate/design.json, git diff <change>~1 <change> --numstat -- <that file> prints `1	1	<that file>`, and the one added line contains `"command": "cargo test --workspace --all-features --no-fail-fast"`.
- For each of those four files, a python3 comparison of its gate array at <change>~1 and at <change>, with the 'tests' leg's command removed from both, finds the two equal: the same trees, the same leg names in the same order, the same requires and the same cadence.
- After python3 scripts/design/render-cluster.py is run on docs/design/directory, docs/design/home, docs/design/secrets and docs/design/lys-gate at <change>, git status --porcelain -- docs/design prints nothing.
- git diff <base> HEAD --name-only -- 'docs/design/*/briefs' docs/design/lys-core, where <base> is the main commit the card branched from, prints nothing.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/home/design.json
- modify: docs/design/secrets/design.json
- modify: docs/design/lys-gate/design.json

**Checklist:**
- C2 — The 'tests' leg command of the directory, home, secrets and lys-gate design.json gate arrays is exactly `cargo test --workspace --all-features --no-fail-fast`, and every other leg is byte-identical to before.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R3: Change the landing hook's test leg in .land/gates.sh

Structural, with one behaviour. In .land/gates.sh the line `leg cargo test --workspace --all-features` becomes exactly `leg cargo test --workspace --all-features --no-fail-fast`, on the same line, after both clippy legs and before both doc legs. The leg() wrapper, the header comment, the other six leg lines and their order, and the exit logic do not change. WHEN a test fails under the new leg, THE SYSTEM SHALL record that leg's status as non-zero and SHALL exit non-zero; it SHALL NOT stop any later leg from running, and SHALL NOT gain a check of the branch name.

**Acceptance:**
- grep -n '^leg cargo test' .land/gates.sh prints exactly `19:leg cargo test --workspace --all-features --no-fail-fast`.
- git diff <change>~1 <change> --numstat -- .land/gates.sh prints `1	1	.land/gates.sh`.
- grep '^leg ' .land/gates.sh prints exactly seven lines, in this order: `leg sh scripts/design/gate.sh`, `leg cargo fmt --check`, `leg cargo clippy --all-targets --all-features -- -D warnings`, `leg cargo clippy --all-targets -- -D warnings`, `leg cargo test --workspace --all-features --no-fail-fast`, `leg cargo doc --no-deps --all-features`, `leg cargo doc --no-deps`.

**Files:**
- modify: .land/gates.sh

**Checklist:**
- C3 — .land/gates.sh's test leg line is exactly `leg cargo test --workspace --all-features --no-fail-fast`.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R4: Change the test line of CLAUDE.md's 'Gates before any commit' block

Structural. In CLAUDE.md's 'Gates before any commit' code block, the test line becomes exactly `cargo test --workspace --all-features --no-fail-fast`, so .land/gates.sh still runs its legs exactly as that block lists them. The block's other five lines and their order, and every other line of CLAUDE.md, do not change. THE SYSTEM SHALL NOT edit the prose after the block, including its 'All five clean' sentence and its quotations of cargo commands.

**Acceptance:**
- grep -n '^cargo test' CLAUDE.md prints exactly one line, `99:cargo test --workspace --all-features --no-fail-fast`.
- Under bash, diff <(grep '^cargo test' CLAUDE.md) <(sed -n 's/^leg \(cargo test .*\)$/\1/p' .land/gates.sh) prints nothing and exits 0.
- git diff <change>~1 <change> --numstat -- CLAUDE.md prints `1	1	CLAUDE.md`.

**Files:**
- modify: CLAUDE.md

**Checklist:**
- C4 — CLAUDE.md's 'Gates before any commit' test line is exactly `cargo test --workspace --all-features --no-fail-fast`, the same command as .land/gates.sh's test leg.

**Stories:**
- S2 (AI Agent, Runs the gates by hand before a commit) — As an agent running the gates by hand from CLAUDE.md, I want its test line to be the landing hook's test command so that my run measures what landing measures.

### R5: Add --no-fail-fast to CI's test step

Structural, with one behaviour. In .github/workflows/ci.yml the Test step's `run: cargo test --workspace` becomes exactly `run: cargo test --workspace --no-fail-fast`. Its env (LYS_REQUIRE_GO), the step's name, every other step and every other job do not change. WHEN a pull request's CI run has failing tests in more than one test binary, THE SYSTEM SHALL report every failing binary in that run and SHALL fail the step. THE SYSTEM SHALL NOT add --all-features to the step: the missing --all-features is a recorded finding, not changed here.

**Acceptance:**
- grep -n 'run: cargo test' .github/workflows/ci.yml prints exactly one line, `46:        run: cargo test --workspace --no-fail-fast`.
- grep -c -- '--all-features' .github/workflows/ci.yml prints `0` at <change>.
- git diff <change>~1 <change> --numstat -- .github/workflows/ci.yml prints `1	1	.github/workflows/ci.yml`.

**Files:**
- modify: .github/workflows/ci.yml

**Checklist:**
- C5 — .github/workflows/ci.yml's Test step runs exactly `cargo test --workspace --no-fail-fast`.

**Stories:**
- S3 (Contributor, Reads CI results on a pull request) — As a contributor reading CI on a pull request, I want the test step to report every failing test binary so that I can fix them all from one run.

### R6: Make the change one commit and prove no declared test leg lacks --no-fail-fast

WHEN the change is committed, THE SYSTEM SHALL carry every edit of R1 to R5 in exactly one commit, <change>, which changes the eight files R1 to R5 name and no other, and at <change> the grep `grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/*/design.json .land/gates.sh | grep -vc -- --no-fail-fast` SHALL print 0. The grep's command line and output are recorded in the pull request's evidence and in the proof document of R9. THE SYSTEM SHALL NOT add a script, a test or any other file to carry the check, and no other commit on the card's branch SHALL change any of the eight files.

**Acceptance:**
- At <change>, grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/*/design.json .land/gates.sh | grep -vc -- --no-fail-fast prints `0`.
- At <change>~1, the same grep prints `6`.
- git show --name-only --format= <change> prints exactly these eight paths: .github/workflows/ci.yml, .land/gates.sh, CLAUDE.md, docs/design/directory/design.json, docs/design/home/design.json, docs/design/lys-gate/design.json, docs/design/project.json, docs/design/secrets/design.json.
- git log --format=%H <base>..HEAD -- .github/workflows/ci.yml .land/gates.sh CLAUDE.md docs/design/directory/design.json docs/design/home/design.json docs/design/lys-gate/design.json docs/design/project.json docs/design/secrets/design.json prints exactly one line, the hash of <change>.

**Checklist:**
- C6 — The changes of C1 to C5 are one commit, and the grep over docs/design/project.json, every docs/design/*/design.json and .land/gates.sh for test legs without --no-fail-fast prints 0.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R7: Keep the design gate green and show the tests leg's command in the gate log

WHEN the gate round runs at <change>, THE SYSTEM SHALL report the tests leg green, and the round's log SHALL show the tests leg's logged command line containing 'cargo test' followed by '--no-fail-fast'; that logged line is what shows the leg is still classed as a battery, because the ledger matches its marker 'cargo test' against running command lines and logs each leg's command as '$ command'. WHEN sh scripts/design/gate.sh runs at the card's head, THE SYSTEM SHALL exit 0. THE SYSTEM SHALL NOT change the ledger or the method to print a battery class.

**Acceptance:**
- sh scripts/design/gate.sh, run at the card's head, exits 0 and prints no line beginning `rendered markdown differs`.
- The green gate round's log at <change> holds a line beginning `-- leg tests at venue ` followed immediately by the line `$ cargo test --workspace --all-features --no-fail-fast`, and that leg's part of the log ends with the lines `(exit 0)` and `(measured-green)`.
- git diff <base> HEAD --name-only -- scripts/design prints nothing.

**Checklist:**
- C7 — sh scripts/design/gate.sh exits 0 at the card's head, and the green gate round's log shows the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R8: Run one gate round over a local scratch branch with one planted failing test

WHEN one gate round runs over the local branch scratch/lysgate-001, cut from <change> in a local worktree at the venue that serves heavy builds and full gates, THE SYSTEM SHALL show in the tests leg's part of that round's log the planted failure together with every other test binary's result, and the tests leg SHALL exit non-zero. The scratch branch adds one commit on <change> that adds one test, scratch_planted_failure, whose body is assert_eq!(1, 2), to crates/lys/tests/certified_attestation_tests.rs: crates/lys is the first entry of Cargo.toml's [workspace] members, so every binary after it runs later in the log, and adding a test to an existing test binary adds no binary. The baseline is the green gate round of R7 at <change>. The tests leg's part of a log runs from its line beginning `-- leg tests at venue ` to the next line beginning `-- leg `, or to the end of the log. THE SYSTEM SHALL NOT push the scratch branch to origin, SHALL NOT open a pull request for it, SHALL NOT land or merge it, and SHALL NOT add the planted test to the card's branch; after the round the worktree and the branch are deleted.

**Acceptance:**
- In the tests leg's part of the scratch round's log, the number of lines containing `test result: ` equals the number in the tests leg's part of the green round's log at <change>.
- In the tests leg's part of the scratch round's log, exactly 1 line contains `test result: FAILED`, and in the green round's log at <change> no line contains `test result: FAILED`.
- The tests leg's part of the scratch round's log contains the line `test scratch_planted_failure ... FAILED` and the line `(exit 101)`.
- After the round, git ls-remote origin 'refs/heads/scratch/*' prints nothing, and git branch --list 'scratch/*' in the clone that held the worktree prints nothing.
- git log --format=%H <base>..HEAD -- crates prints nothing.

**Checklist:**
- C8 — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the green round at the same head, and no ref under scratch/ exists on origin after it.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R9: Write the proof document that names the scratch round's log

Structural. docs/design/lys-gate/PROOF-LYSGATE-001.md records, in a commit after <change>: the R6 grep's command line and its output; the green round's and the scratch round's ids; the scratch round's log by its path in the ledger's gate log store and its SHA-256; the counts R8 measures (the `test result: ` lines in the tests leg's part of each log, and the `test result: FAILED` lines in the scratch log); and the command git ls-remote origin 'refs/heads/scratch/*' with its empty output after the round. THE SYSTEM SHALL NOT copy any line of either round's log into the document, and SHALL NOT name a path outside the gate log store for the log.

**Acceptance:**
- shasum -a 256 run on the log path the proof names prints the SHA-256 the proof names, a string of 64 lowercase hexadecimal characters.
- The `test result: ` counts the proof names for the scratch round and the green round are equal to each other and to the counts R8 measures, and the `test result: FAILED` count it names is 1.
- Under bash, grep -Fxc -f <(grep -v '^[[:space:]]*$' <the log path the proof names>) docs/design/lys-gate/PROOF-LYSGATE-001.md prints `0`.
- The proof holds the line `grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/*/design.json .land/gates.sh | grep -vc -- --no-fail-fast` followed immediately by the line `prints: 0`.
- git log --format=%H <change>..HEAD -- docs/design/lys-gate/PROOF-LYSGATE-001.md prints at least one line, and git log --format=%H <base>..<change> -- docs/design/lys-gate/PROOF-LYSGATE-001.md prints nothing.

**Files:**
- create: docs/design/lys-gate/PROOF-LYSGATE-001.md

**Checklist:**
- C9 — docs/design/lys-gate/PROOF-LYSGATE-001.md names the scratch round's log by its path in the gate log store, its SHA-256 and its counts, and holds no line of the log.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

## Boundaries

- SHALL NOT change any gate leg's command other than the test leg, nor any leg's name, order, requirements or cadence.
- SHALL NOT remove --all-features from any gate test leg, and SHALL NOT add --all-features to CI's test step.
- SHALL NOT raise any timeout or split any test.
- SHALL NOT edit any file under docs/design/*/briefs or any release document.
- SHALL NOT create, edit or delete anything under docs/design/lys-core, including DESIGN.md line 142 and CHECKLIST.md item C63.
- SHALL NOT edit the vendored method under scripts/design (validate.py, check-coverage.py, render-cluster.py, render-brief.py, schemas, workers), the method project or the ledger.
- SHALL NOT add a script for the check that no test leg lacks --no-fail-fast.
- SHALL NOT change crate source, test source or any wire format on the card's branch.
- SHALL NOT push the scratch branch or its planted test to origin, open a pull request for it, or land or merge it.
- SHALL NOT add a mechanism that refuses scratch/ branches at landing.
- SHALL NOT copy any line of a gate round's log into the proof document.
- SHALL NOT run the full gate round or the scratch round on the authoring machine; both run at the venue that serves heavy builds and full gates.

## Verification

- Run the R6 grep at <change> and at <change>~1: it prints 0, then 6.
- Run git show --numstat --format= <change>: each of the eight files shows exactly one insertion and one deletion, and no other path is listed.
- Run sh scripts/design/gate.sh at the card's head: it exits 0.
- Read the green round's log at <change>: the tests leg's '$ command' line is `$ cargo test --workspace --all-features --no-fail-fast` and the leg is green.
- Read the scratch round's log against the green round's: the `test result: ` counts in the tests leg's parts are equal, exactly one of the scratch lines contains `test result: FAILED`, and the tests leg exits 101.
- Run git ls-remote origin 'refs/heads/scratch/*': it prints nothing.
- Check PROOF-LYSGATE-001.md's named SHA-256 against the log it names with shasum -a 256.
