---
type: brief
id: HOME-024
cluster: home
title: Judge the hand-built HOME-001 code in crates/lys-home at 0073b966 through the chain's whole judgement and record each check's outcome in a proof, without changing the crate
---

# HOME-024: Judge the hand-built HOME-001 code in crates/lys-home at 0073b966 through the chain's whole judgement and record each check's outcome in a proof, without changing the crate

> **Cluster:** home
> **Blocked by:** A worktree of lys at 0073b966 on the build host with toolchain 1.97.1 installed, A working jev_ask.awl route (Jev key and ledger) able to judge a file against the empty tree, Read access to the step 5 board for the map of its seven cards to HOME-001 rows
> **Design anchor:**
> - ADR-031 — The context record's cross-kind order is the order the harness's request gives — The context record's documents follow the order the harness's request gives wherever it and the read order differ: appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index. The MCP configuration sits straight after the appended instructions, both harness-side inputs ahead of the first message, until a card measures a real server's tools position. The within-directory order (CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md) is unchanged. Entries already written under the old order stand as written and no data member is added to mark the order; the documents name the old order as superseded by the commit that lands HOME-011 and by that commit's date. Rejected: keeping the read order, since the request is what reached the model; rewriting or migrating entries already written, since the record is append-only; adding a data member or relying on harness_version to mark the order, since the old order was written under 2.1.283, the new one is written under the version the re-measurement ran on, which may be the same, and the harness version does not tell the two orders apart.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
> **Checklist:**
> - C83 — docs/design/home/PROOF-CHAIN.md names commit 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and toolchain 1.97.1, maps every HOME-001 row to its files present or absent at that commit, names R7, R10 and R12 unbuilt, and maps the seven step 5 cards to their rows.
> - C84 — The proof holds one Jev line per file of crates/lys-home at 0073b966, 25 in all, each with model, run id and verdict.
> - C85 — The proof records the seven commands .land/gates.sh runs at 0073b966 with each exit status and outcome, and the ast-grep leg as not measured naming ngzIkkpd.
> - C86 — Every finding is its own proof line marked still true at a named main head or answered by a named commit, and only still-true findings in crates/lys-home or docs/design/home are listed for the step 5 board.
> - C87 — The proof records the missing LOSS-ACCOUNT.md, record/mod.rs's functions and claude_code/mod.rs's consts and fn as findings with their citations and counts.
> - C88 — The proof gives each of the seven step 5 cards the verdict done or in review by row, and never done to 0BgjD59r or qY9mBz-y.
> - C89 — The landing changes nothing under crates/lys-home and passes src_pr, src_land and sh scripts/design/gate.sh.
> - C90 — After the green landing the step 5 board matches the proof: done cards point at the landing, in-review cards at their finding cards.
> **Stories:**
> - S35 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want each check of the chain's judgement recorded with the commit and toolchain it was measured on, so that I can rerun the deterministic checks at that commit and see the same outcome.
> - S36 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want every finding recorded as its own line marked still true or answered by a named commit, so that only what is still wrong on main becomes a card.
> - S37 (Lead, Owns the cards of a board and sets them done) — As the lead who owns the step 5 board, I want a card set done only when the chain has judged its rows' files and no open finding touches them, so that done means judged by the chain and never just merged.

## Purpose

HOME-001's built rows reached main by hand before every carded row had to go through its board's chain, so nothing on main says the chain has judged that code and the cards carrying those rows cannot honestly be done. This brief runs the chain's whole judgement over crates/lys-home as it stands at 0073b966 (Jev per file, fmt, clippy pedantic, tests, the gate's other legs, and ast-grep recorded as not measured), records every outcome and every finding in one proof document against that commit and toolchain, and lands it without changing a byte of the crate, so a card is done only when the chain has judged its row's files and no open finding touches them (ADR-031).

## Task

Measure, do not change. Every compile, test battery and gate of this card runs on Dean's laptop, never on the Mac the card is written on; the words' sentence about the one-build queue on this Mac is dropped. Make a worktree of lys at 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff there with toolchain 1.97.1 (rust-toolchain.toml at that commit) and run the seven commands .land/gates.sh runs at that commit, verbatim, workspace-wide as the gate runs them; fmt is measured by `cargo fmt --check`, the gate's own form, never the rewriting `cargo fmt --all`. Clippy pedantic is the workspace lint table's `pedantic = warn` under `-D warnings`, measured through the two clippy legs. Ask Jev about each of the crate's 25 files at that commit through jev_ask.awl, against the empty tree, cut per part as the chain cuts a new file, with no new src_land mode; Jev's lines are evidence (model, run id, verdict), not a rerunnable command, so the rerun promise of the acceptance covers the seven deterministic legs at the named commit and toolchain only. ast-grep is recorded as not measured, naming card ngzIkkpd, which writes lys's first rule set; no borrowed rule set is run. Only rows whose files are on main at 0073b966 are judged: R7, R10 and R12 are unbuilt and named so, and their cards (0BgjD59r for R7, qY9mBz-y for R10) are never set done here. Each finding is its own line, checked against the current main head: a finding answered there names its fixing commit and gets no card; one still true in crates/lys-home or docs/design/home becomes one card on the step 5 board; one outside them is shown whole and named for its own board. Three findings stand on the repository's rules whatever Jev says: the missing docs/design/home/LOSS-ACCOUNT.md (R4), record/mod.rs carrying functions (R1, R2), and claude_code/mod.rs carrying consts and a fn (R3, R4). A card is done only if no finding still true touches its rows' files; green means the landing passed src_pr and src_land, never that the crate is clean. The measurement at 0073b966 and src_land's gate on the landing head are recorded apart, each with its sha. Out of scope: any new capability in lys-home, any change to a HOME-001 row, writing LOSS-ACCOUNT.md, moving any function, and any card other than the seven. Every requirement writes into the one proof document, so the primary-file rule is met by the proof's sections rather than by separate files.

## Requirements

### R1: Frame the proof: the measured commit, the toolchain, the row map and the card map

THE SYSTEM SHALL create docs/design/home/PROOF-CHAIN.md whose first section names the commit every leg of this judgement was measured on as 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff, the toolchain as the channel 1.97.1 that rust-toolchain.toml pins at that commit together with the `rustc --version` and `cargo --version` lines printed where the legs ran, and states that the legs ran from the root of a worktree checked out at that commit on the build host that runs every compile, test battery and gate. It SHALL hold one table row per HOME-001 requirement, R1 to R12, listing every path of that row's files.create and files.modify with `present` or `absent` as `git cat-file -e 0073b966:<path>` reports it, and SHALL name R7, R10 and R12 as unbuilt rows that are not judged and need their own words and briefs. It SHALL hold the map of the seven step 5 cards that carry R1 to R11 to the HOME-001 rows each carries, by card id, as the step 5 board holds it, with 0BgjD59r carrying R7 and qY9mBz-y carrying R10. THE SYSTEM SHALL NOT judge a row any of whose code files is absent at that commit, SHALL NOT edit, reword or renumber any HOME-001 row, and SHALL NOT record as this judgement anything measured on a checkout of another commit.

**Acceptance:**
- `grep -c 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff docs/design/home/PROOF-CHAIN.md` prints a number of 1 or more, and the frame section names the toolchain channel `1.97.1`.
- The row table holds exactly 12 rows, R1 to R12, and its present and absent marks equal what `git cat-file -e 0073b966:<path>` reports for each listed path.
- The row table marks docs/design/home/LOSS-ACCOUNT.md absent in R4's row; crates/lys-home/examples/passthrough.rs and docs/design/home/PROOF-PROXY.md absent in R7's row; 17 paths absent in R10's row; crates/lys-home/src/record/handover.rs and docs/design/home/PROOF-HANDOVER.md absent in R12's row; and every listed path of R1, R2, R3, R5, R6, R8, R9 and R11 present.
- R7, R10 and R12 are each named as unbuilt, needing their own words and brief, and no Jev line, leg outcome or card verdict of this proof is attributed to them as judged.
- The card map lists exactly seven card ids, every row R1 to R11 appears under a card, 0BgjD59r lists R7 and qY9mBz-y lists R10.

**Files:**
- create: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C83 — docs/design/home/PROOF-CHAIN.md names commit 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and toolchain 1.97.1, maps every HOME-001 row to its files present or absent at that commit, names R7, R10 and R12 unbuilt, and maps the seven step 5 cards to their rows.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want each check of the chain's judgement recorded with the commit and toolchain it was measured on, so that I can rerun the deterministic checks at that commit and see the same outcome.

### R2: Record Jev's verdict on each file of the crate at the measured commit

WHEN the build judges crates/lys-home at 0073b966, THE SYSTEM SHALL ask Jev through jev_ask.awl about each of the 25 paths `git ls-tree -r --name-only 0073b966 crates/lys-home` prints, the change being that file added against the empty tree and cut per part as the chain cuts a new file, and SHALL record in docs/design/home/PROOF-CHAIN.md one line per path with the parts asked, the model, the run id, the verdict (`patched`, `solved`, `unsure` or `not_asked`), the patched value when Jev gives one, held, and Jev's reason. IF a file's verdict is `unsure`, THEN THE SYSTEM SHALL ask once more with the whole file as one part and record both asks, and the line SHALL read `unsure` when the second ask is also unsure. IF the verdict is `not_asked`, THEN the line SHALL record the file as not measured with Jev's reason, and IF that reason is that the chain skipped a file it should have read, THEN THE SYSTEM SHALL record a finding against the chain, for the chain's board. THE SYSTEM SHALL record the Jev lines as evidence, SHALL NOT present them as a command a reader can rerun, SHALL NOT ask Jev about any path outside crates/lys-home, SHALL NOT add a src_land mode, and SHALL NOT let a `solved` verdict close a finding another leg holds.

**Acceptance:**
- The Jev section holds exactly 25 file lines, and their paths equal, as a set, the 25 lines `git ls-tree -r --name-only 0073b966 crates/lys-home` prints.
- Every Jev line carries a non-empty model, a non-empty run id, and a verdict that is exactly one of `patched`, `solved`, `unsure` and `not_asked`.
- Every Jev line whose first ask was `unsure` carries a second ask whose part is the whole file.
- Every Jev line reading `not_asked` reads not measured and carries Jev's reason.
- No Jev line names a path outside crates/lys-home: docs/design/home/RECORD.md, PROOF-RESUME.md, PROOF-FEWSHOT.md, PROOF-CANON.md and canon/canon.jsonl have no Jev line.
- The Jev section states that its lines are evidence and not a rerunnable command.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C84 — The proof holds one Jev line per file of crates/lys-home at 0073b966, 25 in all, each with model, run id and verdict.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want each check of the chain's judgement recorded with the commit and toolchain it was measured on, so that I can rerun the deterministic checks at that commit and see the same outcome.

### R3: Run the gate's seven deterministic legs at the measured commit and record the ast-grep leg as not measured

THE SYSTEM SHALL run, from the root of the worktree at 0073b966 with toolchain 1.97.1, each of the seven commands .land/gates.sh runs at that commit, verbatim, workspace-wide and in its order: `sh scripts/design/gate.sh`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --no-deps --all-features`, `cargo doc --no-deps`; and SHALL record in docs/design/home/PROOF-CHAIN.md for each leg the exact command, the commit, its exit status and its outcome, `pass` for exit status 0 and `fail` for any other, and for a failing leg the failing crate or design cluster, the command and the first error it printed. Clippy pedantic SHALL be measured by the two clippy legs through the workspace lint table's `pedantic` at `warn` under `-D warnings`, with no flag added. THE SYSTEM SHALL record the ast-grep leg as not measured, naming card ngzIkkpd as the reason: lys has no ast-grep rule set until that card lands. THE SYSTEM SHALL state that the rerun promise covers these seven deterministic legs at that commit and toolchain only, and not Jev. THE SYSTEM SHALL NOT run fmt in its rewriting form, SHALL NOT borrow an ast-grep rule set, SHALL NOT change a byte of the worktree before a leg runs, and SHALL NOT record the landing head's gate as this measurement, nor this measurement as the landing's gate.

**Acceptance:**
- The legs section holds exactly seven leg lines, whose commands are, verbatim and in order: `sh scripts/design/gate.sh`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --no-deps --all-features`, `cargo doc --no-deps`.
- Every leg line names 0073b966, an integer exit status, and `pass` when the exit status is 0 and `fail` when it is not.
- Every leg line reading `fail` names the failing crate or design cluster, the command and the first error line.
- The proof holds an ast-grep line that reads not measured and names ngzIkkpd.
- The rerun statement names the seven commands, 0073b966 and toolchain 1.97.1, and says that Jev's lines are not covered by it.
- Running each recorded command from the root of a worktree made by `git worktree add <dir> 0073b966`, with toolchain 1.97.1, gives the exit status the proof records for it.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C85 — The proof records the seven commands .land/gates.sh runs at 0073b966 with each exit status and outcome, and the ast-grep leg as not measured naming ngzIkkpd.

**Stories:**
- S35 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want each check of the chain's judgement recorded with the commit and toolchain it was measured on, so that I can rerun the deterministic checks at that commit and see the same outcome.

### R4: Record each finding as its own line, checked against the current main head

THE SYSTEM SHALL record in docs/design/home/PROOF-CHAIN.md every finding of every leg as its own line: each `patched` and each `unsure` Jev line, each diagnostic of a failing deterministic leg, each finding against the chain, and each repository-rule finding of R5. Each line SHALL carry a finding id, the leg, the path and line it concerns, what was found (for a `patched` line, what Jev found), the HOME-001 rows whose files include that path, and exactly one status against the main head the proof names by full sha: `still true at <sha>`, or `answered by <sha>` naming the commit that fixed it. WHEN a finding still true lies in crates/lys-home or docs/design/home, THE SYSTEM SHALL list it under findings for the step 5 board, one card each, worded from its line, with the act that answers it; for an `unsure` line that act is a person's read of that file, recorded as a card. WHEN a finding lies outside crates/lys-home and docs/design/home, THE SYSTEM SHALL list it under findings outside this card's scope with the board it belongs to, a lys-core finding on the Lys root board and a design cluster's finding on that cluster's board, and it SHALL hold no HOME-001 row. WHEN a finding is against the chain, THE SYSTEM SHALL list it for the chain's board, holding no row. THE SYSTEM SHALL NOT give an answered finding a card, SHALL NOT put two findings on one line, and SHALL NOT put a finding outside crates/lys-home and docs/design/home on the step 5 board.

**Acceptance:**
- Every finding line carries exactly one of `still true at <sha>` and `answered by <sha>`, and `git cat-file -t <sha>` prints `commit` for every sha named.
- Every Jev line reading `patched` or `unsure` is cited by exactly one finding line, and every failing leg line is cited by one finding line or more.
- Every finding line under findings for the step 5 board reads still true and names a path under crates/lys-home or docs/design/home.
- Every finding line under findings outside this card's scope names its board and holds no HOME-001 row.
- No finding line reading answered appears under findings for the step 5 board.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C86 — Every finding is its own proof line marked still true at a named main head or answered by a named commit, and only still-true findings in crates/lys-home or docs/design/home are listed for the step 5 board.

**Stories:**
- S36 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want every finding recorded as its own line marked still true or answered by a named commit, so that only what is still wrong on main becomes a card.

### R5: Record the three findings against the repository's own rules

THE SYSTEM SHALL record in docs/design/home/PROOF-CHAIN.md, whether or not Jev or a leg raises them, three findings against the repository's own rules, each as its own line under R4's form. (a) docs/design/home/LOSS-ACCOUNT.md, which HOME-001 R4 lists in files.create, does not exist at 0073b966 nor at the main head the proof names by `git cat-file -e`; the line cites docs/design/home/briefs/HOME-001.json line 148, docs/design/home/briefs/HOME-001.md line 137, docs/design/home/DESIGN.md line 84 and docs/design/home/design.json line 135 as they stand at 7b53625, holds R4, and names as the act that answers it a card that writes LOSS-ACCOUNT.md from render.rs as it stands on main. (b) crates/lys-home/src/record/mod.rs carries functions against the rule that a mod.rs carries only pub mod, pub use and module docs; the line records its line count and fn count at 0073b966 and at the main head, holds every row whose files include that path (R1 and R2), and names as the answering act a card that moves those functions into named modules under record/ so mod.rs carries only pub mod, pub use and docs. (c) crates/lys-home/src/harness/claude_code/mod.rs carries pub consts and a fn beside its pub mods under the same rule; the line records what the file carries at 0073b966 and at the main head, records the fn projects_slug as answered by ca7b462 where the head carries it as `pub use paths::projects_slug`, holds R3 and R4 while any pub const or fn remains in the file at the head, and names as the answering act a card that moves what remains into a named module. THE SYSTEM SHALL NOT write LOSS-ACCOUNT.md, SHALL NOT move any function or const, and SHALL NOT give R4 the verdict done on the strength of render.rs.

**Acceptance:**
- The LOSS-ACCOUNT finding names docs/design/home/LOSS-ACCOUNT.md and the four citations HOME-001.json:148, HOME-001.md:137, DESIGN.md:84 and design.json:135, and holds R4.
- The record/mod.rs finding records 552 lines and 36 fn items at 0073b966, the values `git show 0073b966:crates/lys-home/src/record/mod.rs | wc -l` and `git show 0073b966:crates/lys-home/src/record/mod.rs | grep -cE '\bfn [a-z_0-9]+'` print, records the values the same two commands print at the main head it names, and holds R1 and R2.
- The claude_code/mod.rs finding records the four pub consts HARNESS, PROVIDER, API and AUTHORED and the fn projects_slug at 0073b966, and records projects_slug as answered by ca7b462.
- Read at head 7b53625, crates/lys-home/src/harness/claude_code/mod.rs carries the four pub consts and `pub use paths::projects_slug`, so the claude_code/mod.rs finding measured against that head reads still true for the consts and holds R3 and R4.
- `git diff --stat <landing base>..<landing head> -- docs/design/home/LOSS-ACCOUNT.md crates/lys-home` prints nothing.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C87 — The proof records the missing LOSS-ACCOUNT.md, record/mod.rs's functions and claude_code/mod.rs's consts and fn as findings with their citations and counts.

**Stories:**
- S36 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want every finding recorded as its own line marked still true or answered by a named commit, so that only what is still wrong on main becomes a card.

### R6: Give each of the seven cards its verdict by row

THE SYSTEM SHALL record in docs/design/home/PROOF-CHAIN.md one verdict for each of the seven step 5 cards of R1's map: `done` when every row the card carries is built and no finding still true at the main head touches any file of those rows, and `in review` otherwise, naming each finding id that holds it. WHILE the finding line citing a `patched` or `unsure` Jev line on a row's file reads `still true at <sha>`, that sha being the main head the proof names, THE SYSTEM SHALL hold the row on it. THE SYSTEM SHALL NOT hold a row on a `patched` or `unsure` Jev line whose finding line reads `answered by <sha>`, nor on a `solved` or `not_asked` line. Cards 0BgjD59r and qY9mBz-y SHALL be recorded as not set done by this card, their rows unbuilt. THE SYSTEM SHALL state that green means the landing passed src_pr and src_land and never that the crate is clean. THE SYSTEM SHALL NOT give `done` to a card any of whose rows is held or unbuilt, and SHALL NOT give a verdict to any card other than the seven.

**Acceptance:**
- The verdict table holds exactly seven lines, one per card id of R1's card map.
- Each card carrying a row whose files include a path named by an R5 finding line recorded as `still true at <sha>`, that sha being the main head the proof names, reads `in review` and names that finding id.
- 0BgjD59r and qY9mBz-y read not set done.
- A card whose only `patched` or `unsure` Jev line is cited by a finding line reading `answered by <sha>` reads `done`, provided every row it carries is built and no finding line reading `still true at <sha>` at the main head the proof names touches a file of those rows.
- No card reading `done` carries a row whose files include a path named by a finding line reading still true.
- The proof holds the statement that green means the landing passed src_pr and src_land and does not mean the crate is clean.

**Files:**
- modify: docs/design/home/PROOF-CHAIN.md

**Checklist:**
- C88 — The proof gives each of the seven step 5 cards the verdict done or in review by row, and never done to 0BgjD59r or qY9mBz-y.

**Stories:**
- S37 (Lead, Owns the cards of a board and sets them done) — As the lead who owns the step 5 board, I want a card set done only when the chain has judged its rows' files and no open finding touches them, so that done means judged by the chain and never just merged.

### R7: Land the proof through src_pr and src_land without changing the crate

WHEN the build lands, THE SYSTEM SHALL land through src_pr and src_land, with src_land's Jev reading the landing diff and src_land's gate running on the landing head, and that gate's outcome SHALL be recorded as the landing's own gate with the landing head's sha, apart from the 0073b966 measurement. The landing diff SHALL hold docs/design/home/PROOF-CHAIN.md, this card's brief and design rows in docs/design/home and the markdown rendered from them, and the roadmap and decision rows written with this brief. THE SYSTEM SHALL NOT change any path under crates/lys-home, SHALL NOT change any HOME-001 row, and SHALL NOT land any other file.

**Acceptance:**
- `git diff --stat <landing base>..<landing head> -- crates/lys-home` prints nothing.
- `git diff --name-only <landing base>..<landing head>` lists only paths under docs/design.
- `git diff <landing base>..<landing head> -- docs/design/home/briefs/HOME-001.json` prints nothing.
- `sh scripts/design/gate.sh` exits 0 at the landing head.
- src_pr and src_land each record a pass for this card, and src_land's record names the landing head's sha and its gate outcome.

**Files:**
- create: docs/design/home/briefs/HOME-008.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C89 — The landing changes nothing under crates/lys-home and passes src_pr, src_land and sh scripts/design/gate.sh.

**Stories:**
- S37 (Lead, Owns the cards of a board and sets them done) — As the lead who owns the step 5 board, I want a card set done only when the chain has judged its rows' files and no open finding touches them, so that done means judged by the chain and never just merged.

### R8: After a green landing, set the board from the proof

WHEN the landing is green through src_pr and src_land, THE SYSTEM SHALL file one card on the step 5 board for each finding the proof lists under findings for the step 5 board, worded from its proof line; SHALL set each of the seven cards whose verdict reads `done` to done with a pointer to the landing commit; and SHALL keep each card whose verdict reads `in review` in review with a pointer to each finding card that holds it. THE SYSTEM SHALL NOT set 0BgjD59r or qY9mBz-y done, SHALL NOT file a finding outside crates/lys-home and docs/design/home on the step 5 board, and SHALL NOT change any card other than the seven beyond filing finding cards.

**Acceptance:**
- Every card whose verdict in the proof reads `done` reads done on the step 5 board and names the landing commit's sha.
- Every card whose verdict reads `in review` reads in review on the step 5 board and names one finding card for each finding id the proof lists against it.
- The step 5 board gains exactly one new card per finding line under findings for the step 5 board, and each new card names its finding id.
- 0BgjD59r and qY9mBz-y hold the same state after the board acts as before them.

**Checklist:**
- C90 — After the green landing the step 5 board matches the proof: done cards point at the landing, in-review cards at their finding cards.

**Stories:**
- S37 (Lead, Owns the cards of a board and sets them done) — As the lead who owns the step 5 board, I want a card set done only when the chain has judged its rows' files and no open finding touches them, so that done means judged by the chain and never just merged.

## Boundaries

- No byte under crates/lys-home changes: the landing diff under that path is empty.
- No HOME-001 row (R1 to R12) is edited, reworded or renumbered.
- docs/design/home/LOSS-ACCOUNT.md is not written by this card, and no function or const is moved out of record/mod.rs or claude_code/mod.rs.
- No compile, test battery or gate of this card runs anywhere but the build host that runs every compile; the measurement is taken on a worktree at 0073b966, never on the landing head.
- fmt is measured only in its check form; the rewriting form is never run on the worktree.
- No ast-grep rule set is borrowed, and no src_land mode is added.
- Jev reads only the 25 files of crates/lys-home at 0073b966; the rows' documents are not asked.
- No card other than the seven step 5 cards is set done or changed, beyond filing the finding cards the proof lists; 0BgjD59r and qY9mBz-y are never set done.
- A finding outside crates/lys-home and docs/design/home is recorded in the proof and never filed on the step 5 board.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- `git diff --stat <landing base>..<landing head> -- crates/lys-home` prints nothing.
- `grep -c 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff docs/design/home/PROOF-CHAIN.md` prints 1 or more.
- The proof's Jev section has 25 lines whose paths equal the output of `git ls-tree -r --name-only 0073b966 crates/lys-home`.
- The proof's legs section has seven lines naming the seven commands .land/gates.sh runs at 0073b966, and one ast-grep line naming ngzIkkpd.
- Rerun `sh .land/gates.sh` from the root of `git worktree add <dir> 0073b966` with toolchain 1.97.1: each leg's exit status equals the one the proof records.
- The proof holds the three repository-rule findings, and the verdict table gives `in review`, naming the finding id, to each card carrying a row whose files include a path named by a finding recorded as still true at the main head the proof names.
- `sh scripts/design/gate.sh` exits 0 at the landing head.
- The step 5 board, read after the board acts, matches the proof's verdict table card for card.
