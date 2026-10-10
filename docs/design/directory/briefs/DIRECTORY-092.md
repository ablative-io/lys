---
type: brief
id: DIRECTORY-092
cluster: directory
title: An install test's model proxy is stopped with its estate, and the estate a killed test leaves is swept and named
---

# DIRECTORY-092: An install test's model proxy is stopped with its estate, and the estate a killed test leaves is swept and named

> **Cluster:** directory
> **Design anchor:**
> - ADR-114 — An installed identity product names its build and upgrades itself in place, keeping the previous build to return to — Every Lys binary answers --version with the commit and dirty state it was built from, stamped at build time; a build with no commit to read says so in those words, never a made-up value. An exported build may be given LYS_BUILD_COMMIT as exactly 40 lowercase hexadecimal characters: its stamp is `<commit>; stated`, distinguishing the builder's statement from a commit read from Git; invalid values or disagreement with a tracked checkout's HEAD refuse the build, and changing or removing the variable refreshes the stamp. `lys identity upgrade` takes a folder of newly built binaries (and optionally a screens package), checks each one's --version, stops the service and the broker by their exit events, swaps the binaries by rename keeping the previous set beside them, starts the new ones and waits for them ready; if any new binary fails to start or become ready, it puts the previous set back, starts it, and fails naming what broke. Data, credentials, configuration and the compose services are never touched by an upgrade. Rejected: making install restart what differs, which would mix a first install's promises with an upgrade's risks; asking people to copy binaries by hand.
> - ADR-054 — lys recognises test code by structure the file carries, for ast-grep and clippy alike — Test code is a file whose first line is #![cfg(test)], a #[cfg(test)] mod body, a #[test] fn body, or a path under a crate's tests/ directory; every file only ever compiled as a test carries the first-line marker, and clippy.toml's allow-unwrap-in-tests, allow-expect-in-tests and allow-panic-in-tests replace the per-module #![allow]. Rejected: an exemption list of file names in the rule, and rewriting the test helpers' calls.
> **Checklist:**
> - C729 — Estate's cleanup stops the model proxy (run/proxy.pid) with the runner, directory service and secrets broker on close and on Drop, and a proxy it cannot stop fails close naming proxy.pid (DIRECTORY-092 R1).
> - C730 — Each install-test estate lives under the lys-estate- prefix with its owner and compose project recorded; making an estate sweeps and names every prefixed folder whose owner is gone, stopping its services through their exit locks only (DIRECTORY-092 R2).
> **Stories:**
> - S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

## Purpose

On 10 October 2026 Percy found 10 orphaned `lys proxy serve` processes on Tom's Mac, each with parent 1, up to 20 hours old, run from target/debug/lys (unlinked by later builds) with --home under /var/folders/…/T/.tmp*/data/proxy/home. Waffles carded it at 13:24. They are the model proxies the install tests start. `lys identity install` starts the proxy detached as its own Unit (crates/lys/src/identity/install/proxy.rs, `unit` and `start`; pid file run/proxy.pid, exit lock run/proxy.exit), beside the runner, the directory service and the secrets broker. The install tests' fixture is `Estate` (crates/lys/tests/identity_install/estate.rs). Its `cleanup` stops only `["runner.pid", "identity.pid", "secrets.pid"]` (line 110), on both of its paths: `close` and the unclosed `Drop`. It never stops proxy.pid, then the TempDir is removed, so the proxy outlives its estate on every path, passing or failing. The one orphan still running at 13:3x confirms that path: its root folder is gone, and only a dropped estate removes it. The upgrade fixture `Scratch` already stops run/proxy.pid in its Drop (crates/lys/src/identity/upgrade/scratch_tests.rs:390), and `lys identity stop` stops the proxy (identity/stop.rs:158), so the gap is the install fixture alone. A test nextest kills at terminate-after (300 s since 6edebc51) never drops its estate, so it leaves every one of the four services and its folder. That path follows DIRECTORY-078's pattern for `Scratch`: a fixed prefix, an owner recorded in the folder, and a sweep when the next estate is made.

## Task

Name the fixture and fix it at its cause. In `Estate::cleanup`, stop the model proxy through run/proxy.pid and its exit lock alongside the runner, the directory service and the secrets broker, on both `close` and `Drop`. Make each estate's folder under a fixed prefix and record its owning test process and compose project in it. Making an estate first sweeps every prefixed folder whose owner no longer runs: it stops that folder's four services through their exit locks, takes its compose project down, removes the folder, and prints one line naming each folder it removed. Extend the cleanup tests to prove both paths.

## Requirements

### R1: An estate's cleanup stops its model proxy with the other services

Behavioural. WHEN an `Estate` is closed or dropped unclosed, THE SYSTEM SHALL stop every detached service the install started under its root: the runner, the directory service, the secrets broker and the model proxy, read from run/runner.pid, run/identity.pid, run/secrets.pid and run/proxy.pid. Each is stopped as `stop_service` stops one today: a held exit lock means it runs and it is killed, then the cleanup waits for the lock. A service whose exit lock is free is not signalled. A proxy that cannot be stopped fails `close` naming run/proxy.pid, as the other three do.

**Acceptance:**
- an_install_failure_leaves_no_owned_service_running starts its blocked fixture for each of `runner`, `identity`, `secrets` and `proxy`, writes run/<name>.pid for each, and after the estate is dropped unclosed finds every one of the four exit locks free and each fixture ended by signal 9.
- A test that writes run/proxy.pid naming an unparseable pid gets an error from `Estate::close` whose text names proxy.pid, as a_successful_test_reports_teardown_failure does for identity.pid.
- a_first_install_and_a_sign_in_never_name_the_issuer opens run/proxy.exit before it closes its estate, and after `close` returns Ok takes that lock exclusively without blocking (the proxy is gone).

**Files:**
- modify: crates/lys/tests/identity_install/estate.rs
- modify: crates/lys/tests/identity_install/cleanup_tests.rs
- modify: crates/lys/tests/identity_install.rs

**Checklist:**
- C729 — Estate's cleanup stops the model proxy (run/proxy.pid) with the runner, directory service and secrets broker on close and on Drop, and a proxy it cannot stop fails close naming proxy.pid (DIRECTORY-092 R1).

**Stories:**
- S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

### R2: The estate a killed test leaves is swept and named when the next estate is made

Behavioural. WHEN an `Estate` is made, THE SYSTEM SHALL make its root folder in the system temporary folder under the fixed prefix `lys-estate-`, and record in it the process id of the test process that owns it and its compose project name. It SHALL first sweep every folder with that prefix whose recorded owner no longer exists: it stops the four services named in R1 through their pid files and exit locks, takes the recorded compose project down with its volumes when the folder holds state/compose.env, removes the folder, and prints one line naming the folder and each service it stopped. A folder whose owner still runs SHALL be left alone. A prefixed folder with no readable owner record SHALL be named in a printed line and left in place. A service whose exit lock is free is never signalled, so a pid reused by another process is never killed.

**Acceptance:**
- A test makes a `lys-estate-` folder recording the pid of a child it has already reaped, with a blocked fixture holding run/proxy.exit and its pid in run/proxy.pid. Making an estate ends that fixture by signal 9, removes the folder, and prints one line naming the folder and `proxy`.
- A `lys-estate-` folder recording the test's own pid is left in place with its fixture still holding its exit lock after an estate is made.
- A `lys-estate-` folder with no owner record is left in place and named in one printed line.
- A `lys-estate-` folder whose owner is gone and whose run/proxy.pid names a pid but whose run/proxy.exit lock is free is removed without any process being signalled: a child the test keeps running under that pid is still running afterwards.

**Files:**
- create: crates/lys/tests/identity_install/estate_sweep_tests.rs
- modify: crates/lys/tests/identity_install/estate.rs
- modify: crates/lys/tests/identity_install/cleanup_tests.rs
- modify: crates/lys/tests/identity_install.rs

**Checklist:**
- C730 — Each install-test estate lives under the lys-estate- prefix with its owner and compose project recorded; making an estate sweeps and names every prefixed folder whose owner is gone, stopping its services through their exit locks only (DIRECTORY-092 R2).

**Stories:**
- S260 (Person running Lys's tests on a shared machine, Runs the test suites on a laptop other seats also build on) — As a person running Lys's tests on a shared machine, I want every process and folder a test makes to be gone when it ends, however it ends, so a killed or failing test never leaves work running for days.

## Boundaries

- Test code only: no change to the installer, `proxy::start`, `services::start_detached`, `lys proxy serve` or any non-test path. The proxy is detached by design, as the runner is.
- SHALL NOT poll, sleep, add a timer, timeout, deadline, watchdog or background loop; every wait ends on the exit lock it waits for.
- SHALL NOT add unsafe, an ignored test, an allow attribute, an underscore rename or a discarded result, and SHALL NOT mark any test serial or change nextest configuration to hide a leak.
- SHALL NOT remove a folder outside the `lys-estate-` prefix or one whose owner still runs, and SHALL NOT signal a process whose exit lock is free.
- The 10 orphans Percy found are stopped by hand on Waffles' word (13:24); this brief prevents new ones and does not sweep folders made before the prefix existed.

## Verification

- cargo nextest run -p lys --test identity_install on the Mac, with Docker running so a_first_install_and_a_sign_in_never_name_the_issuer runs, then `ps -axo ppid,args` shows no `target/debug/lys proxy serve` with parent 1.
- cargo clippy -p lys --all-targets -- -D warnings, and the full Lys battery at the landing head; after the battery, the same ps line names no proxy left by it.
