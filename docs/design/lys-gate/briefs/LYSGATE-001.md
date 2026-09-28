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
> - C6 — The changes of C1 to C5 are one commit, no other commit on the card's branch changes those files except that the lys-gate design.json's gate array alone is held unchanged outside that commit, and the grep over docs/design/project.json, the directory, home, secrets and lys-gate design.json files and .land/gates.sh, named one by one, for test legs without --no-fail-fast prints 0.
> - C7 — sh scripts/design/gate.sh exits 0 at the card's head, and four green gate rounds at the change, one measured against each of the directory, home, secrets and lys-gate design.json files, each show the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.
> - C8 — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the lys-gate green round at the same head, and no ref under scratch/ exists on origin after it.
> - C9 — docs/design/lys-gate/PROOF-LYSGATE-001.md names each of the four green rounds and the scratch round by round id, log path in the gate log store and SHA-256, and the scratch round's counts, holds no line of any of those logs, and lists by file name every other cluster's design.json whose test leg lacks --no-fail-fast as a finding for that cluster's card.
> **Stories:**
> - S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.
> - S2 (AI Agent, Runs the gates by hand before a commit) — As an agent running the gates by hand from CLAUDE.md, I want its test line to be the landing hook's test command so that my run measures what landing measures.
> - S3 (Contributor, Reads CI results on a pull request) — As a contributor reading CI on a pull request, I want the test step to report every failing test binary so that I can fix them all from one run.

## Purpose

The gate's test leg stops at the first failing test binary, so a red round can hide every failure after the first (the design's problem). This brief changes the leg to `cargo test --workspace --all-features --no-fail-fast` everywhere it is declared as a gate leg, in one commit, so every test binary runs, every failure is reported in the one round, and the leg still exits non-zero when any test fails (ADR-038), and proves it with a grep, the gate log and one scratch round whose log is named, never copied.

## Task

In one commit, change the test leg's command to exactly `cargo test --workspace --all-features --no-fail-fast` in docs/design/project.json, in the gate arrays of docs/design/directory/design.json, docs/design/home/design.json, docs/design/secrets/design.json and docs/design/lys-gate/design.json, in .land/gates.sh, and in the test line of CLAUDE.md's 'Gates before any commit' block, so .land/gates.sh still matches that block exactly as its header says; and add --no-fail-fast to the Test step of .github/workflows/ci.yml, which then runs `cargo test --workspace --no-fail-fast`. That one commit touches these eight files and no other; no other commit on the branch changes the seven files outside this cluster, and this cluster's design.json, which the chain's own commits also write, is held to its gate array: no other commit changes that array. The lys-gate design.json is among them because its gate array is copied from project.json and is one of the docs/design/*/design.json the check covers. Re-render every cluster whose design.json changed with scripts/design/render-cluster.py (the gate array is not rendered, so no markdown changes) and keep scripts/design/gate.sh green. Then prove it: the grep over the six files that declare a gate test leg, named one by one and never by a glob, prints 0; the '$ command' line of the tests leg in four green gate rounds at the change, one measured against each changed cluster's design.json (directory, home, secrets and lys-gate), since the ledger measures one cluster's gate per round; one gate round over a local scratch branch with one planted failing test in crates/lys; and docs/design/lys-gate/PROOF-LYSGATE-001.md, which names each of those five rounds' logs by round id, path in the gate log store and SHA-256 and never copies them. In scope: those files and that evidence. Out of scope: every other leg, whose command, order, requirements and cadence stay exactly as they are; raising any timeout; splitting any test; the other clusters' briefs under docs/design/*/briefs and the release documents, which quote cargo commands as acceptance text or instructions, not as gate legs, and are not changed; docs/design/lys-core, whose DESIGN.md line 142 and CHECKLIST.md item C63 quote cargo test over the workspace as acceptance prose in a cluster with no design.json gate, owned by the ast-grep leg card, and are not changed; the vendored method under scripts/design, the method project and the ledger, which are not changed; adding any script. Finding, not changed here: CI's Test step runs without --all-features, so it compiles out the tests behind the unstable-anchor feature that the gate's test leg runs; that difference stays as it is and is left for its own unit. The words' 'each changed leg must still be classed as a battery, shown in the gate log' is read as the logged command line: the ledger matches its battery markers, among them 'cargo test', against the command lines of running processes (the method's workers/ds2_ledger/gate.py:201-223 at method commit 3c3bac7) and logs each leg as '-- leg NAME at venue V' followed by '$ command' (the same file, 409-437), so the tests leg's '$ command' line containing 'cargo test' followed by '--no-fail-fast' is what shows the leg is still a battery. The scratch branch is kept from landing by never reaching origin: it lives in a local worktree at the venue that serves heavy builds and full gates, no pull request is opened for it, and the worktree and branch are deleted after its round. Nothing refuses a scratch/ branch at landing today and this brief adds no such mechanism. The check names its files one by one, so a design.json of a cluster that appears later cannot turn it red; this brief does not change such a file, including anything under docs/design/lys-core, and the proof document records one whose test leg lacks --no-fail-fast as a finding for that cluster's card by file name.

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

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. docs/design/project.json:29 now reads `"command": "cargo test --workspace --all-features --no-fail-fast"`, and it is legs[3], the fourth leg. Row 2: met in the working tree. git diff --numstat shows `1	1	docs/design/project.json`, and the only difference between the removed and added lines is ` --no-fail-fast` before the closing quote. Row 3: met. `python3 scripts/design/validate.py docs/design/project.json` printed OK and exited 0.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/project.json` — trees[0].legs[3] 'tests' command is now `cargo test --workspace --all-features --no-fail-fast` (line 29); nothing else changed
- Checklist delivery:
  - [x] C1 — docs/design/project.json's 'tests' leg command is exactly `cargo test --workspace --all-features --no-fail-fast`. — project.json:29
- Story delivery:
  - [x] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others. — The project gate's tests leg now runs every test binary.

### R2: Change the tests leg of the four cluster gates and re-render them

Structural. In docs/design/directory/design.json, docs/design/home/design.json, docs/design/secrets/design.json and docs/design/lys-gate/design.json, the command of the gate leg named 'tests' becomes exactly `cargo test --workspace --all-features --no-fail-fast`. Each leg's name, requires, cadence and position, every other leg, every other tree and every shorthand stay byte-identical. Each of the four clusters is re-rendered with python3 scripts/design/render-cluster.py <cluster directory>. 'The briefs under docs/design/*/briefs' that the words keep unchanged means the other clusters' briefs; this card's own docs/design/lys-gate/briefs is written by the chain's card and re-render commits and is outside that set. THE SYSTEM SHALL NOT change any field of these files other than the one command string, SHALL NOT edit any other cluster's file under docs/design/*/briefs, and SHALL NOT create, edit or delete anything under docs/design/lys-core.

**Acceptance:**
- For each of docs/design/directory/design.json, docs/design/home/design.json, docs/design/secrets/design.json and docs/design/lys-gate/design.json, git diff <change>~1 <change> --numstat -- <that file> prints `1	1	<that file>`, and the one added line contains `"command": "cargo test --workspace --all-features --no-fail-fast"`.
- For each of those four files, a python3 comparison of its gate array at <change>~1 and at <change>, with the 'tests' leg's command removed from both, finds the two equal: the same trees, the same leg names in the same order, the same requires and the same cadence.
- After python3 scripts/design/render-cluster.py is run on docs/design/directory, docs/design/home, docs/design/secrets and docs/design/lys-gate at <change>, git status --porcelain -- docs/design prints nothing.
- git diff <base> HEAD --name-only -- 'docs/design/*/briefs/*' ':(exclude)docs/design/lys-gate/briefs' docs/design/lys-core, where <base> is the main commit the card branched from, prints nothing.

**Files:**
- modify: docs/design/directory/design.json
- modify: docs/design/home/design.json
- modify: docs/design/secrets/design.json
- modify: docs/design/lys-gate/design.json

**Checklist:**
- C2 — The 'tests' leg command of the directory, home, secrets and lys-gate design.json gate arrays is exactly `cargo test --workspace --all-features --no-fail-fast`, and every other leg is byte-identical to before.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met in the working tree. git diff --numstat shows 1/1 for each of the four files. In each, one sed substitution replaced only the exact string `"command": "cargo test --workspace --all-features"`, so every added line contains the new command. Row 2: met. The only change is that one command string, so every tree, leg name, order, requires, cadence and shorthand is unchanged. Row 3: met. `python3 scripts/design/render-cluster.py` ran on docs/design/directory, home, secrets and lys-gate, and git status --porcelain afterwards listed only the eight edited files, with no markdown under docs/design changed. Row 4: met. Nothing under other clusters' briefs or under docs/design/lys-core was touched.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/directory/design.json` — The gate 'tests' leg command at line 716 now has --no-fail-fast
  - modified: `docs/design/home/design.json` — The gate 'tests' leg command at line 776 now has --no-fail-fast
  - modified: `docs/design/secrets/design.json` — The gate 'tests' leg command at line 185 now has --no-fail-fast
  - modified: `docs/design/lys-gate/design.json` — The one-line gate array at line 247 now has --no-fail-fast in its 'tests' leg command
- Checklist delivery:
  - [x] C2 — The 'tests' leg command of the directory, home, secrets and lys-gate design.json gate arrays is exactly `cargo test --workspace --all-features --no-fail-fast`, and every other leg is byte-identical to before. — All four gate arrays are updated, and every other leg is byte-identical
- Story delivery:
  - [x] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

### R3: Change the landing hook's test leg in .land/gates.sh

Structural. In .land/gates.sh the line `leg cargo test --workspace --all-features` becomes exactly `leg cargo test --workspace --all-features --no-fail-fast`, on the same line, after both clippy legs and before both doc legs, so the tests leg still carries --all-features. The leg() wrapper, the header comment, the other six leg lines and their order, and the exit logic do not change. THE SYSTEM SHALL NOT change any line of .land/gates.sh other than the tests leg's, SHALL NOT remove --all-features from that leg, and SHALL NOT gain a check of the branch name.

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

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. .land/gates.sh:19 is `leg cargo test --workspace --all-features --no-fail-fast`. Row 2: met. numstat shows 1/1. Row 3: met. `grep '^leg '` prints the seven legs in the required order, with the tests leg fifth. The wrapper, the header comment, the exit logic and every other line are unchanged.
- Deviation: (none)
- Files changed:
  - modified: `.land/gates.sh` — Line 19 is now `leg cargo test --workspace --all-features --no-fail-fast`
- Checklist delivery:
  - [x] C3 — .land/gates.sh's test leg line is exactly `leg cargo test --workspace --all-features --no-fail-fast`. — .land/gates.sh:19
- Story delivery:
  - [x] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

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

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. CLAUDE.md:99 is the only `^cargo test` line, and it now reads `cargo test --workspace --all-features --no-fail-fast`. Row 2: met. It matches .land/gates.sh:19 exactly with the `leg ` prefix removed. Row 3: met. numstat shows 1/1. The prose after the block was not edited.
- Deviation: (none)
- Files changed:
  - modified: `CLAUDE.md` — The test line of 'Gates before any commit' at line 99 is now `cargo test --workspace --all-features --no-fail-fast`
- Checklist delivery:
  - [x] C4 — CLAUDE.md's 'Gates before any commit' test line is exactly `cargo test --workspace --all-features --no-fail-fast`, the same command as .land/gates.sh's test leg. — CLAUDE.md:99 matches .land/gates.sh:19
- Story delivery:
  - [x] S2 (AI Agent, Runs the gates by hand before a commit) — As an agent running the gates by hand from CLAUDE.md, I want its test line to be the landing hook's test command so that my run measures what landing measures.

### R5: Add --no-fail-fast to CI's test step

Structural. In .github/workflows/ci.yml the Test step's `run: cargo test --workspace` becomes exactly `run: cargo test --workspace --no-fail-fast`, adding the one flag. Its env (LYS_REQUIRE_GO), the step's name, every other step and every other job do not change. THE SYSTEM SHALL NOT change any other line of ci.yml, and SHALL NOT add --all-features to the step: the missing --all-features is a recorded finding, not changed here.

**Acceptance:**
- grep -n 'run: cargo test' .github/workflows/ci.yml prints exactly one line, `46:        run: cargo test --workspace --no-fail-fast`.
- grep -c -- '--no-fail-fast' .github/workflows/ci.yml prints `1` at <change> and `0` at <change>~1.
- grep -c -- '--all-features' .github/workflows/ci.yml prints `0` at <change>.
- git diff <change>~1 <change> --numstat -- .github/workflows/ci.yml prints `1	1	.github/workflows/ci.yml`.

**Files:**
- modify: .github/workflows/ci.yml

**Checklist:**
- C5 — .github/workflows/ci.yml's Test step runs exactly `cargo test --workspace --no-fail-fast`.

**Stories:**
- S3 (Contributor, Reads CI results on a pull request) — As a contributor reading CI on a pull request, I want the test step to report every failing test binary so that I can fix them all from one run.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. `grep -n 'run: cargo test'` prints `46:        run: cargo test --workspace --no-fail-fast`. Row 2: met. The count of --no-fail-fast is now 1, and the base had 0 because the flag was absent. Row 3: met. The count of --all-features is 0. Row 4: met. numstat shows 1/1, and env LYS_REQUIRE_GO, the step name and every other step are unchanged.
- Deviation: (none)
- Files changed:
  - modified: `.github/workflows/ci.yml` — The Test step at line 46 now runs `cargo test --workspace --no-fail-fast`
- Checklist delivery:
  - [x] C5 — .github/workflows/ci.yml's Test step runs exactly `cargo test --workspace --no-fail-fast`. — ci.yml:46
- Story delivery:
  - [x] S3 (Contributor, Reads CI results on a pull request) — As a contributor reading CI on a pull request, I want the test step to report every failing test binary so that I can fix them all from one run.

### R6: Make the change one commit and prove no declared test leg lacks --no-fail-fast

WHEN the change is committed, THE SYSTEM SHALL carry every edit of R1 to R5 in exactly one commit, <change>, which changes the eight files R1 to R5 name and no other, and at <change> the grep over the six files named one by one, `grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/directory/design.json docs/design/home/design.json docs/design/secrets/design.json docs/design/lys-gate/design.json .land/gates.sh | grep -vc -- --no-fail-fast` SHALL print 0. The grep's command line and output are recorded in the pull request's evidence and in the proof document of R9. THE SYSTEM SHALL NOT add a script, a test or any other file to carry the check. No other commit on the card's branch SHALL change any of the eight files except docs/design/lys-gate/design.json, which the chain's own commits also write; for that file what is measured is its gate array alone, and no other commit on the card's branch SHALL change the gate array: it is identical at <base> and at <change>~1, and identical at <change> and at the card's head. The grep SHALL NOT be written over a glob of docs/design/*/design.json, and THE SYSTEM SHALL NOT change a design.json of a cluster that is not among the four R2 names.

**Acceptance:**
- At <change>, grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/directory/design.json docs/design/home/design.json docs/design/secrets/design.json docs/design/lys-gate/design.json .land/gates.sh | grep -vc -- --no-fail-fast prints `0`.
- At <change>~1, the same grep prints `6`.
- git show --name-only --format= <change> prints exactly these eight paths: .github/workflows/ci.yml, .land/gates.sh, CLAUDE.md, docs/design/directory/design.json, docs/design/home/design.json, docs/design/lys-gate/design.json, docs/design/project.json, docs/design/secrets/design.json.
- git log --format=%H <base>..HEAD -- .github/workflows/ci.yml .land/gates.sh CLAUDE.md docs/design/directory/design.json docs/design/home/design.json docs/design/project.json docs/design/secrets/design.json prints exactly one line, the hash of <change>.
- python3 -c "import json,subprocess,sys;g=lambda r:json.loads(subprocess.check_output(['git','show',r+':docs/design/lys-gate/design.json'])).get('gate');print(g(sys.argv[1])==g(sys.argv[2]))" <base> <change>~1 prints `True`, and the same command with <change> HEAD, run at the card's head, prints `True`.

**Checklist:**
- C6 — The changes of C1 to C5 are one commit, no other commit on the card's branch changes those files except that the lys-gate design.json's gate array alone is held unchanged outside that commit, and the grep over docs/design/project.json, the directory, home, secrets and lys-gate design.json files and .land/gates.sh, named one by one, for test legs without --no-fail-fast prints 0.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

#### R6 — Execution record

**Dev (recorded):**

- Status: blocked
- How: The eight edits of R1 to R5 are in the working tree with no other change. git status lists exactly those eight paths, each at 1/1. Run over the six files named one by one, the R6 grep prints 0 on the working tree now, and at the base it matches the six old command lines. The acceptance rows need <change> to be a commit, but these instructions say to make no commit, so the rows cannot be measured until the reviewed landing commits these eight files as one commit. The lys-gate gate array has not been changed by any other commit.
- Deviation: (none)
- Checklist delivery:
  - [ ] C6 — The changes of C1 to C5 are one commit, no other commit on the card's branch changes those files except that the lys-gate design.json's gate array alone is held unchanged outside that commit, and the grep over docs/design/project.json, the directory, home, secrets and lys-gate design.json files and .land/gates.sh, named one by one, for test legs without --no-fail-fast prints 0. — Waiting for the single landing commit; the grep prints 0 on the working tree
- Story delivery:
  - [ ] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others. — Depends on the commit

### R7: Keep the design gate green and show each changed tests leg's command in a gate round's log

WHEN the ledger measures one gate round at <change> for each of the four clusters R2 names, each round measured against that cluster's design.json over the repository's working tree at <change> (the directory round, the home round, the secrets round and the lys-gate round), THE SYSTEM SHALL report each round's tests leg green, and each round's log SHALL show its tests leg's logged command line containing 'cargo test' followed by '--no-fail-fast'; that logged line is what shows the leg is still classed as a battery, because the ledger matches its marker 'cargo test' against running command lines and logs each leg's command as '$ command'. The ledger measures one cluster's gate per round, so each changed leg is shown by the round measured against its own cluster. The directory and secrets gates declare every leg as requiring place:here, the place the ledger itself runs, so those two rounds are run by a ledger whose own place is the venue that serves heavy builds and full gates, and place:here is that venue for them. The lys-gate round is the green round R8 and R9 compare against. WHEN sh scripts/design/gate.sh runs at the card's head, THE SYSTEM SHALL exit 0. THE SYSTEM SHALL NOT change the ledger or the method to print a battery class, SHALL NOT show a cluster's changed leg by a round measured against another cluster's design.json, and SHALL NOT run any of the four rounds on the authoring machine.

**Acceptance:**
- sh scripts/design/gate.sh, run at the card's head, exits 0 and prints no line beginning `rendered markdown differs`.
- For each of the directory, home, secrets and lys-gate rounds at <change>, the round's log holds a line beginning `-- leg tests at venue ` followed immediately by the line `$ cargo test --workspace --all-features --no-fail-fast`, and that leg's part of the log ends with the lines `(exit 0)` and `(measured-green)`: four rounds, four such lines.
- git diff <base> HEAD --name-only -- scripts/design prints nothing.

**Checklist:**
- C7 — sh scripts/design/gate.sh exits 0 at the card's head, and four green gate rounds at the change, one measured against each of the directory, home, secrets and lys-gate design.json files, each show the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

#### R7 — Execution record

**Dev (recorded):**

- Status: blocked
- How: The four green gate rounds, for directory, home, secrets and lys-gate, must run against <change> at the venue that serves heavy builds and full gates. The brief forbids running them on this machine, and the instructions forbid running any gate by hand. The workflow runs scripts/design/gate.sh itself. The re-render showed no markdown drift, so I expect it to stay green. scripts/design is unchanged.
- Deviation: (none)
- Checklist delivery:
  - [ ] C7 — sh scripts/design/gate.sh exits 0 at the card's head, and four green gate rounds at the change, one measured against each of the directory, home, secrets and lys-gate design.json files, each show the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`. — Needs venue gate rounds at <change>
- Story delivery:
  - [ ] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others. — Depends on the rounds

### R8: Run one gate round over a local scratch branch with one planted failing test

WHEN one gate round runs over the local branch scratch/lysgate-001, cut from <change> in a local worktree at the venue that serves heavy builds and full gates, measured against docs/design/lys-gate/design.json, the same gate and venue placement as the green round it is compared with, THE SYSTEM SHALL show in the tests leg's part of that round's log the planted failure together with every other test binary's result, and the tests leg SHALL exit non-zero. The scratch branch adds one commit on <change> that adds one test, scratch_planted_failure, whose body is assert_eq!(1, 2), to crates/lys/tests/certified_attestation_tests.rs: crates/lys is the first entry of Cargo.toml's [workspace] members, so every binary after it runs later in the log, and adding a test to an existing test binary adds no binary. The baseline is R7's lys-gate round at <change>, called the green round below. The tests leg's part of a log runs from its line beginning `-- leg tests at venue ` to the next line beginning `-- leg `, or to the end of the log. THE SYSTEM SHALL NOT push the scratch branch to origin, SHALL NOT open a pull request for it, SHALL NOT land or merge it, and SHALL NOT add the planted test to the card's branch; after the round the worktree and the branch are deleted.

**Acceptance:**
- The scratch round's log holds a line beginning `-- leg tests at venue ` followed immediately by the line `$ cargo test --workspace --all-features --no-fail-fast`, the same two lines the green round's log holds for its tests leg, and the round's record names docs/design/lys-gate/design.json as the design it was measured against.
- In the tests leg's part of the scratch round's log, the number of lines containing `test result: ` equals the number in the tests leg's part of the green round's log at <change>.
- In the tests leg's part of the scratch round's log, exactly 1 line contains `test result: FAILED`, and in the green round's log at <change> no line contains `test result: FAILED`.
- The tests leg's part of the scratch round's log contains the line `test scratch_planted_failure ... FAILED` and the line `(exit 101)`.
- After the round, git ls-remote origin 'refs/heads/scratch/*' prints nothing, and git branch --list 'scratch/*' in the clone that held the worktree prints nothing.
- git log --format=%H <base>..HEAD -- crates prints nothing.

**Checklist:**
- C8 — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the lys-gate green round at the same head, and no ref under scratch/ exists on origin after it.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

#### R8 — Execution record

**Dev (recorded):**

- Status: blocked
- How: The scratch round needs <change> to exist and must run in a local worktree at the heavy-build venue. It cannot be run from this authoring session. No scratch branch was created and no crate source was changed.
- Deviation: (none)
- Checklist delivery:
  - [ ] C8 — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the lys-gate green round at the same head, and no ref under scratch/ exists on origin after it. — Needs <change> and the venue
- Story delivery:
  - [ ] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others. — Depends on the scratch round

### R9: Write the proof document that names the scratch round's log

Structural. docs/design/lys-gate/PROOF-LYSGATE-001.md records, in a commit after <change>: the R6 grep's command line and its output; for each of R7's four rounds (directory, home, secrets, lys-gate) its round id, the cluster it was measured against, its log's path in the ledger's gate log store and that log's SHA-256; the scratch round's id, its log's path in the gate log store and its SHA-256; the counts R8 measures (the `test result: ` lines in the tests leg's part of each log, and the `test result: FAILED` lines in the scratch log); the command git ls-remote origin 'refs/heads/scratch/*' with its empty output after the round; and, under a section headed `Findings for other cards`, the path of every docs/design/*/design.json at the card's head, other than the four R2 names, whose gate has a leg whose command contains 'cargo test' and not '--no-fail-fast', one path per line, as a finding for that cluster's card. THE SYSTEM SHALL NOT copy any line of any of the five rounds' logs into the document, and SHALL NOT name a path outside the gate log store for a log.

**Acceptance:**
- shasum -a 256 run on each of the five log paths the proof names prints the SHA-256 the proof names beside it, a string of 64 lowercase hexadecimal characters.
- The proof names four green round ids, one for each of the directory, home, secrets and lys-gate clusters, each beside the cluster it was measured against, its log's path in the ledger's gate log store and that log's SHA-256, and names the scratch round's id beside docs/design/lys-gate/design.json, its log's path and its SHA-256: five rounds, five paths, five SHA-256s.
- The `test result: ` counts the proof names for the scratch round and the green round are equal to each other and to the counts R8 measures, and the `test result: FAILED` count it names is 1.
- Under bash, for each of the five log paths the proof names, grep -Fxc -f <(grep -v '^[[:space:]]*$' <that log path>) docs/design/lys-gate/PROOF-LYSGATE-001.md prints `0`.
- The proof holds the line `grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/directory/design.json docs/design/home/design.json docs/design/secrets/design.json docs/design/lys-gate/design.json .land/gates.sh | grep -vc -- --no-fail-fast` followed immediately by the line `prints: 0`.
- git log --format=%H <change>..HEAD -- docs/design/lys-gate/PROOF-LYSGATE-001.md prints at least one line, and git log --format=%H <base>..<change> -- docs/design/lys-gate/PROOF-LYSGATE-001.md prints nothing.
- At the card's head, python3 -c "import glob,json;named={'docs/design/directory/design.json','docs/design/home/design.json','docs/design/secrets/design.json','docs/design/lys-gate/design.json'};[print(p) for p in sorted(glob.glob('docs/design/*/design.json')) if p not in named and any('cargo test' in l['command'] and '--no-fail-fast' not in l['command'] for t in json.load(open(p)).get('gate',[]) for l in t['legs'])]" prints exactly the paths listed under the proof's section headed `Findings for other cards`, in the same order, and that section holds no other line.

**Files:**
- create: docs/design/lys-gate/PROOF-LYSGATE-001.md

**Checklist:**
- C9 — docs/design/lys-gate/PROOF-LYSGATE-001.md names each of the four green rounds and the scratch round by round id, log path in the gate log store and SHA-256, and the scratch round's counts, holds no line of any of those logs, and lists by file name every other cluster's design.json whose test leg lacks --no-fail-fast as a finding for that cluster's card.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

#### R9 — Execution record

**Dev (recorded):**

- Status: blocked
- How: PROOF-LYSGATE-001.md must name the round ids, log-store paths and SHA-256s of the five venue rounds (R7, R8), and must be committed after <change>. None of those rounds exist yet, so the document cannot be written truthfully. For its 'Findings for other cards' section: docs/design holds only the four named design.json files today, so that section would list no paths.
- Deviation: (none)
- Checklist delivery:
  - [ ] C9 — docs/design/lys-gate/PROOF-LYSGATE-001.md names each of the four green rounds and the scratch round by round id, log path in the gate log store and SHA-256, and the scratch round's counts, holds no line of any of those logs, and lists by file name every other cluster's design.json whose test leg lacks --no-fail-fast as a finding for that cluster's card. — Needs the five rounds' logs
- Story delivery:
  - [ ] S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others. — Depends on R7 and R8

## Boundaries

- SHALL NOT change any gate leg's command other than the test leg, nor any leg's name, order, requirements or cadence.
- SHALL NOT remove --all-features from any gate test leg, and SHALL NOT add --all-features to CI's test step.
- SHALL NOT raise any timeout or split any test.
- SHALL NOT edit any other cluster's file under docs/design/*/briefs (this card's own docs/design/lys-gate/briefs excepted) or any release document.
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
- Compare docs/design/lys-gate/design.json's gate array at <base> with <change>~1, and at <change> with the card's head: both pairs are equal.
- Run sh scripts/design/gate.sh at the card's head: it exits 0.
- Read the logs of R7's directory, home, secrets and lys-gate rounds at <change>: in each, the tests leg's '$ command' line is `$ cargo test --workspace --all-features --no-fail-fast` and the leg is green.
- Read the scratch round's log against the lys-gate round's: the `test result: ` counts in the tests leg's parts are equal, exactly one of the scratch lines contains `test result: FAILED`, and the tests leg exits 101.
- Run git ls-remote origin 'refs/heads/scratch/*': it prints nothing.
- Check each of the five SHA-256s PROOF-LYSGATE-001.md names against the log it names with shasum -a 256.
