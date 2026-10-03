---
type: brief
id: HOME-038
cluster: home
title: A home act's blocks are one durable operation: staged together, placed together, flushed a fixed number of times whatever their count
---

# HOME-038: A home act's blocks are one durable operation: staged together, placed together, flushed a fixed number of times whatever their count

> **Cluster:** home
> **Depends on:** HOME-001, HOME-020
> **Design anchor:**
> - ADR-134 — Leaves and the pin are appended together to checksummed segment files, with one flush per append — A log's leaves are appended as length-prefixed, checksummed records to segment files rolled at a fixed size. Each record carries the pin it makes, so the leaf and the pin are written by one append and made durable by one flush; state.json is no longer rewritten. Appends given together share one flush. A store in the per-file layout is migrated once, on its first writable open.
> - ADR-059 — A named leaf in the file store is a whole, flushed leaf, and the no-replace link is its commit point — A file under a 20-digit leaf name is always a whole, flushed leaf; only dot-prefixed temporary files in leaves/ may be partial. A leaf is written and flushed in a hidden temporary file, then linked to its final name by an operation that refuses to replace, then the temporary name is removed and leaves/ is flushed. The successful link is the commit point: the extent advances past the leaf whatever happens next. A failed temporary-name removal still returns Ok; a failed leaves/ flush returns StoreError::LeafDurabilityUncertain carrying the index, and the handle refuses further appends with it until reopened. A writable open flushes leaves/ before counting and fails with Io when that flush fails; a read-only open (FileLeafStore::open_read_only) counts the named leaves without flushing; neither counts, deletes or changes a leftover temporary file. Rejected: naming the leaf first and writing into it (today's torn-leaf shape); deleting leftover temporary files at open, which would make a read-only open mutate the store; flushing at a read-only open, which would turn a status on read-only media into an error; and reporting a post-link failure as Io, which left the extent behind and made the next append report the store's own leaf as LeafAlreadyWritten.
> - ADR-108 — A Claude Code import builds its session under a staging name and publishes it once — The import command builds its new session under the staging name `<id>.jsonl.importing` beside `sessions/<id>.jsonl`, holding `<id>.lock` throughout: the header and every entry line are written to the staging file with no sync, and the index rows and the head are kept in memory. It then publishes once, in this order: one sync of the staging file; the index written whole to `<id>.index.jsonl` through a temporary file, synced and renamed; the head written once to `<id>.head` through a temporary file, synced and renamed; one sync of the sessions directory; the staging file renamed to `<id>.jsonl`; one more sync of the sessions directory. A crash before the rename leaves no session under the final name, so a re-import is not refused as Exists; the next import of that id and the next open of that session file, each holding the lock, remove the staging file and, when `<id>.jsonl` is absent, the `<id>.index.jsonl` and `<id>.head` a part-way publish left. Rejected: appending in place and leaving a session that refuses by name after a crash; extending reconcile to trim an unsynced tail.
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C195 — A BlockBatch stages an act's new blocks unsynced, and its commit pushes every staged file, flushes the device once, renames every file to its hash name, syncs each touched shard directory once and flushes once more, in that order; a dropped batch places nothing; put and put_file exist only on the batch (HOME-038 R1).
> - C196 — Every act that stores blocks (call ingest and its parts, harness events, import content, the given statement, templates, the proxy's persist fallback) puts them through one batch committed before the entry naming them is appended; a thirty-part call rises the device flushes by exactly two (HOME-038 R2).
> - C197 — BlockStore::io() answers file pushes, directory syncs and device flushes as three counts a test reads, replacing syncs() and the capture timing's block_syncs (HOME-038 R3).
> - C198 — BlockStore::get hashes what it read and refuses a block whose bytes do not hash to the name asked for as BlockDiffers naming both hashes, never repairing or removing it (HOME-038 R4).
> **Stories:**
> - S79 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a recorded call's blocks written as one durable operation with a fixed number of device flushes, so that the home's cost per act does not grow with the parts a reply has and a block that exists is whole.

## Purpose

Tom, 3 October 14:27 to 14:29: storage is done properly; a batch is one durable operation with one full flush, in the code, never leaf by leaf. The log store now does this (ADR-134: leaves and the pin appended together, one flush per append). The home's block store does not. BlockStore::put (crates/lys-home/src/record/blocks.rs:106) and put_file (:148) each write a temporary file, sync_all it, rename it into place and sync_all its shard directory: two device flushes per new block, because on Apple std's sync_all and sync_data are both fcntl(F_FULLFSYNC) (library/std/src/sys/fs/unix.rs:1387 and :1401 in the 1.95.0 toolchain), a flush of the whole drive's cache each. One recorded call puts its raw request, its raw response and one block per request and response part (call.rs:196 and :197, finish_ingest_counted :368 to :388, call/captured.rs:198), then appends its entry with the session batch's four syncs (batch.rs). A call with thirty parts costs sixty-four full flushes for its blocks and four for its entry. HOME-020 R1 counted the block store's syncs and left them unchanged, in scope for this brief. This brief makes an act's blocks one durable operation: every new block of the act is staged unsynced, pushed to the drive together, flushed once, placed under its hash name together, and the names flushed once; the count of device flushes is two per act whatever the block count; and a block read is checked against its name, so a block that is not whole is refused by name and never reused by name.

## Task

Replace BlockStore::put and put_file with a block batch: BlockStore::batch() opens a BlockBatch; BlockBatch::put(bytes) and put_file(path) stage a new block as a temporary file in its shard directory with no sync, or report a held block as reused without writing; BlockBatch::commit(self) pushes every staged file to the drive (libc::fsync on Apple, where std has no plain fsync; std's sync_all elsewhere), flushes once (fcntl(F_FULLFSYNC) on the store's root descriptor on Apple; the per-file syncs are that flush elsewhere), renames every staged file to its hash name, syncs each touched shard directory once, and flushes once more; a batch dropped without commit discards its temporary files and places nothing. A single block is a batch of one. Every act that puts blocks puts them through one batch committed before the entry naming them is appended: ingest_call, ingest_call_files, ingest_outcome and their parts (record/call.rs, call/captured.rs), harness events (harness/claude_code/events.rs:92 and :235), the import's content blocks (harness/claude_code/import/content.rs:70), the given statement (record/given_statement.rs:57), the templates store (record/templates.rs:41, harness/claude_code/launch.rs:183) and the proxy's persist fallback (proxy/persist.rs:85 and :177). The sync counter becomes three counts, file pushes, directory syncs and device flushes, read by the tests that prove the bound. BlockStore::get verifies the bytes it read against the hash it was asked for and refuses a mismatch by name. lys-home gains the libc crate for the two Apple calls, behind one function in block_batch.rs that every other file calls; nothing else in the workspace uses libc today. Out of scope, with its numbers: the proxy spool's own seal (proxy/spool.rs:84, one sync_all at close) and admit_spool's directory syncs (blocks.rs admit_spool, persist.rs:55 to :57 and :150 to :158), which are the proxy's capture write and stay as they are; the session entry batch (four syncs, batch.rs), already one durable operation; the block layout blocks/<hh>/<hash>; pruning of blocks no entry names.

## Requirements

### R1: A block batch stages, pushes, flushes, places and flushes, with a fixed flush count

Behavioural. A new file crates/lys-home/src/record/block_batch.rs defines BlockBatch, opened by BlockStore::batch(&self). BlockBatch::put(&mut self, bytes: &[u8]) -> Result<Put, HomeError> hashes the bytes; when blocks/<hh>/<hash> is a file, or the batch already staged that hash, it answers Put { hash, new: false } and writes nothing; otherwise it writes the bytes to .<hash>.<pid>.<nonce>.tmp in the shard directory, creating the directory, with no sync, and answers Put { hash, new: true }. BlockBatch::put_file(&mut self, source: &Path) copies the file to a temporary file in the store root while hashing, then moves the temporary file to its shard directory's temporary name, unsynced, and answers as put does. BlockBatch::commit(self) -> Result<Committed, HomeError> does, in this order and never another: for every staged file, one push (libc::fsync on Apple; File::sync_all elsewhere), counted as a file push; one device flush on the store root's descriptor (fcntl(F_FULLFSYNC) on Apple; File::sync_all of the root directory elsewhere), counted as a device flush; for every staged file, rename to its hash name, a rename that finds the name already placed by another writer discarding the staged file and counting that block as reused; for every shard directory a rename touched, one directory sync (File::sync_all), counted as a directory sync; one more device flush, counted. Committed carries the hashes placed, the hashes reused and the three counts. Drop of an uncommitted BlockBatch removes every staged temporary file and places nothing. BlockStore::put and put_file are removed; a caller with one block opens a batch, puts it and commits. The two Apple calls live in one function in block_batch.rs; no other file names libc. THE SYSTEM SHALL NOT sync a staged file before commit, SHALL NOT rename a staged file before the first device flush of its batch, SHALL NOT place a hash name whose bytes are not whole and pushed, and SHALL NOT read a held block to decide reuse.

**Acceptance:**
- A batch of twelve new blocks spread over four shard directories, committed, reports twelve file pushes, four directory syncs and two device flushes, and the same twelve bytes put again in a new batch and committed report zero of each and twelve reuses, with no file created.
- A batch of three new blocks dropped without commit leaves no file under any hash name and no temporary file in the store; the three hashes put afterwards are new.
- A batch whose rename finds the hash already placed by another writer (the test places the file between staging and commit) reports that hash reused, leaves one file, and the file's bytes hash to its name.
- grep finds `fn put(` and `fn put_file(` only in block_batch.rs, `libc::` only in block_batch.rs, and the order push, flush, rename, directory sync, flush in commit's body read against the spec by the reviewer.

**Files:**
- create: crates/lys-home/src/record/block_batch.rs
- create: crates/lys-home/src/record/block_batch_tests.rs
- modify: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/Cargo.toml
- modify: Cargo.toml

**Checklist:**
- C195 — A BlockBatch stages an act's new blocks unsynced, and its commit pushes every staged file, flushes the device once, renames every file to its hash name, syncs each touched shard directory once and flushes once more, in that order; a dropped batch places nothing; put and put_file exist only on the batch (HOME-038 R1).

**Stories:**
- S79 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a recorded call's blocks written as one durable operation with a fixed number of device flushes, so that the home's cost per act does not grow with the parts a reply has and a block that exists is whole.

### R2: Every act puts its blocks through one batch, committed before the entry that names them

Behavioural. Each act that stores blocks opens one BlockBatch, puts every block of the act through it, commits it, and only then appends the entry carrying the hashes: ingest_call (raw request, raw response, then every request and response part), ingest_call_files, ingest_outcome and the captured-parts writer in call/captured.rs share one batch per call; a harness event's block (harness/claude_code/events.rs) is a batch of one committed before its entry; the import's content blocks (harness/claude_code/import/content.rs) go through one batch per imported entry, committed before that entry is written to the staging file; the given statement's COSE block and the template store's bytes are each a batch of one; the proxy's persist fallback (proxy/persist.rs:85 and :177) puts its copied capture through a batch of one. A commit that fails leaves the act's entry unwritten and the error is the act's error, named as today's HomeError::io with the operation and path. The counts: a call ingested with thirty parts rises the block store's device flushes by exactly two, its file pushes by the number of new blocks, and its directory syncs by the number of shard directories touched; the session's own syncs stay four. THE SYSTEM SHALL NOT append an entry naming a hash before the batch that placed it has committed, and SHALL NOT open more than one batch for one act.

**Acceptance:**
- A test ingests one call with a thirty-part response into a fresh home and reads the block store's counts before and after: device flushes +2, file pushes equal to the new blocks the call made, directory syncs equal to the shard directories touched; record_tests.rs's existing block-sync expectations (the two-block put at :449 to :454) are rewritten to the three counts.
- A test makes commit fail (the store root made read-only before commit, restored after) and shows the call's entry absent from the session and the error naming the operation and path; the same call afterwards succeeds and is recorded once.
- grep finds every former `blocks.put(` and `blocks.put_file(` call site replaced by a batch, with the commit before the append in each act's body, listed in the commit message by file and line.

**Files:**
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/call/captured.rs
- modify: crates/lys-home/src/record/given_statement.rs
- modify: crates/lys-home/src/record/templates.rs
- modify: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/import/content.rs
- modify: crates/lys-home/src/harness/claude_code/launch.rs
- modify: crates/lys-home/src/proxy/persist.rs
- modify: crates/lys-home/src/record/record_tests.rs
- modify: crates/lys-home/src/record/call_tests.rs

**Checklist:**
- C196 — Every act that stores blocks (call ingest and its parts, harness events, import content, the given statement, templates, the proxy's persist fallback) puts them through one batch committed before the entry naming them is appended; a thirty-part call rises the device flushes by exactly two (HOME-038 R2).

**Stories:**
- S79 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a recorded call's blocks written as one durable operation with a fixed number of device flushes, so that the home's cost per act does not grow with the parts a reply has and a block that exists is whole.

### R3: The block store's cost is three counts a test reads

Structural, with its behaviour stated. BlockStore::syncs() -> u64 becomes BlockStore::io() -> BlockIo, a public Copy value in record/io_counts.rs with three u64 fields: file_pushes, directory_syncs and device_flushes, each rising by one for the matching call in BlockBatch::commit and for nothing else; a block read counts nothing. The proxy's timing (persist.rs:127 timing.block_syncs) and the Session's IoCounts keep their meaning: timing carries the three counts under their names, and IoCounts.syncs stays the session's own. The counters are atomics held by the BlockStore, so a read through &self counts. THE SYSTEM SHALL NOT measure time, SHALL NOT count process-wide, and SHALL NOT change what any read does.

**Acceptance:**
- A fresh BlockStore reports BlockIo { file_pushes: 0, directory_syncs: 0, device_flushes: 0 }; after R1's twelve-block batch it reports 12, 4, 2; after the reused batch the same.
- The proxy's capture timing report names file_pushes, directory_syncs and device_flushes where it named block_syncs, and the test that read block_syncs reads them.
- grep finds no `syncs()` on BlockStore and no `block_syncs` in the workspace.

**Files:**
- modify: crates/lys-home/src/record/io_counts.rs
- modify: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/proxy/persist.rs
- modify: crates/lys-home/src/proxy/capture.rs
- modify: crates/lys-home/src/record/blocks_tests.rs
- modify: crates/lys-home/src/record/record_tests.rs

**Checklist:**
- C197 — BlockStore::io() answers file pushes, directory syncs and device flushes as three counts a test reads, replacing syncs() and the capture timing's block_syncs (HOME-038 R3).

**Stories:**
- S79 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a recorded call's blocks written as one durable operation with a fixed number of device flushes, so that the home's cost per act does not grow with the parts a reply has and a block that exists is whole.

### R4: A block read is checked against its name and refused by name when it differs

Behavioural. BlockStore::get(hash) hashes the bytes it read and, when they do not hash to the name asked for, answers HomeError::BlockDiffers { hash, found } naming the hash asked for and the hash found, and never the bytes; a missing block stays NoBlock. verify_all keeps its whole-store answer. The check is the one hash the read already pays for in verify_all, now paid on every read, so a block torn by a crash between a rename and the names flush (R1's window, the only one left) is refused when read and never trusted by name. THE SYSTEM SHALL NOT repair, rewrite or remove a differing block on read, and SHALL NOT skip the check for any caller.

**Acceptance:**
- A test writes bytes under the wrong hash name directly into a store and reads that hash: BlockDiffers naming both hashes; reading a missing hash is NoBlock as today; reading a whole block answers its bytes.
- Every caller of get that matched on NoBlock compiles against the new variant with no `_ =>` arm added; grep finds no wildcard arm over HomeError added in this brief's commits.

**Files:**
- modify: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/src/record/blocks_tests.rs

**Checklist:**
- C198 — BlockStore::get hashes what it read and refuses a block whose bytes do not hash to the name asked for as BlockDiffers naming both hashes, never repairing or removing it (HOME-038 R4).

**Stories:**
- S79 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a recorded call's blocks written as one durable operation with a fixed number of device flushes, so that the home's cost per act does not grow with the parts a reply has and a block that exists is whole.

## Boundaries

- No byte of any recorded entry changes, and no block's name or place changes: blocks/<hh>/<hash> stays the layout and a block put before this brief is read by the same name after it.
- The session entry batch (record/batch.rs), the staged import's publish (ADR-108) and the index and head files are untouched; the session's own sync count stays four per published batch.
- The proxy spool's seal (proxy/spool.rs:84) and admit_spool's rename and directory syncs (blocks.rs admit_spool; persist.rs:55 to :57 and :150 to :158) are out of scope and keep their counts; a later brief measures them.
- No new command, flag or report field other than the three counts replacing block_syncs in the capture timing.
- libc is used for two calls on Apple only, in one function in block_batch.rs; no other file in the workspace names it, and no other platform-specific code is added.
- No block is pruned, repaired or rewritten; a differing block is refused on read and left for an operator to see with verify_all.

## Verification

- cargo fmt --all exits 0 and git status --porcelain prints the same lines after it as before it.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0, on Dean.
- cargo nextest run --workspace --all-features and cargo test --doc --workspace exit 0, on Dean.
- sh scripts/design/gate.sh: its printed failures, read from the output and never from the exit code, are none.
- The counts in R1 to R3 are asserted by the tests named, read from BlockIo, never from a clock or a sleep.
- The commit message lists every former put and put_file call site by file and line and the batch that replaced it, and names the libc version added and the two calls it is used for.
