---
type: brief
id: DIRECTORY-093
cluster: directory
title: The Rauthy pin test starts no process that outlives it: its fetch runs no detached maintenance
---

# DIRECTORY-093: The Rauthy pin test starts no process that outlives it: its fetch runs no detached maintenance

> **Cluster:** directory
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. An exported build may be given LYS_BUILD_COMMIT as exactly 40 lowercase hexadecimal characters: its stamp is `<commit>; stated`, distinguishing the builder's statement from a commit read from Git; invalid values or disagreement with a tracked checkout's HEAD refuse the build, and changing or removing the variable refreshes the stamp. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> - ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
> **Checklist:**
> - C731 — The id001 pin test's fetch runs with --no-auto-maintenance, so it starts no detached git maintenance that outlives the test (DIRECTORY-093 R1).
> **Stories:**
> - S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

## Purpose

The Lys battery at 91e34d4f (10 Oct 2026, Mac) printed one LEAK in its identity leg: lys::identity_deploy id001_pin_clone_vendor_rauthy_is_the_pinned_ablative_commit (1.249 s). A LEAK means the test's output stayed open 100 ms after the test process exited. Waffles ruled at 15:16 and 15:23: a fixture row, the test waits on its child, briefed today. The test's `git` helper (crates/lys/tests/identity_deploy.rs:209) runs every git command through Command::output, which waits on that command. But `git -C vendor/rauthy fetch --quiet origin ablative` ends by starting `git maintenance run --auto --quiet --detach`. Measured on the Mac with GIT_TRACE=1 and git 2.47.1: `trace: run_command: git maintenance run --auto --quiet --detach`. The test never waits on that detached process. Five warm runs of the test alone showed no LEAK, so the holder is seen only under load. Either way it is a process the test started and did not wait on, and the right fix is that the test starts none.

## Task

Run the test's fetch with `--no-auto-maintenance`, so the fetch itself starts no detached maintenance. Prove with a trace that the test's git commands start no `git maintenance` and no process outlives them.

## Requirements

### R1: The pin test's fetch starts no detached maintenance

Behavioural. WHEN id001_pin_clone_vendor_rauthy_is_the_pinned_ablative_commit fetches origin's ablative branch into vendor/rauthy, THE SYSTEM SHALL run the fetch with `--no-auto-maintenance`, so that no `git maintenance` process is started. Every other git command the test runs SHALL still run through the waiting `git` helper.

**Acceptance:**
- The fetch call in identity_deploy.rs carries `--no-auto-maintenance` beside `--quiet`.
- A test in identity_deploy.rs runs the same fetch arguments through the helper with GIT_TRACE=1 on the command, and asserts that the trace names no `git maintenance` run_command line.
- `cargo nextest run -p lys --all-features --test identity_deploy -E 'test(id001_pin_clone)'` run 20 times on the Mac under the full Lys battery's load prints no LEAK line.

**Files:**
- modify: crates/lys/tests/identity_deploy.rs

**Checklist:**
- C731 — The id001 pin test's fetch runs with --no-auto-maintenance, so it starts no detached git maintenance that outlives the test (DIRECTORY-093 R1).

**Stories:**
- S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

## Boundaries

- Test code only: no change to vendor/rauthy, .gitmodules, versions.json or any git configuration on the machine.
- SHALL NOT mark the test serial, add a sleep or retry, or change nextest's leak timeout to hide the line.
- The test still fetches from origin and still proves the pinned commit is an ancestor of origin's ablative branch.

## Verification

- cargo clippy -p lys --all-features --test identity_deploy -- -D warnings; the identity leg of the next Lys battery prints no LEAK line for id001.
