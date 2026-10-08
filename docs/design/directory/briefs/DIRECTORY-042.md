---
type: brief
id: DIRECTORY-042
cluster: directory
title: Decide a grant once per request: one decision per check, a filtered relationship read, indexes for holders and changes, a pooled SpiceDB client with a cached schema
---

# DIRECTORY-042: Decide a grant once per request: one decision per check, a filtered relationship read, indexes for holders and changes, a pooled SpiceDB client with a cached schema

> **Cluster:** directory
> **Blocked by:** DIRECTORY-025 landed on main: the build starts only when the roadmap row linking DIRECTORY-025 reads landed.
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C326 — One decision per check (DIRECTORY-042 R1), proved by a counting test that fails at the base.
> - C327 — A decision reads only the relationships of the grants on its path (DIRECTORY-042 R2), proved by a counting test that fails at the base.
> - C328 — Holders and changes are indexed, not rebuilt (DIRECTORY-042 R3), proved by a counting test that fails at the base.
> - C329 — Use events do not round-trip the mirror one by one (DIRECTORY-042 R4), proved by a counting test that fails at the base.
> - C330 — Revocation derives its deletions from the book (DIRECTORY-042 R5), proved by a counting test that fails at the base.
> - C331 — Who-holds answers from one snapshot (DIRECTORY-042 R6), proved by a counting test that fails at the base.
> - C332 — The SpiceDB schema is read when it can have changed, not before every call (DIRECTORY-042 R7), proved by a counting test that fails at the base.
> - C333 — SpiceDB calls use a pooled async client, outside the directory lock, with no timeout (DIRECTORY-042 R8), proved by a counting test that fails at the base.
> **Stories:**
> - S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

POST /grants/check is the enforcement point for every action in the estate. Today it makes the whole decision twice, each decision reads every relationship the engine holds, every permitted check writes and re-reads the mirror, and every SpiceDB call opens a fresh blocking connection, reads the schema first, and carries a 5 second timeout, all while holding the directory and grants mutexes. The cost grows with every grant anyone ever holds.

## Task

Fix the eight findings below in crates/lys-identity/src/grants and crates/lys-identity-server's grants and SpiceDB client, each proved by a counting test that fails at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: One decision per check

Behavioural. crates/lys-identity-server/src/grants.rs:306 and :151, crates/lys-identity/src/grants/authority.rs:373. With a SpiceDB engine, engine_permits calls grants.explain() and grants.check() calls explain() again. Add a lys-identity method that decides once and records the use after an engine check the caller supplies (or that takes a Permit it has already decided), keeping the order: decide, engine check, record the use. The route calls it once.

**Acceptance:**
- A test with a counting relationship store and a counting engine shows POST /grants/check makes 1 decision and 1 relationship read, where the base makes 2 of each.
- The existing grant check tests, including refusals and the use record, stay green unchanged.

**Files:**
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity/src/grants/authority.rs

**Checklist:**
- C326 — One decision per check (DIRECTORY-042 R1), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R2: A decision reads only the relationships of the grants on its path

Behavioural. crates/lys-identity/src/grants/authority.rs:418, permission.rs:202. explain() reads the entire relationship set on every decision, even when the caller holds no candidate grant, and confirm() scans the whole set twice per hop. Read lazily, only after a candidate passes effective(), and only the relationships whose resource is a grant on the path (a filtered read per grant id on SpiceDB, a range keyed by resource in the memory store); confirm looks each up instead of scanning.

**Acceptance:**
- A test with 10 000 relationships held by other people and a counting store shows a decision for a caller holding one grant reads only that grant's relationships, and a caller holding no candidate grant reads none.
- Decisions equal the base's across the existing grant and lineage tests.

**Files:**
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/permission.rs

**Checklist:**
- C327 — A decision reads only the relationships of the grants on its path (DIRECTORY-042 R2), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R3: Holders and changes are indexed, not rebuilt

Behavioural. crates/lys-identity/src/grants/projection.rs:126 and :276. held_by() filters every grant record on every decision, and changes_from() rebuilds the change map from every record on every projection. Keep a holder index (IdentityId to grant ids) and the change map maintained in apply(), as by_resource already is, rebuilt once in book_state decode.

**Acceptance:**
- A test with 10 000 grants shows held_by for one holder visits only that holder's grants, and project() after one new event visits one event, not every record.
- book_state decode rebuilds both indexes and a round-trip test shows them equal to an incrementally built book.

**Files:**
- modify: crates/lys-identity/src/grants/projection.rs

**Checklist:**
- C328 — Holders and changes are indexed, not rebuilt (DIRECTORY-042 R3), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R4: Use events do not round-trip the mirror one by one

Behavioural. crates/lys-identity/src/grants/commit.rs:133. Every permitted check records a use event, so the next explain() runs project(), which for each event re-reads the mirror revision over HTTP and writes, even though use and refused events change no relationship. Track the mirror revision locally after a successful write (re-read it only on a precondition refusal) and move the marker over a run of events that change nothing in one write.

**Acceptance:**
- A test records 100 permitted checks and then one explain(), through a counting SpiceDB double, and shows at most 1 revision read and 1 write, where the base does 100 of each.
- A test where the mirror's revision moved underneath still re-reads and settles exactly as before.

**Files:**
- modify: crates/lys-identity/src/grants/commit.rs

**Checklist:**
- C329 — Use events do not round-trip the mirror one by one (DIRECTORY-042 R4), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R5: Revocation derives its deletions from the book

Behavioural. crates/lys-identity/src/grants/commit.rs:124, permission.rs:193, revocation.rs:77. delta() for a revocation reads the whole relationship set and filters it by an O(R x tree) scan. Derive the deletions as relationships_of(grant) for every grant in the revoked subtree, which is deterministic from the book, with no read.

**Acceptance:**
- A test revoking a grant with a 3-deep subtree among 10 000 unrelated relationships shows 0 relationship reads and deletes exactly the base's set.
- Deleting a relationship that was never written (a grant issued under an already revoked line) stays a no-op on both stores, as the base.

**Files:**
- modify: crates/lys-identity/src/grants/commit.rs
- modify: crates/lys-identity/src/grants/permission.rs
- modify: crates/lys-identity/src/grants/revocation.rs

**Checklist:**
- C330 — Revocation derives its deletions from the book (DIRECTORY-042 R5), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R6: Who-holds answers from one snapshot

Behavioural. crates/lys-identity-server/src/grants.rs:381. /grants/who calls explain() once per candidate holder, re-running project() and re-reading the relationships each time. Split explain into a preparation step (settle, project, stale check, uncertain-operation checks, one read) and a per-holder step, and have who prepare once and evaluate every holder against it; check and explain use the same two steps.

**Acceptance:**
- A test with 50 candidate holders shows /grants/who makes 1 projection and 1 relationship read, where the base makes 50.
- Its answers equal the base's.

**Files:**
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity/src/grants/authority.rs

**Checklist:**
- C331 — Who-holds answers from one snapshot (DIRECTORY-042 R6), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R7: The SpiceDB schema is read when it can have changed, not before every call

Behavioural. crates/lys-identity-server/src/spicedb.rs:297, :349, :390. check(), write() and read() each POST /v1/schema/read first. Hold the kind set in SpiceDb, filled at open and updated by write_schema, and re-read it only when a kind the call needs is missing from it.

**Acceptance:**
- A test through a counting HTTP double shows 100 checks make 1 schema read, where the base makes 100.
- A test where the needed kind is missing from the held set re-reads once and then answers as the base.

**Files:**
- modify: crates/lys-identity-server/src/spicedb.rs

**Checklist:**
- C332 — The SpiceDB schema is read when it can have changed, not before every call (DIRECTORY-042 R7), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R8: SpiceDB calls use a pooled async client, outside the directory lock, with no timeout

Behavioural. crates/lys-identity-server/src/spicedb_http.rs:22, grants.rs:112-123. post_json resolves DNS, opens a new std TcpStream with Connection: close, blocks on read_to_end and sets 5 second timeouts, for every call, from async handlers, while with_grants holds the directory and grants std mutexes. Use one reqwest client with keep-alive shared by the process, await it without holding either mutex (restructure the lock scopes so no std MutexGuard lives across an await), and set no timeout: a refused, reset or closed connection is an error that names the call and its cause.

**Acceptance:**
- A test through a local counting HTTP server shows 100 calls open 1 connection, where the base opens 100.
- grep -n 'timeout\|Duration::from' crates/lys-identity-server/src/spicedb_http.rs prints nothing.
- No std::sync::MutexGuard is held across an .await in grants.rs, shown by clippy's await_holding_lock lint enabled for the crate and green.

**Files:**
- modify: crates/lys-identity-server/src/spicedb_http.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/Cargo.toml

**Checklist:**
- C333 — SpiceDB calls use a pooled async client, outside the directory lock, with no timeout (DIRECTORY-042 R8), proved by a counting test that fails at the base.

**Stories:**
- S144 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.

## Amendments

### Amendment 1: Overlap with DIRECTORY-025, found after firing; run 04667e0b cancelled before any round

- **Date:** 2026-09-28
- **By:** Waffles

DIRECTORY-025 (building from 737461d) replaces the SpiceDB HTTP client with a pure-Rust gRPC client and answers every check, why and who through one evaluator at one revision. This brief's R1, R6, R7 and R8 therefore apply to the code 025 lands, not to spicedb_http.rs: after 025, a check decides once, who is answered from one revision, the schema is not read before every call, the client holds one reused connection, and it carries no timeout (CLAUDE.md, No time limits); where 025 already meets a row, the dev record shows the counting test that proves it and the row is not rebuilt. R2 to R5 (filtered relationship read, holder and change indexes, use events without a mirror round trip per event, revocation from the book) stand, measured against the code after 025.
