---
type: brief
id: HOME-036
cluster: home
title: Memory view reads without deep copies or per-lantern rescans
---

# HOME-036: Memory view reads without deep copies or per-lantern rescans

> **Cluster:** home
> **Blocked by:** HOME-020 landed on main (it edits record/call.rs): the build starts only when `git log --oneline origin/main --grep=HOME-020` names its last requirement's commit.
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C186 — Records decode without a deep copy (HOME-036 R1).
> - C187 — Recall scans epilogues once for all lanterns (HOME-036 R2).
> **Stories:**
> - S77 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose memory view is read many times a session, I want each read to decode and scan once, so that recall stays light under everything else.

## Purpose

Every memory view request deep-copies stored JSON before decoding it, and rescans every epilogue once per lantern cloning its text.

## Task

Remove the repeated work named in each requirement, keeping every answer identical, each saving proved by a counting test.

## Requirements

### R1: Records decode without a deep copy

Behavioural. crates/lys-home/src/record/given.rs (about line 182), record/call.rs (about line 414) and record/given_statement.rs (about line 81) call serde_json::from_value(data.clone()). Decode from a borrow (T::deserialize(&data) / serde_json::Value as a Deserializer by reference) or take ownership where the caller is done with the value.

**Acceptance:**
- A test decodes the same stored records before and after and gets equal values; grep shows no from_value(<x>.clone()) left in crates/lys-home/src/record.

**Files:**
- modify: crates/lys-home/src/record/given.rs
- modify: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/given_statement.rs

**Checklist:**
- C186 — Records decode without a deep copy (HOME-036 R1).

**Stories:**
- S77 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose memory view is read many times a session, I want each read to decode and scan once, so that recall stays light under everything else.

### R2: Recall scans epilogues once for all lanterns

Behavioural. crates/lys-home/src/record/recall.rs rows_of (about line 108) scans every epilogue once per lantern and clones each epilogue's text. Build the lantern-to-epilogue index in one pass and borrow the text into the rows until it must be owned.

**Acceptance:**
- A counting test with L lanterns and E epilogues shows E epilogue visits, not L x E, and no text clone before output; the rows are unchanged.

**Files:**
- modify: crates/lys-home/src/record/recall.rs

**Checklist:**
- C187 — Recall scans epilogues once for all lanterns (HOME-036 R2).

**Stories:**
- S77 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose memory view is read many times a session, I want each read to decode and scan once, so that recall stays light under everything else.

## Boundaries

- SHALL NOT change any answer, stored byte, hash, signature or wire shape: every change is the same result with less work, and a test pins the result before and after.
- SHALL NOT add a timeout, deadline, sleep, poll interval, cache expiry by clock, #[allow], #[ignore] or any bypass.
- Every file stays under 500 lines of code and ast-grep stays at zero hits.
- A counting test proves each saving (calls, reads, clones, scans or round trips counted), never a wall-clock timing.
- Where a line number has moved, the function named is the target; the dev record names where it now is.

## Verification

- The full Lys gate and, where the screens change, the surface checks exit 0 at the card's head, measured by the card round.
- Each requirement's counting test is red on the base and green at the head.
