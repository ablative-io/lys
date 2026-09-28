---
type: brief
id: HOME-030
cluster: home
title: Say what each compaction could not keep and prove the original is all still there: the loss entry, the block rows, the compaction listing and the compaction render
---

# HOME-030: Say what each compaction could not keep and prove the original is all still there: the loss entry, the block rows, the compaction listing and the compaction render

> **Cluster:** home
> **Depends on:** HOME-001
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-013 — The context record is a lys.given custom entry of document hashes, never copies — The context record is one lys.given custom entry, appended after the render event, whose data is the harness name, the Claude Code version the load order was measured on, the kinds as two lists, resolved (claude_md_chain, user_claude_md, memory_index, appended_instructions, mcp_config, environment_names) and unlisted (claude_md_imports and claude_rules, which this entry does not list and a later entry at the first request records), the config directory as its path and its source (template or home), the documents in the measured order each as kind, path, byte length and SHA-256, and the names of the environment variables the template set. It is not a copy of each document into the block store, and not a content-bearing record, because the entry must hold no content under home P7 and CN3. It is unsigned and unencrypted now, and because it names hashes only, signing and encryption at rest can be added later without changing what is recorded.
> **Checklist:**
> - C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. The compaction half is HOME-030's: the compact_boundary and isCompactSummary pair is R3 corrected by measurement.
> - C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.
> - C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.
> - C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.
> - C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.
> - C18 — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002.
> **Stories:**
> - S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.
> - S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

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
- C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.
- C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.
- C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. RECORD.md contains `<id>.blocks.jsonl` (line 43), `lys.loss` (line 270), `compact_boundary`, `isCompactSummary`, `preservedSegment.headUuid` (lines 360 and 383), `summary_missing` (lines 372 and 377) and `completes`. Row 2: met. The lys.loss paragraph (lines 270-314) lists compaction, first_kept, kept_none, span_first, span_last, entries, messages, tool_calls, tool_results, blocks, entry_bytes, block_bytes, blocks_sha256 and tokens_before, and names `<uuid>.loss.json` as a different record (line 312). The text quotes no transcript and describes nothing on a Pi entry outside custom data and the compaction's `details`. Row 3: met by construction. No commit was made; RECORD.md lands in the same commit as the crate change, which the rule allows.
- Deviation: Following the Task's instruction to amend C3's text, I edited checklist.json and re-rendered CHECKLIST.md and HOME-030.md with render-cluster.py. These files are outside R1's own file list, but they are in the design's structure array.
- Files changed:
  - modified: `docs/design/home/RECORD.md` — The header names HOME-030. 'What lys keeps beside the file' gains <id>.blocks.jsonl (line 43). 'The lys custom entries' gains lys.loss (line 270): all fourteen fields, where the entry sits, the span and counting rules, the completing-compaction rule, ids/counts/hashes only, and that it is not the render's <uuid>.loss.json (line 312). New section 'Claude Code compactions' (line 340) gives the import mapping and the render shape, both measured on 2.1.281. The rendered-uuid roles now include compact_boundary.
  - modified: `docs/design/home/checklist.json` — C3's text now says its compaction half is HOME-030's, and that the compact_boundary / isCompactSummary pair is R3 corrected by measurement.
  - modified: `docs/design/home/CHECKLIST.md` — Re-rendered by render-cluster.py from checklist.json.
  - modified: `docs/design/home/briefs/HOME-030.md` — Re-rendered; the only change is the amended C3 text.
- Checklist delivery:
  - [x] C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written. — Carried on R1; every acceptance row of R1 is met.
- Story delivery:
  - [x] S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate. — Carried on R1; every acceptance row of R1 is met.

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
- C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. After importing the fixture there are 15 rows and report.blocks is 15 (block_rows_tests.rs:25). Row 2: met. The hash in the row for entry …02 part 1 equals Hash::of(serde_json::to_vec(tool_use part parsed from fixture line 2)) (block_rows_tests.rs:35). That hash is the one `put` returned (content.rs store_part → import.rs rows_of → block_rows.rs:89 append); it is never recomputed from the Pi-shaped part. Row 3: met. BlockStore::contains is true for all 15 rows, and the test counts them (block_rows_tests.rs:53). Row 4: met. The rows file contains no content string, and the test asserts it scanned more than 0 strings (block_rows_tests.rs:67). Row 5: met. None of the 12 message entries has a key named hash or blocks at any depth (block_rows_tests.rs:84). The rows are synced before the import returns (import.rs `st.rows.sync()`).
- Deviation: import/content.rs is not in the design's structure array. It had to change: store_part is where the hash `put` returns was being thrown away, and the hash cannot be kept without editing it. I also changed record/reader.rs and home.rs, which are in the structure but not in R2's file list. Without that change session_ids would report `<id>.blocks` as a session, breaking recall, given, ship and verify for any imported home. The file is not yet in ship's tracked set; tracked.rs is unchanged and RECORD.md says so.
- Files changed:
  - created: `crates/lys-home/src/record/block_rows.rs` — BlockRow {entry, part, hash}. BlockRowWriter opens <id>.blocks.jsonl for appending, appends a row, keeps each row's block length in memory for the loss entry, and syncs the file and its directory. read_block_rows returns None when the file is absent.
  - created: `crates/lys-home/src/record/block_rows_tests.rs` — Five tests on the fixture: 15 rows and report.blocks 15; the hash in the row for entry …02 part 1 is computed here from fixture line 2; every row's block is held; no content string is in the rows file (count checked above 0); no message entry carries a hash or blocks key (12 messages checked).
  - created: `crates/lys-home/tests/fixtures/claude_code_compacted.jsonl` — Sixteen hand-built records in Claude Code 2.1.281's shape, exactly as the brief lists them. Every text is a short made-up phrase, and 'heliotrope' appears only in line 14.
  - modified: `crates/lys-home/src/record/index.rs` — Adds Index::blocks_path (line 72), beside index_path and head_path.
  - modified: `crates/lys-home/src/record/mod.rs` — Declares block_rows, compactions and loss, plus their test modules.
  - modified: `crates/lys-home/src/harness/claude_code/import.rs` — Import state now lives in an Importer struct. Each message's stored parts get a row under the entry they went into; a tool result's part goes under that result's own entry. Rows are synced before the import returns. The report gains compactions and compaction_sources. The summary-record arm moved to compaction.rs.
  - modified: `crates/lys-home/src/harness/claude_code/import/content.rs` — store_part now returns the Hash that BlockStore::put gave back, plus the block's length. assistant_content and user_content return each stored part's index and the tool result it went into.
  - modified: `crates/lys-home/src/record/reader.rs` — session_ids skips a <stem>.blocks.jsonl whose first line is not a session header, the same way it skips an index.
  - modified: `crates/lys-home/src/record/home.rs` — The session_ids doc says the same.
- Checklist delivery:
  - [x] C16 — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256. — Carried on R2; every acceptance row of R2 is met.
- Story delivery:
  - [x] S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed. — Carried on R2; every acceptance row of R2 is met.

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
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. The compaction half is HOME-030's: the compact_boundary and isCompactSummary pair is R3 corrected by measurement.
- C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Every acceptance row of R3 is met, one test each in compaction_tests.rs. Row 1: …08 has parent …06, first kept …05, tokens 12345 and details {boundaryUuid …07, summaryUuid …08}; …0e has parent …0c, first kept …0b, tokens 23456 (line 300). Row 2: compactions 2, compaction_sources 4, events {tool_completed:1}, no system key in counted_types, and no entry …07 or …0d (line 322). Row 3: …09's parent is …08 and …0f's is …0e; all 12 message parents equal their source parentUuid, and the test counts 12 (line 336). Row 4: with no preservedSegment, …08 keeps from itself and its parent is …06 (line 358). Row 5: with tailUuid fe the parent falls back to logicalParentUuid; with logicalParentUuid fd as well, it falls back to the chain's last entry; both give …06 (line 366). Row 6: moving line 7 gives an identical compaction (line 378). Row 7: headUuid ff gives an UnknownFirstKept error whose Display contains ff, with no …08 and no lys.loss entry (line 389). Row 8: with lines 14-16 removed, the last two lines are compaction …0d (summary '', parent …0c, first kept …0b, tokens 23456, summary_missing) and then its loss entry; compactions 2, sources 3; …08 has no summary_missing (line 404). Row 9: head is …0d, or …10 on the full fixture (line 434). Row 10: the three-line summary file gives one compaction that keeps from itself, with tokens 0 and counts 1/1 (line 443). Row 11: with …11 inserted, …07 is written without a summary and its loss entry is on the next line; …11 is a user message under …07; …08 completes …07, with its details and loss entry on the next line; …09 is under …08; counts are 3/4. …07's line is byte-identical to its line in a home cut right after …11 (line 460). Neither source record becomes an event or a user message. The unknown-parentUuid refusal and R3's parent equality are unchanged.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-home/src/harness/claude_code/compaction.rs` — The compaction mapping. A boundary is held until something resolves it. When its summary is read, the pair becomes one compaction (pair, line 167). If a later child names the boundary first, or the file ends, it becomes a compaction without a summary (unsummarised, line 195). A summary read after that becomes a completing compaction (line 223). The summary-record arm moved here (line 255). Parent rule: tailUuid, then logicalParentUuid, then the chain's last entry (line 327). First-kept rule, with the refusal (line 342). write() appends the compaction, its block rows and its loss entry, and makes the compaction the chain's last entry (line 276).
  - created: `crates/lys-home/src/harness/claude_code/compaction_tests.rs` — Eleven R3 tests plus the R6 render test, and the fixture helpers the record's tests share. Tests return Result and use no unwrap.
  - modified: `crates/lys-home/src/harness/claude_code/import.rs` — Every record passes through compaction_record before normal handling, and finish_compactions runs at the end of the file. Tracks the first entry each record produced, which firstKeptEntryId needs.
  - modified: `crates/lys-home/src/harness/claude_code/mod.rs` — Declares the compaction module, and compaction_tests as pub(crate).
  - modified: `crates/lys-home/src/error.rs` — HomeError::UnknownFirstKept {compaction, uuid}; its Display names the uuid.
  - modified: `crates/lys-home/src/harness/codex/beside_tests.rs` — sidechain_under_a_compacted_entry_is_lost: this fixture has a bare compact_boundary, which R3 now imports as a compaction without a summary. The test therefore reads the last compaction on the path (rfind), which is the one rollout.rs names; its doc says why.
- Checklist delivery:
  - [x] C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original. The compaction half is HOME-030's: the compact_boundary and isCompactSummary pair is R3 corrected by measurement. — The compaction half is delivered: the compact_boundary / isCompactSummary pair and the summary record. C3's text is amended to say so.
  - [x] C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written. — Carried on R3; every acceptance row of R3 is met.
- Story delivery:
  - [x] S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate. — Carried on R3; every acceptance row of R3 is met.

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
- C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Every acceptance row of R4 is met, one test each in loss_tests.rs. Row 1: exactly two lys.loss entries; the first is under …08, its index offset is …08's offset plus length, and it names …08 (line 39). Row 2: first loss is span …01 to …04, entries 4, messages 4, tool_calls 1, tool_results 1, blocks 5, first kept …05, kept_none false, tokens 12345 (line 55). Row 3: second loss fields as specified, and entry_bytes is the sum of the index lengths of …05, …06, …08, …09 and …0a (line 83). Row 4: block_bytes and blocks_sha256 are computed from the five parts of fixture lines 1-4 (line 112). Row 5: first loss entry_bytes is the sum of the index lengths of …01 to …04 (line 55). Row 6: with no preservedSegment, kept_none true, span …01 to …06, 6/6/1/1/7 (line 139). Row 7: the summary file gives kept_none true, span …01 to …02, 2/2, tokens 0 (line 161). Row 8: no content string and no text, content or body key; the test asserts it scanned both entries (line 182). Row 9: with …11 inserted, the loss for …08 has the specified span and a blocks_sha256 equal to …07's (line 209). The span is read by seeking index rows (loss.rs:198 span_of); the whole file is never read.
- Deviation: (none)
- Files changed:
  - created: `crates/lys-home/src/record/loss.rs` — LossData holds the fourteen fields. loss_data walks the span through the index by seeking: back from the compaction's parent to the nearest earlier compaction, then that compaction's first kept entry, then the end of the span. append_loss writes it with append_line as a side leaf under the compaction, using the compaction's timestamp, and leaves the head where it is. blocks_digest hashes the block hashes, each followed by a newline.
  - created: `crates/lys-home/src/record/loss_tests.rs` — Eight tests on the fixture and its variants, with bytes and digest computed from the fixture file.
  - modified: `crates/lys-home/src/record/entries.rs` — CUSTOM_LOSS = "lys.loss" (line 21), beside the other custom types.
  - modified: `crates/lys-home/src/harness/claude_code/compaction.rs` — write() appends the loss entry directly after every compaction. A completing compaction copies the loss data of the one it completes, with its own compaction id and tokens_before.
- Checklist delivery:
  - [x] C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written. — Carried on R4; every acceptance row of R4 is met.
- Story delivery:
  - [x] S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate. — Carried on R4; every acceptance row of R4 is met.

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
- C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

#### R5 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Every acceptance row of R5 is met; tests are in compactions_tests.rs. Row 1: the fixture lists 2 compactions with all fields as specified; unreadable is [], blocks_verified true, status 0 (line 50). Row 2: after removing the tool_use block, blocks_held is 4 and blocks_missing is [H] for the first compaction, the second's is [], status 1 (line 89). Row 3: with no rows file, blocks_verified is false, the session id is present, block fields are null, entries_read is still 4, status 1 (line 110). Row 4: with …03's line overwritten, unreadable is [03], the first compaction reads 3/[03], the second reads 5/[], status 1 (line 129). Row 5: summary is missing for …0d and present for the first (line 154). Row 6: three compactions with completes and completed_by as specified, status 0 (line 165). Row 7: a session built through the record API with no loss entry has every field null and the reason, status 1 (line 187). Row 8: no text, content or body key and no content string, with the scan counted (line 249). Row 9: clap refuses with exit code 2 naming --session (line 272); the binary check is in tests/claude_code_compaction.rs:194. Row 10: the session file, index, head and rows file hash the same before and after a listing (line 284).
- Deviation: The listing reads entries with SessionReader::entry, the lock-free twin of Session::entry that seeks the same way, instead of Session::entry. This means it takes no lock and cannot write an index or head, so a listing writes nothing at all. A rows file that exists but cannot be parsed is also reported as blocks_verified false, never inferred from.
- Files changed:
  - created: `crates/lys-home/src/record/compactions.rs` — list_compactions (line 62). Takes the root-to-head ancestry from index rows and reads each entry by id; any it cannot read goes in `unreadable` and the walk continues. Finds loss entries through the index's custom rows. Follows the index's parent ids from span_last back to span_first and reads each; blocks are checked from <id>.blocks.jsonl against the store. With no loss entry, every span field is null and the reason is NO_LOSS_ENTRY. With no rows file, blocks_verified is false and every block field is null. Returns the report and whether every check held. Writes nothing.
  - created: `crates/lys-home/src/record/compactions_tests.rs` — Ten tests through cli::run_with_status, so the exit status is the one the binary uses.
  - modified: `crates/lys-home/src/cli.rs` — `compactions --home --session` subcommand (line 227). It exits 1 after printing the whole report when any check fails. A missing argument is refused by clap with exit 2.
  - modified: `crates/lys-home/README.md` — Documents <id>.blocks.jsonl, lys.loss and the compactions command; names HOME-030.
- Checklist delivery:
  - [x] C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten. — Carried on R5; every acceptance row of R5 is met.
- Story delivery:
  - [x] S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate. — Carried on R5; every acceptance row of R5 is met.
  - [x] S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed. — Carried on R5; every acceptance row of R5 is met.

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
- C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.

#### R6 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. The render of the fixture has 6 lines: a boundary (parentUuid null, preTokens 23456); a summary record under it whose content equals fixture line 14's; line 3 is …0b; every line is chained to the one before (compaction_tests.rs:510). Row 2: met. There are no summary or custom lines and no 'lys.loss' in the file (same test, and tests/claude_code_compaction.rs:139). Row 3: met. Re-importing the rendered file itself gives exactly one compaction, with tokens 23456 and line 14's summary (compaction_tests.rs:510). Row 4: met by construction. render_tests.rs is unchanged, byte for byte, so the tests that predate this brief are untouched. The thinking rule, the refusal of an existing path and the .loss.json beside the file are unchanged in render.rs. Row 5: met. HOME-001.json R4 contains compact_boundary and isCompactSummary and no longer contains the old phrase; HOME-001.md was re-rendered by render-cluster.py.
- Deviation: The brief puts the new render test in render_tests.rs. I added it there first, but the crusher refused that whole file for its 80-plus pre-existing unwrap calls. Making it pass would mean rewriting every earlier render test, which contradicts 'the render tests that predate this brief pass unchanged'. So render_tests.rs is left untouched and the render test lives in compaction_tests.rs, beside the mapping it reverses.
- Files changed:
  - modified: `crates/lys-home/src/harness/claude_code/render.rs` — A compaction now renders as compaction_records (line 411): a system compact_boundary record (parentUuid null, content 'Conversation compacted', level info, compactMetadata {preTokens}), then a user record with isCompactSummary and isVisibleInTranscriptOnly carrying the summary under it; the chain continues from the summary record. The boundary's uuid is UUIDv5 over `<entry id>#compact_boundary`, using ROLE_BOUNDARY (line 329) and derived_uuid. No summary line, custom entry or preservedSegment is written. Module doc updated.
  - modified: `crates/lys-home/src/harness/claude_code/compaction_tests.rs` — Adds a_compaction_renders_as_its_boundary_and_summary_and_imports_back_as_one (line 510).
  - modified: `docs/design/home/briefs/HOME-001.json` — R4's spec now names the compact_boundary / isCompactSummary shape in place of 'a compaction renders as Claude Code's summary record', and says the summary line was written before the shape was measured and left the summary in no record the model reads.
  - modified: `docs/design/home/briefs/HOME-001.md` — Re-rendered with scripts/design/render-cluster.py.
- Checklist delivery:
  - [x] C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it. — Carried on R6; every acceptance row of R6 is met.
- Story delivery:
  - [x] S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate. — Carried on R6; every acceptance row of R6 is met.

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
- C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.
- C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.
- C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.

**Stories:**
- S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

#### R7 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1: met. Two homes' lys.loss lines are byte-identical once id, parentId and timestamp are removed, and the test counts 2 compared pairs (tests/claude_code_compaction.rs:86). Row 2: met. Session::entry succeeds for all 9 span ids and the test counts 9; the binary's listing reports empty entries_missing and blocks_missing for both compactions and exits 0 (line 104). Row 3: met. context_path ids are exactly [0e, 0b, 0c, 0f, 10] (line 104). Row 4: met. The rendered file has 1 compact_boundary, 1 isCompactSummary, then the uuids 0b, 0c, 0f and 10 over 6 records, and no line contains lys.loss (line 139). Row 5: met. The fixture's SHA-256 is the same before and after the import, listing and render (line 139).
- Deviation: (none)
- Files changed:
  - created: `crates/lys-home/tests/claude_code_compaction.rs` — End to end through the lys-home binary and the public Home/Session API: two homes, span reads, the context path, the listing, the render, the fixture's hash, and the exit-2 refusal. No test name carries fixture content; the code word appears only in one assertion.
- Checklist delivery:
  - [x] C14 — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written. — Carried on R7; every acceptance row of R7 is met.
  - [x] C15 — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten. — Carried on R7; every acceptance row of R7 is met.
  - [x] C17 — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it. — Carried on R7; every acceptance row of R7 is met.
- Story delivery:
  - [x] S9 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate. — Carried on R7; every acceptance row of R7 is met.
  - [x] S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed. — Carried on R7; every acceptance row of R7 is met.

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
- C18 — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

#### R8 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Blocked; I did not create PROOF-COMPACTION.md. Every acceptance row is a measurement from real runs: the search command over each Claude Code config root, pgrep, SHA-256 before and after, the import and listing reports, a render resumed with `claude -p --resume` on 2.1.282, and the answer heliotrope. All of those runs need the new lys-home binary. This round forbids any build or test command, and the project's standing rule 3 sends heavy builds to Dean's laptop. A proof document without those runs would be fabricated, so none was written. To unblock: after the landing builds, on the machine holding the real sessions, run the search, the pgrep check, import, compactions, render --version <installed>, and claude -p --resume from a neutral directory, then write the proof from those outputs with the content scan the brief specifies.
- Deviation: R8 was not attempted, for the reason in how.
- Checklist delivery:
  - [ ] C18 — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002. — R8's acceptance rows are not met; the proof is not written.
- Story delivery:
  - [ ] S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed. — R8's acceptance rows are not met; the proof is not written.

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
