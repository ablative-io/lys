---
type: brief
id: HOME-035
cluster: home
title: Stop the record replaying and re-reading: indexed call lookup, one open check, batched durable appends, streamed rendering
---

# HOME-035: Stop the record replaying and re-reading: indexed call lookup, one open check, batched durable appends, streamed rendering

> **Cluster:** home
> **Blocked by:** HOME-020 landed on main: the build starts only when `git log --oneline origin/main --grep=HOME-020` names its R2, R4 and R6 commits.
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C181 — Call idempotency is a lookup, not a replay (HOME-035 R1), proved by a counting test that fails at the base.
> - C182 — Opening a session checks the index without a syscall per row (HOME-035 R2), proved by a counting test that fails at the base.
> - C183 — Bulk writers append in batches with one fsync per file (HOME-035 R3), proved by a counting test that fails at the base.
> - C184 — Rendering a launch moves data once and hashes what it writes as it writes (HOME-035 R4), proved by a counting test that fails at the base.
> - C185 — Call parts are borrowed and each body file read once (HOME-035 R5), proved by a counting test that fails at the base.
> **Stories:**
> - S76 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

Every model call and every session open in lys-home repeats work it has already done: the call idempotency check re-reads every call entry in the session, opening a session reads one byte per index row, bulk writers fsync three files per entry, and a launch renders through double clones and then re-reads what it wrote. Lys runs behind every agent session, so this cost is paid everywhere, all the time.

## Task

Fix the five findings below in crates/lys-home, each proved by a counting test that fails at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: Call idempotency is a lookup, not a replay

Behavioural. crates/lys-home/src/record/call.rs:167, index.rs:375. find_call runs on every ingest_call, ingest_call_files and ingest_outcome and calls session.customs_everywhere(CUSTOM_CALL), which walks every index row and seeks, reads and parses every lys.call entry to compare call_id. Put the call_id in the index row (an optional key column beside custom, written by append for lys.call entries and read at open with the rest of the index), so the check is a hash lookup on data already loaded. An index written before this change, without the column, is read once at open to fill the lookup and never again. No call entry is re-read to deduplicate.

**Acceptance:**
- A test ingests 200 calls into one session and, through a counting reader over the session file, shows that the 201st ingest_call reads no call entry from the session file (0 entry reads), where the base reads 200.
- A test with an index written in the old shape shows the duplicate call is still refused exactly as before and the old index is upgraded in one pass.

**Files:**
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/index.rs

**Checklist:**
- C181 — Call idempotency is a lookup, not a replay (HOME-035 R1), proved by a counting test that fails at the base.

**Stories:**
- S76 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R2: Opening a session checks the index without a syscall per row

Behavioural. crates/lys-home/src/record/index.rs:171. from_cached, run by Index::read and Index::load on every Session::open and SessionReader::open, seeks and reads one byte per row to confirm each row ends in a newline. Keep the contiguity check and the last-row-ends-at-file-length check, check the newline of the final row, and detect a foreign index of equal length by a digest of the session file's final row bytes (or its first and last rows) stored in the index head; keep the full per-row check only in verify (ship and fetch).

**Acceptance:**
- A test opens a session of 10 000 rows through a counting file and shows at most 4 reads of the session file on open, where the base does 10 000.
- The existing tests for a truncated file and for an index from another file stay green, and a new test with a foreign index of the same total length is still refused on open.

**Files:**
- modify: crates/lys-home/src/record/index.rs

**Checklist:**
- C182 — Opening a session checks the index without a syscall per row (HOME-035 R2), proved by a counting test that fails at the base.

**Stories:**
- S76 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R3: Bulk writers append in batches with one fsync per file

Behavioural. crates/lys-home/src/record/session.rs:212, helpers.rs:42, index.rs:315, index/head.rs:36. Every append_entry, append and append_copied reopens and fsyncs the session file, the index file and a new head file with a directory fsync. Keep the session and index files open on Session; add a batch append that writes all lines and fsyncs once, writes all index rows and fsyncs once, then writes the head once, keeping the durable order (lines, then rows, then head). Use it in import_claude_code, write_fork, handover write_successor and fetch arrivals; import appends without moving the head and moves it once at the end, as import.rs:323 already does.

**Acceptance:**
- A test imports a 500-entry Claude Code transcript through a counting filesystem double and shows 1 fsync of the session file, 1 of the index file and 1 head write, where the base does 500 of each.
- A crash-ordering test that fails the batch after the lines are durable but before the rows shows the next open recovers the session exactly as the existing recovery tests require.

**Files:**
- modify: crates/lys-home/src/record/session.rs
- modify: crates/lys-home/src/record/helpers.rs
- modify: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/record/index/head.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/record/fork.rs
- modify: crates/lys-home/src/record/handover.rs

**Checklist:**
- C183 — Bulk writers append in batches with one fsync per file (HOME-035 R3), proved by a counting test that fails at the base.

**Stories:**
- S76 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R4: Rendering a launch moves data once and hashes what it writes as it writes

Behavioural. crates/lys-home/src/harness/claude_code/render.rs:176, launch.rs:159, given.rs. render_claude_code clones every assistant message's content array and then each part again, keeps the whole file as Vec<Value>, writes with two syscalls per record, and render_launch then re-reads and re-hashes every file it wrote, and resolve_given reads instructions.md and mcp.json a third time. Take entries by value and move the values into records (or serialise from borrowed data), stream records through a BufWriter wrapped in a SHA-256 hashing writer so each file's hash is known when writing ends, and pass those hashes to the manifest and resolve_given instead of re-reading.

**Acceptance:**
- A test renders a 1 000-message session through a counting filesystem and shows each written file is opened once for writing and never read back during render_launch, and that the recorded hashes equal a SHA-256 of the files on disk.
- The rendered bytes are identical to the base's for the existing render fixtures.

**Files:**
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/launch.rs
- modify: crates/lys-home/src/given.rs

**Checklist:**
- C184 — Rendering a launch moves data once and hashes what it writes as it writes (HOME-035 R4), proved by a counting test that fails at the base.

**Stories:**
- S76 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R5: Call parts are borrowed and each body file read once

Behavioural. crates/lys-home/src/record/call/parts.rs:44, parts.rs:36, call.rs:245. request_parts_of deep-clones every part only to serialise and hash it, and ingest_call_files reads each body file twice (once to parse, once in blocks.put_file). Return parts borrowed from the parsed request (or move them out of it), read each body file once into memory, and hash, store and parse from that one buffer.

**Acceptance:**
- A test ingests a call with 3 body files through a counting reader and shows each body file read exactly once, where the base reads each twice.
- The stored blocks' digests equal the base's for the existing call fixtures.

**Files:**
- modify: crates/lys-home/src/record/call/parts.rs
- modify: crates/lys-home/src/record/call.rs

**Checklist:**
- C185 — Call parts are borrowed and each body file read once (HOME-035 R5), proved by a counting test that fails at the base.

**Stories:**
- S76 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.
