---
type: brief
id: LYSLOGSTORE-005
cluster: lys-log-store
title: The proof tree keeps hashes only, is built by streaming, and never hashes a leaf twice
---

# LYSLOGSTORE-005: The proof tree keeps hashes only, is built by streaming, and never hashes a leaf twice

> **Cluster:** lys-log-store
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C20 — A hash-only proof tree built by streaming (LYSLOGSTORE-005 R1), proved by a counting test that fails at the base.
> **Stories:**
> - S8 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Purpose

The first proof after every start reads every leaf of the log into memory, copies each again into the tree, keeps every leaf's bytes resident for good although proofs need only hashes, and every later append hashes each leaf twice. It grows with the log, and the certificate and identity ledgers serve proofs from it.

## Task

Replace the leaf-holding proof tree in crates/lys-log-store (and the lys-core tree it uses) with a hash-only tree built by streaming, proved by counting tests that fail at the base. Where a file named here has moved or been split on main since 9712505, the fix follows the code to where it now lives and the dev record names the new path.

## Requirements

### R1: A hash-only proof tree built by streaming

Behavioural. crates/lys-log-store/src/frontier_log.rs:250 and :309, crates/lys-core/src/merkle/tree.rs. proof_tree() calls leaves_from(0), collecting every leaf into a Vec, reconstruct_from_raw_leaves copies each again, the tree keeps all leaf bytes for good, and append hashes each new leaf twice (frontier.push and tree.append_raw). Add a tree type that holds only the RFC 6962 leaf and interior hashes, is fed the leaf hashes one at a time from the store, and on append takes the hash frontier.push already computed. Inclusion and consistency proofs are byte-identical to the current tree's.

**Acceptance:**
- A test over a 100 000-leaf log shows proof_tree() holds no leaf bytes (the tree's retained size is hashes only: at most 2 x 32 bytes per leaf) and never holds more than one leaf in memory while building.
- A test shows each append hashes the leaf once, where the base hashes it twice.
- Property tests over random logs show every inclusion and consistency proof equals the base tree's and verifies with lys-core's verifier.

**Files:**
- modify: crates/lys-log-store/src/frontier_log.rs
- modify: crates/lys-core/src/merkle/tree.rs

**Checklist:**
- C20 — A hash-only proof tree built by streaming (LYSLOGSTORE-005 R1), proved by a counting test that fails at the base.

**Stories:**
- S8 (Estate operator, Runs Lys behind every agent and session) — As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

## Boundaries

- SHALL NOT change any answer, refusal, record, wire format, stored byte or audit line: every existing test stays as it is and green; a changed expected value is a defect, not an update.
- SHALL NOT add any timeout, deadline, sleep, poll interval or bound in seconds, and SHALL remove any this brief names (CLAUDE.md, No time limits).
- SHALL NOT add #[allow], #[ignore], a _-prefixed binding or any other bypass; clippy with -D warnings, ast-grep scan and the file-length leg stay green, and no file passes 500 lines of code.
- SHALL NOT add a dependency that is not already in Cargo.lock unless the dev record names it and why nothing in the lock serves.
- SHALL NOT measure speed with a clock in any test: each requirement is proved by counting the work done (reads, parses, scans, syscalls through a counting double, lock acquisitions), never by elapsed time.

## Verification

- cargo fmt --all --check, both clippy legs with -D warnings, cargo test --workspace --all-features --no-fail-fast, both cargo doc legs, ast-grep scan --config sgconfig.yml, sh scripts/design/gate.sh and (once LYSGATE-002 has landed) sh scripts/file-length.sh all exit 0 at the card's head, measured by the card round, never by the builder's own run.
- Every counting test named in an acceptance line fails at the card's base commit and passes at its head: the dev record names each test and quotes its failing assertion at the base.
