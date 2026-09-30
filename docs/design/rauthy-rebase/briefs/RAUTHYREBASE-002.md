---
type: brief
id: RAUTHYREBASE-002
cluster: rauthy-rebase
title: An interrupted fork gate is never a pass: the wrapper waits for its container to be terminal, and success needs a completed verdict bound to this run and this commit
---

# RAUTHYREBASE-002: An interrupted fork gate is never a pass: the wrapper waits for its container to be terminal, and success needs a completed verdict bound to this run and this commit

> **Cluster:** rauthy-rebase
> **Blocked by:** The fork's container gate, .land/test.sh and .land/identity-link-gate.sh as carried by ablative-io/rauthy pull request 3 (branch archie/audit-link-typed-args, head 1b2be5b3, which contains 76a394eb). This card builds on that head, or on the fork's ablative branch once pull request 3 has landed there, and changes nothing else in it.
> **Design anchor:**
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> **Checklist:**
> - C17 — The base wrapper's pass on a detached, still-running builder is shown red against real Docker (RAUTHYREBASE-002 R1).
> - C18 — A result comes only from a stopped container, and Running true with ExitCode 0 is refused by name (RAUTHYREBASE-002 R2).
> - C19 — Success needs a completed verdict bound to this invocation and commit, and missing, truncated, stale or wrong-commit verdicts are refused by name (RAUTHYREBASE-002 R3).
> - C20 — Cancellation reaches everything the gate owns, keeps its result, and cleanup never overwrites the outcome (RAUTHYREBASE-002 R4).
> - C21 — The venue's generic gate receipt is surveyed for the same defect (RAUTHYREBASE-002 R5).
> **Stories:**
> - S7 (Reviewer, Reads the rebase pull request) — As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

## Purpose

The Rauthy fork's tests leg (.land/test.sh at 76a394eb) creates a PostgreSQL container and a builder container, runs `docker start -a` on the builder, then reads `docker inspect --format '{{.State.ExitCode}}'` and exits with it. It never checks that the container has stopped. A running container reports ExitCode 0, so when the attached client goes away first, the wrapper reads 0 and passes. On 28 September 2026 the gate for exact source 76a394eb30bca5cbd516d90d110449bd1c57d008 was cancelled on Waffles' word (receipt /Users/deanwhiting/.aion/clones/src-logs/chippy-rauthy-20260928T100850Z). TERM to the builder container left its compile running, and TERM to the owned `docker start` client detached it. The wrapper then read ExitCode 0 and cleaned up, and both it and the outer runner wrote exit 0. The log ended during the wasm build, before the frontend, Clippy, tests or any verdict. Chippy classified the run as cancelled, never green, but nothing in the gate itself would have stopped it being taken as a pass. The in-container script (.land/identity-link-gate.sh) ends on `cargo test`, and its own exit is the only sign that every leg ran.

## Task

Make the fork gate's success mean that every leg of this invocation finished and passed on this exact commit. The wrapper takes a result only from a container Docker reports as stopped. Success requires a completed verdict that the in-container script writes after its last leg and that names this invocation and commit. Cancellation reaches every process and container the wrapper owns and keeps its cancelled result. Cleanup never overwrites the outcome. Prove it with a real regression against Docker that detaches the attached client while the container is still running, driven by events and not by a clock. Survey the venue's generic gate receipt boundary and record whether it has the same defect.

## Requirements

### R1: Red first, against real Docker

The report SHALL quote the regression of R2 to R5 failing on the base head, with the command, its exit code and the failing lines. In particular, the base wrapper exits 0 after its attached client is detached while the builder container is still running.

**Acceptance:**
- Each new case is named with its failure on the base head, quoted from the run.
- The detached-client case on the base shows the wrapper's exit 0 while `docker inspect` on the builder reported State.Running true.

**Files:**
- create: docs/design/identity/reports/RAUTHYREBASE-002-gate-verdict.md

**Checklist:**
- C17 — The base wrapper's pass on a detached, still-running builder is shown red against real Docker (RAUTHYREBASE-002 R1).

**Stories:**
- S7 (Reviewer, Reads the rebase pull request) — As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

### R2: A result comes only from a stopped container

WHEN `docker start -a` returns for the builder, for any reason, THE SYSTEM SHALL establish from Docker that the container has stopped before reading its exit code. It waits on Docker's own stop signal (`docker wait` on the owned container), never on a sleep or a poll. If the container is still running because the attached client went away, the gate SHALL refuse by name (client_detached while the builder was running), stop the owned container, and exit non-success. A state of Running true with ExitCode 0 is never read as a result.

**Acceptance:**
- The regression launches the real wrapper on a fixture tree against Docker. It waits for the fixture leg's started line on the attached stream, sends TERM to the wrapper's `docker start` client while the container is running, and asserts a non-success exit with the refusal named in its output.
- A unit case feeds the wrapper's result reader State.Running true with ExitCode 0 and gets the named refusal.
- A builder that stops on its own with a non-zero code yields that failure, even when every cleanup step succeeds.

**Files:**
- create: vendor/rauthy/.land/tests/interrupted-gate.sh
- create: vendor/rauthy/.land/tests/fixture/identity-link-gate.sh
- modify: vendor/rauthy/.land/test.sh

**Checklist:**
- C18 — A result comes only from a stopped container, and Running true with ExitCode 0 is refused by name (RAUTHYREBASE-002 R2).

**Stories:**
- S7 (Reviewer, Reads the rebase pull request) — As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

### R3: Success needs a completed verdict bound to this invocation and commit

THE SYSTEM SHALL mint an invocation identifier in the wrapper for each run, pass it into the container, and have .land/identity-link-gate.sh write a verdict file only after its last required leg has finished with success. The verdict names the invocation identifier, the exact commit (`git rev-parse HEAD` of the tree under test) and each required leg with its exit status, and it is written to a path unique to the invocation and moved into place whole. The wrapper SHALL report success only when the builder has stopped with exit 0 and that verdict is present, parses, names this invocation and this commit, and lists every required leg as passed. A verdict that is missing, truncated, stale from another invocation, for another commit, or missing any leg is refused by name. A line printed by a test, or a verdict file left by an earlier run, is never enough.

**Acceptance:**
- Injected cases for a missing, a truncated, a stale (other invocation) and a wrong-commit verdict, and a verdict with one leg absent, each yield their own named refusal and non-success.
- Killing the in-container script just after its last leg and before the verdict is published yields non-success naming the missing verdict.
- A genuine run in which every leg passes yields exactly one verdict that matches its invocation and commit, and the wrapper exits 0.

**Files:**
- create: vendor/rauthy/.land/tests/interrupted-gate.sh
- create: vendor/rauthy/.land/tests/fixture/identity-link-gate.sh
- modify: vendor/rauthy/.land/test.sh
- modify: vendor/rauthy/.land/identity-link-gate.sh

**Checklist:**
- C19 — Success needs a completed verdict bound to this invocation and commit, and missing, truncated, stale or wrong-commit verdicts are refused by name (RAUTHYREBASE-002 R3).

**Stories:**
- S7 (Reviewer, Reads the rebase pull request) — As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

### R4: Cancellation reaches everything the gate owns and keeps its result

WHEN the wrapper receives TERM, INT or HUP, THE SYSTEM SHALL stop the owned builder and PostgreSQL containers, and any process it started, and SHALL exit with the cancellation's own status and a named gate_cancelled line, never 0. Cleanup SHALL remove only the network and containers named for this invocation, SHALL first save the builder's log to a file kept for the venue's evidence, and SHALL never turn a non-success into success. A cleanup failure after an otherwise passing run is reported by name as non-success, while the gate's own outcome stays in the output.

**Acceptance:**
- Interrupting the wrapper during the fixture's wasm, frontend, Clippy and test legs each ends in gate_cancelled with a non-zero exit, and no container or network named for that invocation remains.
- An injected cleanup failure after a failed leg keeps the failure, and after a passing run it is named as a cleanup failure and exits non-success.
- The builder's log from each interrupted run is kept where the venue collects evidence.

**Files:**
- create: vendor/rauthy/.land/tests/interrupted-gate.sh
- modify: vendor/rauthy/.land/test.sh

**Checklist:**
- C20 — Cancellation reaches everything the gate owns, keeps its result, and cleanup never overwrites the outcome (RAUTHYREBASE-002 R4).

**Stories:**
- S7 (Reviewer, Reads the rebase pull request) — As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

### R5: The venue's generic gate receipt is surveyed

The report SHALL show where the venue's generic gate step (src_gate) turns a gate's exit into a receipt, and whether an unfinished or detached gate can reach a success receipt there. It SHALL record evidence either that it already refuses, or of the defect, named as a card of its own for the venue's owner. The survey reads source and retained receipts only. It runs nothing in Aion, and it changes no venue code in this card.

**Acceptance:**
- The report names the file and lines where the venue reads the gate's result, and quotes them.
- The report states refuses or defective, with the evidence, and when defective names the new card.

**Files:**
- modify: docs/design/identity/reports/RAUTHYREBASE-002-gate-verdict.md

**Checklist:**
- C21 — The venue's generic gate receipt is surveyed for the same defect (RAUTHYREBASE-002 R5).

**Stories:**
- S7 (Reviewer, Reads the rebase pull request) — As the reviewer deciding whether a fork change may land, I want a gate that was interrupted or cut short to say so by name and never read as a pass, so that I never land a tree nobody finished testing.

## Boundaries

- SHALL NOT omit, skip or reorder any gate leg or test, or relax any existing check.
- SHALL NOT change the cargo target directory: no --target-dir, CARGO_TARGET_DIR or cargo configuration target-dir.
- SHALL NOT touch any live service, any container, network or volume not named for the invocation, Tom's installed Lys, or the fork checkouts of other seats.
- SHALL NOT wait on a clock in any new code or test. The regression's handshake is the fixture's started line and Docker's own stop signal.
- SHALL NOT run anything in Aion or change venue code. The src_gate survey is read-only.
- SHALL NOT add a timeout, deadline, watchdog, unsafe, ignored test or lint suppression.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Implementation follows card_build_v3, src_pr and src_land on the fork with the fork's full gate on Dean, at normal width.
- The detached-client regression runs against real Docker on Dean, and its red on the base head and green on the candidate are both quoted with exit codes.
- A genuine all-legs run of the candidate is quoted with its single matching verdict.
