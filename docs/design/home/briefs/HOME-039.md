---
type: brief
id: HOME-039
cluster: home
title: A raw body is kept whole as content-defined chunks that repeat across calls, a placed block is claimed until its entry lands, and an unclaimed block is reclaimed while Lys runs
---

# HOME-039: A raw body is kept whole as content-defined chunks that repeat across calls, a placed block is claimed until its entry lands, and an unclaimed block is reclaimed while Lys runs

> **Cluster:** home
> **Depends on:** HOME-038, LYSLOGSTORE-009
> **Design anchor:**
> - ADR-139 — A raw body is kept whole as content-defined chunks; a placed block is claimed until its entry lands; an unclaimed block is reclaimed under the store's lock — A raw body at or over 64 KiB is stored as content-defined chunks, each a block, under one manifest block (lys/chunks/v1: the length, then the chunk hashes in order) whose hash the record carries where it carried the whole body's; the chunker is a Gear rolling hash with a table derived from SHA-256 of the byte value, a minimum of 8 KiB, an average of 32 KiB and a maximum of 128 KiB, so boundaries depend on the bytes alone and a call's new turn adds one or two new chunks while the rest repeat. A body under the threshold stays one block. Every block batch writes a claim under blocks/pending before its first device flush naming its session, its entry and its new hashes; the act releases the claim after its entry is appended; a batch reusing a claimed hash records itself as an adopter on that claim. Reclaim runs under blocks/.lock held exclusively (commits hold it shared) and, for a claim older than one hour, unlinks the claim when its entry or an adopter's entry is in the index, else frees the claim's blocks that no newer claim names, as one act with one counted flush, at the proxy worker's start and hourly. Rejected: a retention period (Tom's to set; this makes one cheap later, as an unlink of manifests no live entry names); a mark-and-sweep over every session's entries (reads history on every sweep, against ADR-112); reference counts beside blocks (a second copy of the truth); removing on read or at open (a read-only open must not mutate, ADR-059); a new chunking crate (eighty lines beside sha2, with a golden test, is reviewable in full).
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> - ADR-138 — Lys's log stays on lys-log-store's own segment engine; haematite stays the intended second backend, on named conditions — Lys's log keeps lys-log-store's own segment engine. Why, by name: 1. Nothing in a Lys log is superseded. A record is appended once and never rewritten (LYSLOGSTORE-008), so haematite's reclaim of superseded pages has nothing to reclaim here. Lys's unreachable bytes are whole files (kept migration copies, unreferenced home blocks), and removing them is an unlink. 2. Lys acknowledges a leaf only after the flush that covers it. Haematite's ordinary stores do not: native production runs FsyncPolicy::CommitOnly (crates/haematite/src/shard/actor/native/boot.rs:577 at af5e18a) and N1 stands. Haematite's STOR-012 LeafLog, written for Lys's log and sealed from that path by its ADR-040, does: LeafLog::append takes one contiguous batch with its pin and flushes its frame before returning (crates/haematite/src/api/leaf_log.rs:177-178, present at tag v0.15.0, one device flush per act counted in api/leaf_log_count_tests.rs). So this is not a reason against the move; it says the move is onto LeafLog and never onto an ordinary store. (Corrected 7 October on Apollo's post 446b2b0e.) 3. A Lys log is evidence a verifier reads without the server: the segment format, its CRCs and the tiles of LYSLOGSTORE-006 are read by lys log and the anchor with no database engine. 4. Haematite's own criteria 2, 3, 5 and 6 are written but not built or measured (Apollo, 7 October 12:22, post 6d2037f7: prune designed in STOR-013, startup still reads in proportion to history until format 4, no bound test, not installed). Lys would wait on them for what its own engine does with an unlink. Haematite under Lys's log is in scope and gets done, by Apollo's team (Tom, 3 October, rule 11). The move is scheduled haematite work now, not a waiting list (Waffles, 7 October, post 1afd2474). It waits on four conditions, each a named haematite brief: 1. per-append durability, meaning an acknowledgement only after the flush that covers it: met by STOR-012's LeafLog in haematite 0.15.0, with its flushes counted, and its start-up read bound (leaf_log_open_bound.rs) in 0.15.1 once published; 2. prune and online reclaim installed and measured (STOR-013 and STOR-014); 3. a start-up that reads no history (STOR-015); 4. a green bound test (STOR-014). When all four are green at a named haematite commit, the Lys log move gets its own Lys brief, and that brief goes into the next Lys piece. Any Lys store that comes to hold values rewritten in place, rather than appended, goes on haematite and not into the log.
> **Checklist:**
> - C199 — A raw body at or over 64 KiB is stored as Gear-chunked blocks (8 KiB minimum, 32 KiB average, 128 KiB maximum) under one lys/chunks/v1 manifest whose hash the record carries, with raw_request_chunks and raw_response_chunks saying so, and call_whole reads it back byte for byte or refuses it naming the manifest and the chunk (HOME-039 R1).
> - C200 — BlockBatch::commit writes a claim under blocks/pending naming the session, the entry and every new hash before its first device flush, every act releases it with landed after its append, and a batch that reuses a claimed hash records itself as the claim's adopter (HOME-039 R2).
> - C201 — BlockStore::reclaim, under blocks/.lock held exclusively against commits, unlinks the claim of a landed entry and frees the blocks of a claim older than the grace whose entry and adopters never landed, as one act with one counted flush, run at the proxy worker's start and hourly with its four counts reported (HOME-039 R2).
> - C202 — The bound test is red at main for a growing conversation and for a committed act that never landed, green at the card's head, and blocks/pending and the chunk manifests have their STORES.md rows (HOME-039 R3).
> **Stories:**
> - S80 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a session's recorded calls to cost the bytes that are new in each call, and a block that no entry names to be reclaimed while Lys runs, so that moving every seat onto Lys does not fill the disk.

## Purpose

Tom, 10 October 19:2x: "It doesn't look like that storage size bug is under control yet. Please make sure that it's fully implemented. I don't want anything else derailing it. We are constantly running out of hard drive space." Read on 10 October 21:3x, sizes only, no store file opened: the live install's data path is under 10 MiB and has no homes directory yet, because no seat runs on Lys yet. Lys's growth is in front of us and starts the day the seats move, so it is bounded before that day. Three things are unbounded today. First, ingest_call puts the raw request and the raw response whole as blocks beside the parts (crates/lys-home/src/record/call.rs:233 and :234; the proxy places each spooled body by rename in persist.rs place_body). The Messages API and the Responses API are stateless: every request carries the whole conversation so far, so a session of n calls stores every prefix of its conversation again, about n squared over two times the average turn in raw request bytes, and no two of those bodies hash alike, so the hash-named store (blocks/<hh>/<hash>) dedupes none of it. A session of 500 calls whose request bodies average 200 KiB stores about 100 MiB of raw request, nearly all of it bytes already held under another name. The parts are one block per message part and do dedupe across calls; the raw bodies are the growth. Second, a block is placed before the entry naming it is appended (HOME-038 R2), so a crash or a refused append between the two leaves a block nothing names. Nothing finds it: verify_all (blocks.rs:250) reads every block to check its bytes against its name, not whether an entry names it, and nothing removes it. The proxy's journal (journal.rs:5, recover at :605) re-appends a call whose record was written, and covers nothing else; HOME-038 names the other acts (harness events, import content, the given statement, templates). Third, LYSLOGSTORE-009's bound test (R5) drives sessions through the runner and the proxy, but its mix has no growing conversation, so it is green with the quadratic growth in place. Retention: evidence is kept for good unless Tom sets a period (ADR-138). This brief sets no period. It makes what is kept cost what is new in it, claims every placed block until its entry lands, reclaims an unclaimed block while Lys runs, and makes the bound test red at main against both. If Tom sets a period for raw bodies later, it is an unlink of the chunk manifests no live entry names, under the same lock, and nothing here is undone.

## Task

After HOME-038 and LYSLOGSTORE-009 are built: store every raw body at or over 64 KiB as content-defined chunks, each a block, named by one manifest block whose hash the record carries where it carries the whole body's hash today, read back byte for byte, with the boundaries a function of the bytes alone and fixed by a golden test (R1); make every block batch write a pending claim naming its new blocks, its session and its entry before its first device flush, release the claim once the entry is appended, and reclaim, under the block store's own lock, the blocks of a claim older than the grace whose entry never landed, with each count on the report (R2); add the growing-conversation mix and a crashed act to the bound test so it is red at main for both, and give blocks/pending and the chunk manifests their STORES.md rows (R3). The numbers, decided here under Tom's rule of 10 October 06:2x that open numbers are decided with a stated reason: the chunking threshold is 64 KiB, because a body under it is one block and one hash and costs more as a manifest than as itself; the chunker is a Gear rolling hash with a minimum of 8 KiB, an average of 32 KiB and a maximum of 128 KiB, because a Claude Code turn adds between one and a few tens of KiB to the next request and a 32 KiB average means each new call adds one or two new chunks while the rest repeat; the claim grace is one hour, because the longest act between a commit and its append is a staged import's publish (ADR-108), measured in minutes, and one hour is a bound no act approaches while an orphan's cost for an hour is one act's blocks; reclaim runs at the proxy worker's start and once an hour while it runs, because the proxy is the home's one long-lived writer and an hour matches the grace. Out of scope: any retention period for evidence (Tom's to set, ADR-138); chunking of parts, which already dedupe by hash; the session entry batch and the index; moving any store onto haematite.

## Requirements

### R1: A raw body at or over the threshold is stored as content-defined chunks under one manifest, and read back whole

Behavioural. A new file crates/lys-home/src/record/chunks.rs defines chunk(bytes: &[u8]) -> Vec<Range<usize>>: a Gear rolling hash over a 256-entry table of u64 constants derived in the file from SHA-256 of the byte value (the table is a const checked by a test against that derivation), a minimum chunk of 8 KiB, a boundary where the hash's low bits match the 32 KiB mask, a maximum of 128 KiB, and the last chunk whatever is left; the boundaries depend on the bytes alone and on nothing else. A manifest block is bytes beginning with the line `lys/chunks/v1`, then one line naming the body's byte length, then one 64-hex hash per line in chunk order. BlockBatch gains put_body(bytes) and put_body_file(path): a body under 64 KiB is put as one block as today and answers Body { hash, chunks: None }; a body at or over it is chunked, each chunk put through the same batch (held chunks reused, new chunks staged), the manifest put as one more block, and the answer is Body { hash: the manifest's hash, chunks: Some(count) }. The proxy's place_body (persist.rs) keeps its rename for a body under the threshold and chunks a spooled body at or over it from the spool file without reading it into memory whole. The call record keeps raw_request and raw_response as the hash it carries today and gains raw_request_chunks and raw_response_chunks, each Option<u64>, absent for a whole body, so a record written before this brief reads as whole. call_whole (call/whole.rs body) reads a manifest by its hash, checks the first line and the length, reads each chunk, and answers the body's bytes; a chunk that is missing or differs answers the existing unreadable shape naming the manifest hash and the chunk hash, never a partial body. THE SYSTEM SHALL NOT store a body at or over the threshold whole, SHALL NOT let a chunk boundary depend on anything but the bytes, SHALL NOT answer a reassembled body whose length differs from the manifest's, and SHALL NOT change what any record written before this brief reads as.

**Acceptance:**
- A golden test chunks a fixed 1 MiB input generated from a seeded xorshift in the test and asserts the exact boundary offsets written in the test file; every chunk but the last is between 8 KiB and 128 KiB; changing the minimum, the mask, the maximum or one table entry fails it.
- Round trips of bodies of 0 bytes, 64 KiB minus one, exactly 64 KiB, 1 MiB and 10 MiB through put_body and call_whole are byte for byte equal; the first two answer chunks None and are one block each; the rest answer Some(count) with count equal to the manifest's lines.
- Fifty calls are ingested where each request body is the previous body plus 2 KiB of new bytes, starting at 100 KiB: the bytes of new blocks placed for calls two to fifty, summed from each Committed's placed hashes and their sizes, are under three times the last body's size, and at main, with whole bodies, the same sum is over twenty times it; this test is the red at main the handback quotes.
- A record written at main, with raw_request and no raw_request_chunks, reads whole through call_whole unchanged; grep finds no reader of raw_request that does not consult raw_request_chunks.

**Files:**
- create: crates/lys-home/src/record/chunks.rs
- create: crates/lys-home/src/record/chunks_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/block_batch.rs
- modify: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/call/captured.rs
- modify: crates/lys-home/src/record/call/whole.rs
- modify: crates/lys-home/src/proxy/persist.rs
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/src/record/call_tests.rs

**Checklist:**
- C199 — A raw body at or over 64 KiB is stored as Gear-chunked blocks (8 KiB minimum, 32 KiB average, 128 KiB maximum) under one lys/chunks/v1 manifest whose hash the record carries, with raw_request_chunks and raw_response_chunks saying so, and call_whole reads it back byte for byte or refuses it naming the manifest and the chunk (HOME-039 R1).

**Stories:**
- S80 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a session's recorded calls to cost the bytes that are new in each call, and a block that no entry names to be reclaimed while Lys runs, so that moving every seat onto Lys does not fill the disk.

### R2: A batch claims its new blocks until the entry naming them lands, and an unclaimed block is reclaimed under the store's lock

Behavioural. A new file crates/lys-home/src/record/pending.rs defines the claim: BlockBatch::commit (HOME-038 R1) writes, as one more staged file pushed and flushed with the batch's first device flush, blocks/pending/<batch id>.json naming the session id, the entry id the act will append, the time, and every hash the batch is placing as new; the batch's two device flushes stay two. Committed gains landed(self) -> Result<(), HomeError>, called by every act of HOME-038 R2 after its entry is appended and its session batch published: it unlinks the claim with no flush, because a claim that outlives its entry is found landed by the next reclaim. A batch that reuses a held hash lists the pending directory once per batch; when a claim names that hash, the batch appends one line to that claim naming its own session and entry as an adopter, so a block one act abandoned and another act took is never reclaimed from under the second. BlockStore::reclaim(grace) runs holding blocks/.lock exclusively through flock; commit holds it shared, so no reclaim runs beside a commit and no two reclaims run together; a lock not taken at once is waited for, never skipped. For each claim older than grace: when the claim's entry is in its session's index (Index::read, one indexed read), or any adopter's entry is in its session's index, the claim is unlinked and every hash kept; otherwise every hash the claim names is unlinked unless a newer claim names it, then the claim is unlinked; a session file that is absent counts as its entry absent. The act is one durable operation: unlinks, then one sync of each touched shard directory, then one device flush, counted as HOME-038 R3's counts. Reclaim answers Reclaimed { claims_landed, claims_reclaimed, blocks_freed, bytes_freed }, and the proxy's persist worker runs it at its start and once an hour while it runs, writing the four counts to the start record and to stderr as one lys-proxy: reclaim line each time. THE SYSTEM SHALL NOT unlink a block an entry names, SHALL NOT unlink a block a claim younger than the grace names, SHALL NOT run reclaim without the exclusive lock, and SHALL NOT read any session's entries to decide; only the index rows named by the claim and its adopters are read.

**Acceptance:**
- A batch of five new blocks is committed and never landed; with the clock stepped past the grace, reclaim reports one claim reclaimed and five blocks freed, the five hashes are absent and the pending directory is empty; the same batch landed before the step reports one claim landed and nothing freed.
- A committed, unlanded batch places hash X; a second batch in another session reuses X and lands; after the grace, reclaim keeps X, reports the first claim reclaimed with its other blocks freed, and the second session's entry reads whole.
- Reclaim run in a thread while a commit holds the shared lock does not unlink until the commit returns, shown by the order of the two counted device flushes; a claim younger than the grace is neither landed nor reclaimed.
- The proxy worker's start record carries the four counts, and ingesting one call through the proxy leaves no file under blocks/pending once its report is delivered.

**Files:**
- create: crates/lys-home/src/record/pending.rs
- create: crates/lys-home/src/record/pending_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/record/block_batch.rs
- modify: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/given_statement.rs
- modify: crates/lys-home/src/record/templates.rs
- modify: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/import/content.rs
- modify: crates/lys-home/src/proxy/persist.rs
- modify: crates/lys-home/src/proxy/journal.rs
- modify: crates/lys-home/src/error.rs
- modify: crates/lys-home/src/record/block_batch_tests.rs

**Checklist:**
- C200 — BlockBatch::commit writes a claim under blocks/pending naming the session, the entry and every new hash before its first device flush, every act releases it with landed after its append, and a batch that reuses a claimed hash records itself as the claim's adopter (HOME-039 R2).
- C201 — BlockStore::reclaim, under blocks/.lock held exclusively against commits, unlinks the claim of a landed entry and frees the blocks of a claim older than the grace whose entry and adopters never landed, as one act with one counted flush, run at the proxy worker's start and hourly with its four counts reported (HOME-039 R2).

**Stories:**
- S80 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a session's recorded calls to cost the bytes that are new in each call, and a block that no entry names to be reclaimed while Lys runs, so that moving every seat onto Lys does not fill the disk.

### R3: The bound test is red at main for the growing conversation and the crashed act, and the new names have their rows

Behavioural. crates/lys/tests/store_bound.rs (LYSLOGSTORE-009 R5) gains two parts of its mix. One: a hundred of its sessions are conversations, each of forty calls through the proxy whose request body is the previous body plus a 2 KiB turn, starting at 100 KiB, with the response 4 KiB; the bound B's term for those calls is the last body plus forty times 2 KiB plus forty times two chunks of 128 KiB plus forty manifests per session, and the test fails when the raw blocks measure over it. Two: ten acts are committed and not landed (a batch committed through the test's own home handle with no append), the clock is stepped past the grace, and the test fails when the data path has not returned to within B after the next reclaim, or when any pending file remains. Both are shown red at main before R1 and R2, and the handback quotes each red with the five largest names over their share. docs/design/lys-log-store/STORES.md gains the rows LYSLOGSTORE-009 R1 asks for blocks/pending (creator BlockBatch::commit; holds claims; ended by landed or reclaim; opened by reclaim only; no growing in-process collection; bytes per act one claim file) and for chunk manifests (creator put_body; evidence kept with the record; read by call_whole; bytes per act the new chunks and one manifest). THE SYSTEM SHALL NOT measure time in either part, and SHALL NOT let the bound pass by widening B beyond the terms stated.

**Acceptance:**
- Red at main for both parts, quoted in the handback with the names over their share.
- Green at the card's head, with B, the measured total, the raw block bytes of the conversations and the four reclaim counts printed.
- STORES.md has the two rows with their file and line cites, and store_inventory (LYSLOGSTORE-009 R1) passes with blocks/pending present.

**Files:**
- modify: crates/lys/tests/store_bound.rs
- modify: docs/design/lys-log-store/STORES.md

**Checklist:**
- C202 — The bound test is red at main for a growing conversation and for a committed act that never landed, green at the card's head, and blocks/pending and the chunk manifests have their STORES.md rows (HOME-039 R3).

**Stories:**
- S80 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a session's recorded calls to cost the bytes that are new in each call, and a block that no entry names to be reclaimed while Lys runs, so that moving every seat onto Lys does not fill the disk.

## Boundaries

- No retention period for any evidence is set or implied; every block an entry names is kept, and a record written before this brief reads as it did (ADR-138).
- Parts stay one block per part and are not chunked; the block layout blocks/<hh>/<hash> stays, with blocks/pending and blocks/.lock the only new names under it.
- The session entry batch, the index, the head and the staged import's publish (ADR-108) are untouched; the session's own sync count stays four.
- A batch's device flushes stay two (HOME-038 R1); the claim rides the first flush. Reclaim is its own act with its own counted flush.
- No new crate: the chunker is written in chunks.rs with sha2, already a workspace dependency, for its table; libc stays in block_batch.rs as HOME-038 left it.
- The chunking parameters and the grace are constants in the code with their reasons beside them, not configuration; a change to any of them is a brief.
- No block is removed by any read, any open or verify_all; reclaim is the only remover and runs only under the exclusive lock.

## Verification

- cargo fmt --all exits 0 and git status --porcelain prints the same lines after it as before it.
- cargo clippy --workspace --all-targets -- -D warnings prints no warning, read from its output.
- cargo nextest run --workspace and cargo test --doc --workspace print no failure, read from their output.
- sh scripts/design/gate.sh: its printed failures, read from the output and never from the exit code, are none.
- R1's red at main (the fifty-call sum) and R3's two reds at main are quoted in the handback before the fix commits.
- Every count in R1 to R3 is read from Committed, Reclaimed and BlockIo, never from a clock or a sleep.
- The commit message names the chunking parameters, the grace, and each act of HOME-038 R2 with the line where it calls landed.
