---
type: brief
id: DIRECTORY-078
cluster: directory
title: The upgrade tests' fake services end with their test on every path, and the folder a killed test leaves is swept and named
---

# DIRECTORY-078: The upgrade tests' fake services end with their test on every path, and the folder a killed test leaves is swept and named

> **Cluster:** directory
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. An exported build may be given LYS_BUILD_COMMIT as exactly 40 lowercase hexadecimal characters: its stamp is `<commit>; stated`, distinguishing the builder's statement from a commit read from Git; invalid values or disagreement with a tracked checkout's HEAD refuse the build, and changing or removing the variable refreshes the stamp. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> - ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
> **Checklist:**
> - C483 — A test's fake services end when its process ends, including SIGKILL. (DIRECTORY-078 R1).
> - C484 — A fake service the fixture cannot stop fails its test by name. (DIRECTORY-078 R1).
> - C485 — The folder a killed test leaves is swept and named the next time a Scratch is made. (DIRECTORY-078 R2).
> **Stories:**
> - S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

## Purpose

On 3 October Buckley found 21 orphaned `cat …/T/.tmpXXXX/work/hold` processes on Dean, each with parent 1 and about 40 hours old, and 23 `.tmp*/work` folders still on disk. They are the upgrade tests' fake services. `Scratch::laid_out` (crates/lys/src/identity/upgrade/scratch_tests.rs:209) makes a temporary folder with `work/hold` as a FIFO. Every fake binary is a shell stub that ends in `exec cat` on that FIFO, and nothing ever writes to it. The only cleanup is `impl Drop for Scratch` (line 372), which stops the units through their pid files and only prints when a stop fails. So a test process that ends without unwinding (a nextest timeout, a signal, an abort) leaves every stub and its folder behind. A refused stop leaves its stub and says so only on stderr. It matches the leaked handles nextest reported on Pikelet's omission scenario on 2 October. Waffles carded it on 3 October 08:58, ordered behind the two heads then in flight, and at 09:00 accepted the direction with one addition: the sweep names what it removed.

## Task

Make the fake services of every test that uses `Scratch` end when that test's process ends, on every path including a panic, a failed assertion and SIGKILL. Make the folder a killed test could not remove be swept, by name, the next time a `Scratch` is made. `Scratch` holds the hold FIFO open for reading and writing for its whole life, so the kernel closes it with the test process however that process ends, and each stub's `cat` reads end of file and exits. The temporary folder gets a fixed prefix and records the test process that owns it. Making a `Scratch` first removes every folder with that prefix whose owner process no longer exists, and prints one line naming each folder it removed. A stop that `Drop` cannot make fails the test instead of printing. The new code goes in its own test-only module beside scratch_tests.rs, which is already 486 lines. In scope: the scratch fixture, its stubs and the new module with its tests. Out of scope: the installer, the runner, adopt::stop itself, the gate scripts, and every non-test path.

## Requirements

### R1: A test's fake services end when its process ends, however it ends

Behavioural. WHILE a `Scratch` exists, THE SYSTEM SHALL hold its `work/hold` FIFO open for both reading and writing in the test process, so that WHEN the test process ends for any reason, including SIGKILL, every stub blocked on that FIFO reads end of file and exits. WHEN a `Scratch` is dropped, THE SYSTEM SHALL stop each unit it started, and a unit that cannot be stopped SHALL fail the test naming the unit and the refusal.

**Acceptance:**
- A child test process makes a `Scratch` with services started and is killed with SIGKILL. The parent then waits on each stub's exit through its exit lock, never by sleeping or polling, and every stub from that run has exited.
- A test that panics after making a `Scratch` leaves no stub from that `Scratch` running.
- A unit whose stop is refused fails the test, naming the unit's binary and the refusal; it is not printed and passed.

**Files:**
- create: crates/lys/src/identity/upgrade/scratch_hold_tests.rs
- modify: crates/lys/src/identity/upgrade/scratch_tests.rs
- modify: crates/lys/src/identity/upgrade.rs

**Checklist:**
- C483 — A test's fake services end when its process ends, including SIGKILL. (DIRECTORY-078 R1).
- C484 — A fake service the fixture cannot stop fails its test by name. (DIRECTORY-078 R1).

**Stories:**
- S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

### R2: The folder a killed test leaves is swept and named the next time a Scratch is made

Behavioural. WHEN a `Scratch` is made, THE SYSTEM SHALL make its temporary folder under one fixed prefix and record in it the process id of the test process that owns it, and SHALL first remove every folder with that prefix whose recorded owner no longer exists, printing one line naming each folder removed. A folder whose owner still runs SHALL be left alone. A folder with that prefix that records no readable owner SHALL be named in a printed line and left in place.

**Acceptance:**
- After the R1 child is killed, making a new `Scratch` removes the child's folder and prints one line naming it.
- A folder whose recorded owner is a running process is not removed and is not named.
- A folder with the prefix and no readable owner is named and left in place.

**Files:**
- create: crates/lys/src/identity/upgrade/scratch_hold_tests.rs
- modify: crates/lys/src/identity/upgrade/scratch_tests.rs

**Checklist:**
- C485 — The folder a killed test leaves is swept and named the next time a Scratch is made. (DIRECTORY-078 R2).

**Stories:**
- S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

## Boundaries

- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait in these tests ends on the exit it waits for.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result.
- SHALL NOT mark any test serial or change nextest configuration to hide a leak.
- SHALL NOT change the installer, the runner, adopt::stop or any non-test code path.
- SHALL NOT remove a folder outside the fixed prefix, or one whose owner still runs.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Built through the board's card chain on its own branch: red first (the R1 kill test fails on eed40419), then green, with fmt, Clippy pedantic in both configurations, nextest on crates/lys, ast-grep and the 500-line file limit.
- A full nextest run of crates/lys on Dean reports no leaked handles, and afterwards `pgrep -f 'work/hold'` finds nothing and no folder with the prefix remains.
