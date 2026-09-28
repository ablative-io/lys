---
type: brief
id: DIRECTORY-043
cluster: directory
title: Identity server hot paths: indexed runtime state, no snapshot clones, no blocking I/O on async workers, one pass per memory view, receipts without re-reading the log
---

# DIRECTORY-043: Identity server hot paths: indexed runtime state, no snapshot clones, no blocking I/O on async workers, one pass per memory view, receipts without re-reading the log

> **Cluster:** directory
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C334 — Runtime state is indexed (DIRECTORY-043 R1), proved by a counting test that fails at the base.
> - C335 — Snapshots serialise by reference (DIRECTORY-043 R2), proved by a counting test that fails at the base.
> - C336 — Directory writes never block an async worker (DIRECTORY-043 R3), proved by a counting test that fails at the base.
> - C337 — A session listing locks once and clones nothing it does not return (DIRECTORY-043 R4), proved by a counting test that fails at the base.
> - C338 — A memory view reads each session once, off the async worker (DIRECTORY-043 R5), proved by a counting test that fails at the base.
> - C339 — A receipt is rebuilt from the stored coordinate, not by re-reading leaves (DIRECTORY-043 R6), proved by a counting test that fails at the base.
> **Stories:**
> - S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

The identity server answers every agent's runtime reports, directory writes, memory views and agent views. It scans every session ever reported on each report, deep-copies whole stores to snapshot them, runs fsync on async workers under a mutex every route waits on, reads every session twice per memory view, and re-reads up to 1 024 leaf files to rebuild one receipt.

## Task

Fix the six findings below in crates/lys-identity-server and crates/lys-identity/src/restart.rs, each proved by a counting test that fails at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: Runtime state is indexed

Behavioural. crates/lys-identity-server/src/runtime_state.rs:142, :148, :157, runtime_store.rs:205. Held::operation() walks every report of every session ever kept on each incoming report, and session() and hold() scan every session. Keep maps from operation id and session id to slot beside sessions, serde(skip), rebuilt once in decode(), fold() and settle(), updated in hold(); equality compares sessions only.

**Acceptance:**
- A test with 10 000 kept sessions shows one report visits 1 session, where the base visits all 10 000.
- The stored snapshot bytes equal the base's.

**Files:**
- modify: crates/lys-identity-server/src/runtime_state.rs
- modify: crates/lys-identity-server/src/runtime_store.rs

**Checklist:**
- C334 — Runtime state is indexed (DIRECTORY-043 R1), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R2: Snapshots serialise by reference

Behavioural. runtime_state.rs:200, requests_state.rs:70, certificates_store.rs:163, teams_state.rs:246, stops_state.rs:121, service_accounts_state.rs:169, reviews_state.rs:95. Each encode builds Sealed { held: self.clone() }, a deep copy of the whole folded state, only to serialise it. Serialise a borrowing SealedRef<'a> { format, held: &'a Held } and keep the owned Sealed for decode only.

**Acceptance:**
- For each of the seven stores a test shows the encoded bytes are identical to the base's for its existing fixture.
- grep over the seven files for 'held: self.clone()' prints nothing.

**Files:**
- modify: crates/lys-identity-server/src/runtime_state.rs
- modify: crates/lys-identity-server/src/requests_state.rs
- modify: crates/lys-identity-server/src/certificates_store.rs
- modify: crates/lys-identity-server/src/teams_state.rs
- modify: crates/lys-identity-server/src/stops_state.rs
- modify: crates/lys-identity-server/src/service_accounts_state.rs
- modify: crates/lys-identity-server/src/reviews_state.rs

**Checklist:**
- C335 — Snapshots serialise by reference (DIRECTORY-043 R2), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R3: Directory writes never block an async worker

Behavioural. crates/lys-identity-server/src/routes.rs:231. with_directory() takes a std Mutex and runs the mutation, a leaf file write with fsync and on cadence a snapshot, inside the async handler, so every directory route waits behind one fsync on a tokio worker. Run the locked section on a single writer thread fed by a channel (or spawn_blocking), with reads served without waiting on a write in progress where the state allows it.

**Acceptance:**
- A test holds a directory write inside a blocked counting filesystem and shows a concurrent directory read route answers before the write is released.
- No std::sync::MutexGuard is held across an .await in routes.rs, shown by clippy's await_holding_lock lint enabled for the crate and green.

**Files:**
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C336 — Directory writes never block an async worker (DIRECTORY-043 R3), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R4: A session listing locks once and clones nothing it does not return

Behavioural. crates/lys-identity-server/src/runtime_api.rs:209, network_store.rs:146, runtime_store.rs:252. view() locks the network mutex, searches machines linearly and clones a whole Machine per session listed, and report() returns standing(), a clone of the whole Tracked with its launch JSON, which launch_api clones again. Lock the network store once per listing and build id to (name, runtime) once; have report() answer a view built from borrowed data.

**Acceptance:**
- A test listing 500 sessions shows 1 network lock acquisition, where the base takes 500.
- Listing and report answers equal the base's.

**Files:**
- modify: crates/lys-identity-server/src/runtime_api.rs
- modify: crates/lys-identity-server/src/network_store.rs
- modify: crates/lys-identity-server/src/runtime_store.rs
- modify: crates/lys-identity-server/src/launch_api.rs

**Checklist:**
- C337 — A session listing locks once and clones nothing it does not return (DIRECTORY-043 R4), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R5: A memory view reads each session once, off the async worker

Behavioural. crates/lys-identity-server/src/memory_api.rs:203, lys-home recall.rs:160, given.rs:244. GET /agents/{id}/memory calls recall_all and then last_given, each opening and parsing every session of the home with blocking std fs calls in the async handler. Add a lys-home function that derives both lanterns and last-given from one reader per session in one pass, and run it on spawn_blocking.

**Acceptance:**
- A test with a home of 20 sessions shows the route opens each session file once, where the base opens each twice.
- The view's answer equals the base's.

**Files:**
- modify: crates/lys-identity-server/src/memory_api.rs
- modify: crates/lys-home/src/recall.rs
- modify: crates/lys-home/src/given.rs

**Checklist:**
- C338 — A memory view reads each session once, off the async worker (DIRECTORY-043 R5), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R6: A receipt is rebuilt from the stored coordinate, not by re-reading leaves

Behavioural. crates/lys-identity/src/restart.rs:328, read_api.rs:233, directory.rs:165 and :395, commit.rs:158. Ledger::entry() reads every leaf file from the nearest 1 024-leaf checkpoint and rehashes each to rebuild one receipt, on every agent view, every retried operation (twice in transition()) and for grants. Keep each leaf's coordinate (index, tree size, root, 32 bytes) when it is folded, beside the log, so entry() reads one leaf and verifies it; in transition() reuse the first answered() result.

**Acceptance:**
- A test with a 5 000-leaf ledger shows entry() for leaf 4 000 reads 1 leaf file, where the base reads up to 1 024.
- The receipts equal the base's byte for byte, and a coordinate store removed from disk is rebuilt from the log on open and gives the same receipts.

**Files:**
- modify: crates/lys-identity/src/restart.rs
- modify: crates/lys-identity-server/src/read_api.rs
- modify: crates/lys-identity/src/directory.rs

**Checklist:**
- C339 — A receipt is rebuilt from the stored coordinate, not by re-reading leaves (DIRECTORY-043 R6), proved by a counting test that fails at the base.

**Stories:**
- S145 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.
