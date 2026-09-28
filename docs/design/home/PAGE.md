# home — what was asked, what it means, and what was written

## The words, as they were typed

A compaction is the first derived record, because the harness itself produces it: Claude Code writes a summary record and keeps everything before it in its own file, and the importer already turns that record into Pi's compaction entry with its summary and first kept entry. What the home does not yet say is what the compaction could not keep, and nothing yet proves, on a compacted session, that the original is all still there. This card adds both. When a compaction entry enters a session, on import of a Claude Code file that carries a summary record, the importer appends directly after it a lys.loss custom entry that points at the compaction by entry id and names what fell outside the kept range: the first and last entry id of the span it summarises, from the root or the previous compaction to the entry before the first kept one, and the counts of entries, messages, tool calls, tool results and blocks in that span with their bytes, and the tokensBefore the harness reported; it names entry ids and hashes and never a word of content. The summarised entries are not touched: they stay on the tree, their blocks stay in the store, and the context path is Pi's reading as today, the compaction first, then the kept entries. A lys-home subcommand lists a session's compactions: for each, the compaction entry, its loss entry, and a check that every entry in the summarised span is still readable by id and every block it references is still held, printed as a JSON report of ids, counts and hashes. A render for Claude Code writes the summary record and the kept entries as R4 does now, and never the loss entry, so the harness sees what it expects while the home keeps the whole. Acceptance is that a fixture Claude Code file with a summary record imports to a session whose compaction entry is followed by a loss entry naming that compaction, the span's first and last ids and counts that match the fixture; that every entry before the first kept one is still on the tree with its blocks present, checked by the listing; that the context path is the compaction followed by the kept entries and nothing from the span; that the rendered file holds the summary record and the kept entries and no loss line; that importing the same fixture twice into two homes gives byte-identical loss entries apart from ids and timestamps; that a summary record whose first kept entry is not on record is refused by uuid as the importer refuses an unknown parent today; and that one real compacted Claude Code session is imported on this Mac and its listing recorded in a proof document as counts and hashes only. Not in scope: the home compacting a session itself; translation, which is stage 4b and its own card; changing what Claude Code writes; the handover letter, which is HOME-001 R12. Filed by Archie on Tom's roadmap stage 4 of 22 September 2026, a compaction is stored beside its original, points at it and says what it could not keep, the original never replaced, and on home DESIGN P1, P2 and P7 and HOME-001 R3 and R4 at lys main 0073b966, on 26 September 2026.

## What the survey found, and its angles

The words ask that when a Claude Code file that holds a compaction is imported, the home writes a lys.loss custom entry straight after the compaction entry. That entry names the compaction, the first and last entry ids of the span it summarised, counts and bytes for that span, and the harness's tokensBefore, and it holds no content. A new lys-home subcommand lists each compaction with its loss entry and checks that every summarised entry and its blocks are still held. The Claude Code render stays as R4 has it and never writes the loss entry. There are also two proofs: a fixture proof and one real compacted session imported on this Mac. The tree conflicts with the words' premise. Claude Code 2.1.2xx does not write a `summary` record for a compaction. It writes a `system`/`compact_boundary` record with `compactMetadata.preservedSegment` and `preTokens`, then a `user` record marked `isCompactSummary`. The importer maps neither of these to a compaction entry today, and the renderer's `summary` line is not that shape.

### What the tree holds

- `crates/lys-home/src/harness/claude_code/import.rs` — Today it turns only a `type:"summary"` record into a Compaction entry (lines ~258-285). It sets `first_kept_entry_id` to the compaction's own fresh id and `tokens_before: 0`, which means nothing is kept and the harness's count is dropped. A `compact_boundary` record goes through `event_of` as a `lys.harness_event` system event: its null parentUuid falls back to the chain leaf. The `isCompactSummary` record imports as an ordinary user message. The loss entry would be appended here, and so would the unknown-first-kept refusal. The file is already 473 code lines against the 500 limit.
- `crates/lys-home/src/harness/claude_code/events.rs` — `event_of` maps `system` records to KIND_SYSTEM and keeps only subtype, level and a few counts in detail. This is where compact_boundary lands today. If the boundary becomes a compaction entry, this mapping has to stop claiming it or has to sit beside the new one.
- `crates/lys-home/src/harness/claude_code/render.rs` — A Compaction renders as `{"type":"summary","summary","leafUuid":prev}`. It does not advance `prev`, so the first kept record renders with parentUuid null. Custom entries are skipped (`_ => {}`), so a lys.loss entry is already never rendered. The words' rule that a render never writes the loss entry holds by construction here, but the words' claim that the harness then sees what it expects does not.
- `crates/lys-home/src/record/mod.rs` — `context_path()` (lines 419-450) follows Pi's buildSessionContext: the last compaction on the path, then the path entries from `first_kept_entry_id` up to it, then everything after it. A lys.loss entry that is a child of the compaction and on the path comes back in `context_path()` as a custom entry. The acceptance test 'compaction followed by the kept entries and nothing from the span' must say how it treats that entry. `customs_everywhere` can find loss entries off the path.
- `crates/lys-home/src/record/entries.rs` — Holds the lys custom-type constants (lys.harness_event, lys.call, lys.authored, lys.inherited). A new `lys.loss` constant goes here. The Compaction body already keeps unknown Pi fields (`details`, `fromHook`) in `rest`.
- `crates/lys-home/src/record/blocks.rs` — A content-addressed store: put returns hash and new, get by hash. The listing's 'every block it references is still held' check reads from here. However, import's `store_part` throws away the returned hash, and message entries keep their content inline in Pi shape, so no entry names a block.
- `crates/lys-home/src/cli.rs` — Holds the subcommands (import, render, canon, fewshot, ingest-call, resume-check) and the JSON-report-only output rule of R9. The compaction listing is added here. The file is at 398 code lines.
- `docs/design/home/RECORD.md` — The written contract for lys custom entries and the context path. lys.loss has to be written down here, per the repo rule that a test needs a second party. The file's 'The loss account' section currently covers only R4's render loss.
- `docs/design/home/briefs/HOME-001.json` — R3 and R4 are the specs the words build on. R3 says 'summary records become compaction entries pointing at the first kept entry' and refuses an unknown parentUuid by uuid. R4 says 'a compaction renders as Claude Code's summary record followed by the kept entries'. R3's acceptance requires every message entry's parent to equal the source parentUuid, which limits where the loss entry can sit.
- `docs/design/home/design.json` — The cluster's Structure table, principles P1/P2/P7 and constraints CN3/CN4. A new brief (and its proof document) has to be listed here, and `scripts/design/gate.sh` checks coverage and that the rendered markdown matches.
- `~/.claude/projects/**/*.jsonl (Claude Code 2.1.2xx transcripts)` — This is the real shape the importer must meet: a `system` record with `subtype:"compact_boundary"`, `parentUuid:null`, `logicalParentUuid` (the pre-compaction leaf) and `compactMetadata {trigger, preTokens, postTokens, cumulativeDroppedTokens, preservedSegment {headUuid, anchorUuid, tailUuid}}`. It is followed by a `user` record with `isCompactSummary:true` whose parentUuid is the boundary. Later records chain to that summary record (anchorUuid). headUuid..tailUuid are kept records that sit before the boundary in the file.
- `/Users/tom/Developer/tools/harness/pi/packages/coding-agent/src/core/session-manager.ts @ 3d5cbe98` — CompactionEntry {summary, firstKeptEntryId, tokensBefore, details?, fromHook?} (lines 67-76). buildSessionContext (around line 372) emits the summary first, then the kept entries from firstKeptEntryId, then the entries after the compaction. Custom entries are not model context. This is the reading the words call 'Pi's reading as today'.

### What was already decided

- home DESIGN P1 — The original bytes are never rewritten; a compaction is a derived record stored beside its source and pointing at it. The loss entry and listing carry this out on the home side.
- home DESIGN P2 / CN4 — The record is Pi's session tree. lys adds entry kinds only as custom entries and never adds a field to Pi's grammar. So lys.loss must be `custom`, and a block reference cannot be added as a field on a Pi message.
- home DESIGN P7 / CN3 — Transcript contents never appear in output, logs, errors, test names or pages; only hashes, counts and offsets. This covers the loss entry, the listing report and the proof document.
- home DESIGN P4 — A content block is stored once by its hash and entries reference blocks. The implementation stores the blocks but no entry records their hashes.
- home DESIGN CN7 — Reading a path seeks through the index and never loads the whole file. A span walk over a large compacted session should respect this.
- home DESIGN Goal 4 — A written loss account for every derived record. This card is the compaction's share of that goal. The Structure row docs/design/home/LOSS-ACCOUNT.md does not exist in the tree.
- HOME-001 R1 — The context path is entries from the latest compaction's firstKeptEntryId onward plus the compaction summary, as Pi reads it. An entry id already on record is refused, and so is a parent that comes after its child.
- HOME-001 R3 — 'summary records become compaction entries pointing at the first kept entry'. An unknown parentUuid is refused by uuid. The acceptance requires every message entry's parent uuid to equal the source record's parentUuid.
- HOME-001 R4 — A compaction renders as Claude Code's summary record followed by the kept entries. Custom entries do not render. An existing target path is refused.
- HOME-001 R8 — `system` records become lys.harness_event entries at their exact place on the chain under their own uuid. The compact_boundary record is imported this way today.
- HOME-001 R9 — Every subcommand prints a JSON report of hashes, counts and paths, with no key named text, content or body, and exits 2 naming any missing argument.
- HOME-001 R12 — The handover letter is excluded by the words.
- home CHECKLIST C3 / C4 — C3 already claims that 'compaction summaries' import and C4 claims the render. Neither has a test: no test file in lys-home mentions summary or compaction on import.
- PROOF-RESUME.md — The R5 proof used a 242-record Archie session with no compaction, so no compacted import has been measured yet.
- CONTEXT-ROADMAP-2026-09-22 stage 4 — A compaction is stored beside its original, points at it and says what it could not keep, and the original is never replaced. This is the authority the words cite.
- RM-005 — 'Give a session a home': this is the roadmap row (status briefed, HOME-001) that the home cluster's work hangs from.

### What was measured

- Claude Code session files on this Mac across ~/.claude, ~/.claude-waffles, ~/.claude-de and ~/.claude-apollo holding a compaction marker: 135 files
- compact_boundary system records in those files: 3,415 (3,283 trigger manual, 132 auto)
- compact_boundary records carrying compactMetadata.preservedSegment {headUuid, anchorUuid, tailUuid}: 2,922 of 3,415; the 493 without it are all from versions before 2.1.281
- compact_boundary records written by 2.1.281 or later: 161, every one with a preservedSegment
- compact_boundary records whose parentUuid is null: 3,415 of 3,415
- compact_boundary records whose preservedSegment.headUuid is not on record before the boundary: 5 (all pre-2.1.281)
- `type:"summary"` records in the same 135 files: 1
- files with compact_boundary under ~/.claude/projects (2,890 jsonl files): 35, each also holding isCompactSummary records
- files with compactions written by 2.1.281 or later: 65
- largest compacted session files: 3,504,675,296 bytes (926 boundaries), 2,368,051,315 bytes (469), 2,199,439,072 bytes (585)
- one measured 2.1.282-era Norn session: records and compactions: 28,093 records, 30 compact_boundary records; headUuid sits 2 to 20 records before the boundary, and tailUuid equals logicalParentUuid in 29 of 30
- installed Claude Code version: 2.1.282
- code lines (non-blank, non-comment) in import.rs / cli.rs / render.rs / record/mod.rs: 473 / 398 / 263 / 417, against a 500 limit
- lys-home source and test lines: 6,618 lines across 25 files
- tests in lys-home that exercise summary or compaction import or render: 0 (record_tests.rs has one hand-built Compaction in the R1 fixture)
- places where the importer records a block hash on an entry: 0: store_part keeps only put.new, message content stays inline in Pi shape; only lys.harness_event.record and lys.call carry hashes
- tokens_before the importer writes on a compaction today: 0 (a constant)
- docs/design/home/LOSS-ACCOUNT.md and PROOF-PROXY.md: both listed in Structure, neither exists
- Pi checkout at /Users/tom/Developer/tools/harness/pi: HEAD 3d5cbe98, CompactionEntry at session-manager.ts:67-76
- tree commit: 0073b96, matching the words' 'lys main 0073b966'

### What it means for the other projects

- aion — The card runs through aion's chain (brief_card, sign-off, card_build_v3, src_pr, src_land) on inputs naming the lys repository, commit 0073b96, the card and the brief. aion's code does not change.
- cambium — The card sits on the Cambium board under Tom's roadmap stage 4. Nothing in Cambium changes.
- method — The new brief and any design.json Structure rows must validate against the method schemas vendored under scripts/design (validate.py, check-coverage.py, render-cluster.py). No change to method.
- argus — Argus's warden requests the manual compactions that produce these files (3,283 of 3,415 boundaries are trigger manual). The listing gives a later Argus view hashes and counts it could read, but nothing in Argus changes on this card.

### The decisions it stands on

- ADR-004 (honour) — lys-home stays standalone. The import, listing and proof need no manifold, aion or broker.
- ADR-007 (honour) — The subcommand only lists and reports. Neither it nor the proof starts a harness on anyone's behalf.
- ADR-003 (honour) — Who may read or resume a home is not changed. The loss entry and listing add no grant and bypass none.
-  (new) — lys.loss is a new lys custom entry type: its data fields, its place in the tree and the determinism rule. It needs recording in RECORD.md beside lys.call, lys.harness_event, lys.authored and lys.inherited, following P2.
-  (new) — If the lead chooses the real shape, Claude Code's compaction mapping becomes a recorded rule: compact_boundary plus isCompactSummary maps to the Pi compaction entry, with firstKeptEntryId set to preservedSegment.headUuid and tokensBefore set to preTokens. It replaces HOME-001 R3's 'summary records become compaction entries' and R4's 'summary record' render for 2.1.281.

### What it requires

- Importing a fixture Claude Code file with a compaction yields exactly one lys.loss custom entry per compaction entry, and its data names that compaction's entry id.
- The loss entry's first and last span ids equal the fixture's first entry after the root or the previous compaction and the entry before the first kept one, and its counts of entries, messages, tool calls, tool results and blocks equal the fixture's counts.
- The loss entry's tokensBefore equals the value the harness recorded in the fixture.
- The loss entry's serialised data contains no message, thinking, tool input or tool result text (a test scans it against the fixture's content strings).
- Two imports of the same fixture into two fresh homes give loss entries that are byte-identical once ids and timestamps are masked.
- Every entry in a compaction's span is still readable by id after import, and the listing reports it as present.
- The listing reports every block the span references as held, and reports a missing block by hash (tested by removing one block file).
- context_path() on the imported fixture holds the compaction, then the kept entries, then later entries, and no entry from the span.
- The Claude Code render of the imported fixture holds the compaction's records and the kept entries, and no line carrying lys.loss.
- A fixture whose compaction names a first kept entry that is not on record is refused with that uuid in the error, and no session file is written.
- The listing subcommand prints one JSON report with no key named text, content or body, and exits 2 naming a missing required argument.
- One real compacted Claude Code session is imported on this Mac, and its listing is recorded in a proof document under docs/design/home with counts, ids and hashes only, the source file's SHA-256 equal before and after.
- The new brief, its checklist item and its Structure rows pass scripts/design/gate.sh.
- All gates pass: fmt, clippy in both feature shapes, tests with --all-features, doc in both shapes, and the design gate.

### What must not change

- No file under ~/.claude/projects (or any config root) is rewritten, truncated or moved (CN1).
- No field is added to Pi's header or to any Pi entry outside custom.data. lys.loss is a custom entry and the compaction entry keeps Pi's shape (P2, CN4).
- The summarised entries and their blocks are never removed or rewritten, and a block is never overwritten (P1, R2).
- No transcript content in the loss entry, the listing, errors, logs, test names or the proof document (P7, CN3).
- The existing unknown-parentUuid refusal and R3's parent-equality for message entries stay as they are.
- R4's thinking rule, the refuse-an-existing-path rule and the loss.json beside a rendered file stay as they are.
- No Norn crate or type enters lys-home (CN6).
- The home does not compact a session, translate one, or write a handover on this card.
- No source file goes over 500 code lines, and there is no #[allow], #[ignore] or other lint bypass.
- Heavy builds and full gates run on Dean's laptop. Only warm single-crate checks run on this Mac.

### What we must put in place first

- The lead answers whether this card maps Claude Code's compact_boundary/isCompactSummary pair, because no real 2.1.281 file carries the summary record the words describe.
- The lead answers how 'every block it references' is measured, because entries record no block hashes today.
- A fixture Claude Code file in the 2.1.281 compaction shape: a pre-compaction chain, compact_boundary with preservedSegment, the isCompactSummary record, and later records chained to it. It must be hand-built with no real content.

### The risks

- The card lands against `type:"summary"` alone and passes its fixture, while every real compacted session on this Mac (3,415 boundaries) still imports with no compaction entry and no loss entry. The real-session proof then finds nothing to list.
- A span walk over a multi-GB session (up to 3.5 GB, 926 compactions) may read far more than the path, against CN7, or run slowly on this Mac.
- The block check agrees with itself when it re-derives hashes from the same code that stored them. A drift test must delete a real block and count that exactly that one was reported.
- Placing the loss entry on the chain would re-parent the record after the compaction and break R3's parentUuid equality. Placing it as a side leaf means a path-only reader never sees it.
- The meaning of the kept range changes once kept entries sit before the boundary in the file (headUuid..tailUuid). Counting them into the span would inflate the counts and wrongly show them as summarised.
- Five older boundaries name a headUuid that is not before them. With a strict refusal, those files will not import at all.
- The name lys.loss may be confused with R4's `.loss.json` render loss account, so RECORD.md must tell the two apart.
- Claude Code 2.1.282 is now installed while the proofs name 2.1.281, so a version drift in the compaction shape could go unmeasured.

### Still open

- Claude Code 2.1.281 records a compaction as a compact_boundary system record plus an isCompactSummary user record, not as a summary record. Does this card teach the importer that pair, so that the first kept entry is preservedSegment.headUuid, tokensBefore is compactMetadata.preTokens, and the summary comes from the isCompactSummary message? Or does it attach the loss entry only to today's `type:"summary"` path, which occurs once on this Mac? The sentence of the words it stands on: "A compaction is the first derived record, because the harness itself produces it: Claude Code writes a summary record and keeps everything before it in its own file, and the importer already turns that record into Pi's compaction entry with its summary and first kept entry.". Why only the lead can settle it: crates/lys-home/src/harness/claude_code/import.rs maps only `type:"summary"` to a Compaction, with firstKeptEntryId set to its own id and tokensBefore 0. Across 135 real files there are 3,415 compact_boundary records and 1 summary record. Today a compact_boundary becomes a lys.harness_event and the summary becomes a plain user message. Without the new mapping, the real-session proof yields no compaction entry, and so no loss entry either.
- For a compaction, should the Claude Code render write what 2.1.281 itself writes (a compact_boundary record, then an isCompactSummary user record, then the kept records), or keep R4's `{type: summary, leafUuid}` line? The sentence of the words it stands on: "A render for Claude Code writes the summary record and the kept entries as R4 does now, and never the loss entry, so the harness sees what it expects while the home keeps the whole.". Why only the lead can settle it: crates/lys-home/src/harness/claude_code/render.rs writes `{"type":"summary","summary","leafUuid"}` and does not advance the parent chain, so the first kept record has parentUuid null. That is not the compaction shape 2.1.281 writes, and it puts the summary text in no record the model is given. A resumed session would lose the summary rather than see what it expects.
- No home entry records which blocks it came from. Should the card add a block reference for each entry, which under P2 can only be a lys custom entry and never a new field on a Pi message? Or is 'every block it references is still held' measured some other way, for example by re-hashing parts or by the harness_event record hashes only? The sentence of the words it stands on: "A lys-home subcommand lists a session's compactions: for each, the compaction entry, its loss entry, and a check that every entry in the summarised span is still readable by id and every block it references is still held, printed as a JSON report of ids, counts and hashes.". Why only the lead can settle it: import.rs store_part discards the hash that BlockStore::put returns, and message entries keep their parts inline in Pi's reshaped form (toolCall, toolResult, thinkingSignature). Re-hashing an entry's part therefore does not give the stored block's hash, and the block check has nothing to follow. The design (P4) says entries reference blocks, but the tree does not.
- A compact_boundary with no preservedSegment (493 records, all older than 2.1.281, including the 3.5 GB Waffles file) names no first kept entry. Should it be refused, or imported as a compaction that keeps nothing, with the whole pre-boundary chain as its span? The sentence of the words it stands on: "Acceptance is that a fixture Claude Code file with a summary record imports to a session whose compaction entry is followed by a loss entry naming that compaction, the span's first and last ids and counts that match the fixture; that every entry before the first kept one is still on the tree with its blocks present, checked by the listing; that the context path is the compaction followed by the kept entries and nothing from the span; that the rendered file holds the summary record and the kept entries and no loss line; that importing the same fixture twice into two homes gives byte-identical loss entries apart from ids and timestamps; that a summary record whose first kept entry is not on record is refused by uuid as the importer refuses an unknown parent today; and that one real compacted Claude Code session is imported on this Mac and its listing recorded in a proof document as counts and hashes only.". Why only the lead can settle it: The words refuse only a first kept entry that is 'not on record'. They do not cover a compaction that names none. That choice decides whether older compacted sessions on this Mac import at all or are refused.

### The units beyond the first

- Render a compaction for Claude Code in the harness's own compact_boundary shape — If the lead keeps R4's summary line on this card, making a resumed rendered session carry its compaction the way 2.1.281 writes it is a separate change to render.rs and needs its own resume measurement.
- Block references on home entries — If the lead chooses a per-entry block manifest (a lys custom entry), it changes every import, not only compacted ones, and needs its own contract and round-trip gates.
- Import older compaction shapes (no preservedSegment, head not before boundary) — 493 boundaries from versions before 2.1.281 name no kept range and 5 name one out of order. Handling them is a separate decision and set of fixtures if this card refuses them.
- LOSS-ACCOUNT.md: the written loss account across render and compaction — The design lists it in Structure and it does not exist. It covers R4's render losses as well as compaction's, which is wider than this card.

### The smallest complete shape

One brief in the home cluster (for example HOME-002) with one landable change to lys-home. The change contains: the importer's compaction mapping, for whichever shape the lead chooses (and the compact_boundary pair if chosen); the lys.loss entry appended at each compaction with deterministic span ids, counts, bytes and tokensBefore; the refusal of an unknown first kept entry by uuid; the compaction listing subcommand with its span-readable and blocks-held check; the render left writing no loss line; RECORD.md naming lys.loss; the fixture tests for every acceptance line; and the proof document holding one real compacted 2.1.281 session's listing as counts and hashes.

## The roadmap row

- **RM-006** — Say what a compaction could not keep, and prove the original is all still there (feature, idea)
- Summary: When a compacted Claude Code session is imported into its home, each compaction entry is followed by a lys.loss entry naming the span it summarised as ids, counts and hashes, the hash of every stored part is kept beside the session, a lys-home subcommand lists each compaction and checks that every summarised entry and block is still held, and the compaction renders for Claude Code in the shape the measured version writes. Proved on a fixture and on one real compacted session.
- Asked by: tom on 2026-09-26T06:36:44+10:00
- Context: The card's words for roadmap stage 4 (a compaction is stored beside its original, points at it and says what it could not keep), on the home design's P1, P2 and P7 and HOME-001 R3 and R4, with the lead's answers to the card's survey (the compact_boundary pair, the render shape, the block rows, the keep-nothing compaction, the boundary whose summary never arrives, and the listing's exit status).
- Quote: A compaction is the first derived record, because the harness itself produces it: Claude Code writes a summary record and keeps everything before it in its own file, and the importer already turns that record into Pi's compaction entry with its summary and first kept entry. What the home does not yet say is what the compaction could not keep, and nothing yet proves, on a compacted session, that the original is all still there. This card adds both. When a compaction entry enters a session, on import of a Claude Code file that carries a summary record, the importer appends directly after it a lys.loss custom entry that points at the compaction by entry id and names what fell outside the kept range: the first and last entry id of the span it summarises, from the root or the previous compaction to the entry before the first kept one, and the counts of entries, messages, tool calls, tool results and blocks in that span with their bytes, and the tokensBefore the harness reported; it names entry ids and hashes and never a word of content. The summarised entries are not touched: they stay on the tree, their blocks stay in the store, and the context path is Pi's reading as today, the compaction first, then the kept entries. A lys-home subcommand lists a session's compactions: for each, the compaction entry, its loss entry, and a check that every entry in the summarised span is still readable by id and every block it references is still held, printed as a JSON report of ids, counts and hashes. A render for Claude Code writes the summary record and the kept entries as R4 does now, and never the loss entry, so the harness sees what it expects while the home keeps the whole. Acceptance is that a fixture Claude Code file with a summary record imports to a session whose compaction entry is followed by a loss entry naming that compaction, the span's first and last ids and counts that match the fixture; that every entry before the first kept one is still on the tree with its blocks present, checked by the listing; that the context path is the compaction followed by the kept entries and nothing from the span; that the rendered file holds the summary record and the kept entries and no loss line; that importing the same fixture twice into two homes gives byte-identical loss entries apart from ids and timestamps; that a summary record whose first kept entry is not on record is refused by uuid as the importer refuses an unknown parent today; and that one real compacted Claude Code session is imported on this Mac and its listing recorded in a proof document as counts and hashes only. Not in scope: the home compacting a session itself; translation, which is stage 4b and its own card; changing what Claude Code writes; the handover letter, which is HOME-001 R12. Filed by Archie on Tom's roadmap stage 4 of 22 September 2026, a compaction is stored beside its original, points at it and says what it could not keep, the original never replaced, and on home DESIGN P1, P2 and P7 and HOME-001 R3 and R4 at lys main 0073b966, on 26 September 2026.
- Cluster: home; briefs: HOME-002
- Notes: Further units, not written: LOSS-ACCOUNT.md: the written loss account across render and compaction.

## The design

---
type: design
cluster: home
title: The home: a session held under its identity, resumable by any harness that can be measured
---

# The home: a session held under its identity, resumable by any harness that can be measured

> **Cluster:** home

## Intention

A session's history lives under the agent's identity, not under a harness. The record is what the model was given and what came back, in order, with each provider's native blocks kept whole. A harness's own resume file is rendered from that record on demand, so an agent moves machines, harnesses or providers and continues; the original bytes are never rewritten.

## Problem

Today a session exists only as its harness's file. Archie's Claude Code file measured 141,931,097 bytes on 24 September 2026: the conversation itself is under 3% of it, tool results 14%, thinking 6%, and about 60% is harness bookkeeping the model never sees; Waffles' file is 3.37 GB and Claude Code loads it whole. Nothing central holds a session, so nothing can render a smaller working session, move one to another device, swap provider and come back, or say what a session was given. Norn already has the canonical event model (SessionEvent, conversion to provider messages, compaction as a derived event, provider epoch boundaries) proved by its own tests; Claude Code has no importer to it and no renderer from it.

## Solution

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it.

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

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-012 — A compaction's loss is a lys.loss custom entry beside it, and a session's block hashes are a lys file beside the session — Each compaction entry is followed in the file by a lys.loss custom entry, a side leaf under the compaction, whose data names the summarised span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks, their bytes, a digest of the span's block hashes and the harness's tokensBefore, deterministic so two imports agree apart from ids and timestamps. Block hashes are kept in <id>.blocks.jsonl beside the session, one {entry, part, hash} row per stored part, as the index and head are kept. Rejected: a new field on a Pi message or a custom entry per message for block references, which adds to Pi's grammar or doubles every import's entries; re-hashing parts or following only harness-event record hashes, which cannot find the stored blocks; and placing the loss entry on the chain, which would re-parent the record after the compaction and break the importer's parent equality.
- ADR-013 — Claude Code's compaction is read and rendered in the shape the measured version writes — The importer reads a compact_boundary record and its isCompactSummary record as one Pi compaction entry: the summary is the isCompactSummary message's text, the first kept entry is the entry of preservedSegment.headUuid (the compaction itself when there is no preservedSegment, so it keeps nothing), tokensBefore is compactMetadata.preTokens, and Pi's details field names both source records by uuid; a named first kept entry not on record is refused by uuid. The summary record path stays for the file that carries one. The render writes a compact_boundary record, then the isCompactSummary record, then the kept records, with the parent chain advancing through all three, measured on the installed Claude Code version. Rejected: attaching the loss entry only to the summary record path, which almost no file uses, and keeping R4's summary line, which a resumed session would not read.

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.

## Non-Goals

- Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for). — Signing comes when asked for (Tom, 22 September 16:27); every stage here works without it.
- Encryption at rest and moving a home between devices (stage 3 preconditions). — Stage 3's three preconditions (identity and read authority, encryption before bytes leave, the resume evidence) are their own brief.
- Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement.
- Lanterns and forks at a coordinate (stages 5 and 5b). — Lanterns and forks stand on a proved resume; this brief supplies that proof.
- Adopting, wrapping or calling Norn's session code; Pi's code is read as the reference and not vendored. — Tom, Dot 13:28: not Norn. Pi's tree is the reference.

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
| `docs/design/home/briefs/HOME-002.json` | the second brief: a compaction's loss entry, the block rows beside a session, the compaction listing, the compaction render, and the proof | HOME-002 |
| `docs/design/home/briefs/HOME-002.md` | its rendered markdown | HOME-002 |
| `docs/design/home/PROOF-COMPACTION.md` | measured: one real compacted Claude Code session imported and listed as counts, ids and hashes, and the compaction render resumed on a named Claude Code version | HOME-002 |
| `crates/lys-home/src/record/block_rows.rs` | the block rows beside a session, <id>.blocks.jsonl: one {entry, part, hash} row per content part stored at import | HOME-002 |
| `crates/lys-home/src/record/block_rows_tests.rs` | gates on the block rows: one row per stored part, the hash put returned, no content | HOME-002 |
| `crates/lys-home/src/record/loss.rs` | the lys.loss entry: a compaction's summarised span walked by seeking, counted, and written as ids, counts and hashes | HOME-002 |
| `crates/lys-home/src/record/loss_tests.rs` | gates on the loss entry: span bounds, counts, bytes and digest against the fixture | HOME-002 |
| `crates/lys-home/src/record/compactions.rs` | the compaction listing: each compaction, its loss entry, and the span's entries read by id and blocks checked as held | HOME-002 |
| `crates/lys-home/src/record/compactions_tests.rs` | gates on the listing: every entry read, a removed block named by hash, an absent rows file reported unverified | HOME-002 |
| `crates/lys-home/src/harness/claude_code/compaction.rs` | Claude Code's compaction records (the compact_boundary and isCompactSummary pair, and the summary record) into one Pi compaction entry | HOME-002 |
| `crates/lys-home/src/harness/claude_code/compaction_tests.rs` | gates on the compaction mapping: first kept, tokensBefore, keep-nothing, the unknown first kept refused by uuid | HOME-002 |
| `crates/lys-home/src/harness/claude_code/render_tests.rs` | gates on the render, including the compaction's compact_boundary shape | HOME-002 |
| `crates/lys-home/src/error.rs` | the crate's errors, including an unknown first kept entry refused by uuid | HOME-002 |
| `crates/lys-home/tests/claude_code_compaction.rs` | end to end on the compacted fixture: import, two homes, context path, listing, render | HOME-002 |
| `crates/lys-home/tests/fixtures/claude_code_compacted.jsonl` | a hand-built Claude Code 2.1.281 file with two compactions and no real content | HOME-002 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'The home', 'The home is portable: lanterns, translation and forks', and steps 4 and 5
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Archie's roadmap; this cluster is its stage 4 and the resume half of stage 3
- `docs/design/identity/briefs/CONTEXT-001.json` — stage 1, byte-for-byte capture and the lys-home crate this cluster extends
- `docs/design/secrets/briefs/SECRETS-002.json` — the door's proxy (R1) that the tee joins once it exists
- `$PI/packages/coding-agent/src/core/session-manager.ts` — Pi's session tree at 3d5cbe98 (github.com/earendil-works/pi): header, entries, leaf (branch at :1125 moves an in-memory cursor; _buildIndex:754 restores the last physical entry on reopen; loadEntriesFromFile:438 reads the whole file)
- `$PI/packages/ai/src/api/transform-messages.ts` — per-provider thinking: same provider, api and model keeps blocks whole; otherwise text, opaque dropped
- `$PI/packages/coding-agent/src/core/branch-summarization.ts` — how an abandoned branch's work is carried into the new one
- `$CCFLARE/README.md` — ccflare at 95c4c6a (github.com/snipeship/ccflare): the concept of a native pass-through that keeps request history; reference only, not used (Tom, Dot 13:34)

## Constraints

- **CN1** — No file under ~/.claude/projects is rewritten or truncated by this cluster; a render writes a new file under a new uuid only, and refuses an existing path by name.
- **CN2** — A rendered file never carries a provider-native opaque block that came from a different provider or model family than the one it is rendered for; the loss account names each block dropped by hash.
- **CN3** — Transcript contents never appear in output, logs, errors, test names or pages; hashes, counts, offsets and event ids only.
- **CN4** — Every home file parses with Pi's parseSessionEntries at 3d5cbe98 unchanged: the header line first, every entry with id, parentId and timestamp, lys data only inside custom entries.
- **CN5** — The pass-through proxy forwards every header and the streamed body unchanged and stores nothing but the measurement; it is an example binary, never a service.
- **CN6** — No Norn crate is a dependency of lys-home and no Norn type is copied into it.
- **CN7** — Reading the root-to-leaf path of a home file reads only the entries on that path plus the index, never the whole file (Pi loads the whole journal; Chippy 13:33); the head is persisted beside the file, never inferred from the last physical entry on reopen.


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
> - C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. A compaction is read as Claude Code 2.1.281 measurably writes it, a compact_boundary system record and its isCompactSummary user record becoming one compaction entry (first kept entry from preservedSegment.headUuid, tokensBefore from compactMetadata.preTokens); this corrects R3's summary record by measurement, and the summary record is still read where a file carries one (the compaction half is HOME-002's).
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
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. A compaction is read as Claude Code 2.1.281 measurably writes it, a compact_boundary system record and its isCompactSummary user record becoming one compaction entry (first kept entry from preservedSegment.headUuid, tokensBefore from compactMetadata.preTokens); this corrects R3's summary record by measurement, and the summary record is still read where a file carries one (the compaction half is HOME-002's).

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
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. A compaction is read as Claude Code 2.1.281 measurably writes it, a compact_boundary system record and its isCompactSummary user record becoming one compaction entry (first kept entry from preservedSegment.headUuid, tokensBefore from compactMetadata.preTokens); this corrects R3's summary record by measurement, and the summary record is still read where a file carries one (the compaction half is HOME-002's).
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
title: Say what each compaction could not keep and prove the original is all still there: the loss entry, the block rows, the compaction listing and the compaction render
---

# HOME-002: Say what each compaction could not keep and prove the original is all still there: the loss entry, the block rows, the compaction listing and the compaction render

> **Cluster:** home
> **Depends on:** HOME-001
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A compaction's loss is a lys.loss custom entry beside it, and a session's block hashes are a lys file beside the session — Each compaction entry is followed in the file by a lys.loss custom entry, a side leaf under the compaction, whose data names the summarised span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks, their bytes, a digest of the span's block hashes and the harness's tokensBefore, deterministic so two imports agree apart from ids and timestamps. Block hashes are kept in <id>.blocks.jsonl beside the session, one {entry, part, hash} row per stored part, as the index and head are kept. Rejected: a new field on a Pi message or a custom entry per message for block references, which adds to Pi's grammar or doubles every import's entries; re-hashing parts or following only harness-event record hashes, which cannot find the stored blocks; and placing the loss entry on the chain, which would re-parent the record after the compaction and break the importer's parent equality.
> - ADR-013 — Claude Code's compaction is read and rendered in the shape the measured version writes — The importer reads a compact_boundary record and its isCompactSummary record as one Pi compaction entry: the summary is the isCompactSummary message's text, the first kept entry is the entry of preservedSegment.headUuid (the compaction itself when there is no preservedSegment, so it keeps nothing), tokensBefore is compactMetadata.preTokens, and Pi's details field names both source records by uuid; a named first kept entry not on record is refused by uuid. The summary record path stays for the file that carries one. The render writes a compact_boundary record, then the isCompactSummary record, then the kept records, with the parent chain advancing through all three, measured on the installed Claude Code version. Rejected: attaching the loss entry only to the summary record path, which almost no file uses, and keeping R4's summary line, which a resumed session would not read.
> **Checklist:**
> - C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. A compaction is read as Claude Code 2.1.281 measurably writes it, a compact_boundary system record and its isCompactSummary user record becoming one compaction entry (first kept entry from preservedSegment.headUuid, tokensBefore from compactMetadata.preTokens); this corrects R3's summary record by measurement, and the summary record is still read where a file carries one (the compaction half is HOME-002's).
> - C14 — When a compaction entry enters a session on import (a compact_boundary whose summary record never arrives included, as a compaction with an empty summary, and a summary read after that as a second compaction entry that completes the first without rewriting it), a lys.loss custom entry follows it, points at it by entry id and names what fell outside the kept range: the span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks with their bytes, and the tokensBefore the harness reported, as ids, counts and hashes and never content.
> - C15 — A lys-home subcommand lists a session's compactions, each with its loss entry and a check that every entry in the summarised span is readable by id and every block it references is held, as a JSON report of ids, counts and hashes; it prints the whole report, then exits non-zero when an entry cannot be read, a block is missing, the session's blocks are unverified or a compaction has no loss entry (its reason given as unknown), naming each by entry id, block hash, session id or compaction id, and exits 0 only when every compaction's loss entry and every block in every span was read.
> - C16 — The hash of every content part stored at import is kept beside the session in <id>.blocks.jsonl, one row per part naming its entry, so the blocks an entry references can be checked as held; a session without the file is reported as unverified.
> - C17 — A compaction renders for Claude Code as the target version writes it (a compact_boundary record, then the isCompactSummary record, then the kept records, the parent chain advancing through all three) and the lys.loss entry never renders.
> - C18 — One real compacted Claude Code session is imported on the machine that holds it and its listing recorded in a proof document as counts, ids and hashes only; the compaction render is measured resuming on a named Claude Code version.
> **Stories:**
> - S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.
> - S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a compacted session's listing to name every summarised entry and block as held, by id and hash, with a missing one named, and the compaction render measured on a named harness version, so that a claim that the original is kept is measured rather than assumed.

## Purpose

A compaction is the first derived record because the harness itself produces it. HOME-001 imports sessions but says nothing of what a compaction could not keep, reads only a summary record that one measured file carries while every other compacted file holds compact_boundary and isCompactSummary records, keeps no hash of the blocks it stores, and renders a compaction as a summary line the model never reads. This brief makes a compacted session's home say, beside each compaction, what fell outside its kept range, lets anyone check that every summarised entry and block is still held, and renders the compaction in the shape the measured harness version writes (design principles P1, P2, P4, P6 and P7; ADR-012 and ADR-013).

## Task

Extend lys-home in the order of the requirements: RECORD.md's contract first (R1), then the block rows (R2), the compaction mapping (R3), the loss entry (R4), the listing (R5), the render (R6), the end-to-end gates (R7) and the proof (R8). The fixture `crates/lys-home/tests/fixtures/claude_code_compacted.jsonl` is sixteen hand-built records of Claude Code 2.1.281's shape, every text a short made-up phrase and no real content, with uuids `00000000-0000-4000-8000-0000000000NN` for NN = 01 to 10 (hex), in this file order, and `…NN` abbreviates `00000000-0000-4000-8000-0000000000NN` wherever this brief writes it: 01 user text (parentUuid null); 02 assistant with a text part and a tool_use part (parent 01); 03 user holding only the tool_result for that tool_use (parent 02); 04 assistant text (parent 03); 05 user text (parent 04); 06 assistant text (parent 05); 07 system compact_boundary (parentUuid null, logicalParentUuid 06, compactMetadata {trigger "manual", preTokens 12345, preservedSegment {headUuid 05, anchorUuid 08, tailUuid 06}}); 08 user isCompactSummary true (parent 07, message content a string summary); 09 user text (parent 08); 0a assistant text (parent 09); 0b user text (parent 0a); 0c assistant text (parent 0b); 0d system compact_boundary (parentUuid null, logicalParentUuid 0c, compactMetadata {trigger "manual", preTokens 23456, preservedSegment {headUuid 0b, anchorUuid 0e, tailUuid 0c}}); 0e user isCompactSummary true (parent 0d, content the string `Fixture summary two. The code word is heliotrope.`); 0f user text (parent 0e); 10 assistant text (parent 0f). The word heliotrope appears in no other record. A file's content strings, wherever this brief scans for them, are every non-empty string value that is the `text` of a text part, the `thinking` of a thinking part, a string anywhere inside a tool_use part's `input`, a tool_result part's `content` when it is a string or the `text` of each text part in it, a `message.content` that is itself a string, and a `summary` field; type names, roles, ids, tool names, model names and every other structural value are not content strings. Split with HOME-001: C3 is HOME-001's row and this brief delivers its compaction half, its text amended to say that the compact_boundary pair is R3 corrected by measurement; HOME-001's R4 text is amended by R6 here. New files go where the design's structure names them; import.rs is at 473 of 500 code lines, so the compaction mapping, including the existing summary arm, moves to harness/claude_code/compaction.rs. Heavy builds and the full gate run where the project's standing rules send them; only warm single-crate checks run where the real session is held, and the real-session proof runs there. A compact_boundary whose isCompactSummary record never arrives imports as a compaction with an empty summary (R3); the listing prints its whole report first, then exits non-zero over an entry it could not read, a missing block, a session whose block rows are absent, or a compaction with no lys.loss entry, and exits 0 only when every compaction has its loss entry and every block in every span was read (R5). Out of scope: the home compacting a session itself; translation (stage 4b, its own card); changing what Claude Code writes; the handover letter (HOME-001 R12); LOSS-ACCOUNT.md.

## Requirements

### R1: Write the compaction contract into RECORD.md before any code changes

RECORD.md is the second party the implementation is held to, so it is written first, in the commit before or the same commit as any change to crates/lys-home. It SHALL gain: (1) under 'What lys keeps beside the file', `<id>.blocks.jsonl`: one JSON row per content part stored at import, `{entry, part, hash}`, where entry is the id of the entry the part went into, part is the part's 0-based index in its source record's content (a string content is part 0), and hash is the SHA-256 hex BlockStore::put returned; appended only, never part of Pi's grammar, rebuildable from the original file by re-importing its parts through the store, and absent for a session imported before it existed, in which case a reader reports that session's blocks as unverified and never guesses. (2) under 'The lys custom entries', `lys.loss`: its data fields {compaction, first_kept, kept_none, span_first, span_last, entries, messages, tool_calls, tool_results, blocks, entry_bytes, block_bytes, blocks_sha256, tokens_before} with the meaning R4 gives each; its place (the line directly after its compaction entry in the file, with the compaction as its parent, a side leaf off the context path); the span rule of R4 (the context the compaction summarised as it stood, less what it keeps, including entries an earlier compaction kept and that earlier compaction entry, and for a completing compaction the span of the compaction it completes) and its counting rule; that it names ids, counts and hashes and never content; and that it is not the render's `<uuid>.loss.json`, which accounts for what a render dropped. (3) a section 'Claude Code compactions' stating the import mapping of R3 (the compact_boundary and isCompactSummary pair, the summary record, a boundary without preservedSegment keeping nothing, a boundary whose isCompactSummary record never arrives imported with an empty summary and `summary_missing` true, an isCompactSummary record read after that compaction was written importing as a second compaction entry that names the first in `details.completes` and leaves it unrewritten, every compaction entry becoming the file chain's last entry, a tailUuid that names no entry on record falling back to logicalParentUuid and then to the chain's last entry, the refusal of an unknown first kept entry by uuid) and the render shape of R6, each naming the Claude Code version it was measured on. The text SHALL NOT quote any transcript and SHALL NOT describe a field on a Pi entry outside custom data and the compaction's own Pi `details` field.

**Acceptance:**
- RECORD.md contains the strings `<id>.blocks.jsonl`, `lys.loss`, `compact_boundary`, `isCompactSummary`, `preservedSegment.headUuid` `summary_missing` and `completes`.
- Each of the fourteen lys.loss field names (compaction, first_kept, kept_none, span_first, span_last, entries, messages, tool_calls, tool_results, blocks, entry_bytes, block_bytes, blocks_sha256, tokens_before) appears in RECORD.md's lys.loss paragraph, and that paragraph names `.loss.json` as a different record.
- `git log --format=%H -- docs/design/home/RECORD.md` on the landed branch lists a commit that is an ancestor of, or equal to, the first commit on the branch that touches crates/lys-home.

**Files:**
- modify: docs/design/home/RECORD.md

**Checklist:**
- C14 — When a compaction entry enters a session on import (a compact_boundary whose summary record never arrives included, as a compaction with an empty summary, and a summary read after that as a second compaction entry that completes the first without rewriting it), a lys.loss custom entry follows it, points at it by entry id and names what fell outside the kept range: the span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks with their bytes, and the tokensBefore the harness reported, as ids, counts and hashes and never content.
- C15 — A lys-home subcommand lists a session's compactions, each with its loss entry and a check that every entry in the summarised span is readable by id and every block it references is held, as a JSON report of ids, counts and hashes; it prints the whole report, then exits non-zero when an entry cannot be read, a block is missing, the session's blocks are unverified or a compaction has no loss entry (its reason given as unknown), naming each by entry id, block hash, session id or compaction id, and exits 0 only when every compaction's loss entry and every block in every span was read.
- C16 — The hash of every content part stored at import is kept beside the session in <id>.blocks.jsonl, one row per part naming its entry, so the blocks an entry references can be checked as held; a session without the file is reported as unverified.

**Stories:**
- S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.

### R2: Keep every stored part's block hash beside the session

WHEN the importer stores a content part through BlockStore::put, THE SYSTEM SHALL append one row {entry, part, hash} to `<id>.blocks.jsonl` beside the session file, where hash is the Hash put returned (today store_part discards it) and entry and part are as RECORD.md says; the path is given by a function beside Index::index_path and Index::head_path. The rows SHALL be durable before the import returns. THE SYSTEM SHALL NOT recompute a hash from the Pi-shaped part, SHALL NOT write any part content, text or length of text into a row, and SHALL NOT add a field to the header or to any Pi entry.

**Acceptance:**
- Importing the fixture writes `<session>.blocks.jsonl` holding exactly 15 rows, and the import report's `blocks` is 15.
- The row with entry `00000000-0000-4000-8000-000000000002` and part 1 carries the SHA-256 hex of `serde_json::to_vec` of the tool_use part parsed from fixture line 2, computed by the test from the fixture file.
- BlockStore::contains is true for the hash of every one of the 15 rows.
- The rows file contains none of the fixture's content strings, and the test counts the content strings it scanned for and asserts the count is greater than 0.
- No message entry in the imported session file carries a key named `hash` or `blocks`.

**Files:**
- create: crates/lys-home/src/record/block_rows.rs
- create: crates/lys-home/src/record/block_rows_tests.rs
- create: crates/lys-home/tests/fixtures/claude_code_compacted.jsonl
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs

**Checklist:**
- C16 — The hash of every content part stored at import is kept beside the session in <id>.blocks.jsonl, one row per part naming its entry, so the blocks an entry references can be checked as held; a session without the file is reported as unverified.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a compacted session's listing to name every summarised entry and block as held, by id and hash, with a missing one named, and the compaction render measured on a named harness version, so that a claim that the original is kept is measured rather than assumed.

### R3: Import Claude Code's compaction records as one Pi compaction entry

WHEN a Claude Code transcript holds a `system` record with subtype `compact_boundary` and a `user` record with `isCompactSummary: true` whose parentUuid is that boundary's uuid, THE SYSTEM SHALL write one Pi compaction entry for the pair when the summary record is read: its id is the summary record's uuid, so a later record whose parentUuid names the summary attaches under the compaction and R3's parent equality holds; its parentId is the boundary's `compactMetadata.preservedSegment.tailUuid` when that names an entry on record, otherwise the boundary's `logicalParentUuid` when that names an entry on record, otherwise the last entry on the file's chain, so a tailUuid that names no entry on record is never refused; its summary is the summary record's message content string; its firstKeptEntryId is the first entry that the record named by `preservedSegment.headUuid` produced; its tokensBefore is `compactMetadata.preTokens`; its timestamp is the summary record's; and its Pi `details` field is `{"boundaryUuid": <boundary uuid>, "summaryUuid": <summary uuid>}` so both originals are traceable. The pair is matched by the summary's parentUuid, not by adjacency (records may sit between the two). The summary's content is stored as a block with a row under the compaction's id, part 0. THE SYSTEM SHALL count the two records in the import report as the compaction's source (`compactions` counts compaction entries, `compaction_sources` counts the records they came from) and SHALL NOT write either as a lys.harness_event entry or as a user message entry, nor count them in `events` or `counted_types`. IF a compact_boundary record's isCompactSummary record has not been read when a later record names the boundary's uuid as its parentUuid, or when the end of the file is reached, THEN THE SYSTEM SHALL write the compaction at that point with the boundary's uuid as its id, an empty summary, the same parentId, firstKeptEntryId and tokensBefore rules, the boundary's timestamp, and details `{"boundaryUuid": <boundary uuid>, "summary_missing": true}`, count it in `compactions` and the boundary in `compaction_sources`, and SHALL NOT write the boundary as a lys.harness_event entry nor refuse the file. WHEN an isCompactSummary record is read after its boundary's compaction was already written that way, THE SYSTEM SHALL write a second compaction entry for the same boundary: its id is the summary record's uuid, its parentId is the summary record's parentUuid (the earlier compaction's id), its summary is the summary record's message content string, its firstKeptEntryId and tokensBefore are the earlier compaction's, its timestamp is the summary record's, and its details are `{"boundaryUuid": <boundary uuid>, "summaryUuid": <summary uuid>, "completes": <earlier compaction's id>}`; the summary's content is stored as a block with a row under this entry's id, part 0; it is counted in `compactions` and the summary record in `compaction_sources`. THE SYSTEM SHALL NOT import that summary record as a user message entry, and SHALL NOT change, rewrite or remove the earlier compaction entry, whose details keep `summary_missing` true. WHEN the boundary carries no preservedSegment, THE SYSTEM SHALL write the compaction with firstKeptEntryId equal to its own id, so it keeps nothing, and SHALL NOT refuse the file. WHEN a `type:"summary"` record is read, THE SYSTEM SHALL write its compaction entry as today (a fresh id, the last main-path message as parent, firstKeptEntryId its own id, tokensBefore 0) and count it in `compactions` and `compaction_sources`; this arm moves out of import.rs into the new compaction module so import.rs stays under 500 code lines. WHEN THE SYSTEM writes a compaction entry by any of these paths (the pair when its summary is read, the fallback at a later child or at the end of the file, the completing entry, and the summary record), THE SYSTEM SHALL make that compaction entry the last entry on the file's chain, so that the next record without an on-record parent attaches under it and the head is set to it when no later record follows; the lys.loss entry after it SHALL NOT become the chain's last entry. IF `preservedSegment.headUuid` names a record not on record when the compaction is written, THEN THE SYSTEM SHALL refuse the import with an error naming that uuid, as an unknown parentUuid is refused, and SHALL NOT append the compaction entry or its loss entry; this is the only compaction refusal. THE SYSTEM SHALL NOT change what any other record imports to, SHALL NOT change the unknown-parentUuid refusal, and SHALL NOT write to the source file.

**Acceptance:**
- Importing the fixture yields a compaction entry with id `00000000-0000-4000-8000-000000000008`, parentId `…06`, firstKeptEntryId `…05`, tokensBefore 12345 and details `{"boundaryUuid": "…07", "summaryUuid": "…08"}`, and one with id `…0e`, parentId `…0c`, firstKeptEntryId `…0b` and tokensBefore 23456.
- The fixture's import report has `compactions` 2, `compaction_sources` 4, `events` equal to `{"tool_completed": 1}` and no `system` key in `counted_types`, and the session holds no entry with id `…07` and none with id `…0d`.
- In the fixture import, entry `…09` has parentId `…08` and entry `…0f` has parentId `…0e`, and every message entry's parent equals its source record's parentUuid (checked over all).
- With both boundaries' preservedSegment removed from the fixture, the import succeeds and entry `…08` has firstKeptEntryId `…08` and parentId `…06` (the logicalParentUuid).
- With the first boundary's preservedSegment.tailUuid replaced by `00000000-0000-4000-8000-0000000000fe`, the import succeeds and entry `…08` has parentId `…06` (the logicalParentUuid); with both that replacement and the boundary's logicalParentUuid replaced by `00000000-0000-4000-8000-0000000000fd`, the import succeeds and entry `…08` has parentId `…06` (the last entry on the file's chain when the summary is read).
- With the fixture's line 7 moved to sit between lines 5 and 6, entry `…08` is written with the same parentId, firstKeptEntryId, tokensBefore and details as unmoved.
- With the first boundary's headUuid replaced by `00000000-0000-4000-8000-0000000000ff`, the import returns an error whose Display contains `00000000-0000-4000-8000-0000000000ff`, and the session holds no entry `…08` and no lys.loss entry.
- With fixture lines 14 to 16 removed, the import succeeds; the last two lines of the session file are a compaction entry with id `…0d`, summary `""`, parentId `…0c`, firstKeptEntryId `…0b`, tokensBefore 23456 and details `{"boundaryUuid": "…0d", "summary_missing": true}`, then a lys.loss entry whose data.compaction is `…0d`; the report has `compactions` 2 and `compaction_sources` 3, and entry `…08`'s details have no `summary_missing` key.
- With fixture lines 14 to 16 removed, Session::head() after the import is `…0d`; on the unmodified fixture, Session::head() after the import is `…10`.
- A three-line file (user `…01` with parentUuid null, assistant `…02` with parent `…01`, then `{"type":"summary","summary":"Fixture summary.","leafUuid":"…02"}`) imports to one compaction entry whose firstKeptEntryId equals its own id and whose tokensBefore is 0, with report `compactions` 1 and `compaction_sources` 1.
- With a user record `00000000-0000-4000-8000-000000000011` whose parentUuid is `…07` inserted between fixture lines 7 and 8, the import succeeds; the session holds a compaction entry with id `…07`, summary `""`, parentId `…06`, firstKeptEntryId `…05`, tokensBefore 12345 and details `{"boundaryUuid": "…07", "summary_missing": true}`, followed on the next line by a lys.loss entry whose data.compaction is `…07`; entry `…11` is a user message with parentId `…07`; entry `…08` is a compaction entry with parentId `…07`, summary equal to fixture line 8's summary string, firstKeptEntryId `…05`, tokensBefore 12345 and details `{"boundaryUuid": "…07", "summaryUuid": "…08", "completes": "…07"}`, followed on the next line by a lys.loss entry whose data.compaction is `…08`; the session holds no user message entry with id `…08`; entry `…07`'s line is byte-identical to the line it had when `…11` was read; entry `…09` has parentId `…08`; and the report has `compactions` 3 and `compaction_sources` 4.

**Files:**
- create: crates/lys-home/src/harness/claude_code/compaction.rs
- create: crates/lys-home/src/harness/claude_code/compaction_tests.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. A compaction is read as Claude Code 2.1.281 measurably writes it, a compact_boundary system record and its isCompactSummary user record becoming one compaction entry (first kept entry from preservedSegment.headUuid, tokensBefore from compactMetadata.preTokens); this corrects R3's summary record by measurement, and the summary record is still read where a file carries one (the compaction half is HOME-002's).
- C14 — When a compaction entry enters a session on import (a compact_boundary whose summary record never arrives included, as a compaction with an empty summary, and a summary read after that as a second compaction entry that completes the first without rewriting it), a lys.loss custom entry follows it, points at it by entry id and names what fell outside the kept range: the span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks with their bytes, and the tokensBefore the harness reported, as ids, counts and hashes and never content.

**Stories:**
- S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.

### R4: Append a lys.loss entry directly after each compaction entry

WHEN the importer appends a compaction entry by any path of R3, THE SYSTEM SHALL append, as the very next line of the session file, a custom entry with customType `lys.loss` (a constant beside lys.harness_event in entries.rs), a fresh id, the compaction's timestamp and the compaction entry as its parent: a side leaf, so the path through the compaction, R3's parent equality and the context path are unchanged. Its data SHALL be: `compaction` the compaction's entry id; `first_kept` its firstKeptEntryId; `kept_none` true exactly when firstKeptEntryId is the compaction's own id; `span_first` and `span_last` the first and last entry ids of the span, both null when the span is empty; `entries` the number of span entries; `messages` the span's message entries (user, assistant and toolResult); `tool_calls` the toolCall parts in the span's assistant messages; `tool_results` the span's toolResult messages; `blocks` the number of block rows whose entry is in the span; `entry_bytes` the sum of the span entries' line lengths in the session file as the index holds them; `block_bytes` the sum of the byte lengths of the blocks those rows name, as held; `blocks_sha256` the SHA-256 hex of those rows' hashes in span order then part order, each followed by one newline; `tokens_before` the compaction's tokensBefore. The span is the context the compaction summarised as that context stood, every entry its summary now stands in for: the entries of the compaction's ancestry that Pi's context reading covered at the compaction's parent, less the entries the compaction keeps. On the ancestry, root first, it starts at the root when no compaction entry sits earlier on that ancestry; otherwise at the nearest earlier compaction's first kept entry when that entry is on the ancestry (so entries an earlier compaction kept, and that earlier compaction entry itself, are in the span), and at that earlier compaction entry when its first kept entry is itself or is not on the ancestry. It ends at the entry whose child on the ancestry is the first kept entry, or at the compaction's parent when the compaction keeps nothing. For a completing compaction of R3 (one whose details carry `completes`), the span is the span of the compaction it completes, and every data field but `compaction` and `tokens_before` equals that compaction's loss entry's. It is one unbroken run of the ancestry, named in ancestry order, so span_first, span_last and the parent ids between them name every id in it. Entries off that ancestry (side leaves such as tool_completed and permission-mode events, earlier loss entries, sidechains) are not in the span; custom entries on it (attachment and system events, lys.authored) are counted in `entries` only. THE SYSTEM SHALL compute the span by seeking its entries through the index and SHALL NOT read the whole session file. The data SHALL NOT carry any text, thinking, tool input, tool result or summary, nor any key named text, content or body, and THE SYSTEM SHALL NOT remove, rewrite or move any span entry or block.

**Acceptance:**
- Importing the fixture yields exactly two lys.loss entries (Session::customs_everywhere); the first has parentId `…08`, its index offset equals the offset plus length of entry `…08`, and its data.compaction is `…08`.
- The first loss entry's data has span_first `…01`, span_last `…04`, entries 4, messages 4, tool_calls 1, tool_results 1, blocks 5, first_kept `…05`, kept_none false and tokens_before 12345.
- The second loss entry's data has compaction `…0e`, first_kept `…0b`, kept_none false, span_first `…05`, span_last `…0a`, entries 5, messages 4, tool_calls 0, tool_results 0, blocks 5 and tokens_before 23456, and its entry_bytes equals the sum of the index lengths of entries `…05`, `…06`, `…08`, `…09` and `…0a`.
- The first loss entry's block_bytes equals the sum of `serde_json::to_vec` lengths of the five content parts of fixture lines 1 to 4 (line 1's string content taken as `{"type":"text","text":…}`), and its blocks_sha256 equals the SHA-256 of those five parts' hashes each followed by a newline in file order, both computed by the test from the fixture file.
- The first loss entry's entry_bytes equals the sum of the index lengths of entries `…01`, `…02`, `…03` and `…04`.
- With both boundaries' preservedSegment removed, the first loss entry has kept_none true, span_first `…01`, span_last `…06`, entries 6, messages 6, tool_calls 1, tool_results 1 and blocks 7.
- The three-line summary file of R3 yields one loss entry with kept_none true, span_first `…01`, span_last `…02`, entries 2, messages 2 and tokens_before 0.
- Neither loss entry's serialised data contains any of the fixture's content strings, the test counts the content strings it scanned for and asserts the count is greater than 0, and neither loss entry has a key named text, content or body.
- On the fixture with the user record `…11` of R3 inserted between lines 7 and 8, the lys.loss entry whose data.compaction is `…08` has span_first `…01`, span_last `…04`, entries 4, blocks 5, first_kept `…05` and kept_none false, and its blocks_sha256 equals that of the lys.loss entry whose data.compaction is `…07`.

**Files:**
- create: crates/lys-home/src/record/loss.rs
- create: crates/lys-home/src/record/loss_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/harness/claude_code/compaction.rs

**Checklist:**
- C14 — When a compaction entry enters a session on import (a compact_boundary whose summary record never arrives included, as a compaction with an empty summary, and a summary read after that as a second compaction entry that completes the first without rewriting it), a lys.loss custom entry follows it, points at it by entry id and names what fell outside the kept range: the span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks with their bytes, and the tokensBefore the harness reported, as ids, counts and hashes and never content.

**Stories:**
- S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.

### R5: List a session's compactions with the span read by id and its blocks checked as held

Add the subcommand `lys-home compactions --home <dir> --session <id>`. WHEN run, THE SYSTEM SHALL take the session's root-to-head ancestry from the index's rows (ids and parent ids, without reading the session file) and read each of those entries on its own by Session::entry, so an entry that cannot be read is named and the walk goes on, never aborting the listing; every id it could not read, on the path or as a loss entry, is listed in the report's top-level `unreadable`. For each compaction entry among the entries read, in path order, THE SYSTEM SHALL report `compaction`, `summary` (the word `missing` when the compaction's details carry `summary_missing` true and no compaction entry on the path names it in `details.completes`, otherwise the word `present`), `completes` (the id in its details' `completes`, or null), `completed_by` (the id of the compaction entry on the path whose details' `completes` names it, or null), `first_kept`, `kept_none`, `tokens_before`, `loss` (the id of the lys.loss entry whose data.compaction names it, found through the index's custom rows and read by Session::entry, or null when there is none), `span_first`, `span_last`, `entries_expected` (the loss entry's entries), `entries_read` (how many of the span's entries Session::entry read, the span's ids taken from the index by following parent ids from span_last back to span_first, each read on its own), `entries_missing` (every span id that could not be read, in ancestry order, the walk continuing past each), `blocks_expected` (the loss entry's blocks), `blocks_held`, `blocks_missing` (the hashes of the span's block rows the store does not hold) and `blocks_sha256` (recomputed from the rows as R4 computes it) and `reason` (null when the compaction has a loss entry, otherwise the string `unknown: no lys.loss entry, imported before HOME-002`). The report is one JSON object `{command: "compactions", session, blocks_verified, unreadable, compactions: [...]}`. WHILE `<id>.blocks.jsonl` is absent, THE SYSTEM SHALL report `blocks_verified` false with the session id and every block field null, and SHALL NOT infer blocks any other way. WHEN a compaction has no loss entry, its span, entries and block fields SHALL be null and its reason SHALL be `unknown: no lys.loss entry, imported before HOME-002`; its span SHALL NOT be guessed from the path. THE SYSTEM SHALL NOT print any transcript, block or summary content nor a key named text, content or body, SHALL NOT write to the session file, its index, head, block rows or the block store, and SHALL NOT start any process. WHEN the report is printed, IF `unreadable` is non-empty, or any compaction's entries_missing or blocks_missing is non-empty, or any compaction has no loss entry, or `blocks_verified` is false, THEN THE SYSTEM SHALL exit 1 after printing the whole report, with each failure named in it by entry id, block hash, compaction id or session id; WHEN `unreadable` is empty, every compaction has its loss entry, `blocks_verified` is true, and every compaction has entries_read equal to entries_expected and empty entries_missing and blocks_missing, THE SYSTEM SHALL exit 0, and in no other case. THE SYSTEM SHALL NOT exit before printing the report because an entry, block, rows file or loss entry is missing or cannot be read. IF a required argument is missing, THEN THE SYSTEM SHALL exit 2 naming it.

**Acceptance:**
- On the fixture import, stdout parses as one JSON object whose `unreadable` is [], whose `blocks_verified` is true and whose `compactions` array has 2 members; the first has compaction `…08`, summary `present`, loss equal to the first lys.loss entry's id, entries_expected 4, entries_read 4, entries_missing [], blocks_expected 5, blocks_held 5, blocks_missing [] and reason null; the second has compaction `…0e`, entries_expected 5, entries_read 5, entries_missing [], blocks_expected 5, blocks_held 5 and blocks_missing []; and the exit status is 0.
- After the block file of the tool_use part of entry `…02` (hash H, from its block row) is removed, the listing's first compaction has blocks_held 4 and blocks_missing exactly [H], and the second compaction's blocks_missing is [], stdout parses as the whole report, and the exit status is 1.
- After `<session>.blocks.jsonl` is removed, the listing reports blocks_verified false and the session id, every compaction's blocks_held, blocks_missing and blocks_sha256 are null, and the first compaction's entries_read is still 4, stdout parses as the whole report, and the exit status is 1.
- After every byte of entry `…03`'s line in the session file except its trailing newline is overwritten in place with `x`, the listing's top-level `unreadable` is exactly [`…03`], its first compaction has entries_read 3 and entries_missing exactly [`…03`], its second compaction has entries_read 5 and entries_missing [], stdout parses as the whole report, and the exit status is 1.
- On the import of the fixture with lines 14 to 16 removed, the listing's second compaction has compaction `…0d` and summary `missing`, and the first has summary `present`.
- On the import of the fixture with the user record `…11` of R3 inserted between lines 7 and 8, the listing's `compactions` array has 3 members in path order: compaction `…07` with summary `present`, completes null and completed_by `…08`; compaction `…08` with summary `present`, completes `…07`, completed_by null, entries_expected 4 and entries_read 4; and compaction `…0e`; and the exit status is 0.
- A session written through the record API holding user entry `…01`, assistant entry `…02` (parent `…01`) and a compaction entry (parent `…02`, firstKeptEntryId `…02`), with no lys.loss entry and no block rows file, lists one compaction whose loss, span_first, span_last, entries_expected, entries_read, entries_missing, blocks_expected, blocks_held, blocks_missing and blocks_sha256 are all null and whose reason is `unknown: no lys.loss entry, imported before HOME-002`; stdout parses as the whole report and the exit status is 1.
- The listing's stdout has no key named text, content or body at any depth and contains none of the fixture's content strings, and the test counts the content strings it scanned for and asserts the count is greater than 0.
- `lys-home compactions --home h` exits 2 and its stderr contains `--session`.
- The SHA-256 of the session file, its index, its head and its block rows file are each the same before and after a listing.

**Files:**
- create: crates/lys-home/src/record/compactions.rs
- create: crates/lys-home/src/record/compactions_tests.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C15 — A lys-home subcommand lists a session's compactions, each with its loss entry and a check that every entry in the summarised span is readable by id and every block it references is held, as a JSON report of ids, counts and hashes; it prints the whole report, then exits non-zero when an entry cannot be read, a block is missing, the session's blocks are unverified or a compaction has no loss entry (its reason given as unknown), naming each by entry id, block hash, session id or compaction id, and exits 0 only when every compaction's loss entry and every block in every span was read.

**Stories:**
- S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a compacted session's listing to name every summarised entry and block as held, by id and hash, with a missing one named, and the compaction render measured on a named harness version, so that a claim that the original is kept is measured rather than assumed.

### R6: Render a compaction for Claude Code in the shape the target version writes, never the loss entry

WHEN rendering for Claude Code a context path whose first entry is a compaction, THE SYSTEM SHALL write, in place of the `{type: summary, leafUuid}` line: a `system` record with subtype `compact_boundary`, content `Conversation compacted`, level `info` and `compactMetadata` `{"preTokens": <tokensBefore>}`, whose parentUuid is null; then a `user` record with `isCompactSummary` true, `isVisibleInTranscriptOnly` true and message `{"role": "user", "content": <summary>}`, whose parentUuid is the boundary's uuid; then the kept and later entries, the first of them with the summary record's uuid as its parentUuid, the chain advancing through all three. Both records carry sessionId, cwd, version, userType, isSidechain and timestamp as the other rendered records do. THE SYSTEM SHALL NOT write a `type:"summary"` line, SHALL NOT write any custom entry (lys.loss included), and SHALL NOT write a preservedSegment. R4's thinking rule, the refusal of an existing path and the `.loss.json` beside the file are unchanged. HOME-001's R4 spec is amended to name this shape in place of 'Claude Code's summary record', stating that the summary line was written before the shape was measured and left the summary in no record the model reads, and HOME-001.md is re-rendered from it.

**Acceptance:**
- Rendering the fixture import writes 6 lines: line 1 has type `system`, subtype `compact_boundary`, parentUuid null and compactMetadata.preTokens 23456; line 2 has type `user`, isCompactSummary true, parentUuid equal to line 1's uuid and message.content equal to fixture line 14's summary string; line 3 has uuid `…0b` and parentUuid equal to line 2's uuid; lines 4 to 6 each have parentUuid equal to the previous line's uuid.
- The rendered fixture file has 0 lines whose type is `summary`, 0 lines whose type is `custom` and 0 lines containing `lys.loss`.
- Re-importing the rendered fixture file into a fresh home yields exactly one compaction entry, with tokensBefore 23456 and summary equal to fixture line 14's.
- The render tests that predate this brief (same model keeps signed thinking, a different model gets a loss account) pass unchanged.
- HOME-001.json's R4 spec contains `compact_boundary` and `isCompactSummary` and no longer contains `a compaction renders as Claude Code's summary record`, and HOME-001.md is byte-equal to what render-cluster.py writes from it.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs
- modify: docs/design/home/briefs/HOME-001.json
- modify: docs/design/home/briefs/HOME-001.md

**Checklist:**
- C17 — A compaction renders for Claude Code as the target version writes it (a compact_boundary record, then the isCompactSummary record, then the kept records, the parent chain advancing through all three) and the lys.loss entry never renders.

**Stories:**
- S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.

### R7: Gate the whole on the compacted fixture, end to end

An integration test SHALL carry the words' acceptance on the fixture through the crate's public surface. WHEN the fixture is imported into two fresh homes, THE SYSTEM SHALL give lys.loss entry lines that are byte-identical once `id`, `parentId` and `timestamp` are removed. WHEN a compacted fixture home is read, every entry in each span SHALL be readable by id, the context path SHALL be the last compaction then its kept entries then the later entries with nothing from any span, and the render SHALL hold the boundary, summary and kept records and no loss line. The test SHALL NOT use any real transcript and its names SHALL NOT carry any of the fixture's content strings.

**Acceptance:**
- The two homes' first lys.loss lines, and their second lys.loss lines, are byte-identical after removing `id`, `parentId` and `timestamp`, and the test counts 2 compared pairs.
- Session::entry succeeds for each of `…01`, `…02`, `…03` and `…04` (the first compaction's span) and for each of `…05`, `…06`, `…08`, `…09` and `…0a` (the second compaction's span), the test counts 9 ids read, and the listing reports entries_missing [] and blocks_missing [] for both compactions.
- The ids of Session::context_path are exactly [`…0e`, `…0b`, `…0c`, `…0f`, `…10`].
- The rendered fixture file holds 1 compact_boundary record, 1 isCompactSummary record and 4 records with uuids `…0b`, `…0c`, `…0f`, `…10`, and 0 lines containing `lys.loss`.
- The fixture file's SHA-256 is the same before and after the import, listing and render.

**Files:**
- create: crates/lys-home/tests/claude_code_compaction.rs

**Checklist:**
- C14 — When a compaction entry enters a session on import (a compact_boundary whose summary record never arrives included, as a compaction with an empty summary, and a summary read after that as a second compaction entry that completes the first without rewriting it), a lys.loss custom entry follows it, points at it by entry id and names what fell outside the kept range: the span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks with their bytes, and the tokensBefore the harness reported, as ids, counts and hashes and never content.
- C15 — A lys-home subcommand lists a session's compactions, each with its loss entry and a check that every entry in the summarised span is readable by id and every block it references is held, as a JSON report of ids, counts and hashes; it prints the whole report, then exits non-zero when an entry cannot be read, a block is missing, the session's blocks are unverified or a compaction has no loss entry (its reason given as unknown), naming each by entry id, block hash, session id or compaction id, and exits 0 only when every compaction's loss entry and every block in every span was read.
- C17 — A compaction renders for Claude Code as the target version writes it (a compact_boundary record, then the isCompactSummary record, then the kept records, the parent chain advancing through all three) and the lys.loss entry never renders.

**Stories:**
- S9 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session's home to say what it could not keep and to point at an original that is all still there, so that a compaction is stored beside its original and never replaces it.
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a compacted session's listing to name every summarised entry and block as held, by id and hash, with a missing one named, and the compaction render measured on a named harness version, so that a claim that the original is kept is measured rather than assumed.

### R8: Prove it on one real compacted session and measure the render's resume

PROOF-COMPACTION.md SHALL record two measurements. First, one real compacted Claude Code session, chosen as the smallest by byte size (ties broken by the lesser path in byte order) of the main-session files (not a subagent's) holding a compact_boundary record under every Claude Code config root on the machine that holds it; the proof records each config root it searched and the exact search command, whose output lists the candidate files with their byte sizes, so that running the recorded command over the recorded roots picks the same file. It SHALL NOT be running: it is established as not running by `pgrep -f <session id>` finding no process, the rule HOME-001's proof applied, with that command and its empty result recorded, and by its SHA-256 being equal before and after. The proof records its byte size, SHA-256 before and after, its count of compact_boundary records and the Claude Code version on them, the exact import and listing commands, the import report, and the listing report, as ids, counts and hashes only. Second, the fixture home rendered with `--version` set to the installed Claude Code's version and resumed by path with `claude -p --resume <rendered file>` and a prompt asking for the code word from the summary, run from a directory that is neither the file's directory nor the config root: the output of `claude --version`, the exact command, its exit status, the one-word answer, the rendered file's SHA-256 before and after, and the continuation file's count of compact_boundary and isCompactSummary records. The proof is scanned for every content string of the real source, whatever its length, and the scan command, the number of content strings scanned and the number found are written in it. THE PROOF SHALL NOT contain any content string of the real session, and SHALL NOT be run against a session that is running.

**Acceptance:**
- PROOF-COMPACTION.md names the real source by size and SHA-256, states its compact_boundary count and their Claude Code version, records `pgrep -f <session id>` with no process found, and its SHA-256 before and after are equal.
- PROOF-COMPACTION.md lists the config roots it searched and the exact search command, and the source it names is the first file of that command's recorded output when ordered by byte size and then by path in byte order.
- The recorded listing's `compactions` array length equals the recorded import report's `compactions`, and every member has entries_read equal to entries_expected, entries_missing [] and blocks_missing [], with blocks_verified true.
- A scan of PROOF-COMPACTION.md for every content string of the real source, whatever its length, each matched where it is bounded on both sides by the start or end of the file or by a character that is neither a letter nor a digit, finds 0 matches, and the scan command, the number of content strings scanned (greater than 0) and the number found (0) are written in the proof.
- The render measurement records `claude --version` reporting 2.1.282, exit status 0, the answer `heliotrope`, equal rendered-file SHA-256 before and after, and the continuation's compact_boundary and isCompactSummary counts.

**Files:**
- create: docs/design/home/PROOF-COMPACTION.md

**Checklist:**
- C18 — One real compacted Claude Code session is imported on the machine that holds it and its listing recorded in a proof document as counts, ids and hashes only; the compaction render is measured resuming on a named Claude Code version.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a compacted session's listing to name every summarised entry and block as held, by id and hash, with a missing one named, and the compaction render measured on a named harness version, so that a claim that the original is kept is measured rather than assumed.

## Boundaries

- SHALL NOT rewrite, truncate or move any file under a Claude Code config root; the import reads the source and the render writes a new path only.
- SHALL NOT add a field to Pi's header or to any Pi entry outside a custom entry's data and the compaction's own Pi `details` field.
- SHALL NOT remove, rewrite or move a summarised entry or its blocks, and SHALL NOT overwrite a block.
- SHALL NOT put transcript content in the loss entry, the block rows, the listing, an error, a log line, a test name or the proof; ids, counts and hashes only.
- SHALL NOT change the unknown-parentUuid refusal or R3's parent equality for message entries.
- SHALL NOT change R4's thinking rule, its refusal of an existing path or the `.loss.json` beside a rendered file.
- SHALL NOT depend on or copy any Norn crate or type.
- SHALL NOT compact, translate or hand over a session, and SHALL NOT change what Claude Code writes.
- SHALL NOT let the listing or the proof start a harness on anyone's behalf, and SHALL NOT add or bypass any grant on who may read or resume a home.
- SHALL NOT let any source file exceed 500 code lines, and SHALL NOT add #[allow], #[ignore], a `_`-prefixed unused binding or #[cfg(any())] beyond the test modules' standing opt-out.
- SHALL NOT write LOSS-ACCOUNT.md or any path outside the design's structure array.
- SHALL NOT run a proof against a session that is running.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/home exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/home exits 0.
- From the repository root: cargo fmt --all -- --check, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps each exit 0.
- From the repository root: sh scripts/design/gate.sh exits 0 after python3 scripts/design/render-cluster.py docs/design/home has rewritten the cluster's markdown.
- grep -rn 'norn' crates/lys-home/Cargo.toml returns nothing.
- grep -rln 'heliotrope' crates/lys-home/src returns nothing (the fixture's code word lives only in the fixture file and the integration test's assertion).
- For each of import.rs, cli.rs, render.rs and record/mod.rs, the count of lines that are neither blank nor comments is at most 500.

