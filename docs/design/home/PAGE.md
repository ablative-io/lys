# home — what was asked, what it means, and what was written

## The words, as they were typed

The HOME-001 rows R1 to R11 were built by hand on 24 September 2026 and reached lys main through the merge 7b47115, before Tom's rule of 25 September that every carded row goes through its board's chain. The seven cards that carry those rows on the step 5 board sit in review with no chain fields, and nothing on main says the chain has judged that code. This card adds no new code and only proves what is already there. It runs the chain's whole judgement over crates/lys-home as it stands on main at 0073b966: Jev per file, fmt, clippy pedantic, tests, ast-grep and the gate. The brief names each check and the command it runs, and the build's proof document records each check's outcome on the crate as found, with the commit it was measured on. If every check passes the build lands with only the proof document and the design rows this card needs. If Jev or the gate finds something, the build records each finding as its own line in the proof document and lands without changing the crate; each finding then becomes a card of its own on the step 5 board, worded from that line. When this card lands green the seven cards are set to done and each points at this card's landing, so that done means judged by the chain and never just merged before the rule. Acceptance is that the proof document names the commit, every check by name and its outcome, and holds no code changes to crates/lys-home; that a reader can rerun each named command at that commit and see the same outcome; and that the landing is green through src_pr and src_land with Jev and the gate. Not in scope: any new capability in lys-home, any change to the eleven rows, or any card other than the seven named. This card builds behind the six Lys briefs in the one-build queue on this Mac under the load guard. Filed by Archie on Waffles' ruling of 07:08 on 26 September 2026 in the Cambium channel, on Tom's rule of 22:26 on 25 September 2026 that every carded row goes through the chain, and on lys main 0073b966.

## What the survey found, and its angles

The card asks the chain to judge crates/lys-home after the fact. The crate was built by hand for HOME-001 and reached main before Tom's 25 September rule. The card runs Jev on each file, fmt, clippy pedantic, tests, ast-grep and the repository gate over the crate exactly as it stands at 0073b966. It records each check's command and outcome in a proof document and lands only that document and its design rows, never a change to the crate. Any finding becomes its own card. When the card lands green, the seven step 5 cards that carry the hand-built rows are set to done and each points at this landing.

### What the tree holds

- `crates/lys-home (23 .rs files, Cargo.toml, README.md)` — This is what gets judged. All of it entered in one commit, 217e81b (40 files, 8,395 insertions), described as card/home-001 at 37ba6e79 'replayed as one commit onto main 34ee2027'. It reached main's first-parent line through 7b47115, which merged main into card/home-001-on-main.
- `.land/gates.sh` — This is 'the gate'. It runs scripts/design/gate.sh, cargo fmt --check, clippy in both feature shapes, cargo test --workspace --all-features and cargo doc in both shapes, runs every leg even after a red one, and exits red if any leg was. src_land and src_gate run this file.
- `docs/design/project.json` — This declares the tree's measured legs. Its fmt leg is `cargo fmt --all`, which rewrites files, not `--check`. A proof that runs the leg as declared would change the crate instead of measuring it.
- `Cargo.toml [workspace.lints.clippy]` — `pedantic = warn` plus unwrap_used, expect_used, panic and others are set there, and the -D warnings legs turn them into errors. So the two clippy legs of the gate already are 'clippy pedantic'. lys-home inherits them through `[lints] workspace = true`.
- `~/.aion/authoring/src_land.awl` — src_land's Jev judges only the pull request's words and its diff from base to head (lines 84, 236 and 271), capped at 100,000 bytes. This card's landing diff is only the proof document and design rows, so src_land's Jev verdict is about the document, not the crate.
- `~/.aion/authoring/jev_ask.awl` — This is the existing way to ask Jev about 'one named part of a change … a diff the caller has cut on whole-file boundaries'. It is the natural carrier for 'Jev per file' over code that already exists: a whole-file diff of each file against the empty tree.
- `~/.aion/authoring/src_gate.awl` — It measures a commit with the repository's own gates without landing anything. That fits running the gate over 0073b966 as found and recording the verdict with every red leg named.
- `~/.aion/authoring/card_build_v3.awl, brief_card.awl, src_pr.awl` — This is the chain the card must pass through (brief_card → sign-off → card_build_v3 → src_pr → src_land). None of those files or atomic_card.awl or atomic_brief.awl mentions ast-grep or Jev, so the per-file Jev and ast-grep checks are something this card's brief has to name, not something the chain already does.
- `docs/design/home/design.json (structure, 45 rows) and briefs/HOME-001.json (R1 to R12)` — This is the cluster the card continues. A new brief (HOME-002) and a structure row for the proof document are 'the design rows this card needs'. The structure also shows which HOME-001 rows have code on main and which do not.
- `crates/lys-home/src/record/mod.rs` — 552 lines, 417 of them code, with 40 fn, impl, struct and enum items. CLAUDE.md says 'mod.rs carries only pub mod / pub use / module docs'. The aion ast-grep sweep has a mod-rs rule (CHECKPOINT-63 line 3593). This is the likeliest finding.
- `scripts/design/gate.sh` — This is the design leg. It validates every cluster, checks coverage and requires the committed markdown to equal what render-cluster.py renders. The new brief and proof rows must pass it.
- `sgconfig.yml (absent in lys)` — lys has no ast-grep rule set. ast-grep 0.44.1 is on this Mac, and sgconfig.yml exists in manifold, haematite, cambium, meridian and others. Which rules 'ast-grep' means for lys is not in the tree.

### What was already decided

- home/DESIGN.md — The home cluster's structure lists 45 paths for HOME-001. Of those, 24 exist on main and 21 do not: the whole proxy module, lys-proxy, examples/passthrough.rs, handover.rs, PROOF-PROXY.md, PROOF-HANDOVER.md and LOSS-ACCOUNT.md are absent.
- HOME-001 — 12 requirement rows, R1 to R12. R7 (pass-through proxy proof) and R10 (lys-proxy) have no code or proof on main. R12 (handover) is a design row added in 7c46392 with no code. Its verification asks only for fmt, clippy default, cargo test --workspace, validate and coverage, not the full chain.
- HOME-001 verification — PROOF-RESUME.md and PROOF-PROXY.md must exist. PROOF-PROXY.md does not, so HOME-001's own acceptance is unmet on main.
- PROOF-RESUME.md — Measured with the lys-home binary built from card/home-001 at 19e3ffe8, not from main. This is a precedent for proofs that name the commit they were measured on.
- CN3 / P7 — Transcript contents never appear in output, logs, errors, test names or pages. The proof document and any finding lines must carry hashes, counts and paths only.
- CLAUDE.md coding standards — No file over 500 code lines, mod.rs only pub mod or pub use, no unwrap and friends in library code, every public item documented, lint bypasses forbidden. These are the standards Jev and ast-grep would judge against.
- CLAUDE.md gates before any commit — fmt --check, clippy in both feature shapes, test --workspace --all-features, and doc in both shapes. This is exactly what .land/gates.sh runs.
- CLAUDE.md carded-row rules 1 to 6 — Every carded row goes through brief_card, sign-off, card_build_v3, src_pr and src_land. The full chain means Jev, fmt, clippy pedantic, tests, ast-grep and gate. Heavy builds and full gates go to Dean's laptop, and only small, warm, single-crate checks run on this Mac.
- ADR-004 — Every project stands alone. lys-home has no Norn or manifold dependency (CN6), so judging it needs nothing outside lys.
- RM-005 — The home roadmap row, briefed, which carries HOME-001. It is the row this card sits under.

### What was measured

- Main commit the words name: 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff (HEAD of this tree, detached, clean)
- Files in crates/lys-home: 25 (23 .rs, Cargo.toml, README.md), 6,618 lines, 241,943 bytes
- Largest file in the crate: src/harness/claude_code/import.rs, 21,979 bytes
- Jev diff limit in src_land: 100,000 bytes. The whole crate (241,943) is over it and every single file is under it.
- Commits that touched crates/lys-home: 1 (217e81b, 25 Sep 2026 09:24 AEST, 40 files, 8,395 insertions)
- Files over 500 raw lines: 4: record/call.rs 589, record/mod.rs 552, record/call_tests.rs 546, harness/claude_code/import.rs 531
- Files over 500 code lines (blank and // lines excluded): 1: record/call_tests.rs 519 (a test file). Library files peak at import.rs 473, call.rs 460 and record/mod.rs 417.
- Items in record/mod.rs other than pub mod or pub use: 40 fn, impl, struct and enum declarations
- #[test] functions in lys-home: 36 (record_tests 10, call_tests 7, canon_tests 4, cached_index 4, blocks_tests 3, import_tests 3, round_trip 3, render_tests 2)
- Lint allows in lys-home non-test source: 0. The only allow is the sanctioned test opt-out at tests/claude_code_round_trip.rs:1.
- HOME-001 structure rows present on main: 24 of 45
- HOME-001 requirement rows: 12 (R1 to R12). R7, R10 and R12 have no code on main.
- ast-grep rule sets in lys: 0 (no sgconfig.yml). ast-grep 0.44.1 is installed at ~/.cargo/bin.
- Jev binary on this Mac: none. Jev is reached through the ds2_ledger worker's jev_verdict action (OpenRouter model typesafe/jev-1.13, per CHECKPOINT-63 line 8488).
- Gate legs in .land/gates.sh: 7 (design, fmt, clippy all-features, clippy, test all-features, doc all-features, doc)
- Toolchain pinned: 1.97.1 with clippy and rustfmt (rust-toolchain.toml). lys-home rust-version is 1.89.
- The seven step 5 cards: Not measurable from this tree. The Cambium board is not in the repository, so their ids and the rows each carries were not read.

### What it means for the other projects

- aion — The card runs through ~/.aion/authoring's brief_card, card_build_v3, src_pr and src_land, and needs jev_ask (per file) and src_gate (the gate at a commit) invoked from the build. Those documents are untracked in ~/.aion/authoring (CHECKPOINT-63 line 8477). src_land's Jev needs a working key; it was a 401 on 25 Sep 23:17.
- cambium — The step 5 board holds the seven cards. They are set to done with a pointer to this landing, and a new card is filed for each finding line. The board's fields and the card ids are not in this tree.
- haematite — None directly. Its sgconfig.yml is one candidate ast-grep rule set if the lead chooses to borrow one.
- argus — None directly. The one-build queue and load guard the words name may be what Argus's queue tools run.
- method — The design-system scripts (validate, check-coverage, render-cluster) must accept the new brief and the proof's structure row, and the ds2_ledger worker serves jev_verdict.

### The decisions it stands on

- ADR-004 (honour) — lys-home stands alone, with no Norn or manifold dependency, and the proof judges it as it stands without pulling in any other project.
-  (new) — 'Done' on a carded row means judged by the chain, not merely merged. Rows built before 25 Sep 2026 are brought under the chain by a proof card that judges the code as found and changes none of it.
-  (new) — The ast-grep rule set lys is measured against has to be named, because lys has none today.

### What it requires

- The proof document names commit 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff as the commit measured.
- The proof document lists Jev (per file), fmt, clippy pedantic, tests, ast-grep and the gate, each with its exact command and its outcome.
- Every one of the 25 files in crates/lys-home has its own Jev verdict line (patched, solved, unsure or not_asked, with the reason), with the file path as the part name.
- The test line counts what ran: the number of lys-home tests executed (36 expected), not only 'passed'.
- Each finding from Jev, ast-grep or the gate is one line in the proof, worded so a card can be filed from it, with no transcript content (CN3).
- `git diff 0073b966 <landed head> -- crates/lys-home` is empty.
- The landing diff contains only the proof document and the home cluster's design rows (a new brief, a structure row and rendered markdown), and scripts/design/gate.sh passes on it.
- Re-running each named command at 0073b966 produces the recorded outcome.
- The landing passes src_pr and src_land with a real Jev verdict (not not_asked) and a green gate.
- After a green landing, each of the seven step 5 cards is done and names this card's landing commit.

### What must not change

- No byte of crates/lys-home changes: no fmt rewrite, no clippy fix, no test added.
- HOME-001 R1 to R12 are not edited.
- No card other than the seven named is moved, apart from the finding cards the words create.
- No new capability in lys-home.
- Transcript contents never appear in the proof or in finding lines (CN3 / P7).
- Existing wire formats and other crates are untouched.

### What we must put in place first

- Jev is reachable. The ds2_ledger jev_verdict needs a working OpenRouter key, which was 401 on 25 Sep; rule 6 says to find it on the estate before calling it Tom's.
- An ast-grep rule set for lys is named and available at the measured commit, or explicitly stated to be the estate sweep's rules.
- A way for the build to ask Jev per file, for example jev_ask called once per file with a whole-file diff against the empty tree.
- The seven step 5 cards identified by id, with the HOME-001 row each carries.

### The risks

- src_land's Jev sees only the proof-document diff, so a 'green with Jev' landing can be mistaken for Jev having judged the crate. The per-file verdicts must be recorded separately and distinctly.
- The fmt leg declared in project.json (`cargo fmt --all`) rewrites files; run as declared, it would change the crate the card must not touch.
- Setting done on cards for R7 and R10, whose code does not exist on main, would mark unbuilt work as judged.
- With no ast-grep rules, the check passes vacuously: a control that never fires reads as a pass.
- record/mod.rs (40 logic items) breaks CLAUDE.md's mod.rs rule; if no named check flags it, the proof can read clean over a known breach.
- Jev verdicts from a model are not deterministic, so 'a reader can rerun and see the same outcome' may not hold for Jev lines unless the model and question are recorded and the claim is limited accordingly.
- Running the full workspace gate on this Mac conflicts with rule 3 (Dean's laptop) and with the load guard.
- main moves past 0073b966 before landing; the proof must stay tied to 0073b966, not to the landing base.
- The Jev key being out (401) leaves every Jev line not_asked, and src_land will not land without a verdict.

### Still open

- R7 (the pass-through proxy proof) and R10 (lys-proxy) have no code or proof on main. Which HOME-001 rows do the seven cards actually carry, and does a card whose row was never built still get set to done? The sentence of the words it stands on: "The HOME-001 rows R1 to R11 were built by hand on 24 September 2026 and reached lys main through the merge 7b47115, before Tom's rule of 25 September that every carded row goes through its board's chain.". Why only the lead can settle it: docs/design/home/design.json lists crates/lys-home/src/proxy/*, src/bin/lys-proxy.rs, examples/passthrough.rs, PROOF-PROXY.md and LOSS-ACCOUNT.md for HOME-001, and none of them exists at 0073b966. The crate itself entered through 217e81b; 7b47115 only merged main into the card branch. If done is set on a card whose row has no code, the board says something is judged that does not exist.
- lys has no ast-grep rules. Which rule set does the ast-grep check run for lys-home: the estate's sweep rules (let_ on Result, mod-rs, lint-bypass, std Mutex in async), a copy of one repository's sgconfig.yml, or a new lys rule set? The sentence of the words it stands on: "It runs the chain's whole judgement over crates/lys-home as it stands on main at 0073b966: Jev per file, fmt, clippy pedantic, tests, ast-grep and the gate.". Why only the lead can settle it: lys has no sgconfig.yml. With no rules, ast-grep passes on anything. With the estate sweep's mod-rs rule, record/mod.rs (40 items) is a finding. The outcome recorded, and whether findings cards appear, depends on the answer.
- Where do the full gate and the workspace test run: in the one-build queue on this Mac, as the words say, or on Dean's laptop, as rule 3 says for heavy builds and full gates? The sentence of the words it stands on: "This card builds behind the six Lys briefs in the one-build queue on this Mac under the load guard.". Why only the lead can settle it: CLAUDE.md rule 3 (Tom, 25 Sep 21:54 and 21:56) says heavy builds and full gates go to Dean's laptop and only small, warm, single-crate checks run on this Mac. .land/gates.sh runs cargo test --workspace --all-features and both doc shapes over the whole workspace.
- If Jev or the gate records a finding, is the landing still 'green', so that the seven cards are set to done, or do they stay in review until the finding cards close? The sentence of the words it stands on: "When this card lands green the seven cards are set to done and each points at this card's landing, so that done means judged by the chain and never just merged before the rule.". Why only the lead can settle it: The words let a card with findings land 'without changing the crate'. They do not say whether that landing counts as green for the seven cards. A reader of the board would see done on code the chain found fault with, or would see the cards stay open.
- Is a breach of CLAUDE.md's standards that the named checks do not flag (for example, record/mod.rs carrying logic) recorded as a finding line, or only what Jev and the gate themselves report? The sentence of the words it stands on: "If Jev or the gate finds something, the build records each finding as its own line in the proof document and lands without changing the crate; each finding then becomes a card of its own on the step 5 board, worded from that line.". Why only the lead can settle it: clippy and the gate will not flag CLAUDE.md's 'mod.rs carries only pub mod / pub use' rule, and record/mod.rs has 40 logic items. Whether it becomes a card depends on whether Jev or ast-grep raises it or the author does.

### The units beyond the first

- One card per finding the proof records, on the step 5 board — The words make each finding its own card, worded from its proof line, and each changes the crate, which this card must not do.
- An ast-grep rule set for lys (sgconfig.yml and rules) in the gate — lys has none, and a check with no rules proves nothing. Adding it to .land/gates.sh changes the gate for every later card, not just this one.
- Build R7 and R10 of HOME-001 (the pass-through proxy proof and lys-proxy) — They are not on main, so they cannot be judged by a proof of what exists. They need their own carded build through the chain.
- A chain step that runs Jev per file over existing code — card_build_v3 and src_land judge diffs only. Judging code already on main recurs for every row built before 25 Sep, so it belongs in the chain rather than being done by hand each time.

### The smallest complete shape

One card through the full chain. It adds a HOME-002 brief to the home cluster (with its rendered markdown and a structure row) and one proof document under docs/design/home that names 0073b966 and records, with the exact command and outcome of each: a Jev verdict for every file in crates/lys-home, fmt --check, both clippy legs (pedantic), the test run with its count, ast-grep under a named rule set, and .land/gates.sh. Any findings are listed one per line. It lands through src_pr and src_land with no change to crates/lys-home, and on a green landing the seven step 5 cards are set to done, each pointing at the landing commit.

## The roadmap row

- **RM-006** — Judge lys-home as found with the chain's whole judgement, so its cards are done only when judged (process, idea)
- Summary: HOME-001's rows R1 to R11 were built by hand and reached main before every carded row had to pass through its board's chain. HOME-002 runs the chain's whole judgement (Jev per file, fmt, clippy pedantic, tests, ast-grep, the gate) over crates/lys-home as it stands at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, records each check's command and outcome in a proof document, changes nothing in the crate, and lets the seven step 5 cards be set done only on a green landing with no finding.
- Asked by: tom on 2026-09-26T07:21:07+10:00
- Context: The proof card filed on the step 5 board for the hand-built HOME-001 rows, surveyed against lys main 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, with the lead's answers to the survey's five questions (which rows have code, the ast-grep rule set, where the gate runs, when a landing is green, which findings are carded) and to the author's two (R4 is not built while docs/design/home/LOSS-ACCOUNT.md is absent; which card a finding on a file no row names holds).
- Quote: The HOME-001 rows R1 to R11 were built by hand on 24 September 2026 and reached lys main through the merge 7b47115, before Tom's rule of 25 September that every carded row goes through its board's chain. The seven cards that carry those rows on the step 5 board sit in review with no chain fields, and nothing on main says the chain has judged that code. This card adds no new code and only proves what is already there. It runs the chain's whole judgement over crates/lys-home as it stands on main at 0073b966: Jev per file, fmt, clippy pedantic, tests, ast-grep and the gate. The brief names each check and the command it runs, and the build's proof document records each check's outcome on the crate as found, with the commit it was measured on. If every check passes the build lands with only the proof document and the design rows this card needs. If Jev or the gate finds something, the build records each finding as its own line in the proof document and lands without changing the crate; each finding then becomes a card of its own on the step 5 board, worded from that line. When this card lands green the seven cards are set to done and each points at this card's landing, so that done means judged by the chain and never just merged before the rule. Acceptance is that the proof document names the commit, every check by name and its outcome, and holds no code changes to crates/lys-home; that a reader can rerun each named command at that commit and see the same outcome; and that the landing is green through src_pr and src_land with Jev and the gate. Not in scope: any new capability in lys-home, any change to the eleven rows, or any card other than the seven named. This card builds behind the six Lys briefs in the one-build queue on this Mac under the load guard. Filed by Archie on Waffles' ruling of 07:08 on 26 September 2026 in the Cambium channel, on Tom's rule of 22:26 on 25 September 2026 that every carded row goes through the chain, and on lys main 0073b966.
- Cluster: home; briefs: HOME-002
- Notes: Further units, not written: One card per finding the proof records, on the step 5 board; An ast-grep rule set for lys (sgconfig.yml and rules) in the gate (HOME-002 adds the rule set as configuration; wiring it into .land/gates.sh is the unit left); Build R7 and R10 of HOME-001 (the pass-through proxy proof and lys-proxy); A chain step that runs Jev per file over existing code.

## The design

---
type: design
cluster: home
title: The home: a session held under its identity, resumable by any harness that can be measured
---

# The home: a session held under its identity, resumable by any harness that can be measured

> **Cluster:** home

## Intention

A session's history lives under the agent's identity, not under a harness. The record is what the model was given and what came back, in order, with each provider's native blocks kept whole. A harness's own resume file is rendered from that record on demand, so an agent moves machines, harnesses or providers and continues; the original bytes are never rewritten. Code this cluster built before the chain existed is judged by the chain as it stands, and a card is done only when that judgement holds on the rows it carries: done means judged, never merely merged.

## Problem

Today a session exists only as its harness's file. Archie's Claude Code file measured 141,931,097 bytes on 24 September 2026: the conversation itself is under 3% of it, tool results 14%, thinking 6%, and about 60% is harness bookkeeping the model never sees; Waffles' file is 3.37 GB and Claude Code loads it whole. Nothing central holds a session, so nothing can render a smaller working session, move one to another device, swap provider and come back, or say what a session was given. Norn already has the canonical event model (SessionEvent, conversion to provider messages, compaction as a derived event, provider epoch boundaries) proved by its own tests; Claude Code has no importer to it and no renderer from it. HOME-001's rows R1 to R11 were built by hand and reached main before every carded row had to pass through its board's chain, so nothing on main records the chain's judgement of crates/lys-home, lys has no ast-grep rule set to judge it with, and two of those rows (R7, R10) have no code on main at all.

## Solution

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it.

HOME-002 brings the hand-built rows under the chain without touching them. It measures crates/lys-home exactly as it stands at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and writes docs/design/home/PROOF-CHAIN.md: the commit measured and the tools measured with; one Jev verdict per file (jev_ask, the whole file as a diff against the empty tree, the path as the part); fmt in its checking form; the two clippy legs, which are clippy pedantic because the workspace lints set pedantic to warn under -D warnings; the workspace test run with lys-home's executed tests counted; an ast-grep scan under the estate's four sweep rules (ADR-013), which this card adds as configuration; and .land/gates.sh. Every finding a named check reports is its own line, naming the HOME-001 rows it holds (the rows naming its file, the file a test sibling tests, or the card named by the pull request that introduced its lines), and becomes its own card; what the author observes and no check reports sits under its own heading and is carded only on the lead's word. A table reads HOME-001 at that commit and records, row by row, which named files exist; a row is built only when every file it names, code and document alike, is there, so no card is set done over work that does not exist, and a row whose code is there without its document is a finding. src_land's Jev judges this card's landing diff, which is the proof and design rows; the per-file verdicts are the crate's, and the proof says so. The seven step 5 cards are set done only on a green landing that records no finding (ADR-012).

## Principles

- **P1** — The original bytes are never rewritten. A rendered resume file, a compaction and a translation are derived records stored beside their source and pointing at it (CONTEXT-ROADMAP-2026-09-22.md stage 4).
- **P2** — The record is Pi's session tree as read from the Pi checkout at 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts: SessionHeader, SessionEntryBase {type, id, parentId, timestamp}, message, model_change, compaction {summary, firstKeptEntryId}, branch_summary {fromId, summary}, label, custom {customType, data}). lys adds entry kinds only as custom entries; it never adds a field to Pi's grammar and never adopts Norn's SessionEvent (Tom, Dot 13:28).
- **P3** — A provider's opaque blocks (Anthropic signed thinking, OpenAI encrypted reasoning) stay on the assistant message with its provider, api and model, and are rendered whole only when all three match the target (Pi transform-messages.ts:95-109); another model gets readable thinking as plain text and opaque blocks dropped, each named by hash in the loss account (Tom, Dot 13:24: keep reasoning traces per provider so a session can swap and swap back).
- **P4** — A content block is stored once by its hash; requests that resend the whole conversation reference blocks, they do not copy them.
- **P5** — Capture the model traffic and the harness's local events both, so the two can be mapped against each other (Tom, Dot 13:23); neither alone is the record.
- **P6** — A resume path is a per-harness, per-version measurement, never an assumption: Claude Code 2.1.281's --resume takes a session id and reads ~/.claude/projects/<cwd-slug>/<id>.jsonl; a seeded two-record file resumed there at 13:25 on 24 September and answered from its content.
- **P7** — Transcript contents never appear in a post, a log line, an error, a test name or a rendered page; status carries hashes, counts and offsets only (CONTEXT-001 privacy rule).
- **P8** — A sandbox or a VM is a target profile the launch template renders into, never a special case in the core; credentials are supplied at launch on the target and never carried in the home (Tom, Dot 13:30; CONTEXT-ROADMAP stage 3).
- **P9** — The canon is one curated, versioned series of examples every new session starts from: each entry is one rule stated short plus one real exchange that shows it lived (verify before claiming, a correction taken well, a refusal named, careful work), drawn from every agent's sessions, distilling the collective experience so far: 'our learnings in one another' (Tom, Dot 13:44 to 13:46; Waffles 0169c353). It is not a letter from one session to its successor. It lives in the lys repository at canon/canon.jsonl in Pi's grammar and changes only through src_commit and review, like code. Genuine thinking is kept whole where a real turn produced it and replays only to the same provider, api and model; thinking is never authored.
- **P10** — A handover is a letter the outgoing session writes to its successor in its own real thinking and answer; it enters the successor as a lys.inherited entry marked as a predecessor's memory, never as the successor's own experience, and replays only to the same provider, api and model. No thinking is ever authored; only thinking a model produced is kept (Tom, Dot 13:38 to 13:40: 'you are waking up ... I am another one, I'm helping you have part of my memory ... like a parent imparting a wish to a child'; Waffles 52d53451).
- **P11** — A proof of code already on main measures and never mends: it names the commit it measured, runs each check in its non-rewriting form, records every finding as its own line, and leaves every fix to a card of its own.

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-012 — Done on a carded row means judged by the chain, never merely merged — A carded row built before the chain is brought under it by a proof card that runs the chain's whole judgement over the code as found and changes none of it; its card is set done only on a green landing of that proof with no finding on the rows it carries, and only when its rows have code at the commit measured. Rejected: setting the cards done because their code is on main, and counting a landing that records a finding as green.
- ADR-013 — lys is measured by ast-grep under the estate's four sweep rules — lys's ast-grep rule set is the estate's sweep rules (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async), added as sgconfig.yml and rules/ast-grep byte-identical to cambium's, as configuration and not crate code. Rejected: a copy of one repository's whole sgconfig.yml with its repository-specific rules, and a new rule set written for lys.

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.
- PROOF-CHAIN.md records the chain's whole judgement of crates/lys-home at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff: every check by name, command and outcome, one Jev verdict per file, every finding as its own line, with no byte of the crate changed.

## Non-Goals

- Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for). — Signing comes when asked for (Tom, 22 September 16:27); every stage here works without it.
- Encryption at rest and moving a home between devices (stage 3 preconditions). — Stage 3's three preconditions (identity and read authority, encryption before bytes leave, the resume evidence) are their own brief.
- Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement.
- Lanterns and forks at a coordinate (stages 5 and 5b). — Lanterns and forks stand on a proved resume; this brief supplies that proof.
- Adopting, wrapping or calling Norn's session code; Pi's code is read as the reference and not vendored. — Tom, Dot 13:28: not Norn. Pi's tree is the reference.
- Fixing any finding the proof records, or any new capability in lys-home. — The card proves what is there; each finding becomes its own card on the step 5 board.
- Adding ast-grep to .land/gates.sh or docs/design/project.json. — Changing the gate changes it for every later card; it is its own unit.
- Building HOME-001 R7 and R10 (the pass-through proxy proof and lys-proxy). — They have no code on main, so there is nothing to judge; they are built through the chain as their own cards.
- A chain step that runs Jev per file over code already on main. — It recurs for every row built before the chain; it belongs in the chain as its own unit, not in this card.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/home/briefs/HOME-001.json` | the first brief: common record, Claude Code importer and renderer, the two proofs | HOME-001 |
| `docs/design/home/briefs/HOME-001.md` | its rendered markdown | HOME-001 |
| `docs/design/home/PROOF-RESUME.md` | the measured Claude Code resume: version, command, hashes before and after, what repeated | HOME-001 |
| `docs/design/home/PROOF-PROXY.md` | the measured subscription login through a pass-through proxy: version, headers that mattered, what failed | HOME-001 |
| `docs/design/home/LOSS-ACCOUNT.md` | what the Claude Code render preserves, transforms and cannot carry | HOME-001 |
| `crates/lys-home/src/record/blocks.rs` | content-addressed block store: put by SHA-256, get by hash, never rewritten | HOME-001 |
| `crates/lys-home/src/harness/claude_code/import.rs` | Claude Code JSONL into SessionEvents plus blocks | HOME-001 |
| `crates/lys-home/src/harness/claude_code/render.rs` | SessionEvents into a Claude Code JSONL under a chosen uuid, with the loss account | HOME-001 |
| `crates/lys-home/src/harness/claude_code/mod.rs` | the Claude Code harness profile: transcript root, cwd slug, version measured | HOME-001 |
| `crates/lys-home/src/cli.rs` | import, render, resume-check subcommands | HOME-001 |
| `crates/lys-home/examples/passthrough.rs` | a pass-through HTTP proxy that forwards to the provider unchanged, for the proof only | HOME-001 |
| `crates/lys-home/tests/claude_code_round_trip.rs` | import then render equals the model-visible content; opaque blocks kept whole | HOME-001 |
| `crates/lys-home/src/record/mod.rs` | the home record: Pi's session tree read and written, leaf pointer, root-to-leaf path | HOME-001 |
| `crates/lys-home/src/record/entries.rs` | Pi's entry types as Rust types, plus the lys custom entries lys.harness_event and lys.call | HOME-001 |
| `crates/lys-home/src/record/call.rs` | a proxy call record: request and response block hashes, provider, api, model, timing | HOME-001 |
| `crates/lys-home/src/harness/claude_code/events.rs` | Claude Code's harness-local records (hooks, permission mode, tool completion) as lys.harness_event entries | HOME-001 |
| `docs/design/home/RECORD.md` | the home record written down: Pi's grammar as adopted, the two lys custom entries, the block store, the loss account | HOME-001 |
| `crates/lys-home/src/lib.rs` | module wiring: record and harness | HOME-001 |
| `crates/lys-home/src/harness/mod.rs` | harness profiles; Claude Code first | HOME-001 |
| `crates/lys-home/Cargo.toml` | the crate manifest; gains the passthrough example | HOME-001 |
| `crates/lys-home/README.md` | what the tool does and does not do | HOME-001 |
| `crates/lys-home/src/record/index.rs` | the offset index and the persisted head: a path is read by seeking, never by loading the file | HOME-001 |
| `crates/lys-home/src/proxy/mod.rs` | the proxy module: declarations only | HOME-001 |
| `crates/lys-home/src/proxy/link.rs` | how a call is linked to a session: the measured request key (Claude Code's request metadata) now, the seat's handle when the door's proxy exists | HOME-001 |
| `crates/lys-home/src/bin/lys-proxy.rs` | the proxy binary | HOME-001 |
| `docs/design/home/PROOF-FEWSHOT.md` | the first proof: a hand-written few-shot session file resumed by path from a directory outside the config root | HOME-001 |
| `crates/lys-home/src/record/canon.rs` | the canon: copy chosen real exchanges (or a clearly authored example) into canon.jsonl with a lys.inherited entry naming each source; seed a new session from it | HOME-001 |
| `crates/lys-home/src/record/canon_tests.rs` | gates on the canon: copied whole, never twice, no authored thinking, rendered first | HOME-001 |
| `docs/design/home/PROOF-CANON.md` | measured: a session started from the canon against a plain start, on one card | HOME-001 |
| `canon/canon.jsonl` | the canon itself, versioned by the repository, changed only through src_commit and review | HOME-001 |
| `crates/lys-home/src/record/handover.rs` | the handover: take the outgoing session's letter turn (thinking intact) and seed the successor with it as lys.inherited | HOME-001 |
| `docs/design/home/PROOF-HANDOVER.md` | measured: a successor seeded with an inherited letter, whether 2.1.281 replays the signed thinking, and seeded against plain on one card | HOME-001 |
| `crates/lys-home/src/proxy/forward.rs` | forward a request to the provider and stream the response back unchanged | HOME-001 |
| `crates/lys-home/src/proxy/capture.rs` | bounded spooling of request and response bodies to files while forwarding, handed to the R6 sink | HOME-001 |
| `crates/lys-home/src/proxy/journal.rs` | the open-call journal: a call id written before forwarding, retired after ingest, so a restart records lost calls once | HOME-001 |
| `crates/lys-home/src/proxy/stream.rs` | the coordinator of the stream readers: picks the grammar by api and hands each frame on as it passes | HOME-001 |
| `crates/lys-home/src/proxy/stream_sse.rs` | SSE framing: events and data lines out of a byte stream, retaining byte order, encoding and trailers | HOME-001 |
| `crates/lys-home/src/proxy/stream_messages.rs` | the Messages event grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/stream_chat.rs` | the Chat Completions chunk grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/stream_responses.rs` | the Responses event grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/error.rs` | the proxy's errors and outcomes, named | HOME-001 |
| `crates/lys-home/src/proxy/forward_tests.rs` | forwarding against a loopback fake | HOME-001 |
| `crates/lys-home/src/proxy/stream_tests.rs` | the three grammars, partial streams | HOME-001 |
| `crates/lys-home/src/proxy/link_tests.rs` | linking by the measured key; unlinked; never inferred | HOME-001 |
| `crates/lys-home/src/proxy/journal_tests.rs` | recovery after a kill: one lost record per open call | HOME-001 |
| `docs/design/home/briefs/HOME-002.json` | the proof card: the chain's whole judgement over crates/lys-home as found | HOME-002 |
| `docs/design/home/briefs/HOME-002.md` | its rendered markdown | HOME-002 |
| `docs/design/home/PROOF-CHAIN.md` | the commit measured, each check's command and outcome, one Jev verdict per file, the findings one per line, the row table | HOME-002 |
| `sgconfig.yml` | the ast-grep project root: ruleDirs rules/ast-grep | HOME-002 |
| `rules/ast-grep/mod-rs-declarations-only.yml` | the estate sweep rule mod-rs-declarations-only, byte-identical to cambium's | HOME-002 |
| `rules/ast-grep/no-let-underscore-on-results.yml` | the estate sweep rule no-let-underscore-on-results, byte-identical to cambium's | HOME-002 |
| `rules/ast-grep/no-lint-bypass-attributes.yml` | the estate sweep rule no-lint-bypass-attributes, byte-identical to cambium's | HOME-002 |
| `rules/ast-grep/no-std-mutex-in-async.yml` | the estate sweep rule no-std-mutex-in-async, byte-identical to cambium's | HOME-002 |
| `docs/design/home/DESIGN.md` | design.json rendered |  |
| `docs/design/home/CHECKLIST.md` | checklist.json rendered |  |
| `docs/design/home/USER-STORIES.md` | stories.json rendered |  |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'The home', 'The home is portable: lanterns, translation and forks', and steps 4 and 5
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Archie's roadmap; this cluster is its stage 4 and the resume half of stage 3
- `docs/design/identity/briefs/CONTEXT-001.json` — stage 1, byte-for-byte capture and the lys-home crate this cluster extends
- `docs/design/secrets/briefs/SECRETS-002.json` — the door's proxy (R1) that the tee joins once it exists
- `$PI/packages/coding-agent/src/core/session-manager.ts` — Pi's session tree at 3d5cbe98 (github.com/earendil-works/pi): header, entries, leaf (branch at :1125 moves an in-memory cursor; _buildIndex:754 restores the last physical entry on reopen; loadEntriesFromFile:438 reads the whole file)
- `$PI/packages/ai/src/api/transform-messages.ts` — per-provider thinking: same provider, api and model keeps blocks whole; otherwise text, opaque dropped
- `$PI/packages/coding-agent/src/core/branch-summarization.ts` — how an abandoned branch's work is carried into the new one
- `$CCFLARE/README.md` — ccflare at 95c4c6a (github.com/snipeship/ccflare): the concept of a native pass-through that keeps request history; reference only, not used (Tom, Dot 13:34)
- `crates/lys-home` — at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff: 25 files (23 .rs, Cargo.toml, README.md), 36 #[test] functions, all entered in one commit (217e81b); the thing judged, as found
- `.land/gates.sh` — the gate: scripts/design/gate.sh, cargo fmt --check, clippy in both feature shapes, cargo test --workspace --all-features, cargo doc in both shapes; every leg runs, red if any leg is
- `scripts/design/gate.sh` — the design leg: validate, coverage and rendered markdown equal to what the JSON renders
- `Cargo.toml` — [workspace.lints.clippy] sets pedantic = warn with unwrap_used, expect_used, panic and others; lys-home inherits them
- `rust-toolchain.toml` — pins the toolchain at 1.97.1 with clippy and rustfmt
- `$CAMBIUM/rules/ast-grep` — the estate sweep rules at df4dca5de6899df89b06b94cdeeec3619877fc1d: mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async (haematite carries the same four byte for byte)

## Constraints

- **CN1** — No file under ~/.claude/projects is rewritten or truncated by this cluster; a render writes a new file under a new uuid only, and refuses an existing path by name.
- **CN2** — A rendered file never carries a provider-native opaque block that came from a different provider or model family than the one it is rendered for; the loss account names each block dropped by hash.
- **CN3** — Transcript contents never appear in output, logs, errors, test names or pages; hashes, counts, offsets and event ids only.
- **CN4** — Every home file parses with Pi's parseSessionEntries at 3d5cbe98 unchanged: the header line first, every entry with id, parentId and timestamp, lys data only inside custom entries.
- **CN5** — The pass-through proxy forwards every header and the streamed body unchanged and stores nothing but the measurement; it is an example binary, never a service.
- **CN6** — No Norn crate is a dependency of lys-home and no Norn type is copied into it.
- **CN7** — Reading the root-to-leaf path of a home file reads only the entries on that path plus the index, never the whole file (Pi loads the whole journal; Chippy 13:33); the head is persisted beside the file, never inferred from the last physical entry on reopen.
- **CN8** — HOME-002 changes no byte under crates/lys-home: no fmt rewrite, no clippy fix, no test added; git diff 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff <card head> -- crates/lys-home is empty.


---
type: brief
id: HOME-001
cluster: home
title: Build the home record on Pi's session tree, with the Claude Code importer and renderer and the two measured proofs
---

# HOME-001: Build the home record on Pi's session tree, with the Claude Code importer and renderer and the two measured proofs

> **Cluster:** home
> **Blocked by:** The door's proxy (SECRETS-002 R1) is not landed, so the tee's wiring into it is a later brief in the secrets cluster; this brief defines the call record and ingests captured pairs.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> **Checklist:**
> - C1 — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.
> - C2 — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.
> - C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
> - C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
> - C5 — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.
> - C6 — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.
> - C7 — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.
> - C8 — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.
> - C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.
> - C10 — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.
> - C11 — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.
> - C12 — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.
> - C13 — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.
> **Stories:**
> - S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
> - S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
> - S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.
> - S4 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.
> - S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.
> - S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.
> - S7 (Agent, Runs in a harness and wants to continue somewhere else) — As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.
> - S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

## Purpose

Give a session a home under its identity in the shape Tom chose on 24 September (Pi's tree), and prove three things in this order: first, a few-shot session file written by hand resumes Claude Code by path from anywhere (Tom, Dot 13:34); second, a real session read into the record renders back and resumes on 2.1.281 without repeating work; third, a subscription login survives a pass-through proxy, so the little proxy that records the stream has a place to live. It is a neat place that holds sessions together, not a sub-agent platform (Tom, Dot 13:33). See docs/design/home/design.json for the shape and the constraints.

## Task

Extend the lys-home crate (CONTEXT-001 creates it; if it is not yet on main, create the crate here with the same workspace lints and say so in the dev record) with a record module in Pi's session tree grammar, a content-addressed block store, a Claude Code importer and renderer, harness-event and call entries, and a CLI. Then run the two proofs and write them down with hashes and the exact commands. Pi's code is read at a clone of github.com/earendil-works/pi at commit 3d5cbe98, as the reference and is not vendored; Norn's session code is not used, called or copied. Out of scope: the door-side tee wiring (secrets cluster, after SECRETS-002), lys grants and signing, encryption, moving a home between devices, any harness but Claude Code, lanterns and forks at a coordinate. Ownership inside the brief: Chippy builds R10, the little proxy (the step-3 seam is Chippy's); Archie builds the rest. The first proof (R9, PROOF-FEWSHOT.md) is run before any other requirement is reviewed.

## Requirements

### R1: The home record in Pi's grammar

Define the home record as Pi's session tree: a header line {type:"session", version, id, timestamp, cwd, parentSession?} then entries, each {type, id, parentId, timestamp}, of the kinds message, model_change, compaction {summary, firstKeptEntryId, tokensBefore}, branch_summary {fromId, summary}, label, custom {customType, data} and session_info, with a leaf pointer kept beside the file. WHEN an entry is appended, THE SYSTEM SHALL write it as a child of the leaf and advance the leaf; WHEN the leaf is moved to an earlier entry, THE SYSTEM SHALL rewrite nothing. THE SYSTEM SHALL provide the root-to-leaf path and the context path (entries from the latest compaction's firstKeptEntryId onward, plus the compaction summary) as Pi's buildSessionPath and buildContextEntries do. THE SYSTEM SHALL NOT add a field to the header or to any entry outside custom.data, and SHALL NOT depend on any Norn crate. THE SYSTEM SHALL keep an offset index (entry id to byte offset and parentId) and a persisted head file beside the home file, so that reading the root-to-leaf path seeks to the path's entries only and never loads the whole file, and so that reopening restores the persisted head, not the last physical entry. One owner at a time: opening or creating a session SHALL take an exclusive lock on `<id>.lock` beside the file for the life of the Session, and a second opener in the same or another process SHALL be refused by name. An append is durable as the entry line, then its index row, then the head; IF a step after the line fails, THEN THE SYSTEM SHALL reconcile the index and head from the file before it admits any further act, and a read on a session that could not reconcile SHALL be refused by name. THE SYSTEM SHALL refuse an entry whose id is already on record, and on rebuild SHALL refuse a file holding a duplicate id or a parent that does not precede its child. A session id, and every name the home joins onto a directory, SHALL be one safe path component (letters, digits, `.`, `_`, `-`; not beginning with `.`; at most 200 bytes), refused by name otherwise.

**Acceptance:**
- A fixture file written by lys-home with 12 entries, one compaction and one moved leaf loads with Pi's loadEntriesFromFile at the checkout 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts:438, parseSessionEntries at :284; run through node against a clone of earendil-works/pi at 3d5cbe98 in the proof step) with 12 entries and no migration, and buildSessionContext (:315) on it yields the same message list as lys-home's context_path().
- `context_path()` on that fixture returns the compaction summary entry followed by every entry from firstKeptEntryId to the leaf and nothing before it.
- Moving the leaf to entry 4 and appending entry 13 leaves bytes 0..N of the file identical (N = the length before the move) and entry 13's parentId equal to entry 4's id.
- `Cargo.toml` of lys-home lists no dependency whose name starts with `norn`.
- Reading the path of a 200 MB synthetic home file whose path holds 50 entries reads fewer than 1 MB from the file (measured with a counting reader).
- After a leaf move and reopen, `head()` equals the moved-to entry id, not the last entry appended before the move.
- Opening a session that another Session holds (same process) is refused with SessionHeld; after that owner is dropped the next opener succeeds with the persisted head.
- Appending an entry whose id is on record (with itself as parent) is refused with DuplicateEntry and the path still ends.
- With the index file made unwritable, an append still succeeds: the session reports one reconciliation, the entry is on the path with its parent, the next append needs no reconciliation, and a reopen finds the index and file in step.
- A session file holding a duplicate id, or a parent after its child, is refused on rebuild naming the line.
- Session ids `../x`, `a/b`, `.hidden`, `with space` and `/abs` are refused with BadName and no file is touched.

**Files:**
- create: crates/lys-home/src/record/mod.rs
- create: crates/lys-home/src/record/entries.rs
- create: docs/design/home/RECORD.md
- create: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C1 — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R2: Store content blocks once by hash

WHEN a content block (a message part, a tool result, a request or response body) is put, THE SYSTEM SHALL store it under its SHA-256 as blocks/<hh>/<hash> in the home, fsync it, and return the hash; WHEN the same bytes are put again, THE SYSTEM SHALL return the same hash and write nothing. THE SYSTEM SHALL NOT overwrite or delete a block, and SHALL NOT print, log or include block contents in any error. A block's temporary file name SHALL carry a nonce as well as the pid, so two puts of the same bytes in one process never share a temporary file; a surplus temporary that cannot be removed is reported, never ignored.

**Acceptance:**
- Putting the same 1 MiB block twice leaves one file in the store and the second put performs no write (measured by the directory's mtime and file count).
- `get(hash)` of a missing hash returns an error naming the hash and no other bytes.
- A block file's bytes hash to its name for every block in the store after the R3 import (a test walks the store).

**Files:**
- create: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C2 — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.

### R3: Import a Claude Code JSONL into the record

WHEN given a Claude Code transcript file (records with parentUuid, uuid, type, message, isSidechain, timestamp, as 2.1.281 writes them), THE SYSTEM SHALL produce one home file: each user or assistant record becomes a message entry whose parentId is the entry of the record's parentUuid, with the assistant message carrying provider "anthropic", api "anthropic-messages" and the record's model, and every content part stored as a block by R2 and referenced by hash; thinking parts keep their signature; summary records become compaction entries pointing at the first kept entry; sidechain records (isSidechain true) become entries under the message they branched from with a label entry naming the agent; attachment, system and permission-mode records become lys.harness_event entries as R8 says (attachment and system records at their exact place on the chain, since a message's parentUuid may name one); other non-message records are counted per type and left in the byte-for-byte original. THE SYSTEM SHALL write an import report {records, entries, blocks, bytes_in, bytes_out, counted_types} and SHALL NOT include any message text in it. IF a record's parentUuid names no known record, THEN THE SYSTEM SHALL refuse by uuid rather than attach it to the leaf. WHEN an assistant record carries model `authored` (or the file's first record is the lys.authored marker), THE SYSTEM SHALL import it as an authored message entry (a lys.authored custom entry precedes it, and its message carries provider `authored`, api `authored`, model `authored`), so that in the record an authored turn is never mistaken for one a model produced; the boundary between authored entries and the real turns that follow (the continuation's own model) SHALL be visible as a change of provider on the path.

**Acceptance:**
- Importing Archie's session file (141,931,097 bytes, 79,042 records on 24 September) yields entries equal to its user plus assistant records, blocks whose total bytes are within 5% of the measured 33.4 MB of content parts, and a report with no field longer than 64 characters.
- For every message entry, the parent entry's uuid equals the source record's parentUuid (a test checks all).
- A synthetic file whose one record cites an unknown parentUuid is refused with that uuid in the error and no home file written.
- A thinking part with a signature imports to a thinking block whose thinkingSignature equals the source signature byte for byte.
- Importing the walrus continuation (6 authored records then a real turn on the model that answered) yields a path whose first 6 message entries carry provider `authored` and whose 7th carries the answering model's provider, with exactly one lys.authored entry before the first.

**Files:**
- create: crates/lys-home/src/harness/claude_code/import.rs
- create: crates/lys-home/src/harness/claude_code/mod.rs
- create: crates/lys-home/src/harness/mod.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R4: Render the record as a Claude Code JSONL under a chosen uuid

WHEN asked to render for Claude Code with a target uuid, a cwd and a target model, THE SYSTEM SHALL walk the context path and write records in Claude Code's shape (parentUuid chain, sessionId = the uuid, cwd, version, type, message, timestamp) to ~/.claude/projects/<cwd-slug>/<uuid>.jsonl, where cwd-slug is the cwd with every '/' replaced by '-'; tool_use and tool_result parts keep their pairing by id; a compaction renders as Claude Code's summary record followed by the kept entries. WHILE a thinking block's provider, api and model equal the target's, THE SYSTEM SHALL render it whole with its signature; otherwise THE SYSTEM SHALL render readable thinking as a text part and drop opaque or redacted blocks. THE SYSTEM SHALL write a loss account beside the file listing every dropped block by hash and reason. IF the target path exists, THEN THE SYSTEM SHALL refuse by path and write nothing. WHEN given a home file authored by hand (a header and message entries written as a few-shot prompt, never imported), THE SYSTEM SHALL render it by the same path with no import step; the 13:25 measurement on 24 September (a two-record file written by hand under a chosen uuid, resumed by id, answered from its content) is the first instance. An authored home file SHALL carry a custom entry lys.authored as its first entry, and every render and report of it SHALL say authored: true, so a few-shot demonstration is never read as history of tools that ran (Chippy, 13:35).

**Acceptance:**
- Rendering the R3 import back for the same model and re-importing it yields the same sequence of message block hashes as the original import (round trip test).
- Rendering for a different model produces a file with zero thinking parts carrying a signature and a loss account whose entry count equals the number of signed thinking blocks on the path.
- Rendering to an existing path returns an error naming the path and leaves its bytes and mtime unchanged.
- The rendered file's first record has parentUuid null and every later record's parentUuid is the uuid of the previous record on the path.
- A hand-written home file of one user and one assistant message entry renders to a two-record Claude Code JSONL whose records carry the chosen uuid as sessionId and whose second record's parentUuid is the first's uuid.
- Rendering a home whose first entry is lys.authored prints a report with `"authored": true`; rendering an imported home prints `"authored": false`.

**Files:**
- create: crates/lys-home/src/harness/claude_code/render.rs
- create: docs/design/home/LOSS-ACCOUNT.md
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- C5 — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
- S4 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.
- S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

### R5: Prove the Claude Code resume on 2.1.281

Import one real session of Archie's, render it for the same model under a fresh uuid, record SHA-256 of the original file, then run `claude -p --resume <uuid> --fork-session --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "<a question answerable only from the session's last exchange>"` from the session's cwd. THE SYSTEM SHALL record in docs/design/home/PROOF-RESUME.md: the Claude Code version, the exact command, the answer (redacted to one word), the original's hash before and after (equal), the uuid the fork created, and the count of tool_use ids in the forked file that also appear in the rendered file (SHALL be 0). THE SYSTEM SHALL NOT run the proof against a session that is currently running. The proof SHALL also assert the authored-to-real boundary: after resuming an authored file, the continuation's copied records still carry model `authored` and its new turn carries the real model, and that boundary survives import (R3) and render (R4) unchanged (Waffles, 29dc46e5).

**Acceptance:**
- PROOF-RESUME.md exists with the six fields and the version string `2.1.281`.
- The original's before and after hashes in PROOF-RESUME.md are equal.
- The repeated-tool-action count in PROOF-RESUME.md is 0.
- `lys-home resume-check <rendered> <forked>` computes that count and exits non-zero when it is not 0 (tested with a synthetic duplicate).
- PROOF-RESUME.md records, for the authored continuation, the count of records carrying model `authored` (6) and the real model's name for the new turn, and a re-import of the rendered continuation reports the same counts.

**Files:**
- create: docs/design/home/PROOF-RESUME.md
- create: crates/lys-home/tests/claude_code_round_trip.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C6 — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

### R6: Define the proxy call record and ingest a captured pair

Define the custom entry lys.call: data {provider, api, model, request: [block hash], response: [block hash], status, started_at, duration_ms, stream: bool}. WHEN given a captured request body and response body (files) with the provider, api and model, THE SYSTEM SHALL split the request into its message parts, store each as a block (so a resent conversation adds no new blocks), store the response parts, and append one lys.call entry under the leaf. THE SYSTEM SHALL support the Messages, Chat Completions and Responses request shapes for the split and SHALL keep the raw request and response bodies as blocks too. THE SYSTEM SHALL NOT store any header, and SHALL NOT forward, replay or modify a call. The ingest report SHALL count part blocks new and reused separately from raw body blocks. Every call carries a stable call_id the proxy chooses before forwarding, and ingest SHALL be idempotent on it within a session: a second ingest of a recorded call_id writes nothing and answers with the existing entry. Only a complete call SHALL carry response parts. A call that did not complete (cancelled, partial, unrecorded, lost) SHALL be recorded through an outcome-only ingest that takes whatever bodies exist: a request file that parses gives its parts and model, one that does not (or none) gives neither, and absence SHALL be recorded as absence (no model, no raw hash), never as an empty body or an invented model. The sink SHALL take the response's parts from the proxy when the response was an event stream, since the proxy reads the stream as it forwards it (Chippy d7d98e92, 5834dfdc). Both complete-call entry points (bytes and files) SHALL take the proxy's assembled response parts as an option and, without them, SHALL parse the response body and refuse by name a body that is not JSON, an error body, or one without the api's parts member; a streamed call without assembled parts SHALL be refused. In an outcome ingest a request file that is not JSON or not the api's shape is recorded as absence, and one that cannot be read is an I/O error that propagates.

**Acceptance:**
- Ingesting two consecutive captured Messages requests where the second resends the first's conversation plus one turn adds, among part blocks, exactly the new turn's parts (the report's `part_blocks_new` equals the new parts and `part_blocks_reused` equals the resent parts); the raw request and response bodies are two further blocks each time and are counted under `raw_blocks`, never under part blocks.
- A Chat Completions and a Responses fixture each ingest to one lys.call entry whose request array length equals the fixture's message or input item count.
- An ingested pair's raw bodies are retrievable by the hashes named in the entry and hash to those names.
- The entry's data contains no key named authorization, cookie or x-api-key (a test checks the serialised entry).
- Ingesting the same call_id twice in one session leaves one lys.call entry and the second report says already_recorded.
- An outcome ingest with a half-written request file and no response file yields a record with status lost, no model, a raw request hash and no raw response hash, and an empty response list; an outcome ingest for status complete is refused.
- A complete call ingested from files with proxy-supplied stream parts records those parts and keeps the raw stream body by hash.
- ingest_call from bytes refuses `{not json`, `{}` and an error body for a complete Messages call, and a stream without assembled parts, leaving no lys.call entry; with assembled parts it records them.
- ingest_outcome with a request path that is a directory returns an Io error; with a half-written JSON request it records absence (0 request parts, no model) and one raw block.

**Files:**
- create: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R7: Prove a subscription login through a pass-through proxy

Write examples/passthrough.rs: an HTTP server that forwards every request to the provider base URL given on its command line with headers and streamed body unchanged, and returns the response unchanged; it stores nothing but a line per call {method, path, status, duration_ms} on stderr. Run one seat whose Claude Code login is a subscription (not an API key) with ANTHROPIC_BASE_URL pointing at it, and record in docs/design/home/PROOF-PROXY.md: the Claude Code version, whether the call completed, the status codes seen, which headers had to pass for it to work, and what failed if it did. THE SYSTEM SHALL NOT log, store or print a header value or a body byte.

**Acceptance:**
- PROOF-PROXY.md exists and states one of `completed` or `failed` with the status codes seen.
- The passthrough example's source contains no code path that writes a header value or a body byte to a file, stderr or stdout (review reads the file; the only writes are the four-field line).
- With a loopback fake upstream (a test server on 127.0.0.1 answering GET / with status 418 and a fixed 3-chunk streamed body), `cargo run --example passthrough -- http://127.0.0.1:<port>` answers GET / with status 418 and the same 3 chunks in order, proving the forward path without a provider.

**Files:**
- create: crates/lys-home/examples/passthrough.rs
- create: docs/design/home/PROOF-PROXY.md
- modify: crates/lys-home/Cargo.toml

**Checklist:**
- C7 — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

### R8: Map Claude Code's harness-local records to lys.harness_event entries

Define the custom entry lys.harness_event: data {kind, harness: "claude-code", source_uuid, record, detail}, where record is the whole source record stored as a block by hash. WHEN importing, THE SYSTEM SHALL map records of type attachment with attachment.type hook_success or hook_failure to kind hook, every other attachment record to kind attachment, records of type system to kind system, records of type permission-mode to kind permission_mode, and each tool_result part to kind tool_completed with the tool_use id. A Claude Code file is one chain and attachment and system records carry a uuid and a parentUuid on it (measured on a real session on 24 September: a user record's parentUuid names an attachment 25 times and a system record 7 times), so THE SYSTEM SHALL write those events at their exact place on the chain under the record's own uuid, and SHALL write permission_mode events (no uuid) and tool_completed events as side leaves under the entry they followed; when the import ends the head SHALL stand at the last record of the chain. detail carries only names, ids, exit codes and counts. THE SYSTEM SHALL NOT copy hook output text or tool result bodies into detail (those are blocks under the message, and the record block).

**Acceptance:**
- Importing Archie's session file yields hook events equal to its hook_success plus hook_failure attachment count (8,993 hook_success on 24 September) and tool_completed events equal to its tool_result parts.
- Every lys.harness_event entry written for a record with a uuid has that uuid as its id and the record's parentUuid as its parentId (a test imports a file whose tool result's parent is a hook attachment and checks the tool result's parent is the hook's entry); every tool_completed event's parentId is the tool result message it describes; every permission_mode event's parentId is the chain's leaf at that point.
- After import the head is the last record on the file's chain and the context path holds no side event.
- No lys.harness_event entry's serialised data exceeds 512 bytes, and no detail carries a key named stdout, stderr, content or text.
- The hook event's record hash resolves in the block store to the source record byte for byte.

**Files:**
- create: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/record/entries.rs

**Checklist:**
- C8 — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R9: The CLI: import, render, ingest-call, resume-check; resume is Claude Code's own --resume <path>

Add subcommands to the lys-home binary: `import --home <dir> --claude-code <file>`, `render --home <dir> --claude-code --uuid <uuid> --cwd <dir> --model <id>`, `ingest-call --home <dir> --provider <p> --api <a> --model <m> --request <file> --response <file>`, `resume-check <rendered> <forked>`. Every subcommand SHALL print a JSON report of hashes, counts and paths and SHALL NOT print any transcript, block or body content. IF a required argument is missing, THEN THE SYSTEM SHALL exit 2 naming it. There is no resume launcher: Claude Code 2.1.281 resumes directly from a file path (`claude --resume <path>`), measured by Waffles at 13:36 and by Archie at 13:37 on 24 September, even though --help names only a session id; the continuation is written beside the passed file as <sessionId>.jsonl in the same directory (the authored records copied in, then the new turn), and the passed file is unchanged. THE SYSTEM SHALL provide `fewshot --out <path>` that writes a hand-authored Claude Code JSONL from a turns file (role and text per line), with sessionId a fresh uuid, cwd as given, the parentUuid chain intact, every assistant record's model set to `authored`, and the marker described in R4 (a first record of type custom, customType lys.authored, if 2.1.281 accepts it; else the model value `authored` is the marker and the proof says which). After writing the file, `fewshot` SHALL carry the person's own command, `claude --resume <path>`, as the `resume` member of its JSON report, and SHALL NOT run it (ADR-007). `resume-check` SHALL refuse a transcript with a malformed line by naming the line, SHALL report `repeated_tool_use_ids` (ids appearing more times in the fork than in the rendered file: an id-duplication count, not a proof that no action was repeated under a fresh id) and `new_tool_uses` (tool_use parts in the fork's own records), and SHALL exit non-zero when `repeated_tool_use_ids` is not 0.

**Acceptance:**
- `lys-home render` without --uuid exits 2 and names `--uuid`.
- Each subcommand's stdout parses as JSON and contains no key named text, content or body.
- `lys-home import` on the R3 fixture prints the same counts as the R3 report.
- `lys-home fewshot --out f.jsonl` from a 6-turn turns file writes 6 records whose parentUuid chain is intact and whose assistant records carry model `authored`.
- PROOF-FEWSHOT.md records the first proof: the authored 6-turn file, the directory it was run from (neither the file's directory nor ~/.claude), the exact command `claude -p --resume <path> ...`, the one-word answer, the source hash before and after (equal), the continuation's path (beside the source, named <sessionId>.jsonl) and line count, and which marker 2.1.281 accepted.
- The resume proof in PROOF-RESUME.md is run from a working directory that is neither the file's directory nor the session's cwd, and records that directory.
- `lys-home fewshot --out f.jsonl` prints one JSON report whose `resume` member is exactly one line beginning `claude --resume ` naming the written path, and spawns no process.
- `lys-home resume-check` on a fork with a malformed line is refused naming the line, and its report carries `repeated_tool_use_ids`, `new_tool_uses` and `forked_new_records`.

**Files:**
- create: docs/design/home/PROOF-FEWSHOT.md
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
- C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- C10 — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

### R10: The little proxy: pass the stream through and record each call under its session

Build lys-proxy: an HTTP server that forwards every request to the provider named by its path prefix (/anthropic, /openai) with headers and streamed body unchanged and returns the response unchanged, and, after the response completes, appends one lys.call entry (R6) under the home of the session the call belongs to. WHEN a request carries Claude Code's session key (measured first: the request metadata field 2.1.281 sends; the exact field and format are recorded in PROOF-PROXY.md before this is built), THE SYSTEM SHALL link the call to that session's home; IF no key is present, THEN THE SYSTEM SHALL append the call under an `unlinked` home named by the day and say so in its report. THE SYSTEM SHALL support the Messages, Chat Completions and Responses streams, SHALL NOT buffer a streamed response before forwarding it, SHALL NOT store any header, SHALL NOT retry, balance or swap accounts (ccflare is the concept, not the design), and SHALL NOT alter a byte of a request or response. Outcomes: a call record's status SHALL be one of complete, cancelled (client closed before the response ended), partial (the upstream stream ended early or malformed), unrecorded (the capture write failed after forwarding) or lost (the process died mid-call, detected on restart from a journal of open calls), and only complete SHALL carry a full response block list; THE SYSTEM SHALL NOT report a call complete on any other path. Capture SHALL be bounded: at most N calls (a command-line value, default 64) pending write; above it the proxy still forwards and marks the call unrecorded rather than growing a backlog. A call SHALL be linked only by the key it carries; THE SYSTEM SHALL NOT infer a session from timing, cwd or a previous call. The proxy's mod.rs holds declarations only; forwarding, capture, the journal, the stream grammars and errors each have their own file, and R7's example shares forward.rs rather than a second transport (Chippy 323b67d5). Before forwarding, the proxy writes the call_id to the open-call journal and retires it after the sink answers; on restart every open call is recorded once through the outcome ingest as lost, which the sink's idempotency guarantees. The journal SHALL be durable before a request is forwarded; a capture or sink failure after forwarding SHALL still forward and record unrecorded. IF the journal itself cannot be written before the request is admitted upstream, THEN THE SYSTEM SHALL refuse the request by name with zero upstream calls; after upstream admission THE SYSTEM SHALL keep forwarding, keep the journal, and record unrecorded once storage can accept it; on restart an unresolved journal entry SHALL be recorded lost only when no terminal outcome for it was durable. The transport SHALL set Hyper's retry_canceled_requests(false) explicitly; the measure of no retry is that an upstream disconnect never produces a second upstream request. Above the N capture slots THE SYSTEM SHALL still take the session key from the request body on the forwarding path (a bounded incremental read of the metadata field, or an explicit request-spool budget), so a keyed call in overload is recorded unrecorded under its own session, never placed under `unlinked` for want of capture.

**Acceptance:**
- A streamed Messages response of 200 SSE events is forwarded with the first event delivered before the last is received (measured with a slow upstream fake).
- One call through the proxy adds exactly one lys.call entry to the linked home, and its request blocks equal the parts of the request body.
- A request without the session key lands under the unlinked home and the proxy's report line names `unlinked`.
- The proxy's source has no code path that writes a header value to disk or to a log; review reads proxy/mod.rs and link.rs.
- The proxy exposes no account list, no retry and no routing table: grep for `accounts` and `failover` in crates/lys-home/src/proxy returns nothing, `retry` appears only in the transport's `retry_canceled_requests(false)`, and a test with an upstream fake that disconnects mid-request sees exactly one upstream request.
- A client that closes mid-stream produces one lys.call with status cancelled and no full response block list.
- An upstream fake that ends the SSE stream early produces status partial.
- A capture directory made read-only during a call produces status unrecorded while the client still receives the full response.
- Killing the proxy mid-call and restarting produces one lys.call with status lost for that call, from the open-call journal.
- Two consecutive calls without a key from the same client land under `unlinked`, never under the session of an earlier keyed call.
- With the journal directory unwritable before a request is admitted, the request is refused by name and the upstream fake sees zero requests.
- With the journal directory made unwritable after upstream admission, the client receives the full response and the call is recorded unrecorded once the directory is writable again.
- With the capture bound set to 0, a keyed request is forwarded, its lys.call is recorded unrecorded under the keyed session, and nothing lands under `unlinked`.

**Files:**
- create: crates/lys-home/src/proxy/mod.rs
- create: crates/lys-home/src/proxy/link.rs
- create: crates/lys-home/src/proxy/forward.rs
- create: crates/lys-home/src/proxy/capture.rs
- create: crates/lys-home/src/proxy/journal.rs
- create: crates/lys-home/src/proxy/stream.rs
- create: crates/lys-home/src/proxy/stream_responses.rs
- create: crates/lys-home/src/proxy/stream_chat.rs
- create: crates/lys-home/src/proxy/stream_messages.rs
- create: crates/lys-home/src/proxy/stream_sse.rs
- create: crates/lys-home/src/proxy/error.rs
- create: crates/lys-home/src/proxy/forward_tests.rs
- create: crates/lys-home/src/proxy/stream_tests.rs
- create: crates/lys-home/src/proxy/link_tests.rs
- create: crates/lys-home/src/proxy/journal_tests.rs
- create: crates/lys-home/src/bin/lys-proxy.rs
- modify: crates/lys-home/Cargo.toml
- modify: crates/lys-home/src/lib.rs
- modify: docs/design/home/PROOF-PROXY.md

**Checklist:**
- C11 — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.
- C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R11: The canon: one curated, versioned series of examples every new session starts from

Define the canon as one session file in Pi's grammar at canon/canon.jsonl in the lys repository, versioned by the repository and changed only through src_commit and review, like code. Each example is a custom entry lys.inherited whose data names its source {from_session, from_entries, provider, api, model, curated_at, curated_by, rule} (or {authored: true, curated_at, curated_by, rule} for a clearly authored example), followed by the example's message entries copied whole: the rule stated short in `rule`, and one real exchange that shows it lived. WHEN `lys-home canon add --canon <file> --home <dir> --from <session> --entries <id>... --rule <text> --by <who>` is run, THE SYSTEM SHALL copy those entries whole (thinking blocks with their signatures included) after a lys.inherited entry naming the source; WHEN `lys-home canon add --canon <file> --authored <turns file> --rule <text> --by <who>` is run, THE SYSTEM SHALL add the example with provider, api and model `authored` and SHALL refuse a turns file that contains a thinking block. WHEN a new session is rendered for a harness with `--canon <file>`, THE SYSTEM SHALL place the canon's entries first, before the session's own, and apply R4's thinking rule to every inherited thinking block (whole only to the same provider, api and model; otherwise text, opaque dropped and named in the loss account). THE SYSTEM SHALL NOT compose, edit or author any thinking block, SHALL NOT alter an example's text when copying it, and SHALL NOT change canon.jsonl except by appending through the repository's review. Curation is a person's act: the tool copies what it is told to and records who told it. The proof runs one card twice, from the canon and plain, counting fix rounds and unverified claims, in docs/design/home/PROOF-CANON.md, after first measuring whether 2.1.281 replays a signed thinking block from a resumed file at all (unknown on 24 September). `lys-home canon create --canon <file>` writes the header line of an empty canon and refuses an existing file. The canon is read and appended as a plain file: no index, head or lock beside it, since it lives in the repository. A copied entry keeps its id, so the same example cannot be added twice (refused by that id); only parent links are rewritten to chain onto the canon.

**Acceptance:**
- `lys-home canon add --from <a session with a signed thinking turn> --entries <that exchange> --rule "verify before claiming" --by <who>` appends to canon.jsonl one lys.inherited entry naming that session, entry and rule, and one message entry whose thinking block signature equals the source byte for byte.
- `lys-home canon add --authored` with a turns file that contains a thinking block exits non-zero naming the line and writes nothing; without one, it appends a lys.inherited entry with authored: true and message entries carrying provider, api and model `authored`.
- Rendering a session with --canon for Claude Code writes the canon's message entries before the session's own, with the parentUuid chain running through them.
- Rendering with --canon for a different model than an inherited thinking block's produces a loss account entry for that block and a text part in its place.
- canon/canon.jsonl parses with Pi's loadEntriesFromFile at 3d5cbe98, and its git history shows every change as a reviewed commit.
- PROOF-CANON.md states whether a signed thinking block in a resumed file was replayed by 2.1.281 (measured through R10's proxy, or before it exists through the harness's own file), and records the canon-seeded and plain counts of fix rounds and unverified claims for one named card.
- Adding the same entries a second time is refused naming the entry id and writes nothing.
- The render report names how many canon examples were placed first (`inherited`).

**Files:**
- create: crates/lys-home/src/record/canon.rs
- create: docs/design/home/PROOF-CANON.md
- create: canon/canon.jsonl
- create: crates/lys-home/src/record/canon_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/src/harness/claude_code/render.rs

**Checklist:**
- C12 — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.

**Stories:**
- S7 (Agent, Runs in a harness and wants to continue somewhere else) — As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

### R12: The handover: an outgoing session's letter seeds its successor as inherited memory

A handover is one R11 lys.inherited entry, its data naming the outgoing session as from_session, the letter's single entry id in from_entries, the letter's provider, api and model, when it was written as curated_at and the outgoing session as curated_by, with no rule (a letter is not a rule; that absence is what tells a handover from a canon example), followed by the letter's message entry copied whole (its thinking blocks with their signatures, its text). WHEN `lys-home handover --home <dir> --from <session> --letter <entry id>` is run, THE SYSTEM SHALL copy that turn from the outgoing session into a new successor home as its first entries, a lys.inherited entry then the message, and SHALL record in the successor's session_info that its first memory is inherited. The letter is elicited by a person or the harness with a real prompt to the outgoing session before compaction or retirement (what it knows, what it got wrong and why, how the people like things done, what it wishes it had known); THE SYSTEM SHALL NOT compose, edit or author the letter or any thinking block, and SHALL refuse a letter entry whose assistant message carries provider `authored`. WHILE rendering a successor for Claude Code, THE SYSTEM SHALL apply R4's rule: the inherited thinking renders whole only to the same provider, api and model, otherwise as text with opaque blocks dropped and named in the loss account. The proof SHALL measure first whether 2.1.281 replays a signed thinking block from a resumed file at all (unknown on 24 September), and then run one card twice, seeded and plain, counting fix rounds and unverified claims, in docs/design/home/PROOF-HANDOVER.md.

**Acceptance:**
- `lys-home handover` on the walrus continuation's real turn produces a successor home whose first entry is lys.inherited naming that session and entry, whose second is the copied message with a thinking block whose signature equals the source byte for byte, and whose session_info says inherited.
- `lys-home handover` with a letter entry whose message carries provider `authored` exits non-zero naming the entry and writes nothing.
- PROOF-HANDOVER.md states whether a signed thinking block in a resumed file was replayed by 2.1.281 (measured: the block is present in the continuation's next request as recorded by R10's proxy or, before that exists, by the harness's own file), and records the seeded and plain counts of fix rounds and unverified claims for one named card.
- Rendering the successor for a different model produces a loss account entry for the inherited thinking block and a text part in its place.

**Files:**
- create: crates/lys-home/src/record/handover.rs
- create: docs/design/home/PROOF-HANDOVER.md
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C13 — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.

**Stories:**
- S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

## Boundaries

- No file under ~/.claude/projects is rewritten, truncated or deleted; a render writes a new uuid only and refuses an existing path.
- No Norn crate is depended on and no Norn code is copied; Pi's code is read at 3d5cbe98 and not vendored or edited.
- No signing, hashing into a lys log, anchoring, encryption at rest or sync between devices.
- No door code is changed: the little proxy is lys-home's own binary; its wiring into the door's credential-swapping proxy is a secrets-cluster brief after SECRETS-002 lands.
- No harness but Claude Code, and no translation to another harness's file.
- No transcript, block or body content in any post, log line, error, test name, report or page; hashes, counts, offsets and ids only.
- No proof runs against a session that is currently running, and no credential is copied anywhere.
- The design's structure array is the whole file list; a path outside it is not created.
- Not a sub-agent platform: nothing here spawns, schedules or supervises agents; it holds sessions neatly and renders them (Tom, Dot 13:33).
- Resume is Claude Code's own `--resume <path>`; no launcher, copy or rewrite of a passed file is built.
- No thinking block is ever authored, edited or synthesised; an example's thinking is only what a model produced in a real turn, an example's text is copied, never rewritten, and the canon changes only through review.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/home exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/home exits 0.
- From the repository root: cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings and cargo test --workspace exit 0.
- docs/design/home/PROOF-RESUME.md and PROOF-PROXY.md exist, name Claude Code 2.1.281, and the resume proof's before and after hashes are equal with a repeated-tool-action count of 0.
- grep -rn 'norn' crates/lys-home/Cargo.toml returns nothing.
- grep -rln 'pelican' crates/lys-home returns nothing (no transcript content in tests).
- node -e on the Pi checkout at 3d5cbe98 parses the R1 fixture with parseSessionEntries and reports 12 entries (the command is written in PROOF-RESUME.md).


---
type: brief
id: HOME-002
cluster: home
title: Judge crates/lys-home as found at 0073b966 with the chain's whole judgement, and record it in a proof
---

# HOME-002: Judge crates/lys-home as found at 0073b966 with the chain's whole judgement, and record it in a proof

> **Cluster:** home
> **Blocked by:** The seven step 5 cards' ids and the HOME-001 rows each carries are on the Cambium board, not in this tree, except that card UzVQkTaU carries R3 and R4; R5 and R8 read the rest from the board., Jev is reached through the ds2_ledger worker's jev_verdict with a working OpenRouter key; without one every per-file verdict is not_asked and R3 records the Jev check incomplete, so the key is found on the estate before the build starts., The card builds behind the six Lys briefs ahead of it in the one-build queue.
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-012 — Done on a carded row means judged by the chain, never merely merged — A carded row built before the chain is brought under it by a proof card that runs the chain's whole judgement over the code as found and changes none of it; its card is set done only on a green landing of that proof with no finding on the rows it carries, and only when its rows have code at the commit measured. Rejected: setting the cards done because their code is on main, and counting a landing that records a finding as green.
> - ADR-013 — lys is measured by ast-grep under the estate's four sweep rules — lys's ast-grep rule set is the estate's sweep rules (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async), added as sgconfig.yml and rules/ast-grep byte-identical to cambium's, as configuration and not crate code. Rejected: a copy of one repository's whole sgconfig.yml with its repository-specific rules, and a new rule set written for lys.
> **Checklist:**
> - C14 — sgconfig.yml and the four estate sweep rules (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async) exist in lys, each rule byte-identical to cambium's at df4dca5d.
> - C15 — PROOF-CHAIN.md names 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff as the commit measured, with the toolchain, the ast-grep version and the Jev workflow the outcomes were measured with.
> - C16 — PROOF-CHAIN.md carries one Jev verdict line for each of the 25 files of crates/lys-home at 0073b966.
> - C17 — PROOF-CHAIN.md records fmt, both clippy pedantic legs, the tests with lys-home's executed count, ast-grep with a match count per rule, and the gate with its seven legs, each with its exact command and outcome.
> - C18 — Each finding a named check reports, the absent document of a partly present row among them, is its own line in PROOF-CHAIN.md naming the check, the file and the HOME-001 rows it holds, or that it holds none of the seven; observations no check reports sit under their own heading.
> - C19 — PROOF-CHAIN.md's table records, for HOME-001 R1 to R11, every named file present or absent at 0073b966 and the row's state, and for each of the seven step 5 cards the rows it carries.
> - C20 — No byte under crates/lys-home differs between 0073b966 and the card's head, and scripts/design/gate.sh passes on the card's head.
> - C21 — The card lands through src_pr and src_land with Jev and the gate; the seven cards are set done only on a green landing with no finding, a card carrying a partly present row stays in review, and one card is filed per finding line.
> **Stories:**
> - S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.
> - S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each check's command and outcome recorded against the commit it measured, so that I can rerun any of them at that commit and see the same outcome.
> - S11 (Board reader, Reads the step 5 board to know what the chain has judged) — As a reader of the step 5 board, I want a card set done only when the chain judged its rows' code and found nothing, so that done never means merely merged.
> - S12 (Board reader, Reads the step 5 board to know what the chain has judged) — As a reader of the step 5 board, I want each finding written as its own line naming the check, the file and the rows it touches, so that a card can be filed from that line alone.

## Purpose

HOME-001's rows were built by hand and reached main before every carded row had to pass through its board's chain, so nothing on main says the chain has judged crates/lys-home. This brief adds no code and proves what is there: it runs the chain's whole judgement (Jev per file, fmt, clippy pedantic, tests, ast-grep and the gate) over the crate as it stands at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, records each check's command and outcome in docs/design/home/PROOF-CHAIN.md, and lets the seven step 5 cards be done only when that judgement holds. See docs/design/home/design.json (P11, CN8) and ADR-012 and ADR-013.

## Task

Measure crates/lys-home exactly as it stands at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and write the proof; change nothing in the crate. Add the estate's four sweep rules as lys's ast-grep rule set (configuration, not crate code). Every finding a named check reports is its own line in the proof and becomes its own card on the step 5 board; what the author notices and no check reports goes under `Observed, not measured` and becomes a card only on the lead's word. A landing that records any finding is not green: it lands the proof, and the seven cards stay in review until every finding card on their rows has closed; that later move is not this brief's. Only a card whose rows are built at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff is set done, and a row is built only when every file it names, code and document alike, is in the tree at that commit: R7 and R10 have no code, so the cards carrying them stay in review, recorded not built in the proof's card table, and those rows are built through the chain as their own cards; R4's code is there and its docs/design/home/LOSS-ACCOUNT.md is not, so R4 is recorded partly present, the missing document is a finding carrying R4, and card UzVQkTaU, which carries R3 and R4, stays in review until that finding closes. A finding on a file no HOME-001 row names holds the row that introduced its lines: a *_tests.rs sibling holds the rows of the file it tests, tests/cached_index.rs holds R1, which owns the cached index, and any other such file holds the card named in the title or body of the pull request GitHub gives for the commit that introduced its lines (for error.rs and main.rs, 217e81b322431fe2bad8b21033afe3bbefc23dc0 and pull request 8, which names none); where that pull request names no card, the finding holds none of the seven and its card is filed in todo. The build runs on this Mac in the one-build queue under the load guard, one build at a time, and the full gate runs where the build runs: Tom approved at 06:24 on 26 September 2026, on Dot, that Lys card_build_v3 runs here one at a time under that guard, and that approval is the ruling for Lys cards. Out of scope: any new capability in lys-home, any fix to a finding, any change to HOME-001's rows, any card other than the seven and the finding cards, adding ast-grep to .land/gates.sh, and building R7 or R10.

## Requirements

### R1: Add the estate's four sweep rules to lys as its ast-grep rule set

Add sgconfig.yml at the repository root, whose only key is ruleDirs naming rules/ast-grep, and four rule files under rules/ast-grep: mod-rs-declarations-only.yml, no-let-underscore-on-results.yml, no-lint-bypass-attributes.yml, no-std-mutex-in-async.yml, each byte-identical to the file of the same name under rules/ast-grep in the cambium repository at commit df4dca5de6899df89b06b94cdeeec3619877fc1d. The rule set is configuration, not crate code. The build SHALL NOT change any file under crates/, SHALL NOT add a fifth rule, SHALL NOT edit any rule's pattern, files glob, message or severity, and SHALL NOT add ast-grep to .land/gates.sh or to docs/design/project.json.

**Acceptance:**
- `ls rules/ast-grep` at the card head lists exactly the four files mod-rs-declarations-only.yml, no-let-underscore-on-results.yml, no-lint-bypass-attributes.yml, no-std-mutex-in-async.yml.
- For each of the four files, `git -C <cambium checkout> show df4dca5de6899df89b06b94cdeeec3619877fc1d:rules/ast-grep/<name>.yml` piped to `cmp - rules/ast-grep/<name>.yml` exits 0.
- `ast-grep scan --config sgconfig.yml --json=compact crates/lys-home/src/record/mod.rs` at the card head with ast-grep 0.44.1 exits 1 and its JSON carries at least one match whose ruleId is mod-rs-declarations-only (the rule set fires).
- `git diff --quiet 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff <card head> -- crates` exits 0.

**Files:**
- create: sgconfig.yml
- create: rules/ast-grep/mod-rs-declarations-only.yml
- create: rules/ast-grep/no-let-underscore-on-results.yml
- create: rules/ast-grep/no-lint-bypass-attributes.yml
- create: rules/ast-grep/no-std-mutex-in-async.yml

**Checklist:**
- C14 — sgconfig.yml and the four estate sweep rules (mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async) exist in lys, each rule byte-identical to cambium's at df4dca5d.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each check's command and outcome recorded against the commit it measured, so that I can rerun any of them at that commit and see the same outcome.

### R2: Open the proof with the commit measured and the tools it was measured with

Create docs/design/home/PROOF-CHAIN.md. Its opening section SHALL name the commit measured, 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, in full; the rustc and cargo version strings the pinned toolchain reports; ast-grep's version string; the SHA-256 of sgconfig.yml and of each rule file; that every per-file Jev verdict came from the jev_ask workflow, with R3's cause text as the question it answered; and that every check ran in the one-build queue under the load guard, one build at a time. It SHALL state that src_land's Jev verdict on this card's landing judges the landing diff (the proof, the rule set and the design rows) and is not a verdict on the crate, and that the per-file verdicts of R3 are the crate's. WHEN main moves past 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff before the card lands, THE SYSTEM SHALL keep 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff as the commit measured, and SHALL NOT remeasure at the landing base or name any other commit as the one measured. THE SYSTEM SHALL NOT write transcript content, block content or body content into the document; paths, commands, exit statuses, counts, hashes, verdict words and line numbers only.

**Acceptance:**
- The opening section of docs/design/home/PROOF-CHAIN.md contains the string 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff on a line labelled as the commit measured, and no other 40-hex commit id is labelled as the commit measured.
- The toolchain line names rustc 1.97.1, equal to the channel in rust-toolchain.toml at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff.
- The ast-grep line equals the output of `ast-grep --version` on the build host.
- Each SHA-256 in the opening section equals `shasum -a 256` of the named file at the card head.
- The opening section names jev_ask as the workflow every per-file verdict of the Jev section came from.
- The document contains one sentence stating that src_land's Jev verdict judges the landing diff and is not a verdict on the crate.

**Files:**
- create: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C15 — PROOF-CHAIN.md names 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff as the commit measured, with the toolchain, the ast-grep version and the Jev workflow the outcomes were measured with.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each check's command and outcome recorded against the commit it measured, so that I can rerun any of them at that commit and see the same outcome.

### R3: Ask Jev about every file of crates/lys-home, one file at a time

WHEN the proof is built, THE SYSTEM SHALL ask Jev once for each of the 25 paths `git ls-tree -r --name-only 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff crates/lys-home` lists, through the jev_ask workflow, with part = the path, change = the output of `git diff 4b825dc642cb6eb9a060e54bf8d69288fbee4904 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff -- <path>` (the whole file as a diff against the empty tree), and cause = this text with <path> replaced by the path exactly as `git ls-tree` prints it, which already begins crates/lys-home/: "<path> as it stands on lys main at commit 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff was written for brief HOME-001 (docs/design/home/briefs/HOME-001.json) before the chain judged it. The cause: the coding standards in the repository's CLAUDE.md, and the HOME-001 requirement rows whose files name this path, hold for this file as found." THE SYSTEM SHALL write one line per path in the proof's Jev section: the path, the verdict word (patched, solved, unsure or not_asked, as jev_ask answers it), the patched probability when jev_ask returns one, held, and the reason. A patched verdict is a finding (R6). The Jev check's outcome SHALL be pass when all 25 lines carry a verdict and none is patched, finding when at least one is patched, and incomplete when at least one is not_asked, with each not_asked path named. THE SYSTEM SHALL NOT ask Jev about the crate as one change, SHALL NOT cut a file below whole-file boundaries, SHALL NOT omit a path, SHALL NOT alter the cause text beyond the path, and SHALL NOT record a not_asked line as any other verdict.

**Acceptance:**
- The Jev section holds exactly 25 verdict lines, and the set of their paths equals the output of `git ls-tree -r --name-only 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff crates/lys-home`.
- Every verdict line's verdict word is a member of {patched, solved, unsure, not_asked}.
- The count of Jev finding lines in the findings section equals the count of verdict lines whose word is patched.
- The Jev check's recorded outcome is incomplete when the section holds at least one not_asked line, and each not_asked path is named beside that outcome.
- Running jev_ask again with part crates/lys-home/src/record/blocks.rs, the change `git diff 4b825dc642cb6eb9a060e54bf8d69288fbee4904 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff -- crates/lys-home/src/record/blocks.rs` and the cause text beginning `crates/lys-home/src/record/blocks.rs as it stands on lys main at commit 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff` returns the verdict word the proof records for that path.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C16 — PROOF-CHAIN.md carries one Jev verdict line for each of the 25 files of crates/lys-home at 0073b966.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.

### R4: Run fmt, clippy pedantic, tests, ast-grep and the gate over the crate as found, and record each

THE SYSTEM SHALL run each check below and record in the proof's checks section its command line verbatim, the commit it ran at, its exit status and its outcome word (pass when the exit status is 0, finding otherwise): fmt: `cargo fmt --all -- --check` in a clean checkout of 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff; clippy pedantic, one line per leg (the workspace lints set pedantic to warn and -D warnings makes every warning an error, so these two legs are clippy pedantic): `cargo clippy --all-targets --all-features -- -D warnings` and `cargo clippy --all-targets -- -D warnings` in that checkout; tests: `cargo test --workspace --all-features` in that checkout, recording for each lys-home test target (the library's unit tests, the binary's unit tests, tests/cached_index.rs, tests/claude_code_round_trip.rs) its passed, failed and ignored counts, and the executed count as passed plus failed summed over those four targets; ast-grep: `ast-grep scan --config sgconfig.yml --json=compact crates/lys-home` at the card head, whose crates/lys-home is byte-identical to 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff (R7), recording the match count for each of the four rule ids with a rule that matched nothing written as 0; the gate: `sh .land/gates.sh` in that checkout of 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, recording the status line of each of its seven legs. IF the lys-home executed count differs from the sum over the files of crates/lys-home at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff of `grep -c '#\[test\]'`, or any lys-home target reports an ignored test, THEN THE SYSTEM SHALL record the tests outcome as finding. THE SYSTEM SHALL NOT run `cargo fmt --all` or any other rewriting form of fmt, SHALL NOT run `cargo clippy --fix`, SHALL NOT run a check at a commit other than the one this requirement names for it, and SHALL NOT change the crate to clear a check.

**Acceptance:**
- The checks section holds exactly six check lines, in this order: fmt, clippy-all-features, clippy, tests, ast-grep, gate; each carries its command line exactly as this requirement writes it, a commit, an exit status and an outcome word.
- The fmt, clippy-all-features, clippy, tests and gate lines name 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff as their commit; the ast-grep line names the card head.
- Rerunning each of the five commands in a clean checkout of 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff gives the exit status the proof records for it, and rerunning the ast-grep command at the card head gives the same match count for each of the four rule ids.
- The tests line's recorded executed count equals passed plus failed as cargo reports them, summed over the four lys-home test targets.
- The tests line's outcome is finding if and only if that executed count differs from the sum of `grep -c '#\[test\]'` over the files of crates/lys-home at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff (36), or a lys-home target reports an ignored test.
- The ast-grep line carries four counts, one for each of mod-rs-declarations-only, no-let-underscore-on-results, no-lint-bypass-attributes, no-std-mutex-in-async.
- The gate line carries seven leg status lines, one for each `leg` call in .land/gates.sh at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff.
- `git status --porcelain -- crates/lys-home` in the checkout of 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff prints nothing after all five commands have run.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C17 — PROOF-CHAIN.md records fmt, both clippy pedantic legs, the tests with lys-home's executed count, ast-grep with a match count per rule, and the gate with its seven legs, each with its exact command and outcome.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each check's command and outcome recorded against the commit it measured, so that I can rerun any of them at that commit and see the same outcome.

### R5: Record which HOME-001 rows have their files at the commit, and which rows each of the seven cards carries

THE SYSTEM SHALL read docs/design/home/briefs/HOME-001.json at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and write a row table in the proof: for each of R1 to R11, every path its create and modify arrays name, each marked present or absent by `git cat-file -e 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff:<path>`, and the row's state: built when every named path, code and document alike, is present; not built when a named path under crates/lys-home is absent; partly present when every named path under crates/lys-home is present and a named path outside it is absent. A partly present row is not built, and each of its absent paths is a row table finding (R6). THE SYSTEM SHALL write a card table: for each of the seven step 5 cards that carry HOME-001's rows, the card's id as the board gives it, the HOME-001 rows it carries, and each row's state. THE SYSTEM SHALL NOT mark a row from the design's structure array alone, SHALL NOT edit HOME-001.json, and SHALL NOT mark a row built while a path it names is absent, and SHALL NOT record card UzVQkTaU as carrying rows other than the board gives, which include R3 and R4.

**Acceptance:**
- The row table holds exactly 11 lines, R1 to R11 in order.
- R7 is recorded not built with crates/lys-home/examples/passthrough.rs absent, and R10 is recorded not built with crates/lys-home/src/proxy/mod.rs absent.
- R4 is recorded partly present with docs/design/home/LOSS-ACCOUNT.md absent, and not as built.
- R1, R2, R3, R5, R6, R8, R9 and R11 are recorded built.
- The card table holds exactly 7 lines, each naming a step 5 card id, and the union of their rows is R1 to R11.
- The card table's line for card UzVQkTaU names R3 and R4, and R4's state on it is partly present.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C19 — PROOF-CHAIN.md's table records, for HOME-001 R1 to R11, every named file present or absent at 0073b966 and the row's state, and for each of the seven step 5 cards the rows it carries.

**Stories:**
- S11 (Board reader, Reads the step 5 board to know what the chain has judged) — As a reader of the step 5 board, I want a card set done only when the chain judged its rows' code and found nothing, so that done never means merely merged.

### R6: Write every finding a named check reports as its own line, and keep the author's observations apart

WHEN a named check reports something (a Jev verdict of patched, an ast-grep match, a clippy or fmt diagnostic, a failing or missing test, a gate leg with a non-zero status, or a file R5's row table records absent for a row whose code is present), THE SYSTEM SHALL write it in the proof's findings section as its own line: one line per path for a Jev verdict, one line per rule id and path for ast-grep with every matched line number listed, one line per diagnostic for fmt, clippy, tests and the gate, and one line per absent file for the row table. Each line SHALL name the check, the rule, lint or verdict, the path, the line numbers where the check gives them, and the HOME-001 rows the finding holds, chosen in this order: the rows whose create or modify arrays name the path; for a *_tests.rs sibling, the rows that name the file it tests (record_tests.rs tests record/mod.rs); for tests/cached_index.rs, R1, which creates record/index.rs and so owns the cached index; for an absent file, the row that names it; for any other path, the card named by the pull request that GitHub's commits/<sha>/pulls answers for the commit `git blame 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff -- <path>` gives for the finding's lines, with that commit and pull request number on the line, written as `holds none of the seven` when that pull request names no card. A pull request names a card only when its title or body contains the id of one of the seven step 5 cards as the board gives it; a branch name, a brief id or a row id is not a card. Each line SHALL be worded so a card can be filed from the line alone. Anything the author notices that no named check reports SHALL go under its own heading, `Observed, not measured`, and SHALL NOT appear in the findings section. THE SYSTEM SHALL NOT drop a finding a check reports, including one a repository convention sanctions elsewhere, SHALL NOT join two checks' reports into one line, SHALL NOT fix a finding in this card, SHALL NOT write a line without the rows or `holds none of the seven`, and SHALL NOT put transcript content into a finding line.

**Acceptance:**
- The count of ast-grep finding lines equals the count of distinct (ruleId, file) pairs in the JSON the R4 ast-grep command prints.
- Every finding line names one check from {Jev, fmt, clippy-all-features, clippy, tests, ast-grep, gate, row table}, the name that check gave the finding (the rule id for ast-grep, the lint name for clippy, the verdict word for Jev, `absent` for the row table), and a path.
- A finding line on crates/lys-home/src/record/mod.rs names HOME-001 rows R1 and R2, a finding line on crates/lys-home/src/record/record_tests.rs names R1 and R2, and a finding line on crates/lys-home/tests/cached_index.rs names R1.
- The findings section holds a row table line naming docs/design/home/LOSS-ACCOUNT.md as absent and naming R4.
- Every finding line on crates/lys-home/src/error.rs or crates/lys-home/src/main.rs names commit 217e81b322431fe2bad8b21033afe3bbefc23dc0 and lys pull request 8 (the one GitHub's commits/217e81b322431fe2bad8b21033afe3bbefc23dc0/pulls answers, merged as 7b47115cb08b02c246ff19e3330e5f825ed68163), and reads `holds none of the seven`, because neither that pull request's title nor its body contains a step 5 card id.
- The proof has a heading `Observed, not measured`, and no line under it also appears in the findings section.
- When every check's outcome is pass and the row table records no absent file for a row whose code is present, the findings section holds zero lines and says so.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C18 — Each finding a named check reports, the absent document of a partly present row among them, is its own line in PROOF-CHAIN.md naming the check, the file and the HOME-001 rows it holds, or that it holds none of the seven; observations no check reports sit under their own heading.

**Stories:**
- S12 (Board reader, Reads the step 5 board to know what the chain has judged) — As a reader of the step 5 board, I want each finding written as its own line naming the check, the file and the rows it touches, so that a card can be filed from that line alone.

### R7: Land only the proof, the rule set and the design rows, with the crate untouched

Render the home cluster with render-cluster.py so docs/design/home/briefs/HOME-002.md, DESIGN.md, CHECKLIST.md and USER-STORIES.md equal what the JSON renders to. The card's diff from its merge base with main SHALL hold only these paths: docs/design/home/PROOF-CHAIN.md, sgconfig.yml, the four rules/ast-grep files, docs/design/home/briefs/HOME-002.json and HOME-002.md, docs/design/home/design.json, checklist.json, stories.json, DESIGN.md, CHECKLIST.md, USER-STORIES.md, docs/design/roadmap.json and docs/design/decisions.json. THE SYSTEM SHALL NOT change any byte under crates/, SHALL NOT edit HOME-001.json or HOME-001.md, and SHALL NOT change .land/gates.sh or docs/design/project.json.

**Acceptance:**
- `git diff --quiet 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff <card head> -- crates/lys-home` exits 0.
- `git diff --name-only $(git merge-base main <card head>) <card head>` lists no path outside the set this requirement names.
- `git diff --quiet 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff <card head> -- docs/design/home/briefs/HOME-001.json docs/design/home/briefs/HOME-001.md .land/gates.sh docs/design/project.json` exits 0.
- `sh scripts/design/gate.sh` at the card head exits 0.

**Files:**
- create: docs/design/home/briefs/HOME-002.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C20 — No byte under crates/lys-home differs between 0073b966 and the card's head, and scripts/design/gate.sh passes on the card's head.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each check's command and outcome recorded against the commit it measured, so that I can rerun any of them at that commit and see the same outcome.

### R8: Land through src_pr and src_land, then set the seven cards by the rule and card each finding

WHEN the card is landed, THE SYSTEM SHALL land it through src_pr and then src_land, and the landing is green only when src_land's Jev verdict is solved or unsure and every leg of .land/gates.sh reports status 0. WHEN the landing is green, the proof's findings section holds zero lines and no check's outcome is incomplete, THE SYSTEM SHALL set to done each of the seven cards whose every row R5 records built, each naming the landing commit, and SHALL leave in review each card that carries a row R5 records not built, whose not built state the proof's card table (R5) records. IF the proof's findings section holds any line, THEN THE SYSTEM SHALL leave all seven cards in review and SHALL file one card on the step 5 board per finding line, its words taken from that line, filing in todo each finding card whose line reads `holds none of the seven`. THE SYSTEM SHALL NOT set done a card that carries a row R5 records partly present, so card UzVQkTaU stays in review while the finding naming docs/design/home/LOSS-ACCOUNT.md is open; SHALL NOT set a card done on a landing that is not green; SHALL NOT move any card other than the seven and the finding cards it files; and SHALL NOT file a card from a line under `Observed, not measured`.

**Acceptance:**
- src_pr and src_land each record success for this card, and src_land's record carries a Jev verdict word from {solved, unsure} and seven gate legs at status 0.
- With N lines in the findings section and N greater than 0, the step 5 board carries N new cards, each worded from one finding line, and all seven cards are in review.
- Each new card worded from a finding line that reads `holds none of the seven` is in todo.
- Card UzVQkTaU is in review after the landing.
- With zero lines in the findings section and every check's outcome pass, each of the seven cards whose rows R5 records all built is done and names the landing commit.
- Each card carrying R7 or R10 is in review after the landing, and the proof's card table records R7 and R10 not built on the lines of the cards that carry them.
- No card on the step 5 board other than the seven and the filed finding cards changed state.

**Checklist:**
- C21 — The card lands through src_pr and src_land with Jev and the gate; the seven cards are set done only on a green landing with no finding, a card carrying a partly present row stays in review, and one card is filed per finding line.

**Stories:**
- S11 (Board reader, Reads the step 5 board to know what the chain has judged) — As a reader of the step 5 board, I want a card set done only when the chain judged its rows' code and found nothing, so that done never means merely merged.

## Boundaries

- No byte under crates/ changes: no fmt rewrite, no clippy fix, no test added, no finding fixed.
- HOME-001.json and HOME-001.md are not edited, and no HOME-001 row is changed.
- No check is run in a rewriting form: fmt is measured only as `cargo fmt --all -- --check`, and clippy never with --fix; a rewrite made by any gate leg is never committed.
- The commit measured is 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and no other, even when main moves before the card lands.
- No card is moved except the seven that carry HOME-001's rows R1 to R11, and no card is filed except one per finding line.
- No transcript content, block content or body content appears in the proof, a finding line, a card or a log line.
- .land/gates.sh and docs/design/project.json are not changed, and ast-grep is not added to the gate.
- The design's structure array is the whole file list; no path outside it is created.

## Verification

- From the repository root: python3 scripts/design/validate.py docs/design/home exits 0.
- From the repository root: python3 scripts/design/check-coverage.py docs/design/home exits 0.
- From the repository root: sh scripts/design/gate.sh exits 0 at the card head.
- git diff --quiet 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff <card head> -- crates/lys-home exits 0.
- docs/design/home/PROOF-CHAIN.md names 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, holds 25 Jev verdict lines, six check lines, the findings section, the `Observed, not measured` heading, an 11-line row table and a 7-line card table.
- In a clean checkout of 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, `sh .land/gates.sh` gives the leg statuses the proof records.

