---
type: brief
id: HOME-011
cluster: home
title: Record the given documents in the request's order, re-measured, with the old order named as superseded
---

# HOME-011: Record the given documents in the request's order, re-measured, with the old order named as superseded

> **Cluster:** home
> **Depends on:** HOME-003
> **Blocked by:** measure.py, the fixture PROOF-GIVEN was measured with (SHA-256 db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d), is not in the repository: its one copy is in the scratch tree of the seat that measured PROOF-GIVEN, at /private/tmp/claude-501/-Users-tom-Developer-archie/34c4f280-dc08-4edc-9fe0-82f1f75bee3b/scratchpad/proof-given/measure/measure.py, which /tmp clearing loses. R1 commits it from there by that hash before anything else runs.
> **Design anchor:**
> - ADR-013 — Claude Code's compaction is read and rendered in the shape the measured version writes — The importer reads a compact_boundary record and its isCompactSummary record as one Pi compaction entry: the summary is the isCompactSummary message's text, the first kept entry is the entry of preservedSegment.headUuid (the compaction itself when there is no preservedSegment, so it keeps nothing), tokensBefore is compactMetadata.preTokens, and Pi's details field names both source records by uuid; a named first kept entry not on record is refused by uuid. The summary record path stays for the file that carries one. The render writes a compact_boundary record, then the isCompactSummary record, then the kept records, with the parent chain advancing through all three, measured on the installed Claude Code version. Rejected: attaching the loss entry only to the summary record path, which almost no file uses, and keeping R4's summary line, which a resumed session would not read.
> - ADR-030 — A Claude Code compaction imports as one Pi compaction at the summary record's place, keeping what Claude Code kept, and renders as the compact_boundary pair — A compact_boundary record imports as today, a lys.harness_event on the chain; the isCompactSummary record under it becomes one Pi compaction entry that takes that record's uuid, sits where the record sat as the child of the boundary's entry, carries the record's text as its summary and tokensBefore from compactMetadata.preTokens, and keeps the record only as a block; it is not a message entry. Its first kept entry is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and when there is none nothing is kept and firstKeptEntryId is the empty string, which keeps Pi's string type and is the only value that names no entry, so a compaction never names itself; anchorUuid is not used. A compact_boundary with no isCompactSummary record under it is not a compaction the harness completed: it stays a lys.harness_event, no summary is invented, and the listing reports it as a boundary without a summary. A legacy summary record keeps the entry its leafUuid names. A first kept entry not on record is refused by uuid. A logicalParentUuid naming no record is not refused. The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record holding the summary, then the kept entries, never the legacy summary line and never a custom entry. Rejected: bounding the kept range by anchorUuid; keeping the summary record as a message entry beside the compaction (the summary would be read twice); placing the compaction off the chain or re-parenting the next message (HOME-001 R3's parent-equals-source acceptance stands with no exception); and rendering the legacy summary line, which the current Claude Code does not write.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> **Checklist:**
> - C78 — The context path of an imported compacted session is the compaction, then the kept entries preserved uuids included, then what follows; no span entry is on it and the summary text is on it once.
> - C79 — lys-home compactions prints one JSON report of a session's compactions, each with its loss entry, the span's ids, counts and hashes and a check that every span entry is readable by id and every named block is held, and exits 0 only when nothing is missing and 1 when anything is, naming the first missing item; a compaction with no loss entry or pointing at itself is reported unaccounted by entry id.
> - C80 — The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record, then the kept entries, with no legacy summary line and no lys.loss line, and the summary text once; the boundary record's uuid is derived under render-uuid/v2, alongside v1, whose non-boundary uuids equal v1's, and a render with no compaction stays v1, byte for byte, with the version it used named in its report.
> - C81 — PROOF-COMPACTION.md records one real compact_boundary session imported read-only with its listing as counts and hashes and its source SHA-256 equal before and after, and a rendered compacted fixture resumed on Claude Code 2.1.283 answering from the summary, by hashes only.
> - C82 — A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, with no compaction and no lys.loss entry, and lys-home compactions reports it by entry id as a boundary without a summary.
> **Stories:**
> - S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.
> - S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

## Purpose

The lys.given entry lists a session's documents in the order Claude Code reads the files, with the user CLAUDE.md first, while PROOF-GIVEN measured that the request the harness sends puts the appended instructions, in the system array, ahead of the first user message that carries every CLAUDE.md-family file and the memory index. The record should follow what reached the model. This brief makes the request's order the one rule (ADR-030), stated in RECORD.md, re-measures it with PROOF-GIVEN's own fixture on the installed Claude Code, moves the resolver and its two test files to it, and names the old order as superseded without touching any entry already written.

## Task

The cross-kind order is fixed in one place, resolve_given in crates/lys-home/src/harness/claude_code/given.rs, which pushes user_claude_md before the appended_instructions and mcp_config loop; move that push after the loop and correct the module docs. The new order is appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index; the within-directory order (CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md) is out of scope and unchanged, as are the lys.given data shape (ADR-013), the template_render event, the manifest, the launch line and how the config directory is resolved. The MCP configuration's place straight after the appended instructions is provisional, from the read order, until a later card measures a real server's tools position; this card adds no MCP server and runs measure.py exactly as it is. Order: R1 commits measure.py and re-measures first, since a different order stops the card and a different installed version moves MEASURED_VERSION; then the resolver (R2), its unit tests (R3), the fixture test (R4), RECORD.md (R5) and the rendered design documents (R6). Entries already written under the old order stand; no data member is added, and the documents say the harness version does not tell the two orders apart and that an entry's recorded time against the date of the commit that lands HOME-011 does; that commit is named, not quoted by id, and no later commit writes its id or date in. The re-measurement runs on a machine with Claude Code installed; every compile, test battery and gate runs on the build machine. S12 and S13 are shared with HOME-003, which delivered them in the old order; this brief delivers them in the request's order, and C21 and C23 stay HOME-003's. The earlier real render and the Pi-reader fixture session recorded in PROOF-GIVEN.md were written under the old order and are not re-run. HOME-003's R2 is amended, not rewritten.

## Requirements

### R1: Commit PROOF-GIVEN's measure.py unchanged and append the re-measurement of the request's order to PROOF-GIVEN.md

Commit PROOF-GIVEN's measure fixture, byte for byte, as docs/design/home/proof-given/measure.py, whose SHA-256 is db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d. WHEN the order is re-measured, THE SYSTEM SHALL copy that file unchanged into a fresh scratch directory outside the repository, run it there once with python3, which makes its own three runs, on a machine where Claude Code is installed and a local port is free, and take `claude --version` on the same machine in the same session. Append to docs/design/home/PROOF-GIVEN.md, below its last existing line, one new section, headed as the re-measurement of the request's order and carrying the day it was measured, that records: the command run; the SHA-256 of the copy that was run; the string `claude --version` printed; for each run the request count, the number of system blocks and of messages, where the appended instructions sit (the system array) and the order of the CLAUDE.md-family files and the memory index inside the first user message; that the MCP server contributed no tool, so the MCP configuration's place is taken from the read order, straight after the appended instructions; the order the entry now records, appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index; and the old order, user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index, named as superseded by the commit that lands HOME-011 and by that commit's date, with entries written before that commit standing as written. The section SHALL name 2.1.283 as the version of the earlier measurement and the version `claude --version` printed as the version of this one. IF the request places the kinds in any other order than the one above, THEN THE SYSTEM SHALL record what was measured in the section and SHALL NOT land the card, which goes back to its lead. IF the version printed is not 2.1.283 and the order is the one above, THEN the card goes on, and R2 moves MEASURED_VERSION to the version printed. THE SYSTEM SHALL NOT edit, reorder or remove any line PROOF-GIVEN.md held before, SHALL NOT change measure.py by a byte, SHALL NOT add an MCP server to the measurement, and SHALL NOT put any line of a fixture document, a request body or a marker string in the proof: paths, counts, orders, versions and hashes only.

**Acceptance:**
- `shasum -a 256 docs/design/home/proof-given/measure.py` prints db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- `git diff 7b53625 -- docs/design/home/PROOF-GIVEN.md` shows no removed line, and every added line comes after the file's line 209.
- The new section records a SHA-256 for the copy that was run equal to db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- The new section records the string `claude --version` printed on the measuring run, names that string's version as this measurement's version, and names 2.1.283 as the earlier measurement's version.
- The new section records three runs, and for each the appended instructions in the system array and the user CLAUDE.md first among the CLAUDE.md-family files in the first user message.
- The new section lists the entry's order as appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, in that order.
- The new section contains the word `superseded` beside the old order user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index, and names HOME-011.
- `grep -cE 'MARK[A-Z]+ZQ|markmcpserverzq' docs/design/home/PROOF-GIVEN.md` prints 0.

**Files:**
- create: docs/design/home/proof-given/measure.py
- modify: docs/design/home/PROOF-GIVEN.md

**Checklist:**
- C81 — PROOF-COMPACTION.md records one real compact_boundary session imported read-only with its listing as counts and hashes and its source SHA-256 equal before and after, and a rendered compacted fixture resumed on Claude Code 2.1.283 answering from the summary, by hashes only.

**Stories:**
- S13 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

### R2: List the given documents in the request's order

WHEN resolve_given resolves the documents for a working directory, a config directory and an out directory, THE SYSTEM SHALL return them in this order: the appended instructions file the render wrote (appended_instructions); the MCP configuration the render wrote (mcp_config); the user CLAUDE.md at <config>/CLAUDE.md (user_claude_md); for each directory from the outermost ancestor of the working directory down to it, its CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md in that order (claude_md_chain); then the memory index (memory_index). WHILE the config directory is D/.claude for a directory D on the chain, THE SYSTEM SHALL list D/.claude/CLAUDE.md exactly once, as user_claude_md in the user file's place, and SHALL NOT list it as claude_md_chain. Rewrite the module docs of given.rs to state that this is the order the harness's request gives, that the request's order wins wherever it and the order the harness reads the files in differ, that the MCP configuration's place straight after the appended instructions comes from the read order until a real server's tools position is measured, that MEASURED_VERSION is the version the order was re-measured on, and to word the D/.claude/CLAUDE.md rule as listed once, as user_claude_md. Set MEASURED_VERSION to the version R1's section records `claude --version` printing, and make the harness_version literal asserted in crates/lys-home/src/record/given_tests.rs that same version. In crates/lys-home/README.md, set the version the `lys.given` paragraph names in `the documents Claude Code <version> loads` to the string MEASURED_VERSION holds, in the same change that sets MEASURED_VERSION, so the README never names a version the code no longer measures. THE SYSTEM SHALL NOT change any other line of the README, and SHALL NOT change the within-directory order, CHAIN_FILES, MEASURED_VERSION to any version other than the one R1 measured, how the config directory is resolved, how a document is read, hashed, named or omitted, or any error; SHALL NOT add a data member to lys.given; and SHALL NOT rewrite, migrate or re-read any lys.given entry already written.

**Acceptance:**
- A resolution for a fixture with an out directory holding instructions.md and mcp.json, a config directory c holding CLAUDE.md and the memory index, and a working directory w holding CLAUDE.md returns 5 documents whose kinds in order are appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index.
- In that resolution documents[0].path is `instructions.md`, documents[1].path is `mcp.json` and documents[2].path is c/CLAUDE.md.
- With HOME h, no template CLAUDE_CONFIG_DIR, h/.claude/CLAUDE.md and h/w/CLAUDE.md present, working directory h/w and an empty out directory, the resolution returns 2 documents: user_claude_md at h/.claude/CLAUDE.md, then claude_md_chain at h/w/CLAUDE.md.
- `grep -n 'once, first' crates/lys-home/src/harness/claude_code/given.rs` prints nothing.
- The module docs of given.rs list the five kinds in the order appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index and contain the words `read order`.
- `git diff 7b53625 -- crates/lys-home/src/record/given.rs crates/lys-home/src/record/entries.rs crates/lys-home/src/harness/claude_code/launch.rs` prints nothing.
- The line defining CHAIN_FILES in given.rs is unchanged from 7b53625.
- The string MEASURED_VERSION holds in given.rs is the version R1's section records `claude --version` printing.
- The harness_version literal asserted in crates/lys-home/src/record/given_tests.rs equals the string MEASURED_VERSION holds, and `cargo test -p lys-home --lib record::given_tests` reports 7 tests run, all passed.
- `grep -n 'the documents Claude Code' crates/lys-home/README.md` prints one line, and the version it names is the string MEASURED_VERSION holds in given.rs.
- When MEASURED_VERSION holds 2.1.283, `grep -c '2\.1\.283' crates/lys-home/README.md` prints 1; when it holds any other version, the same command prints 0.
- `git diff 7b53625 -- crates/lys-home/README.md` changes no line other than the one naming the version.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/given.rs
- modify: crates/lys-home/src/record/given_tests.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C78 — The context path of an imported compacted session is the compaction, then the kept entries preserved uuids included, then what follows; no span entry is on it and the summary text is on it once.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R3: Assert the request's order in the given_tests.rs unit tests

In given_tests.rs, replace a_config_claude_md_is_listed_first_as_user_claude_md with a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config, which, on the module's fixture with a CLAUDE.md written in the config directory, asserts the documents' length, then that appended_instructions and mcp_config come before user_claude_md and user_claude_md before every claude_md_chain document. Rename home_dot_claude_claude_md_is_listed_once_first_as_the_user_file_when_home_is_the_config to home_dot_claude_claude_md_is_listed_once_as_the_user_file_when_home_is_the_config, keeping its body. THE SYSTEM SHALL NOT change any other test in given_tests.rs, SHALL NOT put a fixture document's line in a test name, and SHALL NOT remove an assertion that counts the documents before indexing them.

**Acceptance:**
- a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config asserts documents.len() == 5 and the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index in that order, and passes.
- home_dot_claude_claude_md_is_listed_once_as_the_user_file_when_home_is_the_config passes, and `grep -c 'listed_once_first' crates/lys-home/src/harness/claude_code/given_tests.rs` prints 0.
- one_directory_lists_claude_md_then_dot_claude_then_local_whatever_the_creation_order passes, and its body is unchanged from 7b53625.
- `cargo test -p lys-home --lib harness::claude_code::given_tests` reports 15 tests run, all passed.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs

**Checklist:**
- C79 — lys-home compactions prints one JSON report of a session's compactions, each with its loss entry, the span's ids, counts and hashes and a check that every span entry is readable by id and every named block is held, and exits 0 only when nothing is missing and 1 when anything is, naming the first missing item; a compaction with no loss entry or pointing at itself is reported unaccounted by entry id.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R4: Give the fixture test a user CLAUDE.md in its config directory and assert the request's order

In tests/given_record.rs, Fixture::new SHALL write a user CLAUDE.md into the config directory c, holding one fixed fixture line of its own. The first-render test, renamed the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event, SHALL assert given_documents 5, documents.len() 5, the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index in that order, and each document's length and SHA-256 equal to the file on disk it names (out/instructions.md, out/mcp.json, c/CLAUDE.md, w/CLAUDE.md, the memory index), with c/CLAUDE.md at index 2, and harness_version equal, as a literal, to the string MEASURED_VERSION holds; KINDS SHALL hold those five names. The two-renders test and the given listing SHALL assert 5 documents per record, and the one-byte change to w/CLAUDE.md SHALL still change exactly the claude_md_chain hash, asserted at that document's index, 3. The fallback test SHALL assert the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index with HOME's .claude/CLAUDE.md at index 2 and the memory index at index 4. THE SYSTEM SHALL NOT keep any assertion that user_claude_md occurs 0 times, SHALL NOT put the fixture line in a test name, and SHALL NOT change what the rendering process's environment holds for any test.

**Acceptance:**
- `grep -c 'user_claude_md").count(), 0' crates/lys-home/tests/given_record.rs` prints 0.
- the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event asserts report["given_documents"] == 5 and documents.len() == 5 and passes.
- The same test asserts documents[2].path == c/CLAUDE.md and that its length and SHA-256 equal those of the file written there.
- The same test's harness_version literal equals the string MEASURED_VERSION holds in given.rs.
- the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback asserts the kinds appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index and documents[2].path == h/.claude/CLAUDE.md, and passes.
- two_renders_give_equal_lists_and_one_changed_byte_changes_exactly_one_hash asserts 5 documents in each record, differing == ["claude_md_chain"] and after.documents[3].sha256 equal to the changed file's SHA-256, and passes.
- Moving the user_claude_md document back ahead of the appended instructions in resolve_given makes exactly three tests fail: a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config, the_first_render_records_five_documents_in_the_request_s_order_under_the_render_event and the_rendering_process_s_config_dir_is_never_read_and_home_dot_claude_is_the_fallback, recorded as a drift injection in the dev record with the failing test names and the count of passing tests.

**Files:**
- modify: crates/lys-home/tests/given_record.rs

**Checklist:**
- C79 — lys-home compactions prints one JSON report of a session's compactions, each with its loss entry, the span's ids, counts and hashes and a check that every span entry is readable by id and every named block is held, and exits 0 only when nothing is missing and 1 when anything is, naming the first missing item; a compaction with no loss entry or pointing at itself is reported unaccounted by entry id.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R5: State the one rule and the superseded order in RECORD.md

Rewrite the documents part of the lys.given paragraph in docs/design/home/RECORD.md to state one rule: the documents follow the order the harness's request gives, which wins wherever it and the order the harness reads the files in differ, so the entry lists appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index; the MCP configuration sits straight after the appended instructions, both harness-side inputs ahead of the first message, until a real server's tools position is measured; within one directory the order stays CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md; and D/.claude/CLAUDE.md is listed once, as user_claude_md. Name the old order, user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index, as superseded by the commit that lands HOME-011 and by that commit's date; say that entries written under it stand as written, that the old order was written under harness_version 2.1.283, that the harness version does not tell the two orders apart, and that a reader tells an entry's order by comparing its recorded time with that commit's date, an entry written before it listing the old order. THE SYSTEM SHALL NOT claim that harness_version marks the order, SHALL NOT add a data member to the paragraph's data list, and SHALL NOT change any other paragraph of RECORD.md.

**Acceptance:**
- The lys.given paragraph names appended_instructions, mcp_config, user_claude_md, claude_md_chain and memory_index as the documents' order, in that order.
- The paragraph contains the words `request` and `read order` in the sentence stating which order wins.
- The paragraph contains `superseded` and `HOME-011`, and the old order user_claude_md, appended_instructions, mcp_config, claude_md_chain, memory_index.
- The paragraph states that harness_version does not tell the two orders apart, and that an entry's order is told by its recorded time against the landing commit's date.
- `grep -n 'listed once, first' docs/design/home/RECORD.md` prints nothing.
- The paragraph's data list is still exactly `{harness, harness_version, kinds, config_dir, documents, environment}`.

**Files:**
- modify: docs/design/home/RECORD.md

**Checklist:**
- C80 — The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record, then the kept entries, with no legacy summary line and no lys.loss line, and the summary text once; the boundary record's uuid is derived under render-uuid/v2, alongside v1, whose non-boundary uuids equal v1's, and a render with no compaction stays v1, byte for byte, with the version it used named in its report.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

### R6: Record the supersession on HOME-003 and render its markdown

Append one amendment to docs/design/home/briefs/HOME-003.json, as a new `amendments` member holding one entry, dated the day it is written and attributed to HOME-011, whose ruling names R2's cross-kind order (user_claude_md first) as superseded by HOME-011 and ADR-030, the documents now following the request's order appended_instructions, mcp_config, user_claude_md, claude_md_chain, memory_index, with the within-directory order unchanged. Then run scripts/design/render-cluster.py on docs/design/home so briefs/HOME-003.md is what HOME-003.json renders to; S12's wording and every other rendered file of the cluster are already committed as rendered, so the render has nothing else to change. THE SYSTEM SHALL NOT change any key of HOME-003.json other than adding `amendments`, SHALL NOT edit a rendered markdown file by hand and SHALL NOT change any authored JSON of the cluster other than HOME-003.json.

**Acceptance:**
- Loaded as JSON, docs/design/home/briefs/HOME-003.json at 7b53625 and after the change are equal in every key except `amendments`, and after the change `amendments` holds exactly one entry.
- That entry's ruling contains `HOME-011`, `ADR-030` and `superseded`.
- docs/design/home/USER-STORIES.md contains `in the order the request gives them` and does not contain `in the order the harness reads them`.
- `sh scripts/design/gate.sh` exits 0.

**Files:**
- modify: docs/design/home/briefs/HOME-003.json
- modify: docs/design/home/briefs/HOME-003.md

**Checklist:**
- C82 — A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, with no compaction and no lys.loss entry, and lys-home compactions reports it by entry id as a boundary without a summary.

**Stories:**
- S12 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every render to record which instruction documents the session was given, in the order the harness reads them, by path, length and hash, with the environment names it was set, so that I can later check a file on disk against what a session was given without anyone reading its contents.

## Boundaries

- The within-directory order (CLAUDE.md, .claude/CLAUDE.md, CLAUDE.local.md) and CHAIN_FILES do not change.
- No data member is added to lys.given; its six keys, its kinds lists and its config_dir shape stay as they are, and record/given.rs and record/entries.rs are not changed.
- No lys.given entry already written is rewritten, migrated or re-read to change it; the record is append-only.
- measure.py is committed and run byte for byte as it is, and no MCP server is added to the measurement.
- No line PROOF-GIVEN.md already holds is edited, reordered or removed; the new measurement is appended below it.
- No document content, request body or fixture marker appears in PROOF-GIVEN.md, a test name, an error or output: paths, counts, orders, versions and hashes only.
- The template_render event, the manifest, the launch line and the config directory's resolution do not change, and nothing in lys-home launches Claude Code (ADR-007).
- MEASURED_VERSION holds the version the re-measurement ran on: 2.1.283 when that is what is installed, the installed version otherwise; no other version is written.
- No commit after the one that lands HOME-011 writes that commit's id or date into RECORD.md or PROOF-GIVEN.md; a reader finds both in git.
- Measuring a real MCP server's tools position is not part of this brief.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists R3's and R4's named tests as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- shasum -a 256 docs/design/home/proof-given/measure.py prints db17cccb008507eed1ca67ab197a353c6901d09a63d1c284bf8a1f7d49ac245d.
- git diff 7b53625 -- docs/design/home/PROOF-GIVEN.md shows added lines only, all after line 209.
- git diff 7b53625 -- crates/lys-home/src/record/given.rs crates/lys-home/src/record/entries.rs prints nothing.
- With the user_claude_md push moved back ahead of the appended instructions in resolve_given, cargo test -p lys-home fails exactly the three tests R4's drift acceptance names; restored, it passes.
