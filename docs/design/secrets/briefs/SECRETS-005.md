---
type: brief
id: SECRETS-005
cluster: secrets
title: The secrets proxy's request path: hashed handle lookup, indexed lineage, broker work off the async workers, cached routes, one-pass redaction, memoised views
---

# SECRETS-005: The secrets proxy's request path: hashed handle lookup, indexed lineage, broker work off the async workers, cached routes, one-pass redaction, memoised views

> **Cluster:** secrets
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C39 — A presented token is found by a hashed lookup (SECRETS-005 R1), proved by a counting test that fails at the base.
> - C40 — Lineage and endings are indexed (SECRETS-005 R2), proved by a counting test that fails at the base.
> - C41 — Broker work never blocks an async worker, and the permission check runs outside the lock (SECRETS-005 R3), proved by a counting test that fails at the base.
> - C42 — Routes and services are loaded once (SECRETS-005 R4), proved by a counting test that fails at the base.
> - C43 — Answers are capped and redacted in one pass (SECRETS-005 R5), proved by a counting test that fails at the base.
> - C44 — Views ask each permission once per request (SECRETS-005 R6), proved by a counting test that fails at the base.
> **Stories:**
> - S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

lys-secrets sits in front of every use of a credential. Every presentation scans every handle ever issued, every proxied call takes the broker mutex three times inside async handlers and fsyncs audit lines and an anchor under it while a blocking SpiceDB call waits, routes.json is re-read per request, answers are buffered without a cap and redacted once per secret, and the audit and grants views repeat the same permission checks per row.

## Task

Fix the six findings below in crates/lys-secrets, each proved by a counting test that fails at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: A presented token is found by a hashed lookup

Behavioural. crates/lys-secrets/src/broker/admit.rs:37, using.rs:118. find() walks every HandleRecord ever issued and unhexes each stored digest to compare it, on every presentation, and a refusal scans again. Keep the digest as [u8; 32] (decoded once at load) and a map from token digest to handle id, updated on issue and derive; admit() hands the found record out so the refusal path does not search again. The lookup is by SHA-256 of a 32-byte random token, so a map reveals nothing through timing.

**Acceptance:**
- A test with 10 000 issued handles shows one presentation compares 1 record, where the base compares 10 000, and a refused presentation searches once.
- Admission and refusal answers equal the base's.

**Files:**
- modify: crates/lys-secrets/src/broker/admit.rs
- modify: crates/lys-secrets/src/broker/using.rs

**Checklist:**
- C39 — A presented token is found by a hashed lookup (SECRETS-005 R1), proved by a counting test that fails at the base.

**Stories:**
- S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R2: Lineage and endings are indexed

Behavioural. crates/lys-secrets/src/broker/ending.rs:210, :223, :241. standing_below() walks every handle and builds chain() twice per handle; ended_under and ended_with each scan every handle. Keep a parent to children index maintained on derive and walk down from the root; index endings by (person, operation) in fold; chain() yields borrowed ids.

**Acceptance:**
- A test with 10 000 handles shows standing_below for a root with 3 descendants visits 4 handles, and ended_under and ended_with visit none but their answer.
- Answers equal the base's.

**Files:**
- modify: crates/lys-secrets/src/broker/ending.rs

**Checklist:**
- C40 — Lineage and endings are indexed (SECRETS-005 R2), proved by a counting test that fails at the base.

**Stories:**
- S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R3: Broker work never blocks an async worker, and the permission check runs outside the lock

Behavioural. crates/lys-secrets/src/bin/lys-secrets/serve.rs:207-269, audit.rs:318, :399. Every proxied call takes the broker's std Mutex three times inside async handlers and does audit fsyncs, an anchor write with temp-file fsync, rename and directory fsync per line, and a blocking SpiceDB call under it. Run the broker on a dedicated thread behind a channel (or spawn_blocking), make the permission check before taking the broker (at_forward_boundary checks again), and write the anchor once per request after its settlement line instead of once per line.

**Acceptance:**
- A test through a counting filesystem shows one proxied call writes the anchor once, where the base writes it once per audit line.
- A test holds a SpiceDB permission check open and shows a concurrent /_lys route answers before it is released.
- No std::sync::MutexGuard is held across an .await in serve.rs, shown by clippy's await_holding_lock lint enabled for the crate and green.

**Files:**
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/audit.rs

**Checklist:**
- C41 — Broker work never blocks an async worker, and the permission check runs outside the lock (SECRETS-005 R3), proved by a counting test that fails at the base.

**Stories:**
- S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R4: Routes and services are loaded once

Behavioural. crates/lys-secrets/src/bin/lys-secrets/serve.rs:177, view.rs:34, callers.rs:108. forward() reads and parses routes.json and clones one Route on every proxied request; the secrets listing and screen-service requests re-read routes.json and services.json. Load both at serve start into an ArcSwap-style snapshot (Arc replaced on add_route and trust_service), and hold an Arc<Route> across the upstream call instead of cloning.

**Acceptance:**
- A test through a counting filesystem shows 100 proxied requests read routes.json 0 times after start, and a route added through add_route is served on the next request.
- Answers equal the base's.

**Files:**
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/view.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/callers.rs

**Checklist:**
- C42 — Routes and services are loaded once (SECRETS-005 R4), proved by a counting test that fails at the base.

**Stories:**
- S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R5: Answers are capped and redacted in one pass

Behavioural. crates/lys-secrets/src/bin/lys-secrets/serve.rs:300, :361, :379. The upstream answer is read whole with no cap, copied, then rebuilt once per hidden secret by a byte-at-a-time redact; header checks run a naive window scan per secret; route.header is lowercased per request header. Cap the upstream body as requests already are (MAX_BODY, refused by name when exceeded), redact every secret in one pass (memchr::memmem or aho-corasick), and lowercase route.header once. If the body is streamed, a secret split across chunks is still redacted by carrying the longest secret's length less one byte between chunks.

**Acceptance:**
- A test with 5 secrets and a 1 MiB answer shows the answer is scanned once, and every secret, including one split across a chunk boundary if streamed, is redacted exactly as the base redacts it.
- An upstream answer over the cap is refused by a named error, never truncated silently.

**Files:**
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/Cargo.toml

**Checklist:**
- C43 — Answers are capped and redacted in one pass (SECRETS-005 R5), proved by a counting test that fails at the base.

**Stories:**
- S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

### R6: Views ask each permission once per request

Behavioural. crates/lys-secrets/src/bin/lys-secrets/view.rs:117, :167, held.rs:48, broker/scope.rs:82-92, files.rs:218. The audit view calls discovers() for each of up to 200 lines, each making up to three permission checks plus a scope check, and FileGrants re-reads and re-parses grants.json per check; the grants view, held_by and listing do the same per row. Memoise discovers(identity, secret) per request, and have FileGrants parse grants.json once per request (or cache it keyed on the file's modification and size).

**Acceptance:**
- A test of a 200-line audit view naming 3 secrets shows at most 9 permission checks and 1 parse of grants.json, where the base makes up to 600 checks and as many parses.
- View answers equal the base's.

**Files:**
- modify: crates/lys-secrets/src/bin/lys-secrets/view.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/held.rs
- modify: crates/lys-secrets/src/broker/scope.rs
- modify: crates/lys-secrets/src/files.rs

**Checklist:**
- C44 — Views ask each permission once per request (SECRETS-005 R6), proved by a counting test that fails at the base.

**Stories:**
- S22 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.
