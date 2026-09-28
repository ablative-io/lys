# lys-gate — what was asked, what it means, and what was written

## The words, as they were typed

The lys gate's test leg stops at the first failing test binary. Five files declare it as `cargo test --workspace --all-features`: docs/design/project.json (the project gate), docs/design/directory/design.json, docs/design/home/design.json and docs/design/secrets/design.json (the cluster gates), and .land/gates.sh (the landing hook). A run that fails in one crate reports nothing about the binaries and crates after it, so a gate can go red for one reason while hiding others.

This card changes that leg in all five files to `cargo test --workspace --all-features --no-fail-fast`, in one commit. Every test binary then runs, every failure is reported in the one gate run, and the leg still exits non-zero if any test fails. Nothing else in the gate changes: the other legs, their order and their requirements stay exactly as they are. No timeout is raised and no test is split.

It is done when all five files declare the new command and scripts/design/gate.sh stays green. A check over docs/design/project.json, every docs/design/*/design.json and .land/gates.sh must show that no test leg there still runs `cargo test --workspace --all-features` without --no-fail-fast. The ledger that runs the gate recognises a battery by the text "cargo test" in a leg's command, so each changed leg must still be classed as a battery, shown in the gate log. One gate round over a scratch branch that adds a single failing test in one crate must show in its log that failure together with every other test binary measured; that branch is never landed. The briefs under docs/design/*/briefs and the release documents quote cargo commands as acceptance text or instructions, not as gate legs, and this card does not change them.

Keep to the method (scripts/design/validate.py, check-coverage.py and render-cluster.py, run by scripts/design/gate.sh), and render any cluster whose design.json changes. If the survey finds a sentence here open or contradicted by the repository as it stands, it quotes that sentence whole as a question for the lead.

Rulings of the lead, Archie, given on 27 September 2026 to the run 33d9ce6d-afa7-44db-9e71-fbfcbdd480dd in answer to its rounds. That run was fired with cluster lys-core; its round 2 ruling moved the documents to docs/design/lys-gate, so the shape and roadmap checks refused the draft at df7e5b0 under the wrong cluster and the run was cancelled and re-fired with cluster lys-gate. The draft at df7e5b0 on draft/lys-core/33d9ce6d-afa7-44db-9e71-fbfcbdd480dd is the text to carry, moved whole. They are settled here, and the author reopens none of them. R1 (round 1, key survey-question-a58e6386bb6f): The logged command line. The acceptance shows that each changed leg's '$ command' line in the gate log contains cargo test followed by --no-fail-fast, which is what BATTERY_MARKERS matches in the running process. The method project is not changed. The words' 'classed as a battery, shown in the gate log' is read that way, by an appended line that cites gate.py:201-223 and 409-437, not a rewrite. Answered by Archie, lead for lys. R2 (round 1, key survey-question-d98d02682017): Yes, CLAUDE.md changes too, so .land/gates.sh still matches it exactly as its header says. Add CLAUDE.md's 'Gates before any commit' block to the files by an appended line. Only its test line changes, to cargo test --workspace --all-features --no-fail-fast. Its other lines and their order stay as they are. An acceptance line diffs the test leg in CLAUDE.md against .land/gates.sh's. Answered by Archie. R3 (round 1, key survey-question-ab835dbbd3d5): CI changes too, by the same one flag. Its cargo test --workspace step at ci.yml:46 gains --no-fail-fast, so a pull request shows every failing binary. Its missing --all-features is a separate difference: name it in the brief as a finding and do not change it here. Add ci.yml to the files by an appended line. Answered by Archie, 27 September 2026. R4 (round 2, key author-question-0499357d9bc7): Neither. Move this card's documents to a new cluster, docs/design/lys-gate, which holds only this card's design, checklist, stories and brief. Leave docs/design/lys-core untouched: this card adds no design.json, renders nothing there, and removes any lys-core file its draft added. The lys-core cluster has one owner, card 04213a71 (the ast-grep leg). As ruled in its round 2, that card renames the hand-written lys-core documents to *-PRE-METHOD.md before it renders, so the earlier design is kept, not replaced. Two cards must not both create lys-core/design.json. The brief id is the next free for the new cluster's prefix, and the design leg must exit 0 at this card's head. Answered by Archie, lead for lys. R5 (round 3): the briefs under docs/design/*/briefs and the release documents that quote cargo commands are not changed; the brief says so and choices_made records it. R6 (round 3): no timeout is raised and no test is split; the brief says so. R7 (round 3): the scratch branch with the planted failing test is never landed; the brief says so and the branch name carries scratch/ so the landing hook refuses it. R8 (round 3): the change to the five files is one commit, with the exact new command; the brief says so. R9 (round 3): 'every other test binary measured' is shown by counting 'test result:' lines in the scratch round's log and comparing them with the count in a green round at the same head; the two counts are equal and the scratch log holds exactly one FAILED line. Recorded in choices_made. R10 (round 3): the planted failing test goes in the crate whose test binaries run first in the workspace's member order, so every binary after it is visible in the log; the brief names that crate from Cargo.toml's members. The round's log is kept as evidence: the proof document docs/design/lys-gate/PROOF-LYSGATE-001.md names the log's path in the ledger's gate log store, its SHA-256 and its counts, never its content. Recorded in choices_made. R11 (round 3): the check that no test leg lacks --no-fail-fast is a grep recorded in the pull request's evidence and in the proof document, printing 0: grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/*/design.json .land/gates.sh | grep -vc -- --no-fail-fast. No script is added. Recorded in choices_made. R12 (round 3): a new fix row carries this card. Its id, and ADR-038's, follow the rule of 26 September at 20:31: the next free id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote and a fetch of every head immediately before writing, with the heads read recorded in the dev record. The brief that wrote an id first keeps it: RM-019 is held by draft/directory/e5171dbd and draft/home/06103633, so this brief takes the next free past them. This run's cluster input is lys-gate, so the row's cluster and the brief's path agree and the shape and roadmap checks hold. Recorded in choices_made. R13 (round 3): lys-core's DESIGN.md:142 and CHECKLIST C63 are left alone. They are acceptance prose in a cluster with no design.json gate, owned by the ast-grep leg card 04213a71, and the brief names them as unchanged with that reason. Recorded in choices_made.

## What the survey found, and its angles

The card adds `--no-fail-fast` to the lys gate's test leg, so one gate run reports every failing test binary across every crate instead of stopping at the first red binary. The flag goes into the five declared gate files (docs/design/project.json, the three cluster design.json files, .land/gates.sh), plus CLAUDE.md's 'Gates before any commit' block (R2) and CI's test step (R3), all in one commit. The card's documents live in a new cluster, docs/design/lys-gate, carried over from draft df7e5b0. The proof is a grep that prints 0, a green design gate, the logged '$ command' lines, and one scratch round with a planted failure whose log shows every binary.

### What the tree holds

- `docs/design/project.json:29` — The project gate's `tests` leg (requires rust-build, cadence round) reads `cargo test --workspace --all-features`. It is one of the five declarations to change, and validate.py checks it.
- `docs/design/directory/design.json:716` — The directory cluster's gate test leg, with the same command. Changing it means directory's rendered markdown must be re-rendered, or gate.sh's cmp step goes red.
- `docs/design/home/design.json:776` — The home cluster's gate test leg, with the same command. Needs a re-render of the home cluster for the same reason.
- `docs/design/secrets/design.json:185` — The secrets cluster's gate test leg, with the same command. Needs a re-render of the secrets cluster for the same reason.
- `.land/gates.sh:19` — The landing hook's line `leg cargo test --workspace --all-features`. The file is 22 lines. Its `leg` helper already runs every leg after a red one and echoes `--- $* ---`, so the new flag shows in the log. Its header says it matches CLAUDE.md exactly. It has no check on the branch name.
- `CLAUDE.md:99` — The test line of 'Gates before any commit'. R2 adds it to the files, so .land/gates.sh still matches it as its header says. That block's text says 'All five clean' over six commands. This is not changed here.
- `.github/workflows/ci.yml:46` — The CI step `run: cargo test --workspace` (env LYS_REQUIRE_GO=1). R3 adds only `--no-fail-fast`. The missing `--all-features` is to be named as a finding and left unchanged.
- `scripts/design/gate.sh` — The design leg (28 lines). It runs validate.py on decisions.json and project.json. For every cluster with a design.json it runs validate.py, check-coverage.py and render-cluster.py, and cmp's the rendered markdown against what is committed. The new docs/design/lys-gate cluster will be walked by this loop.
- `/Users/tom/Developer/ablative/tools/design-system/workers/ds2_ledger/gate.py:201-223,409-437` — BATTERY_MARKERS = ("cargo test", ...) is matched against `ps -axo command` lines. _run_leg writes `$ {placed.leg.command}` into the leg log. R1 cites these line ranges, and they match the file as it stands (502 lines).
- `Cargo.toml:1-9` — The workspace members, in this order: crates/lys, lys-anchor, lys-anchor-cli, lys-core, lys-home, lys-log-store. R10 picks the crate for the planted failure from this list.
- `docs/design/lys-core/DESIGN.md:142 and docs/design/lys-core/CHECKLIST.md:99 (C63)` — Prose acceptance that quotes `cargo test --workspace`. Under R13 it stays unchanged, and the brief names it with its reason.
- `docs/design/roadmap.json, docs/design/decisions.json` — The new fix row and ADR-038 (R12) are written here. The highest ids on main are RM-016 and ADR-018.

### What was already decided

- CLAUDE.md 'Gates before any commit' — The feature-full test run is the gate, and 81 tests compile out without it. This card keeps --all-features and only adds --no-fail-fast.
- CLAUDE.md 'A test needs a second party' — 'Count what fired, not what passed.' The R9 count of 'test result:' lines and the single FAILED line apply this rule.
- .land/gates.sh header — 'exactly as CLAUDE.md "Gates before any commit" lists them'. That is why R2 changes CLAUDE.md in the same commit.
- scripts/design/gate.sh — Every cluster with a design.json must render to its committed markdown, so each cluster whose design.json changes must be re-rendered.
- lys-core cluster (no design.json) — Owned by card 04213a71 (ast-grep leg, draft/lys-core/04213a71 at 65fad48). R4 keeps this card out of it.
- ds2_ledger gate.py BATTERY_MARKERS — A leg counts as a battery when its command line contains 'cargo test'. Adding --no-fail-fast keeps that substring.
- RM-016 — The latest row on main. It is the pattern a fix or design row follows for a small documents-plus-config change.

### What was measured

- Gate declarations running `cargo test --workspace --all-features` without --no-fail-fast: 5 (project.json:29, directory/design.json:716, home/design.json:776, secrets/design.json:185, .land/gates.sh:19)
- Further files the rulings add: 2 (CLAUDE.md:99; .github/workflows/ci.yml:46, which reads `cargo test --workspace` with no --all-features)
- Total files the one commit changes, before cluster renders and lys-gate documents: 7
- Clusters with a design.json today: 3 (directory, home, secrets). lys-core, lys-anchor, lys-log-store and identity have none.
- docs/design/lys-gate on this tree: does not exist
- Workspace members: 6. The first in Cargo.toml order, and first alphabetically by package, is crates/lys.
- Integration test files per crate (tests/*.rs): lys 4, lys-anchor 5, lys-anchor-cli 1, lys-core 11, lys-home 7, lys-log-store 0. Four crates have a src/main.rs binary target.
- Highest ids on main: RM-016, ADR-018
- Open draft/brief branches on origin: about 60 heads, including 4 under draft/lys-core/*. The draft at df7e5b0 is at refs/heads/draft/lys-core/33d9ce6d-afa7-44db-9e71-fbfcbdd480dd, and that object is not in the local clone.
- Briefs and docs quoting `cargo test --workspace` (R5, left unchanged): 29 files under docs/design/*/briefs and docs/*.md
- gate.py line ranges R1 cites: 201 (BATTERY_MARKERS), 219 (match), 409-437 (_run_leg logs '$ command'). All match the 502-line file.
- Branch-name checks in .land/gates.sh: 0
- `scratch/` handling found in the design-system workers, aion workflows or cambium plugin: 0 matches (grep)

### What it means for the other projects

- method — Nothing changes (R1). ds2_ledger/gate.py BATTERY_MARKERS still matches, because the command keeps 'cargo test'. validate.py, check-coverage.py and render-cluster.py are used as they are.
- cambium — The card runs through the lys board's chain (brief_card → sign-off → card_build_v3 → src_pr → src_land). The scratch round needs a gate round fired over a scratch/ branch that is never proposed or landed.
- aion — The gate round and the landing run on aion workflows, with the heavy battery on Dean's laptop. The scratch round's log is kept in the ledger's gate log store and referenced by path and SHA-256.

### The decisions it stands on

-  (new) — ADR-038 under R12: the lys gate's test leg runs with --no-fail-fast, so one round reports every failing binary. Its id must be re-derived from main plus every open head.
- ADR-004 (honour) — The card changes only lys's own gate declarations, and every project still stands alone.

### What it requires

- docs/design/project.json, directory/design.json, home/design.json, secrets/design.json and .land/gates.sh each declare `cargo test --workspace --all-features --no-fail-fast` as their test leg.
- CLAUDE.md's 'Gates before any commit' test line reads `cargo test --workspace --all-features --no-fail-fast`, its other lines stay in their order, and a diff of that line against .land/gates.sh's leg shows they match.
- .github/workflows/ci.yml's Test step reads `cargo test --workspace --no-fail-fast`, with --all-features still absent, and the brief names that as a finding.
- All seven file changes are one commit.
- The R11 grep prints 0 at the card's head.
- sh scripts/design/gate.sh exits 0 at the card's head, with directory, home and secrets re-rendered and the new lys-gate cluster validating and rendering clean.
- The gate log shows the changed test leg's '$ cargo test --workspace --all-features --no-fail-fast' line.
- A scratch round with one planted failing test in crates/lys shows exactly one FAILED line and as many 'test result:' lines as a green round at the same head.
- docs/design/lys-gate/PROOF-LYSGATE-001.md names the scratch log's path in the gate log store, its SHA-256 and its counts, and none of its content.
- A new fix row and ADR carry next-free ids, and the heads read to derive them are recorded in the dev record.
- docs/design/lys-core is unchanged, and any lys-core file the df7e5b0 draft added is removed.

### What must not change

- The other legs (fmt, both clippys, both docs, design), their order and their `requires` stay exactly as they are.
- --all-features stays on every gate test leg, and CI does not gain it here.
- No timeout is raised and no test is split.
- The briefs under docs/design/*/briefs, the release documents and lys-core DESIGN.md:142 and C63 stay unchanged.
- The method project (design-system, including gate.py) stays unchanged.
- The scratch branch and its planted test are never landed.
- No script is added for the check.
- No lys-core/design.json is created.

### What we must put in place first

- Fetch the df7e5b0 draft (refs/heads/draft/lys-core/33d9ce6d-…), which is not in this clone, so it can be carried over whole.
- Fetch every brief/* and draft/* head just before writing, to derive the next free RM and ADR ids.
- Make sure Dean's laptop venue can run a gate round over a scratch/ branch and keep its log in the ledger's gate log store.

### The risks

- The landing hook as it stands does not refuse scratch/ branches, so a scratch branch pushed to a landable path could land the planted failure.
- `--no-fail-fast` does not keep going past a compile error. A planted test that fails to build would show one error and no binaries, which is the wrong evidence.
- Doc-test 'test result:' lines count towards the R9 totals, so a green round and a scratch round taken at different heads or with different features would not compare.
- Re-rendering the three clusters could pull in unrelated drift if another card changes those design.json files first, causing a merge conflict with the many open directory and home drafts.
- The id race: open heads may already hold the next RM or ADR, and ADR-038 as named may not be free.
- With every binary running, a red gate round takes longer, which the ledger's quiet-box interval may notice.

### Still open

- .land/gates.sh has no branch check, and no refusal of scratch/ turned up in the ledger workers or cambium. What refuses to land the scratch branch: a named mechanism to verify or add, or only never opening a PR/land for it? The sentence of the words it stands on: "R7 (round 3): the scratch branch with the planted failing test is never landed; the brief says so and the branch name carries scratch/ so the landing hook refuses it.". Why only the lead can settle it: .land/gates.sh (22 lines) only runs the seven legs and never reads the branch. A grep for 'scratch/' in tools/design-system/workers, stack/aion and apps/cambium found no refusal. As things stand, the name alone would not stop a landing.

### The units beyond the first

- CI test step gains --all-features to match the gate — R3 names this as a separate finding. It changes which 81 tests CI runs and needs its own row.
- The landing path refuses scratch/ branches — R7 assumes such a refusal, and nothing in .land/gates.sh or the ledger provides one. Adding it belongs to the landing chain, not to this card.
- CLAUDE.md 'All five clean' over six commands — A standing wording mismatch in the gate block. R2 keeps this card to the test line.

### The smallest complete shape

One commit, landed through the chain. It adds --no-fail-fast to the test leg in the five gate files plus CLAUDE.md and ci.yml, re-renders directory, home and secrets, and adds the docs/design/lys-gate cluster (design, checklist, stories, brief LYSGATE-001, PROOF-LYSGATE-001.md) with the new fix row and ADR. Its evidence is the R11 grep printing 0, gate.sh green, the logged command lines, and the counts from one scratch round that is never landed.

## The roadmap row

- **RM-024** — Run every test binary in the gate's test leg with --no-fail-fast (fix, idea)
- Summary: The gate's test leg stops at the first failing test binary, so a red round hides every failure after the first. One commit changes it to `cargo test --workspace --all-features --no-fail-fast` in docs/design/project.json, the directory, home, secrets and lys-gate design.json gate arrays and .land/gates.sh, and, by the lead's rulings, in CLAUDE.md's 'Gates before any commit' block (so .land/gates.sh still matches it exactly) and CI's test step (which gains the one flag only; its missing --all-features is a finding left unchanged). Nothing else in any gate moves. Proved by a grep over the declared legs that prints 0, the green gate round's '$ command' line for the tests leg, one gate round over a local scratch branch with a planted failing test in crates/lys that never reaches origin, and a proof document that names that round's log by path, SHA-256 and counts.
- Asked by: tom on 2026-09-27T09:58:00+10:00
- Context: The gate test-leg card on the Lys board, surveyed against lys main 7b53625 and re-fired with cluster lys-gate after run 33d9ce6d, fired with cluster lys-core, was cancelled; the lead's rulings of its three rounds are carried in the words, and the lead's answer to this run's survey drops the landing-hook refusal of scratch/ names: the scratch branch is kept from landing by never reaching origin. The draft at df7e5b0 on draft/lys-core/33d9ce6d-afa7-44db-9e71-fbfcbdd480dd is carried whole and updated to those rulings.
- Quote: The lys gate's test leg stops at the first failing test binary. Five files declare it as `cargo test --workspace --all-features`: docs/design/project.json (the project gate), docs/design/directory/design.json, docs/design/home/design.json and docs/design/secrets/design.json (the cluster gates), and .land/gates.sh (the landing hook). A run that fails in one crate reports nothing about the binaries and crates after it, so a gate can go red for one reason while hiding others.

This card changes that leg in all five files to `cargo test --workspace --all-features --no-fail-fast`, in one commit. Every test binary then runs, every failure is reported in the one gate run, and the leg still exits non-zero if any test fails. Nothing else in the gate changes: the other legs, their order and their requirements stay exactly as they are. No timeout is raised and no test is split.

It is done when all five files declare the new command and scripts/design/gate.sh stays green. A check over docs/design/project.json, every docs/design/*/design.json and .land/gates.sh must show that no test leg there still runs `cargo test --workspace --all-features` without --no-fail-fast. The ledger that runs the gate recognises a battery by the text "cargo test" in a leg's command, so each changed leg must still be classed as a battery, shown in the gate log. One gate round over a scratch branch that adds a single failing test in one crate must show in its log that failure together with every other test binary measured; that branch is never landed. The briefs under docs/design/*/briefs and the release documents quote cargo commands as acceptance text or instructions, not as gate legs, and this card does not change them.

Keep to the method (scripts/design/validate.py, check-coverage.py and render-cluster.py, run by scripts/design/gate.sh), and render any cluster whose design.json changes. If the survey finds a sentence here open or contradicted by the repository as it stands, it quotes that sentence whole as a question for the lead.

Rulings of the lead, Archie, given on 27 September 2026 to the run 33d9ce6d-afa7-44db-9e71-fbfcbdd480dd in answer to its rounds. That run was fired with cluster lys-core; its round 2 ruling moved the documents to docs/design/lys-gate, so the shape and roadmap checks refused the draft at df7e5b0 under the wrong cluster and the run was cancelled and re-fired with cluster lys-gate. The draft at df7e5b0 on draft/lys-core/33d9ce6d-afa7-44db-9e71-fbfcbdd480dd is the text to carry, moved whole. They are settled here, and the author reopens none of them. R1 (round 1, key survey-question-a58e6386bb6f): The logged command line. The acceptance shows that each changed leg's '$ command' line in the gate log contains cargo test followed by --no-fail-fast, which is what BATTERY_MARKERS matches in the running process. The method project is not changed. The words' 'classed as a battery, shown in the gate log' is read that way, by an appended line that cites gate.py:201-223 and 409-437, not a rewrite. Answered by Archie, lead for lys. R2 (round 1, key survey-question-d98d02682017): Yes, CLAUDE.md changes too, so .land/gates.sh still matches it exactly as its header says. Add CLAUDE.md's 'Gates before any commit' block to the files by an appended line. Only its test line changes, to cargo test --workspace --all-features --no-fail-fast. Its other lines and their order stay as they are. An acceptance line diffs the test leg in CLAUDE.md against .land/gates.sh's. Answered by Archie. R3 (round 1, key survey-question-ab835dbbd3d5): CI changes too, by the same one flag. Its cargo test --workspace step at ci.yml:46 gains --no-fail-fast, so a pull request shows every failing binary. Its missing --all-features is a separate difference: name it in the brief as a finding and do not change it here. Add ci.yml to the files by an appended line. Answered by Archie, 27 September 2026. R4 (round 2, key author-question-0499357d9bc7): Neither. Move this card's documents to a new cluster, docs/design/lys-gate, which holds only this card's design, checklist, stories and brief. Leave docs/design/lys-core untouched: this card adds no design.json, renders nothing there, and removes any lys-core file its draft added. The lys-core cluster has one owner, card 04213a71 (the ast-grep leg). As ruled in its round 2, that card renames the hand-written lys-core documents to *-PRE-METHOD.md before it renders, so the earlier design is kept, not replaced. Two cards must not both create lys-core/design.json. The brief id is the next free for the new cluster's prefix, and the design leg must exit 0 at this card's head. Answered by Archie, lead for lys. R5 (round 3): the briefs under docs/design/*/briefs and the release documents that quote cargo commands are not changed; the brief says so and choices_made records it. R6 (round 3): no timeout is raised and no test is split; the brief says so. R7 (round 3): the scratch branch with the planted failing test is never landed; the brief says so and the branch name carries scratch/ so the landing hook refuses it. R8 (round 3): the change to the five files is one commit, with the exact new command; the brief says so. R9 (round 3): 'every other test binary measured' is shown by counting 'test result:' lines in the scratch round's log and comparing them with the count in a green round at the same head; the two counts are equal and the scratch log holds exactly one FAILED line. Recorded in choices_made. R10 (round 3): the planted failing test goes in the crate whose test binaries run first in the workspace's member order, so every binary after it is visible in the log; the brief names that crate from Cargo.toml's members. The round's log is kept as evidence: the proof document docs/design/lys-gate/PROOF-LYSGATE-001.md names the log's path in the ledger's gate log store, its SHA-256 and its counts, never its content. Recorded in choices_made. R11 (round 3): the check that no test leg lacks --no-fail-fast is a grep recorded in the pull request's evidence and in the proof document, printing 0: grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/*/design.json .land/gates.sh | grep -vc -- --no-fail-fast. No script is added. Recorded in choices_made. R12 (round 3): a new fix row carries this card. Its id, and ADR-038's, follow the rule of 26 September at 20:31: the next free id after main's highest and every open brief/* and draft/* branch's, checked with git ls-remote and a fetch of every head immediately before writing, with the heads read recorded in the dev record. The brief that wrote an id first keeps it: RM-019 is held by draft/directory/e5171dbd and draft/home/06103633, so this brief takes the next free past them. This run's cluster input is lys-gate, so the row's cluster and the brief's path agree and the shape and roadmap checks hold. Recorded in choices_made. R13 (round 3): lys-core's DESIGN.md:142 and CHECKLIST C63 are left alone. They are acceptance prose in a cluster with no design.json gate, owned by the ast-grep leg card 04213a71, and the brief names them as unchanged with that reason. Recorded in choices_made.
- Cluster: lys-gate; briefs: LYSGATE-001
- Notes: Ids under the ruling of 26 September at 20:31, the next free after main's highest and every open brief/* and draft/* head's: RM-024 and ADR-038. Main 7b53625 ends at RM-016 and ADR-018. Heads read with git ls-remote origin and a fetch of every brief/* and draft/* head, then read again immediately before writing: 18 brief/* and 47 draft/* heads on the second reading, one more draft head than the first (draft/home/d26d0f3b, RM-017, ADR-019) and one moved (draft/home/0e229c29, RM-020, ADR-031). The highest RM on any other head is RM-023 (draft/lys-core/95377829) and the highest ADR is ADR-037 (draft/directory/82386e20); RM-019, which this card's draft df7e5b0 carried, is kept by draft/directory/e5171dbd and draft/home/06103633, which wrote it first. Read a third time before this round's amendment: 18 brief/* and 49 draft/* heads, the highest RM on any other head still RM-023 and the highest ADR still ADR-037; RM-024 and ADR-038 are on no head but this card's own drafts, df7e5b0 and draft/lys-gate/7dc46a76. No head holds a lys-gate cluster but that draft, so LYSGATE-001 is the first id under the prefix. docs/design/lys-core is untouched: its one owner is the ast-grep leg card 04213a71. Further units, not written: CI test step gains --all-features to match the gate; The landing path refuses scratch/ branches; CLAUDE.md 'All five clean' over six commands.

## The design

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

## Decisions

- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-038 — The gate's test leg runs every test binary; it never stops at the first failure — Every declaration of the gate's test leg runs `cargo test --workspace --all-features --no-fail-fast`: project.json, every cluster design.json gate, .land/gates.sh and CLAUDE.md's 'Gates before any commit' block; CI's test step gains the same one flag and nothing else. Rejected: keeping cargo's default fail-fast run, which is shorter on a red round but reports only the first failing binary.

## Goals

- docs/design/project.json, the directory, home, secrets and lys-gate design.json gate arrays and .land/gates.sh declare their test leg as exactly `cargo test --workspace --all-features --no-fail-fast`, and the grep over them prints 0.
- CLAUDE.md's 'Gates before any commit' test line and .land/gates.sh's test leg are the same command, `cargo test --workspace --all-features --no-fail-fast`.
- CI's test step runs `cargo test --workspace --no-fail-fast`.
- sh scripts/design/gate.sh exits 0 at the card's head.
- One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the green round at the same head, and no scratch/ ref exists on origin after it.

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

### R7: Keep the design gate green and show each changed tests leg's command in a gate round's log

WHEN the ledger measures one gate round at <change> for each of the four clusters R2 names, each round measured against that cluster's design.json over the repository's working tree at <change> (the directory round, the home round, the secrets round and the lys-gate round), THE SYSTEM SHALL report each round's tests leg green, and each round's log SHALL show its tests leg's logged command line containing 'cargo test' followed by '--no-fail-fast'; that logged line is what shows the leg is still classed as a battery, because the ledger matches its marker 'cargo test' against running command lines and logs each leg's command as '$ command'. The ledger measures one cluster's gate per round, so each changed leg is shown by the round measured against its own cluster. The directory and secrets gates declare every leg as requiring place:here, the place the ledger itself runs, so those two rounds are run by a ledger whose own place is the venue that serves heavy builds and full gates, and place:here is that venue for them. The lys-gate round is the green round R8 and R9 compare against. WHEN sh scripts/design/gate.sh runs at the card's head, THE SYSTEM SHALL exit 0. THE SYSTEM SHALL NOT change the ledger or the method to print a battery class, SHALL NOT show a cluster's changed leg by a round measured against another cluster's design.json, and SHALL NOT run any of the four rounds on the authoring machine.

**Acceptance:**
- sh scripts/design/gate.sh, run at the card's head, exits 0 and prints no line beginning `rendered markdown differs`.
- For each of the directory, home, secrets and lys-gate rounds at <change>, the round's log holds a line beginning `-- leg tests at venue ` followed immediately by the line `$ cargo test --workspace --all-features --no-fail-fast`, and that leg's part of the log ends with the lines `(exit 0)` and `(measured-green)`: four rounds, four such lines.
- For each of the directory and secrets rounds, the run record of the workflow run that measured it names, in the worker attribution of its gate activity's lease, the node declared by the worker at the venue that serves heavy builds and full gates, and not the node declared by the authoring machine's worker: two rounds, two such records.
- git diff <base> HEAD --name-only -- scripts/design prints nothing.

**Checklist:**
- C7 — sh scripts/design/gate.sh exits 0 at the card's head, and four green gate rounds at the change, one measured against each of the directory, home, secrets and lys-gate design.json files, each show the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.

**Stories:**
- S1 (Lead, Reads a red gate round before landing a card) — As a lead reading a red gate round, I want every failing test binary reported in that one round so that one failure cannot hide the others.

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

