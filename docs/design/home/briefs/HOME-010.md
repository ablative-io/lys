---
type: brief
id: HOME-010
cluster: home
title: Canonicalise every lys.given path before it is recorded or compared
---

# HOME-010: Canonicalise every lys.given path before it is recorded or compared

> **Cluster:** home
> **Design anchor:**
> - ADR-012 — A compaction's loss is a lys.loss custom entry beside it, and a session's block hashes are a lys file beside the session — Each compaction entry is followed in the file by a lys.loss custom entry, a side leaf under the compaction, whose data names the summarised span's first and last entry ids, its counts of entries, messages, tool calls, tool results and blocks, their bytes, a digest of the span's block hashes and the harness's tokensBefore, deterministic so two imports agree apart from ids and timestamps. Block hashes are kept in <id>.blocks.jsonl beside the session, one {entry, part, hash} row per stored part, as the index and head are kept. Rejected: a new field on a Pi message or a custom entry per message for block references, which adds to Pi's grammar or doubles every import's entries; re-hashing parts or following only harness-event record hashes, which cannot find the stored blocks; and placing the loss entry on the chain, which would re-parent the record after the compaction and break the importer's parent equality.
> - ADR-013 — Claude Code's compaction is read and rendered in the shape the measured version writes — The importer reads a compact_boundary record and its isCompactSummary record as one Pi compaction entry: the summary is the isCompactSummary message's text, the first kept entry is the entry of preservedSegment.headUuid (the compaction itself when there is no preservedSegment, so it keeps nothing), tokensBefore is compactMetadata.preTokens, and Pi's details field names both source records by uuid; a named first kept entry not on record is refused by uuid. The summary record path stays for the file that carries one. The render writes a compact_boundary record, then the isCompactSummary record, then the kept records, with the parent chain advancing through all three, measured on the installed Claude Code version. Rejected: attaching the loss entry only to the summary record path, which almost no file uses, and keeping R4's summary line, which a resumed session would not read.
> - ADR-029 — What a compaction could not keep is a lys.loss custom entry of ids, counts and SHA-256s — Each compaction the importer writes is followed directly by one lys.loss custom entry, a side leaf whose parent is the compaction entry, whose data names the compaction by entry id, the first and last entry id of the span it summarised (the root-to-first-kept path entries, from the root or the previous compaction to the entry before the first kept one), the counts of that span's entries, messages, tool calls, tool results and blocks, its line bytes and block bytes counted separately, the counts of side-leaf and sidechain entries hanging from it by number only, the tokensBefore the harness reported (null when it reported none), a logicalParentUuid that names no record as unresolved, the SHA-256 of the span's first source line, of its last and of its lines concatenated in file order, the hash of the summary record's block and the span's distinct block hashes. Its keys are: compaction_id, first_kept (null when nothing is kept), span_first, span_last, entries, messages, tool_calls, tool_results, blocks, line_bytes, block_bytes, side_leaf_entries, sidechain_entries, tokens_before, unresolved_logical_parent, first_line_sha256, last_line_sha256, span_sha256, summary_record, block_hashes. Rejected: adding fields to Pi's compaction entry (it breaks CN4 and P2); hashing or listing side leaves and sidechain entries by id (their ids are drawn fresh on every import, so two imports of one file would disagree); and carrying any content, the summary included (P7).
> **Checklist:**
> - C71 — A compact_boundary record followed by its isCompactSummary user record imports as a lys.harness_event for the boundary and one Pi compaction entry under the summary record's uuid, the child of the boundary's entry, whose firstKeptEntryId is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and the empty string when there is none, so its context path is the compaction alone and the pair renders back, and whose tokensBefore is compactMetadata.preTokens; the summary record is not a message entry.
> - C72 — A legacy summary record imports as a Pi compaction entry whose firstKeptEntryId is the entry its leafUuid names, never the compaction's own id.
> - C73 — A compaction whose first kept entry is not on record is refused with an error naming that uuid.
> - C74 — Directly after each compaction entry the importer appends one lys.loss custom entry whose parent is the compaction and whose data holds only ids, counts, byte counts and SHA-256 hashes, with the keys RECORD.md sets down, first_kept among them, null when nothing is kept.
> - C75 — A lys.loss entry's span is the root-to-first-kept path from the root or the previous compaction to the entry before the first kept one; its three SHA-256s are over those entries' source lines in file order, and side-leaf and sidechain entries hanging from the span are counted by number only.
> - C76 — A compact_boundary whose logicalParentUuid names no record in the file imports, and its lys.loss entry names that uuid as unresolved.
> - C77 — Two imports of the compaction fixture into two homes give lys.loss lines equal byte for byte once id, parentId, timestamp and data.compaction_id are masked.
> **Stories:**
> - S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.
> - S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.

## Purpose

The lys.given context record (HOME-003) stores each document's path as the resolver built it from the render's --cwd and config directory, and given-check finds the listed document by comparing those paths as written. One file reached through a symlinked directory, through a path with `..` in it, or through a config directory written with a trailing slash therefore records as a different path, and given-check reports a difference that is not one. This brief canonicalises every absolute path before it is recorded or compared, keeps a path that does not exist as given and names it unresolved in the `given` and `given-check` reports, states the rule in RECORD.md, and proves the three shapes record and compare as one path with a fixture test (CN12, ADR-029).

## Task

Today crates/lys-home/src/harness/claude_code/given.rs builds every recorded path with Path::join from the config directory and from Path::new(cwd).ancestors(), and nothing in crates/lys-home/src canonicalises a path; crates/lys-home/src/cli/given.rs finds the listed document with `document.path == args.path`, its one comparison site. Rust's Path equality already ignores a trailing slash, but a PathBuf serialises with it, and `..` and symlinks are not normalised by it; with a `..` in --cwd the chain walk also visits directories the harness never reads. Add one helper module beside given.rs that canonicalises a given path by the rule ADR-029 fixes, use it in resolve_given (the working directory before the chain walk, the config directory, every absolute document) and in given-check (the listed path and the --path argument, and --file before it is read), and add an `unresolved` list to the `given` and `given-check` reports. The lys.given entry's data keeps exactly its six fields (ADR-013); nothing is added to it. The two written files (instructions.md and mcp.json) stay relative to the render's out directory and are exempt, for the reason RECORD.md already gives. A document path is built by join and cannot carry a trailing slash; only the config directory can, so the trailing-slash shape is proven on the config directory and nowhere else. The memory index's slug keeps taking the working directory as given (projects_slug is unchanged); the chain then walks the canonical working directory while the slug is computed from the one given, so the two may see different forms of one directory, and which form the slug should take is left to the slug rule's own requirement. Existing tests whose expectations are built from a temporary directory (a symlinked /var path on macOS) build them from that directory canonicalised, so the suite asserts the same paths on every platform. Out of scope: the slug rule, canonical paths in any other home record (fork's parentSession, the render manifest's files, render-launch --out), PROOF-GIVEN.md's measured paths, and any harness but Claude Code. Its ids are the next free after main's highest and every open brief branch's, checked against the remote's branch list: main holds up to HOME-007, RM-016, ADR-018, C44, S23 and CN10; brief/home/5898fcb0 holds HOME-008 and RM-011, and brief/home/9e701467 holds HOME-009, RM-012, C70, S31 and CN11; the highest brief-branch RM is RM-017 and ADR is ADR-028. So this brief is HOME-010, with RM-018, ADR-029, C71 to C77, S32, S33 and CN12.

## Requirements

### R1: Canonicalise one given path by the record's rule

Add a module given_path beside given.rs with one enum naming what became of a path: canonical, unresolved, or relative. WHEN it canonicalises a directory path, THE SYSTEM SHALL resolve the whole path, every symlink, `..` and trailing slash included, with std::fs::canonicalize. WHEN it canonicalises a document path, THE SYSTEM SHALL canonicalise the path's parent directory whole, join the path's own final component unchanged, and then read the joined path's own entry with std::fs::symlink_metadata, so a document that is itself a symlink is named at its link position and a dangling symlink counts as existing there; a path whose final component is not a normal name (`..`, or the root) SHALL be canonicalised whole as a directory path. WHEN the path is relative, THE SYSTEM SHALL return it unchanged, marked relative, and SHALL NOT resolve it against the process's working directory. IF a directory path cannot be canonicalised because it does not exist, or a document path's parent cannot be canonicalised because it does not exist, or a document path's own entry does not exist (canonicalize or symlink_metadata answering NotFound, or NotADirectory for a component that is a file), THEN THE SYSTEM SHALL return the path byte for byte as given, marked unresolved, and SHALL NOT drop it, SHALL NOT return an error, and SHALL NOT mark it canonical. IF canonicalize or symlink_metadata fails for any other reason (permission denied, a loop of symlinks), THEN THE SYSTEM SHALL refuse with HomeError::Io whose context is `canonicalising a given path` and whose path is the path given. THE SYSTEM SHALL NOT open or read any file's bytes, and SHALL NOT put anything but the path in an error.

**Acceptance:**
- Under a temporary directory canonicalised as `B`, with `B/real/w/CLAUDE.md` a file and `B/link` a symlink to `B/real`, canonicalising the document path `B/link/w/CLAUDE.md` gives canonical `B/real/w/CLAUDE.md`.
- Canonicalising the document path `B/real/w/../w/CLAUDE.md` gives canonical `B/real/w/CLAUDE.md`.
- Canonicalising the directory path written as the string `B/real/c/` (trailing slash, `B/real/c` a directory) gives canonical `B/real/c`, and serde_json serialises the result as the JSON string `"B/real/c"` with no trailing slash.
- With `B/real/w/CLAUDE.md` a symlink to `B/real/t/target.md`, canonicalising the document path `B/real/w/CLAUDE.md` gives canonical `B/real/w/CLAUDE.md`, not `B/real/t/target.md`.
- Canonicalising the directory path written as `B/gone/c/`, where `B/gone` does not exist, gives unresolved with the string `B/gone/c/` byte for byte.
- Canonicalising the document path `B/real/w/CLAUDE.md/x`, where `B/real/w/CLAUDE.md` is a file, gives unresolved `B/real/w/CLAUDE.md/x`.
- Canonicalising the document path `B/real/w/absent.md`, where `B/real/w` is a directory and `absent.md` does not exist in it, gives unresolved `B/real/w/absent.md`.
- With `B/real/w/dangling.md` a symlink to `B/real/t/none.md`, which does not exist, canonicalising the document path `B/real/w/dangling.md` gives canonical `B/real/w/dangling.md`.
- Canonicalising `instructions.md` gives relative `instructions.md`.
- With `B/locked` a directory of mode 0o000 holding `w/CLAUDE.md`, canonicalising the document path `B/locked/w/CLAUDE.md` returns HomeError::Io whose Display contains `canonicalising a given path` and `B/locked/w/CLAUDE.md`.
- Every case above is built under the temporary directory by the test itself, and the test asserts the count of cases it ran, 10.

**Files:**
- create: crates/lys-home/src/harness/claude_code/given_path.rs
- create: crates/lys-home/src/harness/claude_code/given_path_tests.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C71 — A compact_boundary record followed by its isCompactSummary user record imports as a lys.harness_event for the boundary and one Pi compaction entry under the summary record's uuid, the child of the boundary's entry, whose firstKeptEntryId is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and the empty string when there is none, so its context path is the compaction alone and the pair renders back, and whose tokensBefore is compactMetadata.preTokens; the summary record is not a message entry.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.
- S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.

### R2: Resolve and record the given documents at canonical paths

WHEN resolve_given resolves a session's documents, THE SYSTEM SHALL canonicalise the working directory as a directory path before walking the CLAUDE.md chain and walk the ancestors of the result; SHALL canonicalise the config directory as a directory path and record the result as config_dir.path; SHALL build the user CLAUDE.md and the memory index under the canonical config directory; SHALL record every absolute document's path as the R1 document canonicalisation of the path it was read at; and SHALL compare a chain position with the user CLAUDE.md on their canonical paths, so `D/.claude/CLAUDE.md` is still listed once, first, as user_claude_md. WHEN the working directory or the config directory is unresolved, THE SYSTEM SHALL use it as given and continue. THE SYSTEM SHALL compute the memory index's slug with projects_slug from the working directory exactly as given. THE SYSTEM SHALL NOT canonicalise the paths of the two written files, SHALL NOT resolve a document's final component, SHALL NOT change the measured order, SHALL NOT list an absent position, and SHALL NOT change projects_slug. IF canonicalising any path fails for a reason other than not existing, THEN THE SYSTEM SHALL fail the resolution with R1's refusal, and render-launch SHALL refuse with its existing refusal status 1, with no lys.given entry written.

**Acceptance:**
- Under a canonical temporary directory `B` with `B/link` a symlink to `B/real`, `B/real/w/CLAUDE.md` present and config `B/real/c`, resolving cwd `B/link/w` lists the chain document at `B/real/w/CLAUDE.md`.
- With `B/real/CLAUDE.md` and `B/real/w/CLAUDE.md` present, resolving cwd `B/real/w/../w` lists exactly 2 claude_md_chain documents under `B`, at `B/real/CLAUDE.md` then `B/real/w/CLAUDE.md`.
- Resolving with the config directory `B/real/c/` gives config_dir.path serialised as `B/real/c`, and `B/real/c/CLAUDE.md` listed as user_claude_md at `B/real/c/CLAUDE.md`.
- With config `B/link/.claude`, `B/link` a symlink to `B/real`, cwd `B/real/w`, and `B/real/.claude/CLAUDE.md` and `B/real/w/CLAUDE.md` present, the resolution's absolute documents are exactly 2: `B/real/.claude/CLAUDE.md` as user_claude_md first, then `B/real/w/CLAUDE.md` as claude_md_chain, and no claude_md_chain document has the path `B/real/.claude/CLAUDE.md`.
- Resolving with the config directory `B/gone/c/`, which does not exist, succeeds with config_dir.path serialised as `B/gone/c/` byte for byte and no user_claude_md or memory_index document.
- With cwd `B/link/w` and config `B/real/c`, a MEMORY.md at `B/real/c/projects/<projects_slug("B/link/w")>/memory/MEMORY.md` is listed at that path, and a MEMORY.md placed only under `projects_slug("B/real/w")` gives no memory_index document.
- With `B/real/w/CLAUDE.md` a symlink to `B/real/t/target.md`, the chain document is listed at `B/real/w/CLAUDE.md`.
- The appended_instructions and mcp_config documents are listed at `instructions.md` and `mcp.json`.
- With cwd `B/locked/w` and `B/locked` of mode 0o000, resolution returns HomeError::Io whose Display contains `canonicalising a given path` and `B/locked/w`.
- render-launch with --cwd `B/locked/w`, `B/locked` of mode 0o000, exits 1, stderr contains `canonicalising a given path` and `B/locked/w`, and the session holds 0 lys.given entries afterwards.
- `git diff` against the base of this brief shows no change to crates/lys-home/src/harness/claude_code/paths.rs.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/given.rs
- modify: crates/lys-home/src/harness/claude_code/given_tests.rs
- modify: crates/lys-home/tests/given_record.rs

**Checklist:**
- C72 — A legacy summary record imports as a Pi compaction entry whose firstKeptEntryId is the entry its leafUuid names, never the compaction's own id.
- C73 — A compaction whose first kept entry is not on record is refused with an error naming that uuid.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.

### R3: Compare canonical paths in given-check and name unresolved paths in both reports

WHEN given-check looks for the listed document, THE SYSTEM SHALL compare the R1 document canonicalisation of each listed path with the R1 document canonicalisation of the --path argument, so a record written before this change compares by its canonical form; a relative path SHALL be compared as written; an unresolved path SHALL be compared as written. THE SYSTEM SHALL canonicalise --file as a document path before reading it, and read it as given when it is unresolved. WHEN given-check answers, THE SYSTEM SHALL add to its report the field `unresolved`: the list of the --path argument and then the listed document's path, each as written and each only when unresolved, without repeating a string already in the list; empty when there is none. WHEN `given` lists a session's records, THE SYSTEM SHALL add to each listed record the field `unresolved`: the record's config_dir.path and then its absolute document paths in record order, each as recorded and each only when unresolved at the time of listing; empty when there is none. THE SYSTEM SHALL NOT add a field to the lys.given entry's data, SHALL NOT change the value of any existing report field (`path` and `file` echo the arguments as given), SHALL NOT change the exit statuses 0 matches, 1 differs, 2 refused, and SHALL NOT put a byte of any file in a report or an error. IF canonicalising --path, --file or a listed path fails for a reason other than not existing, THEN given-check SHALL refuse with R1's refusal and exit 2 with nothing on stdout, and `given` SHALL refuse with it.

**Acceptance:**
- With a record listing `B/real/w/CLAUDE.md`, given-check with --path `B/link/w/CLAUDE.md` and --file `B/real/w/CLAUDE.md` exits 0 with `answer` `matches`, `path` `B/link/w/CLAUDE.md` and `unresolved` [].
- The same record checked with --path `B/real/w/../w/CLAUDE.md` exits 0 with `answer` `matches`.
- A lys.given entry appended with the document path `B/link/w/CLAUDE.md`, as a record written before this change holds it, checked with --path `B/real/w/CLAUDE.md` exits 0 with `answer` `matches`.
- A lys.given entry listing `B/gone/CLAUDE.md`, which does not exist, checked with --path `B/gone/CLAUDE.md` and a --file of equal bytes and length exits 0 with `answer` `matches` and `unresolved` ["B/gone/CLAUDE.md"].
- The same entry checked with --path `B/gone/../gone/CLAUDE.md` exits 2, stdout is empty and stderr contains `B/gone/../gone/CLAUDE.md`.
- `given` on a session holding a record whose config_dir.path is `B/gone/c/` and a record whose paths all exist reports `unresolved` ["B/gone/c/"] on the first and [] on the second.
- `given` on a session holding a record that lists the document `B/real/w/absent.md`, which does not exist at listing time while `B/real/w` does, reports `unresolved` ["B/real/w/absent.md"] on that record.
- given-check with --path `B/locked/w/CLAUDE.md`, `B/locked` of mode 0o000, exits 2, stdout is empty and stderr contains `canonicalising a given path` and `B/locked/w/CLAUDE.md`.
- given-check with --path `instructions.md` against a render's record exits 0 with `answer` `matches` for the render's instructions.md.
- The keys of a lys.given entry's data after a render are exactly harness, harness_version, kinds, config_dir, documents and environment.

**Files:**
- modify: crates/lys-home/src/cli/given.rs
- modify: crates/lys-home/tests/given_record.rs

**Checklist:**
- C73 — A compaction whose first kept entry is not on record is refused with an error naming that uuid.
- C74 — Directly after each compaction entry the importer appends one lys.loss custom entry whose parent is the compaction and whose data holds only ids, counts, byte counts and SHA-256 hashes, with the keys RECORD.md sets down, first_kept among them, null when nothing is kept.
- C75 — A lys.loss entry's span is the root-to-first-kept path from the root or the previous compaction to the entry before the first kept one; its three SHA-256s are over those entries' source lines in file order, and side-leaf and sidechain entries hanging from the span are counted by number only.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.
- S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.

### R4: Prove with one fixture test that the three shapes record and compare as one path

Add one test to crates/lys-home/tests/given_record.rs that builds, under a temporary directory canonicalised as `B`, a real directory `B/real` holding `w/CLAUDE.md` and `c/CLAUDE.md`, and a symlink `B/link` to `B/real` made by the test, with no memory index file. It renders the fixture session with render-launch once as the reference (--cwd `B/real/w`, CLAUDE_CONFIG_DIR `B/real/c`) and once for each of three shapes: the symlinked directory (--cwd `B/link/w`, CLAUDE_CONFIG_DIR `B/link/c`), the `..` path (--cwd `B/real/w/../w`, CLAUDE_CONFIG_DIR `B/real/c/../c`), and the trailing slash (--cwd `B/real/w`, CLAUDE_CONFIG_DIR `B/real/c/`). Then it runs given-check on the reference entry for `B/real/w/CLAUDE.md` with --path in the symlinked and the `..` shapes. The existing test functions of the file keep their assertions, and the fixture builds their expectations from its temporary directory canonicalised. The test SHALL NOT depend on a symlink the platform provides, and SHALL NOT compare a render against itself.

**Acceptance:**
- Each of the three shapes' lys.given data serialises `config_dir` and `documents` to the same JSON bytes as the reference render's, and the reference's config_dir.path is `B/real/c`.
- The test asserts it compared exactly 3 shapes against the reference.
- given-check with --path `B/link/w/CLAUDE.md` and with --path `B/real/w/../w/CLAUDE.md` against the reference entry each exits 0 with `answer` `matches`, and the test asserts it ran exactly 2 checks.
- The five test functions the file held before this brief pass with their assertions unchanged.
- `cargo test --workspace --all-features` lists the new test as passed.

**Files:**
- modify: crates/lys-home/tests/given_record.rs

**Checklist:**
- C76 — A compact_boundary whose logicalParentUuid names no record in the file imports, and its lys.loss entry names that uuid as unresolved.

**Stories:**
- S32 (Tom, Owns the platform and reads what a session was given) — As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.

### R5: State the canonicalisation rule in RECORD.md and render the cluster

Extend the lys.given paragraph of docs/design/home/RECORD.md to state: every absolute path is canonicalised before it is recorded or compared; the config directory, and the working directory before the chain is walked, are canonicalised whole; a document's path is its parent directory canonicalised with its own final name kept, so a symlinked document is recorded at its link position; the two written files stay relative to the render's out directory and are the exemption, for the reason the paragraph already gives; a document path is built by join and cannot carry a trailing slash, the config directory can and canonicalisation removes it; a path that cannot be canonicalised because it does not exist is recorded as given, and the `given` and `given-check` reports name it in `unresolved`, while the entry's data keeps exactly its six fields; a canonicalisation failure of any other kind is refused by path and operation; given-check canonicalises both the listed path and the --path argument at check time, so records written before this rule compare correctly; and the memory index's slug is computed from the working directory as given, not canonicalised. Then regenerate the cluster's rendered markdown with render-cluster.py. THE SYSTEM SHALL NOT change what the paragraph says of the order, the kinds, the absent positions or the entry's place under the render event.

**Acceptance:**
- RECORD.md's lys.given paragraph contains the words `canonicalised`, `link position`, `unresolved` and `trailing slash`, and names instructions.md and mcp.json as the exemption.
- RECORD.md's lys.given paragraph states that the slug is computed from the working directory as given.
- RECORD.md still states `Data is exactly {harness, harness_version, kinds, config_dir, documents, environment}`.
- `sh scripts/design/gate.sh` exits 0, so DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-010.md are what their JSON renders to.

**Files:**
- create: docs/design/home/briefs/HOME-010.md
- modify: docs/design/home/RECORD.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C77 — Two imports of the compaction fixture into two homes give lys.loss lines equal byte for byte once id, parentId, timestamp and data.compaction_id are masked.

**Stories:**
- S33 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.

## Boundaries

- No field is added to, removed from or renamed in the lys.given entry's data; `unresolved` lives in the `given` and `given-check` reports only.
- projects_slug and crates/lys-home/src/harness/claude_code/paths.rs do not change; the slug takes the working directory as given.
- The two written files' paths stay relative to the render's out directory and are never canonicalised.
- A document's final component is never resolved; only its parent directory is.
- No path in any other home record (fork's parentSession, the render manifest's files, render-launch --out) is canonicalised.
- given-check keeps its exit statuses 0 matches, 1 differs, 2 refused, and the value of every report field it printed before.
- An absent document position stays omitted, the measured order and MEASURED_VERSION 2.1.283 stay as they are, and the given entry stays a side leaf under the render event with the head unmoved.
- No document content and no variable value appears in any report, error or test name; canonicalisation reads no file's bytes.
- No dependency is added, and the design's structure array is the whole file list.

## Verification

- From the repository root: cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists R1's, R2's, R3's and R4's tests as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0 with no warnings.
- sh scripts/design/gate.sh exits 0.
- grep -rn 'canonicalize' crates/lys-home/src | grep -v '_tests.rs' prints lines in crates/lys-home/src/harness/claude_code/given_path.rs only.
- git diff against the brief's base shows no change to crates/lys-home/src/harness/claude_code/paths.rs or crates/lys-home/src/record/given.rs.
