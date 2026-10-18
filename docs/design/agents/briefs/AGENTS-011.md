---
type: brief
id: AGENTS-011
cluster: agents
title: The bridge test's fixture survives the bridge closing a lane before the second reply is written
---

# AGENTS-011: The bridge test's fixture survives the bridge closing a lane before the second reply is written

> **Cluster:** agents
> **Blocked by:** Found 18 Oct 2026 10:1x AEDT in the Lys workspace battery on the 205 (Linux, load 41) at ff907a65: crates/lys-runner/src/dial/bridge/tests.rs two_next_bridges_are_open_before_either_request_is_answered failed once in four runs with the fixture's own error, "Broken pipe (os error 32)", after the runner said "the server closed the connection before answering". Read in source: the fixture's four workers each write their reply after the barrier (tests.rs lines 47-50); the bridge ends on the first 403 fixture_finished and stop() (bridge.rs lines 86-110) closes the other lane's socket; when the second worker's write! or flush lands after that close, Linux answers EPIPE, the worker returns Err, and remote.join()?? (line 83) reports it as the test's failure. Green 3/3 on the same box a minute later: a race in the fixture, not in the bridge. Lys is moving to a Linux box, so a test red only there is a defect on the venue Lys will live on.
> **Design anchor:**
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C751 — The bridge test's fixture treats a reply write refused BrokenPipe or ConnectionReset after the barrier as the bridge's correct close of a lane already refused on the other, proven red first with the second write ordered after the bridge's join; green 20 of 20 on Linux and macOS; no sleep, retry or serial marker (AGENTS-011 R1).
> **Stories:**
> - S425 (An operator, a person running the estate) — As an operator, I want the runner's own tests to be green on every run on the Linux box Lys moves to, so that a battery's red always names a real defect and never a race in a fixture.

## Purpose

The bridge test proves that both polling lanes arrive before either request is answered. Its fixture must not fail the test when the bridge, correctly refused on one lane, closes the other before the fixture's reply to it is written: that close is the bridge doing its job, and a write refused by a peer that has gone is the fixture's expected outcome after the barrier, never a verdict on the runner.

## Task

Order the fixture's second write against the bridge's close, red first, so the test is green on every run on Linux and macOS, and reports a real failure only when the bridge misbehaves.

## Requirements

### R1: A reply write refused by a closed peer after the barrier is the expected outcome

Behavioural, in the test only. In two_next_bridges_are_open_before_either_request_is_answered, after the barrier releases the two next-poll workers, a worker whose write! or flush of its 403 reply fails with BrokenPipe or ConnectionReset returns Ok: the bridge was refused on the other lane and closed this one, which is the behaviour under test. Every other error on every other write (the epoch GET's reply, a read of a request head) stays an Err with its words. The order is proven red first: a test variant (or the same test with the fixture's second write deliberately delayed until the bridge thread has joined) that writes the second reply only after the bridge has closed the lane, which fails with Broken pipe before the change and passes after it; the delay is an ordering on the fixture's side (a channel or the joined bridge handle), never a sleep.

**Acceptance:**
- Red first in crates/lys-runner/src/dial/bridge/tests.rs: with the second worker's write ordered after the bridge's join, the test fails with "Broken pipe" (Linux) before the change and is green after it; the ordered variant stays as a test of its own, named for the order it forces
- two_next_bridges_are_open_before_either_request_is_answered green 20 of 20 under `cargo nextest run -p lys-runner -E 'test(two_next_bridges)'` repeated, on the 205 (Linux) and on the Mac; counts stated
- a_lost_reply_connection_is_not_a_refusal_and_is_not_replayed and the rest of lys-runner unchanged and green
- no sleep, no timeout and no retry added anywhere in the test

**Files:**
- modify: crates/lys-runner/src/dial/bridge/tests.rs

**Checklist:**
- C751 — The bridge test's fixture treats a reply write refused BrokenPipe or ConnectionReset after the barrier as the bridge's correct close of a lane already refused on the other, proven red first with the second write ordered after the bridge's join; green 20 of 20 on Linux and macOS; no sleep, retry or serial marker (AGENTS-011 R1).

**Stories:**
- S425 (An operator, a person running the estate) — As an operator, I want the runner's own tests to be green on every run on the Linux box Lys moves to, so that a battery's red always names a real defect and never a race in a fixture.

## Boundaries

- SHALL NOT change bridge.rs: the bridge's stop on the first refusal, closing the other lane, is right and is what the test proves.
- SHALL NOT mark the test serial, flaky, ignored or Linux-only, and SHALL NOT add a retry or a sleep: the order is made explicit on the fixture's side.
- SHALL NOT widen what the fixture forgives beyond BrokenPipe and ConnectionReset on a reply write after the barrier.

## Verification

- cargo nextest run -p lys-runner at the head, 20 repeats of the two bridge tests on the 205 and on the Mac with counts; cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --all; file-length gate.
- sh scripts/design/gate.sh at the committed source; parse the validate, coverage and rendered-comparison lines, keeping only failures, never the exit code alone.
- Worst credible failure if this were used in a hospital: a fixture race read as a runner defect holds a release carrying a real fix, or a test that is green by luck hides a bridge that closes a lane before the refusal is read; the ordered variant pins the bridge's close explicitly, so the test is red only for the bridge and never for the venue.
