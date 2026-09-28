---
type: brief
id: HOME-020
cluster: home
title: Remove the work lys-home repeats on every call as a session grows, without changing a recorded byte
---

# HOME-020: Remove the work lys-home repeats on every call as a session grows, without changing a recorded byte

> **Cluster:** home
> **Design anchor:**
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-014 — A lantern is a custom entry in its session, and its note grows only by epilogue entries — A lantern is a `lys.lantern` custom entry appended at its session's head, carrying in custom.data the entry id of its point (an existing entry of the same session that is not itself a lantern or an epilogue, the head or any entry the head has moved past), the note as written, who lit it and when. Its note grows only by `lys.lantern_epilogue` custom entries naming the lantern's entry id and carrying the further words, who added them and when; a lantern's story is its entry followed by its epilogues in order, and nothing is rewritten. Rejected: Pi's `label` entry on the target (it replaces or clears a label rather than growing one, and carries no author or time), a lantern store beside the session outside Pi's grammar (a lantern would stop travelling with its session), and editing the lantern's note in place (the record is append-only, P1).
> - ADR-016 — A rendered file's derived uuids are a fixed, versioned contract — Derive every uuid a render needs and the record does not hold as UUIDv5 under the session's namespace and the name `<entry id>#<role>`, with a closed set of roles (`record` first); the session's namespace is UUIDv5 over one fixed lys namespace (32c05904-d1f1-550c-9eee-2f6c8f98b665, itself UUIDv5 of the RFC 9562 URL namespace over `lys/home/claude-code/render-uuid/v1`) and the id of the session being rendered, so the same entry id in two sessions never derives one uuid. Treat the namespace, the session namespace rule, the name form and the roles as frozen: a change is a new version alongside, never a mutation. The fork and launch cards derive by this scheme. Rejected: drawing fresh ids (nondeterministic), keying on a hash of the entry id alone without a namespace, a role and a session (two roles on one entry collide, two sessions with one hand-authored id collide, and it is not reproducible with standard UUID tooling), mixing in the target session id (the record does not hold it), and leaving the scheme mutable until a later card (every recorded hash would move with it).
> - ADR-017 — A fork is a child session cut from the parent's own lines at a lantern's point, with its ancestry on both sides — A fork resolves a lantern to the session it was lit in, read from the lys.lantern data's lit_in when the record carries it and otherwise by the older-record rule (one holder cuts, several refuse lantern_ambiguous until a session is named), and cuts that session's root-to-point chain at the last assistant message at or before the point, through the index. The child is a new session under the parent's cwd whose header's parentSession is the parent file's path relative to the home, holding each cut entry as the parent file's own line bytes, then one lys.forked_from custom entry as its head naming the parent session, the lantern, the point, the cut entry, whether the coordinate was carried and the carried entry; the parent gains one lys.fork custom entry at its head naming the child. Nothing else is copied and no block is written. Rejected: re-serialising the copied entries (the copy would stop hash-matching the parent's lines), a fork store beside the sessions outside Pi's grammar, cutting at a point no lantern names, and a header field beyond Pi's parentSession.
> - ADR-108 — A Claude Code import builds its session under a staging name and publishes it once — The import command builds its new session under the staging name `<id>.jsonl.importing` beside `sessions/<id>.jsonl`, holding `<id>.lock` throughout: the header and every entry line are written to the staging file with no sync, and the index rows and the head are kept in memory. It then publishes once, in this order: one sync of the staging file; the index written whole to `<id>.index.jsonl` through a temporary file, synced and renamed; the head written once to `<id>.head` through a temporary file, synced and renamed; one sync of the sessions directory; the staging file renamed to `<id>.jsonl`; one more sync of the sessions directory. A crash before the rename leaves no session under the final name, so a re-import is not refused as Exists; the next import of that id and the next open of that session file, each holding the lock, remove the staging file and, when `<id>.jsonl` is absent, the `<id>.index.jsonl` and `<id>.head` a part-way publish left. Rejected: appending in place and leaving a session that refuses by name after a crash; extending reconcile to trim an unsynced tail.
> **Checklist:**
> - C45 — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.
> - C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.
> - C47 — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.
> - C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.
> - C49 — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.
> - C50 — The canon keeps the id set load builds, and adding an example refuses a repeated id by that set, never by walking the loaded entries.
> - C51 — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.
> - C52 — context_path moves entries out of the path it read, customs reads only the path's entries of the custom type asked for, and the render's assistant arm and the importer's assistant content iterate a content array by reference, with no rendered or recorded byte changed.
> - C53 — One function, uuid_string, writes every uuid's 8-4-4-4-12 form: the fewshot's ids are 16 random bytes with the version nibble 4 and the variant nibble 8 set, formatted by it, and every render hash pinned in the tree is unchanged.
> - C54 — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.
> **Stories:**
> - S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
> - S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.
> - S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.
> - S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

## Purpose

lys-home repeats work on every call that grows with the session's length, and one open hides a real I/O error as a stale cache that it rebuilds and rewrites. This brief removes each repetition without changing what is recorded: a call-id map built once per open, a staged import that syncs its line file, its index and its head once each and publishes by rename (ADR-108), one pass over resume_check's ids, the canon's kept id set, an open bounded under CN7 that returns a real I/O error as that error, the clones gone, and one uuid formatter. Counters on the read and sync paths prove the bounds, a test that makes the read fail proves the error, and a masked hash of an import's line bytes proves the record unchanged.

## Task

The targets, at main's lines (fa3dd53, the same in lys-home at a426b9a): find_call (crates/lys-home/src/record/call.rs:332), used by ingest_call (:355), ingest_call_files (:404) and ingest_outcome (:455), reads and deserialises every lys.call entry on every ingest; import_claude_code (src/harness/claude_code/import.rs) appends each entry through append_entry (:135, :170, :195, :224, :239, :277 and :354), four syncs and a rename each, then moves the head once (:319); resume_check (src/cli.rs:501) counts every id of both transcripts once per distinct id; canon's append_example (src/record/canon.rs:364) walks every loaded entry per new one though load built the id set (:150); from_cached (src/record/index.rs:129) seeks and reads one byte per row on every open and its .ok()? turns open, seek and read failures into a stale verdict; context_path clones the entries of a path it owns (src/record/mod.rs:524, :527) and customs (:532) reads the whole path to filter it though index rows carry custom; the render's assistant arm (src/harness/claude_code/render.rs:176-180) and assistant_content (import.rs:441) clone whole content arrays to iterate them; uuid_shaped (cli.rs:430) repeats uuid_string's format (render.rs:324). The render's toolResult arm (render.rs:160) keeps its clone, which the output holds. Build R1's counters first; every bound is measured with them and never by time. The call-id map bounds the reads of an owner that keeps one Session open across calls; the CLI's ingest-call opens the session per call and still reads every lys.call entry once per open, which is its cost after this brief, and index rows carry no call id (a finding for the lys-proxy card, not this one). Only the import command's new session is staged; import_claude_code given an already-open session, and every other append, keeps per-append durability. The sync bound counts the session's own syncs; the block store's two per new block are counted beside it and unchanged. The open reads at most three rows by position, so CN7 holds; a row that is not the file's at a position not checked is refused when it is read, as StaleIndex or Malformed by the read's existing check, and no read rebuilds or retries. The import-bytes proof masks the importer's fresh ids and the header timestamp and pins the parent commit's hash; import determinism stays a non-goal. HOME-013 (open) moves Session out of record/mod.rs; this brief is written against main and whichever lands second redoes its record/mod.rs edits. New logic goes in named files under record (io_counts.rs, call_map.rs, staged.rs, reads.rs), none in record/mod.rs: it gains only module declarations, Session's new fields and the calls into them, it loses Session's reads to reads.rs, and its count of code lines does not rise above the parent commit's. The three test files the requirements extend, record_tests.rs, call_tests.rs and tests/cached_index.rs, were created by HOME-001's commit and stay its files. Out of scope: the record format, the index layout, any new command or flag, import determinism, the block store's syncs, a persisted call-id lookup, and recovering a torn tail.

## Requirements

### R1: Count the entries a session reads and the syncs it and the block store make

Structural, with its behaviour stated. A new file crates/lys-home/src/record/io_counts.rs defines IoCounts, a public Copy value with two u64 fields, entries_read and syncs, and Session exposes its own as Session::io_counts(). A session opened with Session::open starts at zero; one made by Session::create holds the syncs create made. entries_read rises by one for every entry the Session deserialises from its file: through entry, path, context_path, customs, customs_everywhere and the call map's build (R2). syncs rises by one for every sync_all of the session file, its index file and its head file, and every sync of the sessions directory, that the Session's own writes make. BlockStore gains BlockStore::syncs(), rising by one for every sync_all of a block file and every directory sync that put and put_file make. The counters are held by the value that owns them, through atomics, so a read through &self counts. THE SYSTEM SHALL NOT use a process-wide counter, SHALL NOT measure time, and SHALL NOT change what any read or write does, which syncs happen, or their order; SessionReader gains no counter.

**Acceptance:**
- A session made by Home::create_session reports io_counts() equal to IoCounts { entries_read: 0, syncs: 4 }: the session file, the sessions directory, the head file and the sessions directory.
- After three Session::append calls on that session, io_counts().syncs is 16.
- After Session::path() on that session, with the head at the third entry, io_counts().entries_read is 3; after a following Session::entry of the first entry's id it is 4.
- Session::open of that session, with its index and head current, reports IoCounts { entries_read: 0, syncs: 0 }.
- A BlockStore on an empty home reports syncs() 0; after put(b"one") and put(b"two") it reports 4; after put(b"one") again it still reports 4.

**Files:**
- create: crates/lys-home/src/record/io_counts.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/record_tests.rs

**Checklist:**
- C46 — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.

**Stories:**
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

### R2: Find a recorded call by id through a map built once per open

WHEN find_call is first asked for a call id on an open Session, THE SYSTEM SHALL build, once, a map from call id to entry id by reading every lys.call row of the index in file order, whatever its branch and wherever the head stands, and keeping for each call id the first entry that holds it; it SHALL answer this and every later find_call from that map. WHEN any append on the Session writes a lys.call entry (append, append_entry, append_beside, append_under), THE SYSTEM SHALL add its call id to a built map unless the map already holds that id. WHEN the Session reconciles from its file, THE SYSTEM SHALL drop the map, so the next find_call builds it again from the reloaded index. The map, its build and its update live in crates/lys-home/src/record/call_map.rs and are held on Session in one field of type std::sync::Mutex<Option<HashMap<String, String>>>, None until the first find_call and set back to None by a reconcile, so find_call keeps taking &Session and Session stays Send and Sync. THE SYSTEM SHALL NOT hold the map, or any other field this brief adds to Session, in a RefCell, a Cell, an Rc or any other type that is not Sync, and SHALL NOT change the signature of find_call or of any public method of Session. All three ingest paths (ingest_call, ingest_call_files and ingest_outcome) SHALL look up through find_call. THE SYSTEM SHALL NOT read a lys.call entry for a lookup once the map is built, SHALL NOT change what an ingest records, its idempotency, or the entry id a repeated call id returns, and SHALL NOT put a call id in the index or persist the map. The bound is for an owner that keeps one Session open across calls: the CLI's ingest-call opens the session for every call, so it still reads every lys.call entry once per open, when the map is built, and that is its cost after this brief.

**Acceptance:**
- A test in call_tests.rs ingests 100 calls with call ids c000 to c099 through ingest_call on one Session, drops it, reopens it with Session::open and ingests c100 through ingest_call: io_counts().entries_read rises by exactly 100 across that ingest.
- On the same open Session, each of ten further ingests leaves io_counts().entries_read unchanged: c101 to c105 through ingest_call, repeated c000 and c001 through ingest_call, repeated c002 and c003 through ingest_call_files, and repeated c004 through ingest_outcome.
- Each of the five repeated ingests returns the entry id first recorded for its call id, and the session file's length is the same before and after it.
- On a Session whose map is built, a lys.call entry for call id beside appended with append_beside (the head not moved) is found by a following ingest_call of beside, which returns that entry's id and leaves the session file's length unchanged.
- The seven existing tests in call_tests.rs pass unchanged, a_call_is_found_after_a_crash_before_the_head_advanced_and_after_the_head_moved among them.
- call_tests.rs holds a test that calls a generic function bounded by Send + Sync with Session as its type argument, and cargo test -p lys-home --all-features compiles and passes it; the same test compiles and passes when added to call_tests.rs at the parent commit.
- grep -cE 'Cell<|\bRc<' crates/lys-home/src/record/call_map.rs crates/lys-home/src/record/mod.rs crates/lys-home/src/record/io_counts.rs prints each of the three paths followed by :0.

**Files:**
- create: crates/lys-home/src/record/call_map.rs
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/call_tests.rs

**Checklist:**
- C45 — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

### R3: Stop cloning and reading what is already owned

THE SYSTEM SHALL move Session::entry, Session::path, Session::context_path, Session::customs and Session::customs_everywhere out of record/mod.rs into crates/lys-home/src/record/reads.rs, as an impl block of Session with the same names, receivers and return types. THE SYSTEM SHALL build Session::context_path's result by moving entries out of the path it read (mod.rs lines 524 and 527 as main stands), with no Entry clone. THE SYSTEM SHALL have Session::customs (mod.rs line 532) take the path's index rows, keep those whose custom type is the one asked for, and read only those entries, in root-first order. THE SYSTEM SHALL have the Claude Code render's assistant arm (render.rs lines 176 to 180) and the importer's assistant_content (import.rs line 441) iterate the message's content array by reference, cloning a part only where the output holds it. THE SYSTEM SHALL NOT change which entries these return or their order, SHALL NOT change any rendered or recorded byte, and SHALL NOT change the render's toolResult arm (render.rs line 160), whose clone the output holds.

**Acceptance:**
- A test in record_tests.rs on a path of five entries, two of them lys.authored custom entries, gets both from customs("lys.authored") in root-first order, and io_counts().entries_read rises by exactly 2.
- A test in record_tests.rs on a path e1, e2, e3, e4, then a compaction e5 whose first kept entry is e3, then e6 gets from context_path the ids [e5, e3, e4, e6].
- sed -n '/pub fn context_path/,/^    }/p' crates/lys-home/src/record/reads.rs | grep -c 'clone' prints 0.
- sed -n '/^fn assistant_content/,/^}/p' crates/lys-home/src/harness/claude_code/import.rs | grep -c 'a.clone()' prints 0.
- The render_tests and tests/claude_code_round_trip.rs pass unchanged, the_fixture_rendered_through_run_hashes_to_the_constant_pinned_here_and_in_proof_resume among them.

**Files:**
- create: crates/lys-home/src/record/reads.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/record/record_tests.rs

**Checklist:**
- C52 — context_path moves entries out of the path it read, customs reads only the path's entries of the custom type asked for, and the render's assistant arm and the importer's assistant content iterate a content array by reference, with no rendered or recorded byte changed.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R4: Stage an import under a name beside its session and publish it once

WHEN the import command imports a transcript into a new session `<id>`, THE SYSTEM SHALL take the lock `<id>.lock`, refuse with Exists when `sessions/<id>.jsonl` exists, remove any remainder of an earlier part-way import of `<id>` (below), write the header to the staging file `sessions/<id>.jsonl.importing`, and append every imported entry to it with one write per line and no sync, keeping the index rows and the head in memory. It SHALL then publish, in this order: one sync of the staging file; the index rows written whole to `<id>.index.jsonl` through a temporary file, synced and renamed; the head written once to `<id>.head` through a temporary file, synced and renamed; one sync of the sessions directory; the staging file renamed to `<id>.jsonl`; one more sync of the sessions directory: five syncs of the session's own, whatever the record count. The staged session's lines, and the ImportReport, are what import_claude_code writes and reports today. IF the import fails or the process ends before the rename, THEN no `<id>.jsonl` exists; the next import of `<id>` and the next Session::open of `<id>.jsonl`, each after taking the lock, SHALL remove `<id>.jsonl.importing` when it is present and, when `<id>.jsonl` is absent, `<id>.index.jsonl` and `<id>.head`, and sync the sessions directory after any removal. docs/design/home/RECORD.md's section on what lys keeps beside the file SHALL state this rule beside the per-append durability order, as ADR-108 records it. THE SYSTEM SHALL NOT sync, write the index file or write the head per entry during a staged import; SHALL NOT list a staging file as a session; SHALL NOT extend reconcile or trim, rewrite or truncate a session file; SHALL NOT change the per-entry durability of import_claude_code given a session that is already open, or of any other append; and SHALL NOT change the block store's syncs, which stay outside the bound and are counted beside it.

**Acceptance:**
- A test in staged_tests.rs imports tests/fixtures/multi_result.jsonl (6 records) through the staged import and publish: the published session's io_counts().syncs is 5, and BlockStore::syncs() equals 2 times the report's blocks_new, both printed by the test for the record.
- The same test over a transcript of 300 records it writes itself (150 user and 150 assistant text records in one parent chain) gives io_counts().syncs 5.
- A staged import dropped after import_claude_code returns and before publish leaves sessions/<id>.jsonl absent, sessions/<id>.jsonl.importing present, and Home::session_ids() without <id>; the import command then run for the same id and transcript returns Ok, leaves no sessions/<id>.jsonl.importing, and the session it publishes opens with Session::open holding as many entries as its report's entries field.
- With sessions/<id>.jsonl.importing, sessions/<id>.index.jsonl and sessions/<id>.head written by the test and no sessions/<id>.jsonl, Session::open of sessions/<id>.jsonl returns Err(HomeError::Io { .. }) and afterwards none of the three files exists.
- The import command's report for multi_result.jsonl names the file sessions/multi.jsonl under the home.
- The existing import_tests, render_tests and the four tests in tests/claude_code_round_trip.rs pass unchanged, the continuation import into an already-open session among them.
- grep -q 'jsonl.importing' docs/design/home/RECORD.md && echo stated prints stated.

**Files:**
- create: crates/lys-home/src/record/staged.rs
- create: crates/lys-home/src/record/staged_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C47 — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.
- C48 — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.

**Stories:**
- S25 (Developer, Works on lys-home's code beside the record module) — As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

### R5: Count resume_check's tool-use ids in one pass

WHEN resume_check (cli.rs line 501 as main stands) compares a rendered and a forked transcript, THE SYSTEM SHALL count each transcript's tool_use ids in one pass over its records into a map from id to count, and SHALL report repeated_tool_use_ids as the sum, over the distinct ids of the rendered transcript, of the fork's count less the rendered count where that is positive. THE SYSTEM SHALL NOT scan a transcript once per id, and SHALL NOT change any ResumeReport field or the value any field takes.

**Acceptance:**
- resume_check_refuses_a_fork_that_repeats_a_tool_action in tests/claude_code_round_trip.rs passes unchanged.
- A new test in tests/claude_code_round_trip.rs runs the resume-check command over a rendered file of two records whose tool_use ids are [a, b] and [a] and a fork of four records, those two with the same uuids then two new ones with ids [a, b] and [c], and gets rendered_records 2, forked_records 4, forked_new_records 2, repeated_tool_use_ids 2 and new_tool_uses 3.

**Files:**
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/tests/claude_code_round_trip.rs

**Checklist:**
- C49 — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

### R6: Open from the cached index with a bounded number of reads and return a real I/O error as that error

WHEN Index::read finds a cached index beside the session file, THE SYSTEM SHALL check every row in memory (it starts where the row before it ended, the first at the header's end; its len is not zero; its offset plus len does not overflow; its id is not already indexed; its parent, when it has one, is already indexed) and that the last row ends at the session file's length, and SHALL then read, through one buffered reader over the session file opened once, the final byte of the rows at positions 0, n/2 (integer division) and n-1 of the n rows, each distinct position once and in ascending order, with one seek each: at most three one-byte reads after the header whatever n is, and each byte must be a newline. from_cached takes the reader as a parameter of any type that is Read and Seek and returns Result<Option<Index>, HomeError>. IF an in-memory check fails, a checked byte is not a newline, or a read ends early with an unexpected end of file, THEN THE SYSTEM SHALL treat the cache as stale and rebuild from the session file as it does today. IF opening, seeking or reading the session file fails with any other error, THEN THE SYSTEM SHALL return HomeError::Io naming the operation and the session file from Index::read, and so from Index::load, Session::open and SessionReader::open, and SHALL NOT rebuild the index or write any file. THE SYSTEM SHALL NOT read a byte of any other row on a cache hit, SHALL NOT read the whole session file on a cache hit, and SHALL NOT change the index file's layout or the header read. A row whose boundaries sit inside the file's lines at a position that is not checked is no longer caught at open; a read of it is refused by Index::read_rows_from as StaleIndex (the entry's id differs from its row) or as Malformed (the bytes at the row do not parse as an entry), as it is today. THE SYSTEM SHALL NOT add a rebuild of the index or a second attempt to any read after open, and SHALL NOT change what a read returns when the cached index is this file's. Session::entry and Session::path keep taking &self, and neither public signature changes, nor those of context_path, customs and customs_everywhere: the read counters of R1 are AtomicU64 fields, so a read through &self counts and a Session stays Send and Sync, and this requirement adds no field to Session. The bounded check lives in index.rs; record/mod.rs gains none of it.

**Acceptance:**
- A test in record_tests.rs builds a session of 10 entries, reads its cached rows, and calls from_cached with a reader over the session file's bytes that records every seek it is asked for: it returns Ok(Some(index)) holding 10 rows, and the recorded seeks are exactly three, to the final-byte offsets of rows 0, 5 and 9, in that order.
- The same test over sessions of 1 and 2 entries records exactly one seek (row 0) and exactly two seeks (rows 0 and 1).
- from_cached over the 10 rows with a reader whose every read returns an io::Error of kind Other returns Err(HomeError::Io { .. }) whose source has kind Other.
- from_cached over the 10 rows with a reader whose every seek returns an io::Error of kind PermissionDenied returns Err(HomeError::Io { .. }) whose source has kind PermissionDenied.
- from_cached over the 10 rows with a reader whose every read returns an io::Error of kind UnexpectedEof returns Ok(None).
- The four tests in tests/cached_index.rs pass unchanged: the untouched index opens with index_was_rebuilt() false, and the self-parent row, the row naming a parent not yet indexed, and the rows shifted by one byte each open with index_was_rebuilt() true.
- A new test in tests/cached_index.rs on a session of 10 entries whose last cached row claims one byte fewer than it holds opens with index_was_rebuilt() true and a path of 10 entries.
- grep -n '.ok()?' crates/lys-home/src/record/index.rs prints nothing.

**Files:**
- modify: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/record/record_tests.rs
- modify: crates/lys-home/tests/cached_index.rs

**Checklist:**
- C51 — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.

**Stories:**
- S24 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.
- S26 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.

### R7: Keep the canon's id set and refuse a repeat by it

Canon gains a private field holding the set of entry ids load builds (canon.rs line 150), kept when load returns, and a public Canon::contains(id). WHEN an example is added, THE SYSTEM SHALL refuse a new entry whose id the set holds, or which repeats an id earlier in the same addition, with DuplicateEntry naming the canon and the id, as it does today. THE SYSTEM SHALL NOT walk the loaded entries per new entry (canon.rs line 364), SHALL NOT change load's refusals or the canon file, and SHALL NOT change Canon's public fields.

**Acceptance:**
- grep -n 'entries.iter().any' crates/lys-home/src/record/canon.rs prints nothing.
- A test in canon_tests.rs loads a canon of three entries and gets Canon::contains true for each of their ids and false for absent.
- A test in canon_tests.rs adding an example whose entry id is already in that canon gets Err(HomeError::DuplicateEntry { .. }) naming the canon and that id, and the canon file's bytes are the same before and after.
- The four existing tests in canon_tests.rs pass unchanged.

**Files:**
- modify: crates/lys-home/src/record/canon.rs
- modify: crates/lys-home/src/record/canon_tests.rs

**Checklist:**
- C50 — The canon keeps the id set load builds, and adding an example refuses a repeated id by that set, never by walking the loaded entries.

**Stories:**
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

### R8: Write every uuid's hyphenated form through one function

THE SYSTEM SHALL write every uuid's 8-4-4-4-12 form through uuid_string in render.rs (line 324 as main stands). WHEN the fewshot command draws its session id or a record's uuid, THE SYSTEM SHALL draw 16 random bytes, set byte 6 to (byte & 0x0f) | 0x40 and byte 8 to (byte & 0x0f) | 0x80, and format them with uuid_string, so each id carries '4' at character 14 and '8' at character 19, counting from 0, as uuid_shaped's did. THE SYSTEM SHALL NOT keep uuid_shaped's own formatting (cli.rs line 430), SHALL NOT change uuid_string or anything it returns, and SHALL NOT change any rendered byte.

**Acceptance:**
- grep -n 'fn uuid_shaped' crates/lys-home/src/cli.rs prints nothing.
- grep -rn '"{}-{}-' crates/lys-home/src --include='*.rs' | grep -v '_tests.rs' prints exactly one line, in render.rs.
- A new test in tests/claude_code_round_trip.rs runs the fewshot command over six turns and asserts that each record's uuid and sessionId is 36 characters with '-' at characters 8, 13, 18 and 23, '4' at 14 and '8' at 19, and 32 lowercase hex digits once the dashes are removed.
- the_fixture_rendered_through_run_hashes_to_the_constant_pinned_here_and_in_proof_resume and the uuid_string tests in render_tests.rs pass unchanged.

**Files:**
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/tests/claude_code_round_trip.rs

**Checklist:**
- C53 — One function, uuid_string, writes every uuid's 8-4-4-4-12 form: the fewshot's ids are 16 random bytes with the version nibble 4 and the variant nibble 8 set, formatted by it, and every render hash pinned in the tree is unchanged.

**Stories:**
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

### R9: Prove an import's recorded bytes unchanged by a masked hash pinned at the parent commit

A new test file crates/lys-home/tests/import_bytes.rs, using only lys_home items present at the parent commit (cli::run with Command::Import), imports tests/fixtures/multi_result.jsonl into a fresh home as session multi through the import command, reads sessions/multi.jsonl, and masks it: the header line's timestamp value is replaced by the token T, and every run of exactly 32 lowercase hex digits with no hex digit on either side is replaced by # followed by its order of first appearance, counting from 0. It asserts that the SHA-256 of the masked bytes, in lowercase hex, equals a constant pinned in the file, which is the value the same file gives when run at the parent commit the change is built on. THE SYSTEM SHALL NOT change the importer's id draws or the header's timestamp to make the hash hold (import determinism stays a non-goal), SHALL NOT mask anything but those two, and SHALL NOT pin a value taken at the head alone.

**Acceptance:**
- cargo test -p lys-home --test import_bytes passes at the head.
- With crates/lys-home/tests/import_bytes.rs copied into a worktree of the parent commit, cargo test -p lys-home --test import_bytes passes there.
- The masking applied to {"id":"0123456789abcdef0123456789abcdef","parentId":"0123456789abcdef0123456789abcdef","x":"fedcba9876543210fedcba9876543210"} gives {"id":"#0","parentId":"#0","x":"#1"}, asserted in the same file.
- The masking leaves a 64-digit hex block hash and the hyphenated uuid 11111111-1111-4111-8111-111111111111 unchanged, asserted in the same file.
- Two imports of the fixture at the head into two fresh homes give different SHA-256 values of their raw line bytes and equal SHA-256 values of their masked bytes, asserted in the same file.

**Files:**
- create: crates/lys-home/tests/import_bytes.rs

**Checklist:**
- C54 — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.

**Stories:**
- S27 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

## Boundaries

- No byte of any recorded line changes: the session header's fields, every entry, lys.call, lys.harness_event and the ids the importer writes are as today.
- The index file's layout (IndexRow's id, parent, offset, len and custom and their serialisation), the head file's content and the names and places of the index, head and lock files do not change; the only new name is the staging file `<id>.jsonl.importing`.
- No new CLI command or flag, and no field added to any command's report.
- Per-append durability is unchanged for append, append_entry, append_beside, append_under, move_head, lanterns, epilogues, forks, call ingest and an import into an already-open session; only the import command's staged session batches.
- The block store's put and put_file write and sync as today; they are only counted.
- Render output and ADR-016's derived uuids are byte-identical; the render hash pinned in tests/claude_code_round_trip.rs and PROOF-RESUME.md does not change.
- The importer's fresh ids and the header's timestamp are drawn as today; import determinism is not attempted.
- Reconcile is not extended and no session file is trimmed, rewritten or truncated.
- No call id in the index and no call-id lookup persisted beside the file.
- No existing test is edited, removed or ignored; tests are only added.
- No unwrap, expect, panic, todo, unimplemented or unreachable in library code; no #[allow], #[ignore] or _-prefixed unused binding; no file over 500 lines of code.
- The design's structure array is the whole file list; a path outside it is not created.

## Verification

- From the repository root, cargo fmt --all exits 0, and git status --porcelain prints the same lines after it as before it.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- cargo test -p lys-home --all-features -- --list 2>/dev/null | grep ': test$' | sort, run in a worktree at fa3dd53 into before.txt and at the head into after.txt, then comm -23 before.txt after.txt prints nothing; every test so listed passes in the cargo test run above.
- With P=$(git merge-base HEAD origin/main), the parent commit the build is made on: git diff "$P" -- crates/lys-home/tests crates/lys-home/src/record/*_tests.rs crates/lys-home/src/harness/claude_code/*_tests.rs | grep -E '^-[^-]' prints nothing.
- git diff "$P" -- crates/lys-home/src/record/entries.rs crates/lys-home/tests/fixtures docs/design/home/PROOF-RESUME.md prints nothing.
- Copy crates/lys-home/tests/import_bytes.rs into a worktree at "$P" and run cargo test -p lys-home --test import_bytes there; it passes, as it does at the head.
- git show "$P":crates/lys-home/src/record/mod.rs | grep -cvE '^[[:space:]]*(//|$)' prints 472, and test "$(grep -cvE '^[[:space:]]*(//|$)' crates/lys-home/src/record/mod.rs)" -le 472 && echo not-grown prints not-grown.
- git diff "$P" -- crates/lys-home/src/record/mod.rs | grep -cE '^\+.*\bfn ' prints 0.
- grep -n '.ok()?' crates/lys-home/src/record/index.rs, grep -n 'fn uuid_shaped' crates/lys-home/src/cli.rs and grep -n 'entries.iter().any' crates/lys-home/src/record/canon.rs each print nothing.
