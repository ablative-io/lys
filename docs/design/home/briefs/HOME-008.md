---
type: brief
id: HOME-008
cluster: home
title: Import a Claude Code compaction beside its original with a lys.loss account, list and check it, and render it as the compact_boundary pair
---

# HOME-008: Import a Claude Code compaction beside its original with a lys.loss account, list and check it, and render it as the compact_boundary pair

> **Cluster:** home
> **Design anchor:**
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-015 — A lantern's note and epilogues are the person's annotations, not transcript — A lantern's note and its epilogues are the person's own annotations, not transcript. Recall prints them by design; they are the one exception to P7 and CN3, and only recall prints them. Transcript lines (message, tool and compaction content) still never appear in output, logs or errors, and a recall row never carries a line of the transcript around its point. Rejected: treating notes as transcript and printing only their hashes, which makes recall useless to the person who wrote them.
> - ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
> - ADR-029 — What a compaction could not keep is a lys.loss custom entry of ids, counts and SHA-256s — Each compaction the importer writes is followed directly by one lys.loss custom entry, a side leaf whose parent is the compaction entry, whose data names the compaction by entry id, the first and last entry id of the span it summarised (the root-to-first-kept path entries, from the root or the previous compaction to the entry before the first kept one), the counts of that span's entries, messages, tool calls, tool results and blocks, its line bytes and block bytes counted separately, the counts of side-leaf and sidechain entries hanging from it by number only, the tokensBefore the harness reported (null when it reported none), a logicalParentUuid that names no record as unresolved, the SHA-256 of the span's first source line, of its last and of its lines concatenated in file order, the hash of the summary record's block and the span's distinct block hashes. Its keys are: compaction_id, first_kept (null when nothing is kept), span_first, span_last, entries, messages, tool_calls, tool_results, blocks, line_bytes, block_bytes, side_leaf_entries, sidechain_entries, tokens_before, unresolved_logical_parent, first_line_sha256, last_line_sha256, span_sha256, summary_record, block_hashes. Rejected: adding fields to Pi's compaction entry (it breaks CN4 and P2); hashing or listing side leaves and sidechain entries by id (their ids are drawn fresh on every import, so two imports of one file would disagree); and carrying any content, the summary included (P7).
> - ADR-030 — A Claude Code compaction imports as one Pi compaction at the summary record's place, keeping what Claude Code kept, and renders as the compact_boundary pair — A compact_boundary record imports as today, a lys.harness_event on the chain; the isCompactSummary record under it becomes one Pi compaction entry that takes that record's uuid, sits where the record sat as the child of the boundary's entry, carries the record's text as its summary and tokensBefore from compactMetadata.preTokens, and keeps the record only as a block; it is not a message entry. Its first kept entry is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and when there is none nothing is kept and firstKeptEntryId is the empty string, which keeps Pi's string type and is the only value that names no entry, so a compaction never names itself; anchorUuid is not used. A compact_boundary with no isCompactSummary record under it is not a compaction the harness completed: it stays a lys.harness_event, no summary is invented, and the listing reports it as a boundary without a summary. A legacy summary record keeps the entry its leafUuid names. A first kept entry not on record is refused by uuid. A logicalParentUuid naming no record is not refused. The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record holding the summary, then the kept entries, never the legacy summary line and never a custom entry. Rejected: bounding the kept range by anchorUuid; keeping the summary record as a message entry beside the compaction (the summary would be read twice); placing the compaction off the chain or re-parenting the next message (HOME-001 R3's parent-equals-source acceptance stands with no exception); and rendering the legacy summary line, which the current Claude Code does not write.
> **Checklist:**
> - C71 — A compact_boundary record followed by its isCompactSummary user record imports as a lys.harness_event for the boundary and one Pi compaction entry under the summary record's uuid, the child of the boundary's entry, whose firstKeptEntryId is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and the empty string when there is none, so its context path is the compaction alone and the pair renders back, and whose tokensBefore is compactMetadata.preTokens; the summary record is not a message entry.
> - C72 — A legacy summary record imports as a Pi compaction entry whose firstKeptEntryId is the entry its leafUuid names, never the compaction's own id.
> - C73 — A compaction whose first kept entry is not on record is refused with an error naming that uuid.
> - C74 — Directly after each compaction entry the importer appends one lys.loss custom entry whose parent is the compaction and whose data holds only ids, counts, byte counts and SHA-256 hashes, with the keys RECORD.md sets down, first_kept among them, null when nothing is kept.
> - C75 — A lys.loss entry's span is the root-to-first-kept path from the root or the previous compaction to the entry before the first kept one; its three SHA-256s are over those entries' source lines in file order, and side-leaf and sidechain entries hanging from the span are counted by number only.
> - C76 — A compact_boundary whose logicalParentUuid names no record in the file imports, and its lys.loss entry names that uuid as unresolved.
> - C77 — Two imports of the compaction fixture into two homes give lys.loss lines equal byte for byte once id, parentId, timestamp and data.compaction_id are masked.
> - C78 — The context path of an imported compacted session is the compaction, then the kept entries preserved uuids included, then what follows; no span entry is on it and the summary text is on it once.
> - C79 — lys-home compactions prints one JSON report of a session's compactions, each with its loss entry, the span's ids, counts and hashes and a check that every span entry is readable by id and every named block is held, and exits 0 only when nothing is missing and 1 when anything is, naming the first missing item; a compaction with no loss entry or pointing at itself is reported unaccounted by entry id.
> - C80 — The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record, then the kept entries, with no legacy summary line and no lys.loss line, and the summary text once; the boundary record's uuid is derived under render-uuid/v2, alongside v1, whose non-boundary uuids equal v1's, and a render with no compaction stays v1, byte for byte, with the version it used named in its report.
> - C81 — PROOF-COMPACTION.md records one real compact_boundary session imported read-only with its listing as counts and hashes and its source SHA-256 equal before and after, and a rendered compacted fixture resumed on Claude Code 2.1.283 answering from the summary, by hashes only.
> - C82 — A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, with no compaction and no lys.loss entry, and lys-home compactions reports it by entry id as a boundary without a summary.
> **Stories:**
> - S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.
> - S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.
> - S34 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

## Purpose

A compaction is the first derived record, because the harness produces it. This brief makes the home keep a Claude Code compaction beside its original and say what it could not keep: both shapes Claude Code has written import as one Pi compaction entry that keeps what Claude Code kept (ADR-030), a lys.loss entry directly after it names the summarised span by ids, counts and SHA-256s only (ADR-029, CN11), a listing proves every summarised entry and block is still held, and the Claude Code render writes the compaction the way the current Claude Code writes it, measured to resume (P6). The summarised entries are never touched (P1) and no content leaves the home (P7, CN3).

## Task

Today crates/lys-home/src/harness/claude_code/import.rs maps only a legacy summary record, to a compaction whose firstKeptEntryId is its own fresh id with tokensBefore 0 and leafUuid ignored; a compact_boundary record goes to event_of as a system lys.harness_event and its isCompactSummary record imports as a plain user message; and render.rs writes a compaction as a legacy summary line with leafUuid null. This brief replaces that mapping (R2), adds the lys.loss entry (R1, R3), gates the fixture end to end (R4), adds the listing (R5), renders the compact_boundary pair (R6) and proves it (R7). The first kept entry of a boundary is the earliest preserved uuid, never anchorUuid: in the survey's sample of 28 boundaries anchorUuid named the summary record itself in all 28, and all 94 preserved uuids sat before the boundary, so bounding by anchorUuid would count what Claude Code kept as lost. The compaction takes the isCompactSummary record's uuid and sits where it sat, so HOME-001 R3's parent-equals-source acceptance holds with no exception; the loss entry is a side leaf under it, off the chain. import.rs is at 473 of 500 code lines, so the mapping lives in compaction.rs and the span account in span.rs; import.rs gains only the calls and the bookkeeping of each entry's source line offset. With no non-empty preservedMessages.uuids, the first kept entry is the first record after the pair whose parent is the isCompactSummary record, now the compaction entry, so a compaction never names itself. A compact_boundary with no isCompactSummary record under it is not a compaction the harness completed: it imports as the lys.harness_event it imports as today, with no compaction and no loss entry, the home never invents a summary, and the listing reports it by entry id as a boundary without a summary, so the survey's 20 in 1,499 are visible and counted. The listing prints its whole report either way, exits 0 only when every check passes and 1 when any span entry or block is missing, naming the first missing one in the report; a refusal to run at all keeps its own named error and exit status. A pair with no non-empty preservedMessages.uuids that ends the file keeps nothing: it imports as a compaction entry whose firstKeptEntryId is the empty string, which keeps Pi's string type and cannot be mistaken for an id, followed by its lys.loss entry whose first_kept is JSON null; the home's reader turns the empty string into a typed nothing-kept at the one place it parses the field, a non-empty firstKeptEntryId that names no entry stays refused by name, the context path is the compaction alone, the listing reports it among compactions, and the render writes the pair back. The boundary record's rendered uuid comes from render-uuid/v2, a new version alongside v1 that carries v1's derivation of the role record byte for byte and adds the role compact_boundary; v1's closed role list is obeyed, not amended, so a render with no compaction stays v1 and writes the bytes it wrote before, and the render report names the version it used. The proof's real session is a current compact_boundary session from the Claude Code install of the machine the proof runs on, read only; an older 2.0.53 file holding a legacy summary record may be imported as well but does not satisfy that line. Every compile, test battery and gate runs on the build machine; the proof's import and resume run with a lys-home binary built there. Out of scope: the home compacting a session itself; translation (stage 4b, its own card); changing what Claude Code writes; the handover letter (HOME-001 R12); writing loss entries into homes imported before this brief; and any harness but Claude Code.

## Requirements

### R1: Define the lys.loss custom entry

Add the custom type constant `lys.loss` to record/entries.rs and a record/loss.rs module holding its data, serialised with exactly these keys: compaction_id (the compaction's entry id), first_kept (the compaction's first kept entry id, null when nothing is kept), span_first and span_last (entry ids, null for an empty span), entries, messages, tool_calls, tool_results, blocks, line_bytes, block_bytes, side_leaf_entries and sidechain_entries (unsigned integers), tokens_before (an unsigned integer, null when the harness reported none), unresolved_logical_parent (a uuid string, null when none), first_line_sha256, last_line_sha256 and span_sha256 (64 lowercase hex characters, null for an empty span), summary_record (the SHA-256 of the compaction's source record block, null when there is none) and block_hashes (the span's distinct block hashes in first-reference order). In record/entries.rs, give the compaction entry one reader of firstKeptEntryId that returns nothing kept for the empty string and the id otherwise, used by the context path and the listing. WHEN a loss is appended for a compaction, THE SYSTEM SHALL append it as a custom entry whose parentId is the compaction's entry id, as the very next line of the session file after the compaction entry, and SHALL NOT move the head to it. THE SYSTEM SHALL NOT write JSON null or omit firstKeptEntryId in a compaction entry, SHALL NOT add a key to Pi's compaction entry, SHALL NOT put any message, tool or summary text into the data, and SHALL NOT name a side-leaf or sidechain entry by id.

**Acceptance:**
- A unit test serialises a loss with every field set and asserts the data object's keys are exactly the twenty named in the spec, in no other spelling.
- A unit test appends a compaction entry then its loss to a session and asserts the loss line is the session file's next line, its type is custom, its customType is lys.loss, its parentId is the compaction's id, and the session head is still the compaction's id.
- A unit test serialises a loss with tokens_before and unresolved_logical_parent unset and asserts both are JSON null, not 0 and not absent.
- A unit test serialises a loss with first_kept unset and asserts it is JSON null, not the empty string and not absent.
- A unit test parses a compaction line whose firstKeptEntryId is "" and asserts the reader returns nothing kept, and parses one whose firstKeptEntryId is 55555555-5555-4555-8555-555555555555 and asserts the reader returns that id; serialising the first entry back writes "firstKeptEntryId":"".
- `grep -n 'lys.loss' crates/lys-home/src/record/entries.rs` prints one line.

**Files:**
- create: crates/lys-home/src/record/loss.rs
- create: crates/lys-home/src/record/loss_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C74 — Directly after each compaction entry the importer appends one lys.loss custom entry whose parent is the compaction and whose data holds only ids, counts, byte counts and SHA-256 hashes, with the keys RECORD.md sets down, first_kept among them, null when nothing is kept.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.

### R2: Map both Claude Code compaction shapes into one Pi compaction entry

Add crates/lys-home/tests/fixtures/compaction.jsonl, a synthetic Claude Code file of fourteen lines carrying no real transcript content, each record with a fixed timestamp, and every text part, tool_result content and summary a phrase of at least three words distinct from every other's: 1 a user text record 11111111-1111-4111-8111-111111111111 (parentUuid null); 2 an assistant record 22222222-2222-4222-8222-222222222222 with one text part and one tool_use part (id toolu_01), parent line 1; 3 a user record 33333333-3333-4333-8333-333333333333 holding exactly one tool_result part for toolu_01, parent line 2; 4 an assistant text record 44444444-4444-4444-8444-444444444444 with isSidechain true, agentId a1 and parentUuid null; 5 an assistant text record 55555555-5555-4555-8555-555555555555, parent line 3; 6 a user text record 66666666-6666-4666-8666-666666666666, parent line 5; 7 a system record 77777777-7777-4777-8777-777777777777 with subtype compact_boundary, parentUuid null, logicalParentUuid 66666666-6666-4666-8666-666666666666 and compactMetadata {trigger manual, preTokens 12345, preservedMessages {anchorUuid 88888888-8888-4888-8888-888888888888, uuids [66666666-6666-4666-8666-666666666666, 55555555-5555-4555-8555-555555555555]}}; 8 a user record 88888888-8888-4888-8888-888888888888 with isCompactSummary true and isVisibleInTranscriptOnly true, parent line 7, whose message content is one string, the first summary; 9 an assistant record 99999999-9999-4999-8999-999999999999 with one tool_use part (id toolu_02), parent line 8; 10 a user record aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa holding exactly one tool_result part for toolu_02, parent line 9; 11 an assistant text record bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb with isSidechain true, agentId a2 and parentUuid null; 12 an assistant text record cccccccc-cccc-4ccc-8ccc-cccccccccccc, parent line 10; 13 a user text record dddddddd-dddd-4ddd-8ddd-dddddddddddd, parent line 12; 14 a legacy record of type summary whose summary is the second summary, holding a code word that appears nowhere else in the fixture, and whose leafUuid is dddddddd-dddd-4ddd-8ddd-dddddddddddd. In a new harness/claude_code/compaction.rs, called from import.rs, replacing the summary arm that pointed a compaction at itself. WHEN the importer reads a user record with isCompactSummary true whose parentUuid names a compact_boundary record already imported, THE SYSTEM SHALL store that record whole as a block and append one Pi compaction entry whose id is the record's uuid, whose parentId is the boundary's entry id, whose summary is the record's message content text, whose tokensBefore is the boundary's compactMetadata.preTokens, and whose firstKeptEntryId is, when the boundary's compactMetadata.preservedMessages.uuids is present and non-empty, the one of those uuids imported earliest in the file, and otherwise the uuid of the first record after the summary record in the file whose parentUuid is the summary record's uuid, found by reading the source file forward from the summary record's line one line at a time, and the empty string when the file holds no such record; the compact_boundary record itself SHALL import as the lys.harness_event it imports as today. WHEN the importer reads a record of type summary, THE SYSTEM SHALL store it whole as a block and append a Pi compaction entry under a fresh id with the parent it has today, the record's summary, tokensBefore 0 because Pi's field is a number, and firstKeptEntryId the uuid its leafUuid names. IF a uuid in preservedMessages.uuids names no entry already in the session, or a leafUuid names none, THEN THE SYSTEM SHALL refuse the import with an error naming that uuid, as it refuses an unknown parent. IF the boundary's logicalParentUuid names no record in the file, THEN THE SYSTEM SHALL import the boundary under the chain's last entry as today and SHALL NOT refuse. IF a compact_boundary record has no isCompactSummary record under it, THEN THE SYSTEM SHALL import it as the lys.harness_event it imports as today and SHALL NOT append a compaction or a lys.loss entry for it. THE SYSTEM SHALL NOT write the isCompactSummary record as a message entry, SHALL NOT invent a summary, SHALL NOT use compactMetadata.preservedMessages.anchorUuid to find the first kept entry, SHALL NOT set a compaction's firstKeptEntryId to its own id, SHALL NOT re-parent any entry, and SHALL NOT change the id or parent of any message entry, so every message entry's parent still equals its source parentUuid. import.rs SHALL stay at or under 500 code lines.

**Acceptance:**
- Importing the fixture yields an entry 88888888-8888-4888-8888-888888888888 of type compaction whose parentId is 77777777-7777-4777-8777-777777777777, firstKeptEntryId is 55555555-5555-4555-8555-555555555555 (the preserved uuid earlier in the file, though the list names 66666666-6666-4666-8666-666666666666 first), tokensBefore is 12345 and summary is the text of line 8.
- The imported session holds no message entry with id 88888888-8888-4888-8888-888888888888.
- Importing the fixture yields exactly one other compaction entry, whose firstKeptEntryId is dddddddd-dddd-4ddd-8ddd-dddddddddddd and whose id is not dddddddd-dddd-4ddd-8ddd-dddddddddddd and not its own firstKeptEntryId.
- A copy of the fixture holding lines 1 to 10 with line 7's compactMetadata.preservedMessages removed imports with exit 0 to a session whose compaction entry 88888888-8888-4888-8888-888888888888 has firstKeptEntryId 99999999-9999-4999-8999-999999999999.
- A copy of the fixture holding lines 1 to 8 with line 7's compactMetadata.preservedMessages removed imports with exit 0 to a session whose compaction entry 88888888-8888-4888-8888-888888888888 has firstKeptEntryId "" (a JSON string of length 0).
- A copy of the fixture holding lines 1 to 7 only imports with exit 0 to a session holding no compaction entry and no lys.loss entry, whose entry 77777777-7777-4777-8777-777777777777 is a custom entry of customType lys.harness_event.
- A copy of the fixture whose preservedMessages.uuids is [66666666-6666-4666-8666-666666666666, eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee] is refused, and the error's text contains eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee; `lys-home import` on it through `run` exits 1 with eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee on stderr.
- A copy of the fixture whose line 14 leafUuid is ffffffff-ffff-4fff-8fff-ffffffffffff is refused, and the error's text contains ffffffff-ffff-4fff-8fff-ffffffffffff.
- A copy of the fixture whose line 7 logicalParentUuid is ffffffff-ffff-4fff-8fff-ffffffffffff imports with exit 0 and the same two compaction entries.
- The existing round-trip test in tests/claude_code_round_trip.rs that every message entry's parent equals its source parentUuid passes unchanged, and a test asserts the same over every message entry of the imported fixture.
- `git diff` of the landed change shows crates/lys-home/tests/claude_code_round_trip.rs unchanged.
- A line count of crates/lys-home/src/harness/claude_code/import.rs excluding comments, blank lines and tests is at most 500.

**Files:**
- create: crates/lys-home/src/harness/claude_code/compaction.rs
- create: crates/lys-home/src/harness/claude_code/compaction_tests.rs
- create: crates/lys-home/tests/fixtures/compaction.jsonl
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C71 — A compact_boundary record followed by its isCompactSummary user record imports as a lys.harness_event for the boundary and one Pi compaction entry under the summary record's uuid, the child of the boundary's entry, whose firstKeptEntryId is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and the empty string when there is none, so its context path is the compaction alone and the pair renders back, and whose tokensBefore is compactMetadata.preTokens; the summary record is not a message entry.
- C72 — A legacy summary record imports as a Pi compaction entry whose firstKeptEntryId is the entry its leafUuid names, never the compaction's own id.
- C73 — A compaction whose first kept entry is not on record is refused with an error naming that uuid.
- C82 — A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, with no compaction and no lys.loss entry, and lys-home compactions reports it by entry id as a boundary without a summary.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.

### R3: Account for each compaction's span and append its loss entry

In a new harness/claude_code/span.rs. WHEN the importer appends a compaction entry, THE SYSTEM SHALL at once append its lys.loss entry (R1), computed over the span: the entries on the session's root-to-compaction path from the root, or from the previous compaction entry on that path, through the last path entry before whichever of the first kept entry and the compaction entry comes first on the path, or before the compaction entry when nothing is kept, both ends included. first_kept is the compaction's first kept entry id, null when nothing is kept; span_first and span_last are the span's first and last entry ids; entries counts the span's entries; messages counts its message entries of every role; tool_calls counts the toolCall parts of its assistant messages; tool_results counts its message entries whose role is toolResult; block_hashes lists the distinct blocks its entries were stored as (each message part's block, an on-chain harness event's record block, a previous compaction's summary record block) and blocks counts them; block_bytes sums those blocks' byte lengths; line_bytes sums the byte lengths, without line terminators, of the distinct source-file lines the span's entries were imported from; first_line_sha256 and last_line_sha256 are the SHA-256 of the first and last of those lines in file order, without terminator, and span_sha256 the SHA-256 of those lines in file order, each followed by one line feed; side_leaf_entries counts the entries that are not sidechain entries and hang from a span entry off the path, and sidechain_entries the sidechain labels and sidechain records hanging from a span entry; tokens_before is the compaction's reported value (the boundary's preTokens, null for a legacy summary record and for a boundary without one); unresolved_logical_parent is the boundary's logicalParentUuid when it names no record in the file, else null; summary_record is the hash of the block R2 stored for the compaction's record. THE SYSTEM SHALL read span source lines by their offsets and SHALL NOT hold the source file or the session file whole in memory. THE SYSTEM SHALL NOT count or hash any entry from the first kept entry onward, SHALL NOT put a side-leaf or sidechain entry's line or id into a hash or the data, and SHALL NOT read a clock or a random source for any value in the data.

**Acceptance:**
- On the fixture, the entry following 88888888-8888-4888-8888-888888888888 is a lys.loss entry with parentId 88888888-8888-4888-8888-888888888888 and data: compaction_id 88888888-8888-4888-8888-888888888888, first_kept 55555555-5555-4555-8555-555555555555, span_first 11111111-1111-4111-8111-111111111111, span_last 33333333-3333-4333-8333-333333333333, entries 3, messages 3, tool_calls 1, tool_results 1, blocks 4, side_leaf_entries 1, sidechain_entries 2, tokens_before 12345, unresolved_logical_parent null.
- For that loss, the test computes from the fixture file, not from the home, the SHA-256 of line 1, of line 3 and of lines 1 to 3 each followed by a line feed, and the sum of the byte lengths of lines 1 to 3, and asserts first_line_sha256, last_line_sha256, span_sha256 and line_bytes equal them.
- On the copy of the fixture holding lines 1 to 10 with line 7's compactMetadata.preservedMessages removed, the loss following 88888888-8888-4888-8888-888888888888 has span_first 11111111-1111-4111-8111-111111111111, span_last 77777777-7777-4777-8777-777777777777, entries 6, messages 5, tool_calls 1 and tool_results 1, and first_line_sha256, last_line_sha256 and span_sha256 equal to the values the test computes from fixture lines 1 and 7 and from lines 1, 2, 3, 5, 6 and 7.
- On the copy of the fixture holding lines 1 to 8 with line 7's compactMetadata.preservedMessages removed, the session file's next line after the compaction entry 88888888-8888-4888-8888-888888888888 is a lys.loss entry whose parentId is 88888888-8888-4888-8888-888888888888 and whose data has first_kept null, span_first 11111111-1111-4111-8111-111111111111, span_last 77777777-7777-4777-8777-777777777777, entries 6, messages 5, tool_calls 1 and tool_results 1.
- On the fixture, the entry following the legacy compaction is a lys.loss entry with span_first 88888888-8888-4888-8888-888888888888, span_last cccccccc-cccc-4ccc-8ccc-cccccccccccc, entries 4, messages 3, tool_calls 1, tool_results 1, blocks 4, side_leaf_entries 1, sidechain_entries 2, tokens_before null, and first_line_sha256, last_line_sha256, span_sha256 and line_bytes equal to the values the test computes from fixture lines 8, 9, 10 and 12.
- For each loss, the test computes the SHA-256 of serde_json::to_vec of every part of the span's fixture lines and of the whole records the span's blocks hold, asserts block_hashes equals that set, and asserts block_bytes equals the sum of the sizes of the files blocks/<hh>/<hash> the home holds for them.
- Neither loss data names 44444444-4444-4444-8444-444444444444, bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb or any id of a sidechain label or tool_completed side leaf.
- On the copy of the fixture whose line 7 logicalParentUuid is ffffffff-ffff-4fff-8fff-ffffffffffff, the first loss's unresolved_logical_parent is ffffffff-ffff-4fff-8fff-ffffffffffff and every other key equals the original fixture's first loss.
- On the fixture, for each loss the test walks every key of its data except first_kept and asserts that no value is 55555555-5555-4555-8555-555555555555, 66666666-6666-4666-8666-666666666666 or 77777777-7777-4777-8777-777777777777; it asserts that first_kept is the only key of either loss holding any of those three ids, and it prints the number of keys walked per loss, which is greater than 0.

**Files:**
- create: crates/lys-home/src/harness/claude_code/span.rs
- create: crates/lys-home/src/harness/claude_code/span_tests.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C74 — Directly after each compaction entry the importer appends one lys.loss custom entry whose parent is the compaction and whose data holds only ids, counts, byte counts and SHA-256 hashes, with the keys RECORD.md sets down, first_kept among them, null when nothing is kept.
- C75 — A lys.loss entry's span is the root-to-first-kept path from the root or the previous compaction to the entry before the first kept one; its three SHA-256s are over those entries' source lines in file order, and side-leaf and sidechain entries hanging from the span are counted by number only.
- C76 — A compact_boundary whose logicalParentUuid names no record in the file imports, and its lys.loss entry names that uuid as unresolved.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.

### R4: Gate the compaction fixture's import, context path and two-home equality end to end

An end-to-end test in tests/compaction_import.rs runs over the fixture R2 adds, crates/lys-home/tests/fixtures/compaction.jsonl. WHEN the fixture is imported through lys_home::cli::run into a session named `compacted`, THE SYSTEM SHALL give a session whose context path at the head is the legacy compaction then the entry its leafUuid names, and whose context path with the head moved to the last entry before the legacy summary is the first compaction, then the kept entries from the earliest preserved uuid, then what follows. WHEN the fixture is imported into two fresh homes, THE SYSTEM SHALL write lys.loss lines that are equal byte for byte once the entry's id, parentId and timestamp and the data's compaction_id are replaced by a fixed mark. THE SYSTEM SHALL NOT put a span entry on a context path, SHALL NOT put the summary text on a context path more than once, and the test SHALL NOT mask any field but those four.

**Acceptance:**
- With the head at cccccccc-cccc-4ccc-8ccc-cccccccccccc, the context path's entry ids are exactly [88888888-8888-4888-8888-888888888888, 55555555-5555-4555-8555-555555555555, 66666666-6666-4666-8666-666666666666, 77777777-7777-4777-8777-777777777777, 99999999-9999-4999-8999-999999999999, aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa, cccccccc-cccc-4ccc-8ccc-cccccccccccc]: the preserved entries 55555555-5555-4555-8555-555555555555 and 66666666-6666-4666-8666-666666666666 are on it and are outside both losses' counts.
- With the head at the file's head, the context path's entry ids are exactly [the legacy compaction's id, dddddddd-dddd-4ddd-8ddd-dddddddddddd].
- Neither context path holds 11111111-1111-4111-8111-111111111111, 22222222-2222-4222-8222-222222222222 or 33333333-3333-4333-8333-333333333333.
- On the copy of the fixture holding lines 1 to 10 with line 7's compactMetadata.preservedMessages removed, with the head at aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa, the context path's entry ids are exactly [88888888-8888-4888-8888-888888888888, 99999999-9999-4999-8999-999999999999, aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa].
- On the copy of the fixture holding lines 1 to 8 with line 7's compactMetadata.preservedMessages removed, with the head at 88888888-8888-4888-8888-888888888888, the context path's entry ids are exactly [88888888-8888-4888-8888-888888888888].
- On each context path, the count of entries whose summary or message text has the SHA-256 of that path's compaction summary text is exactly 1, counted by hash with the text never printed.
- Two imports of the fixture into two temporary homes give, for each of the two compactions, lys.loss lines whose bytes are equal after masking id, parentId, timestamp and data.compaction_id, and the test's mask list is exactly those four names.
- Every line of the imported session file after the header parses as a JSON object with type, id, parentId and timestamp, and each lys.loss line has type custom and an object data.
- The test names and assertion messages in tests/compaction_import.rs carry no text of the fixture's records.

**Files:**
- create: crates/lys-home/tests/compaction_import.rs

**Checklist:**
- C77 — Two imports of the compaction fixture into two homes give lys.loss lines equal byte for byte once id, parentId, timestamp and data.compaction_id are masked.
- C78 — The context path of an imported compacted session is the compaction, then the kept entries preserved uuids included, then what follows; no span entry is on it and the summary text is on it once.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.
- S34 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

### R5: List a session's compactions and check each span through the index and the block store

Add `lys-home compactions --home <dir> --session <id>` in cli/compactions.rs, wired into the Command enum in cli.rs, over a record/compactions.rs module. WHEN run on a session, THE SYSTEM SHALL print one JSON object {session, compactions, boundaries_without_summary, first_missing} to stdout, where boundaries_without_summary lists in file order the entry id of every lys.harness_event of kind system and subtype compact_boundary that no compaction entry has as its parent, first_missing names the first missing item in report order as {compaction, kind, id} with kind entry or block (the first compaction in file order with anything missing, its entries_missing before its blocks_missing) and is null when nothing is missing, and compactions lists every compaction entry in the session file in file order, each as: compaction (its id), first_kept (null when nothing is kept), tokens_before, loss (the id of the lys.loss entry whose compaction_id names it, or null), status (accounted or unaccounted), act, span (the loss data's ids, counts, byte counts and hashes, without block_hashes, or null) and check {entries_read, entries_missing, blocks_held, blocks_missing}. The check SHALL read every span entry by id through the index, walking parents from span_last to span_first, and SHALL list in entries_missing each id it cannot read and the span_first it never reaches; it SHALL check every hash in the loss's block_hashes and its summary_record against the block store, counting as held a block that is present and whose bytes hash to its name and listing every other hash in blocks_missing. THE SYSTEM SHALL exit 0 when no check lists a missing entry or block and exit 1 when any does, printing the whole report in both cases; IF the command refuses to run, such as for an unknown home or session, THEN THE SYSTEM SHALL exit 1 with that refusal's named error on stderr and no report on stdout. IF a compaction has no lys.loss entry naming it, or its firstKeptEntryId is its own id, THEN THE SYSTEM SHALL report it with status unaccounted, loss null when none, span and check null, and act `re-import the source file into a new home`, and act is null for an accounted compaction. THE SYSTEM SHALL find compaction and lys.loss entries in one pass over the session file holding one line at a time and SHALL NOT load the session file whole. THE SYSTEM SHALL NOT refuse an unaccounted compaction, SHALL NOT write to the home, the session or its index, and SHALL NOT print any summary, message or tool text on stdout or stderr.

**Acceptance:**
- On the imported fixture, the command exits 0, first_missing is null, boundaries_without_summary is [], and the report lists two compactions in file order, the first with compaction 88888888-8888-4888-8888-888888888888, status accounted, act null, check entries_read 3, entries_missing [], blocks_held equal to 5 (its four block_hashes and its summary_record), blocks_missing [].
- On the imported fixture, the second compaction's check has entries_read 4 and entries_missing [] and blocks_missing [], and its span equals its loss data without block_hashes.
- In a copy of the imported home with one block file named in the first loss's block_hashes deleted, the command exits 1, prints the whole report, the report's first check lists exactly that hash in blocks_missing, first_missing is {compaction 88888888-8888-4888-8888-888888888888, kind block, id that hash}, and the second check is unchanged.
- In a copy of the imported home with that block file's bytes changed and its name kept, the report's first check lists exactly that hash in blocks_missing.
- A session holding a compaction entry built by hand whose firstKeptEntryId is its own id and no lys.loss entry is reported with status unaccounted, loss null, span null, check null and act `re-import the source file into a new home`, the command exits 0, and the session file's SHA-256 is equal before and after.
- On the copy of the fixture holding lines 1 to 7 only, imported, the command exits 0 with compactions [] and boundaries_without_summary [77777777-7777-4777-8777-777777777777].
- On the copy of the fixture holding lines 1 to 8 with line 7's compactMetadata.preservedMessages removed, imported, the command exits 0 with one compaction 88888888-8888-4888-8888-888888888888 of status accounted and first_kept null, check entries_read 6 and entries_missing [], and boundaries_without_summary [].
- `lys-home compactions` on a session id the home does not hold exits 1, prints nothing on stdout, and prints the unknown-session error naming that id on stderr.
- The listing leaves the session file, its index and its head file with the SHA-256 they had before it ran.
- A test takes the fixture's text-bearing parts, the text parts of lines 1, 2, 4, 5, 6, 11, 12 and 13, the tool_result contents of lines 3 and 10 and the summaries of lines 8 and 14, trims each of surrounding whitespace, leaves out only parts that are empty after trimming, and compares each remaining part's whole text with the command's stdout and stderr on the imported fixture: 0 occur; the test asserts the number of parts compared is 12 and the number left out is 0, and prints both numbers.

**Files:**
- create: crates/lys-home/src/record/compactions.rs
- create: crates/lys-home/src/record/compactions_tests.rs
- create: crates/lys-home/src/cli/compactions.rs
- create: crates/lys-home/tests/compactions_cli.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C79 — lys-home compactions prints one JSON report of a session's compactions, each with its loss entry, the span's ids, counts and hashes and a check that every span entry is readable by id and every named block is held, and exits 0 only when nothing is missing and 1 when anything is, naming the first missing item; a compaction with no loss entry or pointing at itself is reported unaccounted by entry id.
- C82 — A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, with no compaction and no lys.loss entry, and lys-home compactions reports it by entry id as a boundary without a summary.

**Stories:**
- S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.

### R6: Render a compaction as the compact_boundary and isCompactSummary pair

In render.rs, replacing the legacy summary line, and defining render-uuid/v2 alongside render-uuid/v1 (ADR-016 obeyed, not amended): v2 carries v1's lys namespace, session namespace rule, name form `<entry id>#<role>` and role `record` unchanged, byte for byte in derivation, and adds the one role `compact_boundary`; v1's roles stay closed at `record`. The render report gains a field uuid_scheme naming the version the render used, `render-uuid/v1` or `render-uuid/v2`. WHEN the context path being rendered holds no compaction entry, THE SYSTEM SHALL render under v1, write the same bytes it wrote before this brief, and name `render-uuid/v1` in the report. WHEN the context path holds a compaction entry, THE SYSTEM SHALL render under v2 and name `render-uuid/v2` in the report. WHEN the Claude Code render meets a compaction entry on the context path, THE SYSTEM SHALL write a system record with subtype compact_boundary, content `Conversation compacted`, level info, parentUuid null, logicalParentUuid null and compactMetadata {preTokens: the compaction's tokensBefore}, whose uuid is v2's derivation for the role compact_boundary, the UUID version 5 over the session's namespace and the name `<compaction id>#compact_boundary`, then a user record with isCompactSummary true and isVisibleInTranscriptOnly true whose message is {role user, content: the compaction's summary}, whose uuid is the compaction's id as record_uuid derives it and whose parentUuid is the boundary record's uuid, each carrying the fields every rendered record carries with the compaction's timestamp; the next rendered record's parentUuid SHALL be that summary record's uuid. THE SYSTEM SHALL NOT add a role to render-uuid/v1, SHALL NOT change v1's namespace, session namespace rule, name form or derivation, SHALL NOT derive the uuid of any record but the compact_boundary record differently from v1, SHALL NOT write a record of type summary, SHALL NOT write a lys.loss line or any other custom entry, SHALL NOT change the derived uuid of any role-record entry, and SHALL NOT read a clock or a random source.

**Acceptance:**
- Rendering the imported fixture session `compacted` with its head at cccccccc-cccc-4ccc-8ccc-cccccccccccc writes, in order, a record of type system and subtype compact_boundary whose uuid is 4e12f7de-6b53-55f9-a825-7f1f9254c8a6 (v2's compact_boundary role, computed independently with Python's uuid.uuid5 of uuid.uuid5(32c05904-d1f1-550c-9eee-2f6c8f98b665, 'compacted') and '88888888-8888-4888-8888-888888888888#compact_boundary'), then a user record with isCompactSummary true, uuid 88888888-8888-4888-8888-888888888888 and parentUuid 4e12f7de-6b53-55f9-a825-7f1f9254c8a6, then the records for 55555555-5555-4555-8555-555555555555, 66666666-6666-4666-8666-666666666666, 99999999-9999-4999-8999-999999999999, aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa and cccccccc-cccc-4ccc-8ccc-cccccccccccc, the first of which has parentUuid 88888888-8888-4888-8888-888888888888.
- Rendering the imported fixture at its head writes exactly three records: a compact_boundary system record, an isCompactSummary user record whose parentUuid is the boundary's uuid, and the record for dddddddd-dddd-4ddd-8ddd-dddddddddddd whose parentUuid is the summary record's uuid.
- Rendering the imported copy of the fixture holding lines 1 to 8 with line 7's compactMetadata.preservedMessages removed, at its head, writes exactly two records: a compact_boundary system record and an isCompactSummary user record whose uuid is 88888888-8888-4888-8888-888888888888 and whose parentUuid is the boundary's uuid.
- In both rendered files, no line has type summary and no line contains `lys.loss`.
- In each rendered file, the count of lines whose message content text has the SHA-256 of the compaction's summary is exactly 1, counted by hash.
- Rendering the imported fixture twice at the same head to two paths gives two files of equal SHA-256.
- The existing tests in render_tests.rs and the pinned hash of tests/claude_code_round_trip.rs pass unchanged.
- The render report for the imported fixture session `compacted` at cccccccc-cccc-4ccc-8ccc-cccccccccccc has uuid_scheme `render-uuid/v2`, and every rendered record other than the compact_boundary record has a uuid equal to record_uuid("compacted", its entry id), the v1 derivation.
- Rendering the multi-result fixture, a session with no compaction, through `run` writes a file whose SHA-256 equals the constant pinned in tests/claude_code_round_trip.rs and PROOF-RESUME.md, and its render report has uuid_scheme `render-uuid/v1`.
- A unit test asserts render-uuid/v1's role set is exactly [record] and render-uuid/v2's role set is exactly [record, compact_boundary].

**Files:**
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/render_tests.rs

**Checklist:**
- C80 — The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record, then the kept entries, with no legacy summary line and no lys.loss line, and the summary text once; the boundary record's uuid is derived under render-uuid/v2, alongside v1, whose non-boundary uuids equal v1's, and a render with no compaction stays v1, byte for byte, with the version it used named in its report.

**Stories:**
- S34 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

### R7: Prove it on a real compacted session and a resume on Claude Code 2.1.283, and write it down

Add docs/design/home/PROOF-COMPACTION.md with two sections, carrying hashes, counts, ids, versions and commands only. The first: one Claude Code session file holding at least one compact_boundary record followed by its isCompactSummary record, taken from the Claude Code projects directory of the machine the proof runs on and never written; its SHA-256 before the import and after the listing; its count of compact_boundary records; the `claude --version` output of that machine; the lys-home commit built from; the import command into a scratch home outside the Claude Code configuration directory; and the `lys-home compactions` report for the imported session, with every loss's counts and hashes and its check. The second: the compaction fixture imported and rendered at its head, resumed with `claude -p --resume <rendered file> --fork-session` and a prompt asking for the code word the summary holds, from a directory that is neither the rendered file's directory nor the session's cwd, on a machine where `claude --version` prints 2.1.283, with that output, the exit status, the rendered file's SHA-256, and the SHA-256 of the answer with surrounding whitespace trimmed beside the SHA-256 of the code word. Write the lys.loss entry, both compaction shapes, the empty firstKeptEntryId with its null first_kept, and the context path into RECORD.md, name the subcommand in crates/lys-home/README.md, and regenerate the cluster's rendered markdown and briefs/HOME-008.md with render-cluster.py and render-brief.py. THE SYSTEM SHALL NOT write anything under the Claude Code projects directory, SHALL NOT commit the real session or its home, and SHALL NOT put any line of transcript or summary text in the proof document.

**Acceptance:**
- PROOF-COMPACTION.md records the real source file's SHA-256 twice, before the import and after the listing, and the two are equal.
- PROOF-COMPACTION.md records a compact_boundary count of at least 1, the count of boundaries the listing reports without a summary, and a listing report whose command exited 0 and in which every compaction has status accounted, entries_missing [] and blocks_missing [].
- PROOF-COMPACTION.md records `claude --version` output of 2.1.283 beside the resume command, the resume's exit status 0, and an answer SHA-256 equal to the code word's SHA-256.
- A search of PROOF-COMPACTION.md for the fixture's code word and for each fixture record's text finds nothing.
- RECORD.md contains the strings `lys.loss`, `compact_boundary`, `isCompactSummary`, `preservedMessages.uuids`, `leafUuid` and `first_kept`, and a sentence stating that an empty firstKeptEntryId means nothing is kept.
- crates/lys-home/README.md contains `lys-home compactions`.
- `sh scripts/design/gate.sh` exits 0, so DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-008.md are what their JSON renders to.

**Files:**
- create: docs/design/home/PROOF-COMPACTION.md
- create: docs/design/home/briefs/HOME-008.md
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C81 — PROOF-COMPACTION.md records one real compact_boundary session imported read-only with its listing as counts and hashes and its source SHA-256 equal before and after, and a rendered compacted fixture resumed on Claude Code 2.1.283 answering from the summary, by hashes only.

**Stories:**
- S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.
- S34 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

## Boundaries

- No byte of a source Claude Code file is changed and nothing is written under the Claude Code projects directory (CN1).
- No summarised entry is removed, rewritten or re-parented, no block is deleted, and no existing home is rewritten (P1).
- No field is added to Pi's compaction entry or any other Pi entry; lys data lives only in custom entries (P2, CN4).
- No message, tool or summary text appears in a lys.loss entry, the listing's output, an error, a log line, a test name or the proof document (P7, CN3).
- No change to the derived uuid of any role-record entry or to CN9's byte determinism; render-uuid/v1 keeps its closed role set `record`, and the role compact_boundary exists only in render-uuid/v2, alongside it (ADR-016, ADR-030).
- The render never writes a lys.loss line, a lantern or any other custom entry, and never writes a legacy summary line.
- No compaction is produced by the home, no translation and no handover work, and no harness other than Claude Code.
- A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, and the home never invents a summary; the empty string is the only firstKeptEntryId that names no entry, and it means nothing is kept.
- No file exceeds 500 code lines, and no unwrap, expect or panic in library code.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists the tests of R1 to R6 as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- python3 -c "import uuid;ns=uuid.uuid5(uuid.UUID('32c05904-d1f1-550c-9eee-2f6c8f98b665'),'compacted');print(uuid.uuid5(ns,'88888888-8888-4888-8888-888888888888#compact_boundary'))" prints 4e12f7de-6b53-55f9-a825-7f1f9254c8a6, the render-uuid/v2 compact_boundary uuid R6's test asserts.
- Import crates/lys-home/tests/fixtures/compaction.jsonl into a scratch home with lys-home import, run lys-home compactions on it, and check it exits 0 and the report lists two accounted compactions with empty entries_missing and blocks_missing and first_missing null.
- grep -rn 'anchorUuid' crates/lys-home/src prints no line that reads it to find a first kept entry.
- git diff of the landed change leaves crates/lys-home/tests/claude_code_round_trip.rs unchanged.

## Amendments

### Amendment 1: The importer did not already map a compaction

- **Date:** 2026-09-27
- **By:** the lead of the home card, answering the compaction survey

The words' clause that the importer already turns the summary record into Pi's compaction entry with its summary and first kept entry is wrong: the importer pointed the compaction at itself with tokensBefore 0 and did not read compact_boundary at all. This brief replaces that mapping. The words are kept as they were typed in the roadmap row's provenance.

### Amendment 2: A boundary without a preserved list or without a summary, and the listing's exit status

- **Date:** 2026-09-27
- **By:** the lead of the home card, answering the author's questions

With no non-empty preserved list, the first kept entry is the first entry whose parent is the isCompactSummary record, now the compaction entry; a compaction never names itself. A compact_boundary with no isCompactSummary record under it is not a compaction: it imports as a lys.harness_event as today, with no loss entry, and the listing reports it separately as a boundary without a summary by entry id. The listing prints its full report either way, exits 0 only when every check passes and 1 when any entry or block is missing, naming the first missing one in the report; a refusal to run keeps its own named error and exit status.

### Amendment 3: How a compaction that keeps nothing is written

- **Date:** 2026-09-27
- **By:** the lead of the home card, answering the author's questions

Pi's type is kept: a compaction that keeps nothing writes firstKeptEntryId as the empty string, never JSON null, and the home's reader turns it into a typed nothing-kept at the one place it parses the field; a non-empty string naming no entry stays refused by name, so the empty string is the only sentinel. The lys.loss data carries first_kept as its own key, JSON null in that case and the entry id otherwise, and RECORD.md documents both. Such a pair imports as a compaction entry followed by its loss entry, its context path is the compaction alone, the listing reports it among compactions, and the render writes the pair back.

### Amendment 4: The compact_boundary uuid comes from render-uuid/v2, alongside v1

- **Date:** 2026-09-27
- **By:** the lead of lys, answering the author's question on the boundary uuid's role

ADR-016 is obeyed, not amended: render-uuid/v1's role list is closed at record, and a verifier of v1 must be able to name every role it meets, so compact_boundary cannot join v1. R6 defines render-uuid/v2, which carries v1's roles unchanged, byte for byte in derivation, and adds compact_boundary; the solution's role list names v2 beside v1. A render records in its report which version it used, and a file rendered under v1 keeps verifying under v1. A session with no compaction renders under v1 to the same bytes as before this brief; a session with a boundary renders under v2, and every derived uuid of a non-boundary record equals what v1 derives.

### Amendment 5: Superseded by HOME-030

- **Date:** 2026-09-28
- **By:** Waffles, from the requirement-by-requirement audit of main at d41fa4b by Apollo

The compaction import is built once, to HOME-030, the later design of the same work; HOME-008 is superseded and is not built.
